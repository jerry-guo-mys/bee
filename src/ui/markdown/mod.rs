//! Markdown 渲染管道
//!
//! Phase 3 实现目标：
//! - pulldown-cmark: Markdown 解析
//! - syntect/two-face: 语法高亮
//! - 主题配置支持

mod highlight;
mod renderer;

pub use highlight::Highlighter;
pub use renderer::MarkdownRenderer;
