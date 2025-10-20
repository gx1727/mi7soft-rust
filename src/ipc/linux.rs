//! Linux 平台的共享内存和锁实现（基于 shm 和 pthread_mutex）

use super::{IpcError, IpcMutex, RingBuffer, SharedMemory};
use crate::common::{MUTEX_NAME, SHARED_MEM_NAME};
use libc::{
    c_void, ftruncate, shm_open, O_CREAT, O_RDWR, PROT_READ, PROT_WRITE, S_IRUSR, S_IWUSR,
    PTHREAD_MUTEX_INITIALIZER, PTHREAD_PROCESS_SHARED,
};
use std::ffi::CString;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, Ordering};

// 跨进程互斥锁（基于 pthread_mutex）
// 共享内存中的锁结构
#[repr(C)]
struct SharedMutexData {
    mutex: libc::pthread_mutex_t,
    initialized: AtomicU32, // 0=未初始化, 1=初始化中, 2=已初始化
}

pub struct LinuxIpcMutex {
    shm_fd: i32,
    shared_data: *mut SharedMutexData,
}

impl LinuxIpcMutex {
    // 创建或打开pthread锁
    pub fn new() -> Result<Self, IpcError> {
        let cname = CString::new(MUTEX_NAME).map_err(|e| IpcError::LockError(e.to_string()))?;
        
        // 尝试打开已存在的共享内存
        let existing_fd = unsafe { shm_open(cname.as_ptr(), O_RDWR, S_IRUSR | S_IWUSR) };
        let (shm_fd, is_new) = if existing_fd < 0 {
            // 不存在则创建
            let fd = unsafe {
                shm_open(
                    cname.as_ptr(),
                    O_CREAT | O_RDWR,
                    S_IRUSR | S_IWUSR,
                )
            };
            if fd < 0 {
                return Err(IpcError::LockError(format!(
                    "shm_open 失败: {}",
                    std::io::Error::last_os_error()
                )));
            }
            // 设置共享内存大小
            if unsafe { ftruncate(fd, std::mem::size_of::<SharedMutexData>() as i64) } < 0 {
                return Err(IpcError::LockError(format!(
                    "ftruncate 失败: {}",
                    std::io::Error::last_os_error()
                )));
            }
            (fd, true)
        } else {
            (existing_fd, false)
        };

        // 映射共享内存到进程地址空间
        let shared_data = unsafe {
            libc::mmap(
                null_mut(),
                std::mem::size_of::<SharedMutexData>(),
                PROT_READ | PROT_WRITE,
                libc::MAP_SHARED,
                shm_fd,
                0,
            )
        } as *mut SharedMutexData;
        
        if shared_data == libc::MAP_FAILED as *mut SharedMutexData {
            return Err(IpcError::LockError(format!(
                "mmap 失败: {}",
                std::io::Error::last_os_error()
            )));
        }

        // 使用原子操作确保锁只初始化一次
        if is_new {
            unsafe {
                // 初始化原子标志
                (*shared_data).initialized = AtomicU32::new(0);
                // 使用静态初始化器初始化锁
                (*shared_data).mutex = PTHREAD_MUTEX_INITIALIZER;
            }
        }

        // 等待锁初始化完成或尝试初始化
        unsafe {
            let initialized = &(*shared_data).initialized;
            loop {
                match initialized.compare_exchange_weak(0, 1, Ordering::AcqRel, Ordering::Acquire) {
                    Ok(_) => {
                        // 我们获得了初始化权限
                        let mut attr: libc::pthread_mutexattr_t = std::mem::zeroed();
                        if libc::pthread_mutexattr_init(&mut attr) != 0 {
                            return Err(IpcError::LockError("初始化锁属性失败".to_string()));
                        }
                        if libc::pthread_mutexattr_setpshared(&mut attr, PTHREAD_PROCESS_SHARED) != 0 {
                            return Err(IpcError::LockError("设置锁为进程间共享失败".to_string()));
                        }
                        if libc::pthread_mutex_init(&mut (*shared_data).mutex, &attr) != 0 {
                            return Err(IpcError::LockError("初始化锁失败".to_string()));
                        }
                        libc::pthread_mutexattr_destroy(&mut attr);
                        
                        // 标记初始化完成
                        initialized.store(2, Ordering::Release);
                        break;
                    }
                    Err(current) => {
                        if current == 2 {
                            // 已经初始化完成
                            break;
                        }
                        // 正在初始化中，等待一下
                        std::thread::yield_now();
                    }
                }
            }
        }

        Ok(Self { shm_fd, shared_data })
    }
}

