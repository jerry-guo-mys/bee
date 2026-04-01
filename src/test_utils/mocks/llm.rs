//! Mock LLM 客户端

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::Stream;

use crate::domain::memory::Message;
use crate::llm::{LlmClient, LlmError};

/// Mock LLM 客户端，用于测试
pub struct MockLlmClient {
    responses: Arc<Mutex<Vec<String>>>,
    call_count: Arc<Mutex<usize>>,
}

impl Default for MockLlmClient {
    fn default() -> Self {
        Self {
            responses: Arc::new(Mutex::new(vec!["Mock response".to_string()])),
            call_count: Arc::new(Mutex::new(0)),
        }
    }
}

impl MockLlmClient {
    /// 设置预设响应列表
    pub fn set_responses(&mut self, responses: Vec<String>) {
        let mut inner = self.responses.lock().unwrap();
        *inner = responses;
    }

    /// 添加响应
    pub fn add_response(&mut self, response: String) {
        let mut inner = self.responses.lock().unwrap();
        inner.push(response);
    }

    /// 获取调用次数
    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }

    /// 重置调用计数
    pub fn reset_call_count(&self) {
        *self.call_count.lock().unwrap() = 0;
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(&self, _messages: &[Message]) -> Result<String, LlmError> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;
        let idx = *count;

        let responses = self.responses.lock().unwrap();
        let response = responses
            .get(idx - 1)
            .or_else(|| responses.first())
            .cloned()
            .unwrap_or_else(|| "Mock response".to_string());

        Ok(response)
    }

    async fn complete_stream(
        &self,
        messages: &[Message],
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<String, LlmError>> + Send>>, LlmError>
    {
        let response = self.complete(messages).await?;
        Ok(Box::pin(futures_util::stream::iter(vec![Ok(response)])))
    }

    fn token_usage(&self) -> (u64, u64, u64) {
        (0, 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Message;

    #[tokio::test]
    async fn test_mock_llm_returns_response() {
        let mock = MockLlmClient::default();
        let messages = vec![Message::user("test")];

        let response = mock.complete(&messages).await.unwrap();
        assert_eq!(response, "Mock response");
    }

    #[tokio::test]
    async fn test_mock_llm_call_count() {
        let mock = MockLlmClient::default();
        let messages = vec![Message::user("test")];

        assert_eq!(mock.call_count(), 0);

        let _ = mock.complete(&messages).await;
        assert_eq!(mock.call_count(), 1);

        let _ = mock.complete(&messages).await;
        assert_eq!(mock.call_count(), 2);
    }

    #[tokio::test]
    async fn test_mock_llm_multiple_responses() {
        let mut mock = MockLlmClient::default();
        mock.set_responses(vec!["first".to_string(), "second".to_string()]);

        let messages = vec![Message::user("test")];

        assert_eq!(mock.complete(&messages).await.unwrap(), "first");
        assert_eq!(mock.complete(&messages).await.unwrap(), "second");
        assert_eq!(mock.complete(&messages).await.unwrap(), "first"); // 循环
    }
}
