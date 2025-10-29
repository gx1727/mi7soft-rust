use std::{
    os::unix::io::{AsRawFd, RawFd},
    sync::atomic::{AtomicU32, Ordering},
    ptr,
};
use tokio::io::unix::AsyncFd;
use libc::{syscall, SYS_futex, timespec, c_int};

/// 调用 Linux futex 原语
#[inline]
fn futex(addr: *const u32, op: c_int, val: u32, timeout: *const timespec) -> i32 {
    unsafe { syscall(SYS_futex, addr, op, val, timeout, ptr::null::<u32>(), 0) as i32 }
}

fn futex_wait(addr: &AtomicU32, expected: u32) {
    unsafe {
        futex(
            addr as *const AtomicU32 as *const u32,
            libc::FUTEX_WAIT,
            expected,
            ptr::null::<timespec>(),
        );
    }
}

fn futex_wake(addr: &AtomicU32, n: u32) {
    unsafe {
        futex(
            addr as *const AtomicU32 as *const u32,
            libc::FUTEX_WAKE,
            n,
            ptr::null::<timespec>(),
        );
    }
}

fn create_eventfd() -> RawFd {
    unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) }
}

#[derive(Debug)]
pub struct AsyncFutex {
    pub addr: *mut AtomicU32, // 位于共享内存中的futex变量
    event_fd: AsyncFd<RawFd>, // 异步通知
}

impl AsyncFutex {
    /// 创建一个异步 futex
    pub fn new(addr: *mut AtomicU32) -> std::io::Result<Self> {
        let fd = create_eventfd();
        let async_fd = AsyncFd::new(fd)?;
        Ok(Self { addr, event_fd: async_fd })
    }

    /// 异步等待 futex 值变动
    pub async fn wait_async(&self, expected: u32) -> std::io::Result<()> {
        // 等待 futex
        let addr = unsafe { &*self.addr };

        if addr.load(Ordering::SeqCst) == expected {
            futex_wait(addr, expected);
        }

        // 异步监听 eventfd 唤醒
        let mut guard = self.event_fd.readable().await?;
        guard.clear_ready();
        Ok(())
    }

    /// 唤醒一个或多个等待者
    pub fn wake(&self, n: u32) {
        let addr = unsafe { &*self.addr };
        futex_wake(addr, n);
        unsafe {
            let val: u64 = 1;
            libc::write(self.event_fd.as_raw_fd(), &val as *const u64 as *const _, 8);
        }
    }
}

impl Drop for AsyncFutex {
    fn drop(&mut self) {
        unsafe { libc::close(self.event_fd.as_raw_fd()); }
    }
}
