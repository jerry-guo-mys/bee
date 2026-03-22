//! Markdown 流收集器
//!
//! 实现 newline-gated Markdown 累积：只有完整的逻辑行才会被提交渲染

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

/// Markdown 流收集器
pub struct MarkdownStreamCollector {
    /// 累积的 Markdown 源
    buffer: String,
    /// 上次提交的位置
    last_commit_pos: usize,
    /// 渲染宽度
    width: usize,
    /// 当前工作目录（用于相对路径）
    cwd: Option<String>,
}

impl MarkdownStreamCollector {
    pub fn new(width: usize) -> Self {
        Self {
            buffer: String::new(),
            last_commit_pos: 0,
            width,
            cwd: None,
        }
    }

    pub fn with_cwd(mut self, cwd: String) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// 追加新的 Markdown delta
    pub fn push(&mut self, delta: &str) {
        self.buffer.push_str(delta);
    }

    /// 提交完整的行（以 newline 结尾）
    /// 返回新提交的 Line 列表
    pub fn commit_complete_lines(&mut self) -> Vec<Line<'static>> {
        let mut rendered = Vec::new();

        // 找到最后一个 newline
        if let Some(last_newline) = self.buffer.rfind('\n') {
            // 只处理到最后一个 newline 的内容
            let to_commit = &self.buffer[..=last_newline];

            if to_commit.len() > self.last_commit_pos {
                let new_content = &to_commit[self.last_commit_pos..];

                // 按行分割并渲染
                for line in new_content.lines() {
                    let rendered_line = self.render_line(line);
                    rendered.push(rendered_line);
                }

                self.last_commit_pos = to_commit.len();
            }
        }

        rendered
    }

    /// 渲染单行 Markdown（简化版本，Phase 3 实现完整 Markdown）
    fn render_line(&self, line: &str) -> Line<'static> {
        // TODO: Phase 3 实现完整的 Markdown 解析
        // 目前简化为纯文本
        Line::from(Span::raw(line.to_string()))
    }

    /// 强制提交所有剩余内容（用于流结束）
    pub fn commit_all(&mut self) -> Vec<Line<'static>> {
        if self.buffer.len() > self.last_commit_pos {
            let remaining = &self.buffer[self.last_commit_pos..];
            let mut rendered = Vec::new();

            for line in remaining.lines() {
                rendered.push(self.render_line(line));
            }

            // 处理最后一行（如果没有 newline）
            if !remaining.ends_with('\n') && !remaining.is_empty() {
                rendered.push(self.render_line(remaining));
            }

            self.last_commit_pos = self.buffer.len();
            rendered
        } else {
            Vec::new()
        }
    }

    /// 获取未提交的字符数
    pub fn uncommitted_chars(&self) -> usize {
        self.buffer.len() - self.last_commit_pos
    }

    /// 重置收集器
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.last_commit_pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_commit() {
        let mut collector = MarkdownStreamCollector::new(80);
        collector.push("Hello, ");
        collector.push("World!\n");

        let lines = collector.commit_complete_lines();
        assert_eq!(lines.len(), 1);
        assert_eq!(collector.uncommitted_chars(), 0);
    }

    #[test]
    fn test_partial_commit() {
        let mut collector = MarkdownStreamCollector::new(80);
        collector.push("Line 1\n");
        collector.push("Line 2"); // 没有 newline

        let lines = collector.commit_complete_lines();
        assert_eq!(lines.len(), 1); // 只有 Line 1 被提交
        assert_eq!(collector.uncommitted_chars(), 6); // "Line 2" 未提交
    }

    #[test]
    fn test_commit_all() {
        let mut collector = MarkdownStreamCollector::new(80);
        collector.push("Partial line");

        let lines = collector.commit_all();
        // "Partial line" 会被分成一行
        assert!(lines.len() >= 1);
        assert_eq!(collector.uncommitted_chars(), 0);
    }

    #[test]
    fn test_clear() {
        let mut collector = MarkdownStreamCollector::new(80);
        collector.push("Some text\n");
        collector.clear();

        assert_eq!(collector.uncommitted_chars(), 0);
        assert_eq!(collector.buffer.len(), 0);
    }
}
