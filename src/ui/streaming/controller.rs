//! 流控制器
//!
//! 管理流的生命周期和提交策略

use std::time::{Duration, Instant};

use super::state::StreamState;

/// 流控制器配置
pub struct StreamConfig {
    /// 目标延迟（毫秒）
    pub target_latency_ms: u64,
    /// 每帧最大提交行数
    pub max_lines_per_frame: usize,
    ///  backlog 补偿阈值
    pub backlog_threshold: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            target_latency_ms: 100,
            max_lines_per_frame: 5,
            backlog_threshold: 20,
        }
    }
}

/// 流控制器
pub struct StreamController {
    /// 流状态
    pub state: StreamState,
    /// 配置
    pub config: StreamConfig,
    /// 流开始时间
    start_time: Option<Instant>,
    /// 上次提交时间
    last_commit_time: Option<Instant>,
    /// 流是否结束
    pub is_complete: bool,
}

impl StreamController {
    pub fn new(width: usize) -> Self {
        Self {
            state: StreamState::new(width),
            config: StreamConfig::default(),
            start_time: None,
            last_commit_time: None,
            is_complete: false,
        }
    }

    /// 开始新流
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.is_complete = false;
    }

    /// 推送 delta
    pub fn push_delta(&mut self, delta: &str) {
        if self.start_time.is_none() {
            self.start();
        }
        self.state.push_delta(delta);
    }

    /// 提交行（根据策略）
    pub fn commit(&mut self) -> Vec<ratatui::text::Line<'static>> {
        self.state.commit_lines();
        self.last_commit_time = Some(Instant::now());

        // 根据队列压力决定提交多少行
        let max_lines = if self.state.is_under_pressure() {
            // backlog 模式下提交更多
            self.config.max_lines_per_frame * 2
        } else {
            self.config.max_lines_per_frame
        };

        self.state.drain_lines(max_lines)
    }

    /// 完成流
    pub fn finish(&mut self) {
        self.state.commit_all();
        self.is_complete = true;
    }

    /// 获取经过时间
    pub fn elapsed(&self) -> Duration {
        self.start_time
            .map(|st| st.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    /// 重置控制器
    pub fn clear(&mut self) {
        self.state.clear();
        self.start_time = None;
        self.last_commit_time = None;
        self.is_complete = false;
    }

    /// 获取流状态摘要
    pub fn status(&self) -> StreamStatus {
        StreamStatus {
            is_active: self.start_time.is_some() && !self.is_complete,
            is_complete: self.is_complete,
            queue_len: self.state.queue_len(),
            uncommitted_chars: self.state.collector.uncommitted_chars(),
            elapsed: self.elapsed(),
        }
    }
}

/// 流状态摘要
pub struct StreamStatus {
    pub is_active: bool,
    pub is_complete: bool,
    pub queue_len: usize,
    pub uncommitted_chars: usize,
    pub elapsed: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_lifecycle() {
        let mut controller = StreamController::new(80);

        assert!(!controller.status().is_active);

        controller.push_delta("Hello, ");
        controller.push_delta("World!\n");

        assert!(controller.status().is_active);

        let lines = controller.commit();
        assert_eq!(lines.len(), 1);

        controller.finish();
        assert!(controller.status().is_complete);
    }

    #[test]
    fn test_clear() {
        let mut controller = StreamController::new(80);
        controller.push_delta("Text\n");
        controller.clear();

        assert!(!controller.status().is_active);
        assert_eq!(controller.state.queue_len(), 0);
    }
}
