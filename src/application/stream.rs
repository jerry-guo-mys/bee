//! 流式响应处理

use tokio::sync::broadcast;

/// 流式响应处理器
pub struct StreamHandler {
    sender: broadcast::Sender<String>,
}

impl StreamHandler {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, _receiver) = broadcast::channel(buffer_size);
        Self { sender }
    }

    /// 发送 Token
    pub fn send_token(&self, token: &str) {
        let _ = self.sender.send(token.to_string());
    }

    /// 获取接收器
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }

    /// 发送完成信号
    pub fn complete(&self) {
        // 发送空字符串表示完成
        let _ = self.sender.send(String::new());
    }
}

impl Default for StreamHandler {
    fn default() -> Self {
        Self::new(16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_handler() {
        let handler = StreamHandler::default();
        let mut rx = handler.subscribe();

        handler.send_token("Hello");
        handler.send_token(" ");
        handler.send_token("World");
        handler.complete();

        assert_eq!(rx.try_recv().unwrap(), "Hello");
        assert_eq!(rx.try_recv().unwrap(), " ");
        assert_eq!(rx.try_recv().unwrap(), "World");
    }
}
