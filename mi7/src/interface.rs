use crate::pipe::{DynamicPipe, PipeFactory};
use crate::shared_slot::SlotState;
use crate::{Message, Version, config};
use anyhow::{Error, Result};
use async_channel::{Receiver, Sender, bounded};
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info};

pub trait InterfaceApi: Send + Sync {
    fn handle(&self, message: Message) -> Result<Message>;
}

pub struct Interface {
    version: Version,
    pipe: Arc<Box<dyn DynamicPipe>>,
    tx: Sender<usize>,
    rx: Receiver<usize>,
}

impl Interface {
    pub fn new(version: &str) -> std::result::Result<Interface, Error> {
        info!("启动 Worker Interface");

        // 使用新的通用配置读取方式获取配置信息
        let interface_name = config::string("worker", "interface_name");
        let interface_type = config::string("worker", "interface_type");

        // 创建一个生产者-多个消费者的消息队列
        let (tx, rx) = bounded::<usize>(100); // 创建一个缓冲大小为 100 的通道

        // 创建 pipe
        let pipe = match PipeFactory::connect(&interface_type, &interface_name, true) {
            Ok(pipe) => {
                info!(
                    "配置信息: 队列名称={}, 槽位数={} 槽位大小={}",
                    interface_name,
                    pipe.capacity(),
                    pipe.slot_size()
                );
                Arc::new(pipe)
            }
            Err(e) => {
                error!("连接管道失败: {:?}", e);
                return Err(e);
            }
        };

        let version = Version::from_str(&version).unwrap();
        Ok(Interface {
            version,
            pipe,
            tx,
            rx,
        })
    }

    // 从  async_channel 获取任务
    // 处理
    // 返回
    // , api: Arc<Box<dyn InterfaceApi>>
    pub fn load(&self, consumer_count: i32) -> Result<()> {
        info!("启动 Worker Interface");
        for i in 0..consumer_count {
            let work_rx = self.rx.clone();
            let pipe_for_work = Arc::clone(&self.pipe);

            tokio::spawn(async move {
                loop {
                    // info!("消费者 {} 开始等待接收消息...", i);
                    match work_rx.recv().await {
                        Ok(slot_index) => {
                            info!(
                                "消费者 {} 接收到消息: {} (时间戳: {:?})",
                                i,
                                slot_index,
                                std::time::SystemTime::now()
                            );

                            if let Err(_) =
                                pipe_for_work.set_slot_state(slot_index, SlotState::INPROGRESS)
                            {
                                error!("Listener {} 设置槽位状态失败", slot_index);
                                continue;
                            }

                            // // 接收消息
                            let message = match pipe_for_work.receive(slot_index) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    error!("Listener {} 读取消息失败 {}", slot_index, e);
                                    continue;
                                }
                            };
                            info!(
                                "Listener {} 收到任务 flag={}: {}",
                                slot_index,
                                message.flag,
                                String::from_utf8_lossy(&message.data)
                            );

                            // 这里可以添加实际的消息处理逻辑
                            // 比如调用 router 处理消息
                        }
                        Err(e) => {
                            error!("消费者 {} 接收消息失败: {:?}", i, e);
                            break; // 通道关闭时退出循环
                        }
                    }
                    // info!("消费者 {} 开始等待接收消息... end", i);
                }
            });
        }
        Ok(())
    }

    // 启动
    // 从  从共享内存中 获取任务 ，发送到 async_channel
    pub async fn start(&self) -> Result<()> {
        let work_tx = self.tx.clone();
        let pipe_for_listener = Arc::clone(&self.pipe);
        tokio::spawn(async move {
            loop {
                // 尝试从共享内存中fetch任务的slot_index
                let slot_index = match pipe_for_listener.fetch() {
                    Ok(index) => index,
                    Err(_) => {
                        // fetch中已有 短暂等待
                        continue;
                    }
                };
                info!("Listener 获取任务 {} ", slot_index); ////// 

                // 将获取的 slot_index 发送到 async_channel
                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    work_tx.send(slot_index),
                )
                .await
                {
                    Ok(Ok(())) => {
                        // 发送成功 主动让出 CPU 时间，让消费者有机会处理消息
                        tokio::task::yield_now().await;
                    }
                    Ok(Err(e)) => {
                        // 通道发送错误
                        eprintln!("Failed to send slot index: {:?}", e);
                    }
                    Err(_) => {
                        // 超时
                        eprintln!("Timeout while sending slot index");
                    }
                }
            }
        });

        Ok(())
    }
}
