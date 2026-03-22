//! 多行文本编辑组件
//!
//! 支持多行输入、光标移动、输入历史遍历

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap},
};

use crate::ui::Renderable;

/// 文本区域组件
pub struct Textarea<'a> {
    /// 文本内容（按行存储）
    pub lines: Vec<String>,
    /// 光标位置（行，列）
    pub cursor: (usize, usize),
    /// 滚动偏移
    pub scroll: usize,
    /// 是否聚焦
    pub focused: bool,
    /// 标题
    pub title: &'a str,
    /// 占位符
    pub placeholder: &'a str,
    /// 最大高度
    pub max_height: usize,
}

impl<'a> Textarea<'a> {
    pub fn new(title: &'a str, placeholder: &'a str) -> Self {
        Self {
            lines: vec![String::new()],
            cursor: (0, 0),
            scroll: 0,
            focused: true,
            title,
            placeholder,
            max_height: 8,
        }
    }

    /// 获取当前输入内容
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// 获取光标所在行的内容
    pub fn current_line(&self) -> &str {
        &self.lines[self.cursor.0]
    }

    /// 插入字符
    pub fn insert_char(&mut self, c: char) {
        let (row, col) = self.cursor;
        self.ensure_capacity(row);

        let line = &mut self.lines[row];
        let char_idx = line
            .char_indices()
            .nth(col)
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        line.insert(char_idx, c);
        self.cursor.1 = (col + 1).min(line.chars().count());
    }

    /// 删除字符（Backspace）
    pub fn backspace(&mut self) {
        let (row, col) = self.cursor;

        if col > 0 {
            // 删除当前行前一个字符
            let line = &mut self.lines[row];
            let char_idx = line
                .char_indices()
                .nth(col - 1)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            line.remove(char_idx);
            self.cursor.1 = col - 1;
        } else if row > 0 {
            // 合并到上一行
            let prev_line_len = self.lines[row - 1].chars().count();
            let current_line = self.lines.remove(row);
            self.lines[row - 1].push_str(&current_line);
            self.cursor = (row - 1, prev_line_len);
        }
    }

    /// 删除字符（Delete）
    pub fn delete(&mut self) {
        let (row, col) = self.cursor;
        let line = &self.lines[row];
        let char_count = line.chars().count();

        if col < char_count {
            // 删除当前字符
            let line = &mut self.lines[row];
            let char_idx = line
                .char_indices()
                .nth(col)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            line.remove(char_idx);
        } else if row < self.lines.len() - 1 {
            // 合并下一行
            let next_line = self.lines.remove(row + 1);
            self.lines[row].push_str(&next_line);
        }
    }

    /// 插入新行
    pub fn insert_newline(&mut self) {
        let (row, col) = self.cursor;
        let current_line = self.lines[row].clone();

        let (first_part, second_part) = current_line.split_at(
            current_line
                .char_indices()
                .nth(col)
                .map(|(i, _)| i)
                .unwrap_or(current_line.len()),
        );

        self.lines[row] = first_part.to_string();
        self.lines.insert(row + 1, second_part.to_string());
        self.cursor = (row + 1, 0);
    }

    /// 移动光标
    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let (row, col) = self.cursor;
        let new_row = (row as i32 + dy).clamp(0, self.lines.len() as i32 - 1) as usize;
        let new_col =
            (col as i32 + dx).clamp(0, self.lines[new_row].chars().count() as i32) as usize;
        self.cursor = (new_row, new_col);

        // 更新滚动
        if self.cursor.0 < self.scroll {
            self.scroll = self.cursor.0;
        } else if self.cursor.0 >= self.scroll + self.max_height {
            self.scroll = self.cursor.0 - self.max_height + 1;
        }
    }

    /// 移动到行首
    pub fn move_to_line_start(&mut self) {
        self.cursor.1 = 0;
    }

    /// 移动到行尾
    pub fn move_to_line_end(&mut self) {
        let col = self.lines[self.cursor.0].chars().count();
        self.cursor.1 = col;
    }

    /// 清空内容
    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor = (0, 0);
        self.scroll = 0;
    }

    fn ensure_capacity(&mut self, row: usize) {
        while self.lines.len() <= row {
            self.lines.push(String::new());
        }
    }
}

impl<'a> Renderable for Textarea<'a> {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let is_empty = self.lines.len() == 1 && self.lines[0].is_empty();

        let block = Block::default()
            .title(self.title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if self.focused {
                Color::Cyan
            } else {
                Color::Gray
            }));

        let inner = block.inner(area);

        // 构建文本内容
        let mut text_lines = Vec::new();
        let visible_lines = self
            .lines
            .iter()
            .skip(self.scroll)
            .take(inner.height as usize);

        for (i, line) in visible_lines.enumerate() {
            let mut spans = Vec::new();

            // 显示占位符
            if i == 0 && line.is_empty() && is_empty {
                spans.push(Span::styled(
                    self.placeholder,
                    Style::default().fg(Color::DarkGray),
                ));
            } else {
                // 显示光标
                let (cursor_row, cursor_col) = self.cursor;
                let visible_row = self.scroll + i;

                if visible_row == cursor_row {
                    let left: String = line.chars().take(cursor_col).collect();
                    let cursor_char = line.chars().nth(cursor_col).unwrap_or(' ');
                    let right: String = line.chars().skip(cursor_col + 1).collect();

                    spans.push(Span::raw(left));
                    spans.push(Span::styled(
                        cursor_char.to_string(),
                        Style::default().bg(Color::White).fg(Color::Black),
                    ));
                    spans.push(Span::raw(right));
                } else {
                    spans.push(Span::raw(line));
                }
            }

            text_lines.push(Line::from(spans));
        }

        let paragraph = Paragraph::new(Text::from(text_lines))
            .block(block)
            .wrap(Wrap { trim: false });

        paragraph.render(area, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        self.lines.len().min(self.max_height) as u16 + 2 // +2 for borders
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_char() {
        let mut textarea = Textarea::new("Test", "Enter text");
        textarea.insert_char('H');
        textarea.insert_char('i');
        assert_eq!(textarea.content(), "Hi");
    }

    #[test]
    fn test_backspace() {
        let mut textarea = Textarea::new("Test", "Enter text");
        textarea.insert_char('H');
        textarea.insert_char('i');
        textarea.backspace();
        assert_eq!(textarea.content(), "H");
    }

    #[test]
    fn test_newline() {
        let mut textarea = Textarea::new("Test", "Enter text");
        textarea.insert_char('L');
        textarea.insert_char('i');
        textarea.insert_newline();
        textarea.insert_char('n');
        textarea.insert_char('e');
        assert_eq!(textarea.lines.len(), 2);
        assert_eq!(textarea.lines[0], "Li");
        assert_eq!(textarea.lines[1], "ne");
    }

    #[test]
    fn test_cursor_movement() {
        let mut textarea = Textarea::new("Test", "Enter text");
        textarea.insert_char('H');
        textarea.insert_char('i');
        textarea.move_cursor(-1, 0);
        assert_eq!(textarea.cursor, (0, 1));
    }
}
