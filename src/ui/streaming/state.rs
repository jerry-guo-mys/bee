//! 流状态管理
//!
//! 管理流式输出的队列和提交策略

use std::collections::VecDeque;
use std::time::Instant;

use ratatui::text::Line;

use super::collector::MarkdownStreamCollector;

/// 队列中的行（带时间戳用于年龄策略）
pub struct QueuedLine {
    pub line: Line<'static>,
    pub enqueued_at: Instant,
}

/// 流状态
pub struct StreamState {
    /// Markdown 收集器
    pub collector: MarkdownStreamCollector,
    /// 队列中的行
    pub queued_lines: VecDeque<QueuedLine>,
    /// 是否已经见过 delta（用于初始化）
    pub has_seen_delta: bool,
    /// 队列压力阈值（超过此值触发补偿）
    pub queue_pressure_threshold: usize,
}

impl StreamState {
    pub fn new(width: usize) -> Self {
        Self {
            collector: MarkdownStreamCollector::new(width),
            queued_lines: VecDeque::new(),
            has_seen_delta: false,
            queue_pressure_threshold: 10,
        }
    }

    /// 追加新的 delta
    pub fn push_delta(&mut self, delta: &str) {
        self.has_seen_delta = true;
        self.collector.push(delta);
    }

    /// 提交完整的行到队列
    pub fn commit_lines(&mut self) {
        let lines = self.collector.commit_complete_lines();
        for line in lines {
            self.queued_lines.push_back(QueuedLine {
                line,
                enqueued_at: Instant::now(),
            });
        }
    }

    /// 从队列中取出行用于渲染
    pub fn drain_lines(&mut self, max_lines: usize) -> Vec<Line<'static>> {
        let count = self.queued_lines.len().min(max_lines);
        self.queued_lines.drain(..count).map(|ql| ql.line).collect()
    }

    /// 检查队列压力是否过高
    pub fn is_under_pressure(&self) -> bool {
        self.queued_lines.len() > self.queue_pressure_threshold
    }

    /// 获取队列中的行数
    pub fn queue_len(&self) -> usize {
        self.queued_lines.len()
    }

    /// 强制提交所有剩余内容
    pub fn commit_all(&mut self) {
        let lines = self.collector.commit_all();
        for line in lines {
            self.queued_lines.push_back(QueuedLine {
                line,
                enqueued_at: Instant::now(),
            });
        }
    }

    /// 重置流状态
    pub fn clear(&mut self) {
        self.collector.clear();
        self.queued_lines.clear();
        self.has_seen_delta = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_state_push_and_drain() {
        let mut state = StreamState::new(80);
        state.push_delta("Line 1\n");
        state.push_delta("Line 2\n");

        state.commit_lines();
        assert_eq!(state.queue_len(), 2);

        let lines = state.drain_lines(1);
        assert_eq!(lines.len(), 1);
        assert_eq!(state.queue_len(), 1);
    }

    #[test]
    fn test_queue_pressure() {
        let mut state = StreamState::new(80);
        state.queue_pressure_threshold = 3;

        for i in 0..5 {
            state.queued_lines.push_back(QueuedLine {
                line: Line::from(format!("Line {}", i)),
                enqueued_at: Instant::now(),
            });
        }

        assert!(state.is_under_pressure());
    }

    #[test]
    fn test_commit_all() {
        let mut state = StreamState::new(80);
        state.push_delta("Partial");
        state.commit_all();

        assert!(state.queue_len() >= 1);
    }
}
