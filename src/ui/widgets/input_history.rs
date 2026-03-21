//! 输入历史管理
//!
//! 存储和检索用户输入历史，支持上下箭头遍历

use std::collections::VecDeque;

/// 输入历史管理器
pub struct InputHistory {
    /// 历史 entries（最新的在最后）
    entries: VecDeque<String>,
    /// 当前浏览位置（None 表示在最新输入位置）
    position: Option<usize>,
    /// 临时缓存（用户正在输入的内容）
    temp_cache: Option<String>,
    /// 最大历史数量
    max_size: usize,
}

impl InputHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_size),
            position: None,
            temp_cache: None,
            max_size,
        }
    }

    /// 添加新条目到历史
    pub fn push(&mut self, input: String) {
        if input.trim().is_empty() {
            return;
        }

        // 避免重复添加相同的连续输入
        if self.entries.back().map_or(false, |last| last == &input) {
            return;
        }

        self.entries.push_back(input);

        // 限制历史大小
        while self.entries.len() > self.max_size {
            self.entries.pop_front();
        }

        // 重置位置
        self.position = None;
        self.temp_cache = None;
    }

    /// 上一条历史（上箭头）
    pub fn previous(&mut self, current_input: &str) -> Option<&String> {
        if self.entries.is_empty() {
            return None;
        }

        // 第一次按上箭头时，缓存当前输入
        if self.position.is_none() {
            self.temp_cache = Some(current_input.to_string());
            self.position = Some(self.entries.len() - 1);
        } else if let Some(pos) = &mut self.position {
            if *pos > 0 {
                *pos -= 1;
            }
        }

        self.position.and_then(|p| self.entries.get(p))
    }

    /// 下一条历史（下箭头）
    pub fn next(&mut self) -> Option<&String> {
        if self.entries.is_empty() {
            return None;
        }

        if let Some(pos) = &mut self.position {
            if *pos < self.entries.len() - 1 {
                *pos += 1;
            } else {
                // 回到最新位置（使用缓存的输入）
                self.position = None;
                return None;
            }
        }

        self.position.and_then(|p| self.entries.get(p))
    }

    /// 取消浏览（获取缓存的输入）
    pub fn cancel(&mut self) -> Option<String> {
        self.position = None;
        self.temp_cache.take()
    }

    /// 获取当前浏览位置
    pub fn current_position(&self) -> Option<usize> {
        self.position
    }

    /// 历史条目数量
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 清空历史
    pub fn clear(&mut self) {
        self.entries.clear();
        self.position = None;
        self.temp_cache = None;
    }
}

impl Default for InputHistory {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_previous() {
        let mut history = InputHistory::new(10);
        history.push("First".to_string());
        history.push("Second".to_string());
        history.push("Third".to_string());

        let prev = history.previous("");
        assert_eq!(prev, Some(&"Third".to_string()));

        let prev = history.previous("");
        assert_eq!(prev, Some(&"Second".to_string()));

        let prev = history.previous("");
        assert_eq!(prev, Some(&"First".to_string()));
    }

    #[test]
    fn test_next() {
        let mut history = InputHistory::new(10);
        history.push("First".to_string());
        history.push("Second".to_string());

        history.previous("");
        history.previous("");

        let next = history.next();
        assert_eq!(next, Some(&"Second".to_string()));

        let next = history.next();
        assert!(next.is_none()); // 回到最新位置
    }

    #[test]
    fn test_cancel() {
        let mut history = InputHistory::new(10);
        history.push("Test".to_string());

        history.previous("Current input");
        let cancelled = history.cancel();

        assert_eq!(cancelled, Some("Current input".to_string()));
        assert!(history.current_position().is_none());
    }

    #[test]
    fn test_max_size() {
        let mut history = InputHistory::new(5);

        for i in 0..10 {
            history.push(format!("Entry {}", i));
        }

        assert_eq!(history.len(), 5);
        assert_eq!(history.entries.front(), Some(&"Entry 5".to_string()));
        assert_eq!(history.entries.back(), Some(&"Entry 9".to_string()));
    }

    #[test]
    fn test_empty_input_not_added() {
        let mut history = InputHistory::new(10);
        history.push("".to_string());
        history.push("   ".to_string());
        assert!(history.is_empty());
    }
}
