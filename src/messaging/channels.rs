//! 消息通道管理
//!
//! 统一内部通信通道，基于 tokio mpsc 和 broadcast 通道。

use tokio::sync::{mpsc, broadcast};
use super::messages::AppMessage;

/// 通道配置
pub struct ChannelConfig {
    pub command_buffer_size: usize,
    pub event_buffer_size: usize,
    pub stream_buffer_size: usize,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            command_buffer_size: 100,
            event_buffer_size: 256,
            stream_buffer_size: 512,
        }
    }
}

/// 通道管理器 - 统一创建和管理所有内部通道
pub struct ChannelManager {
    config: ChannelConfig,
}

impl ChannelManager {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            config: ChannelConfig {
                command_buffer_size: buffer_size,
                event_buffer_size: buffer_size * 2,
                stream_buffer_size: buffer_size * 4,
            },
        }
    }

    pub fn with_config(config: ChannelConfig) -> Self {
        Self { config }
    }

    /// 创建命令通道（mpsc，单向流）
    pub fn create_command_channel(&self) -> (mpsc::UnboundedSender<AppMessage>, mpsc::UnboundedReceiver<AppMessage>) {
        mpsc::unbounded_channel()
    }

    /// 创建有界命令通道（带背压）
    pub fn create_bounded_command_channel(&self) -> (mpsc::Sender<AppMessage>, mpsc::Receiver<AppMessage>) {
        mpsc::channel(self.config.command_buffer_size)
    }

    /// 创建事件广播通道（多订阅者）
    pub fn create_event_channel(&self) -> (broadcast::Sender<AppMessage>, broadcast::Receiver<AppMessage>) {
        broadcast::channel(self.config.event_buffer_size)
    }

    /// 创建流式数据通道（Token 流）
    pub fn create_stream_channel(&self) -> (broadcast::Sender<String>, broadcast::Receiver<String>) {
        broadcast::channel(self.config.stream_buffer_size)
    }

    /// 创建一对一直通通道
    pub fn create_direct_channel<T: Send + 'static>(&self, buffer_size: usize) -> (mpsc::Sender<T>, mpsc::Receiver<T>) {
        mpsc::channel(buffer_size)
    }

    /// 创建无阻塞直通通道
    pub fn create_unbounded_direct_channel<T: Send + 'static>(&self) -> (mpsc::UnboundedSender<T>, mpsc::UnboundedReceiver<T>) {
        mpsc::unbounded_channel()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_manager_creation() {
        let manager = ChannelManager::new(100);
        let (tx, rx) = manager.create_command_channel();
        assert!(tx.send(AppMessage::Cancel).is_ok());
        drop(rx);
    }

    #[tokio::test]
    async fn test_bounded_channel_backpressure() {
        let manager = ChannelManager::with_config(ChannelConfig {
            command_buffer_size: 2,
            event_buffer_size: 100,
            stream_buffer_size: 100,
        });

        let (tx, _rx) = manager.create_bounded_command_channel();

        // 发送消息直到缓冲区满
        tx.send(AppMessage::Cancel).await.unwrap();
        tx.send(AppMessage::Cancel).await.unwrap();

        // 第三次应该阻塞（但这里我们只测试能发送）
        let send_result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            tx.send(AppMessage::Cancel)
        ).await;

        assert!(send_result.is_err()); // 超时，说明缓冲区满
    }

    #[test]
    fn test_broadcast_channel() {
        let manager = ChannelManager::new(100);
        let (tx, _rx1) = manager.create_event_channel();
        let _rx2 = tx.subscribe();
        assert_eq!(tx.receiver_count(), 2);
    }
}