impl IpcMutex for LinuxIpcMutex {
    fn lock(&self) -> Result<(), String> {
        unsafe {
            if libc::pthread_mutex_lock(&mut (*self.shared_data).mutex) != 0 {
                Err(format!("pthread锁获取失败: {}", std::io::Error::last_os_error()))
            } else {
                Ok(())
            }
        }
    }

    fn unlock(&self) -> Result<(), String> {
        unsafe {
            if libc::pthread_mutex_unlock(&mut (*self.shared_data).mutex) != 0 {
                Err(format!("pthread锁释放失败: {}", std::io::Error::last_os_error()))
            } else {
                Ok(())
            }
        }
    }
}

impl Drop for LinuxIpcMutex {
    fn drop(&mut self) {
        unsafe {
            // 解除内存映射
            if !self.shared_data.is_null() {
                libc::munmap(
                    self.shared_data as *mut c_void,
                    std::mem::size_of::<SharedMutexData>(),
                );
            }
            // 关闭共享内存文件描述符
            if self.shm_fd >= 0 {
                libc::close(self.shm_fd);
            }
        }
    }
}

// 共享内存（基于 System V 共享内存）
pub struct LinuxSharedMemory {
    shm_fd: i32,
    buffer: *mut RingBuffer,
}

impl LinuxSharedMemory {
    // 创建或打开共享内存
    pub fn new() -> Result<Self, IpcError> {
        let cname = CString::new(SHARED_MEM_NAME).map_err(|e| IpcError::MemoryError(e.to_string()))?;
        
        // 尝试打开已存在的共享内存
        let existing_fd = unsafe { shm_open(cname.as_ptr(), O_RDWR, S_IRUSR | S_IWUSR) };
        let (shm_fd, is_new) = if existing_fd < 0 {
            // 不存在则创建
            let fd = unsafe {
                shm_open(
                    cname.as_ptr(),
                    O_CREAT | O_RDWR,
                    S_IRUSR | S_IWUSR,
                )
            };
            if fd < 0 {
                return Err(IpcError::MemoryError(format!(
                    "shm_open 失败: {}",
                    std::io::Error::last_os_error()
                )));
            }
            // 设置共享内存大小（存放 RingBuffer）
            let size = std::mem::size_of::<RingBuffer>();
            if unsafe { ftruncate(fd, size as i64) } < 0 {
                return Err(IpcError::MemoryError(format!(
                    "ftruncate 失败: {}",
                    std::io::Error::last_os_error()
                )));
            }
            (fd, true)
        } else {
            (existing_fd, false)
        };

        // 映射到内存
        let buffer_ptr = unsafe {
            libc::mmap(
                null_mut(),
                std::mem::size_of::<RingBuffer>(),
                PROT_READ | PROT_WRITE,
                libc::MAP_SHARED,
                shm_fd,
                0,
            )
        };
        if buffer_ptr == libc::MAP_FAILED {
            return Err(IpcError::MemoryError(format!(
                "mmap 失败: {}",
                std::io::Error::last_os_error()
            )));
        }

        // 初始化缓冲区（仅第一次创建时）
        if is_new {
            unsafe {
                std::ptr::write(buffer_ptr.cast::<RingBuffer>(), RingBuffer::new());
            }
        }

        Ok(Self {
            shm_fd,
            buffer: buffer_ptr as *mut RingBuffer,
        })
    }
}

impl SharedMemory for LinuxSharedMemory {
    fn get_buffer(&self) -> &RingBuffer {
        unsafe { &*self.buffer }
    }

    fn get_buffer_mut(&mut self) -> &mut RingBuffer {
        unsafe { &mut *self.buffer }
    }
}

impl Drop for LinuxSharedMemory {
    fn drop(&mut self) {
        // 解除内存映射
        unsafe {
            libc::munmap(
                self.buffer as *mut c_void,
                std::mem::size_of::<RingBuffer>(),
            );
            libc::close(self.shm_fd);
        }
    }
}