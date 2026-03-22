//! 可渲染组件 trait
//!
//! 提供统一的渲染接口，支持组合布局和灵活权重分配

use ratatui::{buffer::Buffer, layout::Rect};

/// 可渲染组件 trait
pub trait Renderable {
    /// 渲染组件到指定区域
    fn render(&mut self, area: Rect, buf: &mut Buffer);

    /// 获取期望高度（给定宽度）
    fn desired_height(&self, _width: u16) -> u16 {
        1
    }

    /// 获取光标位置（相对于 area）
    fn cursor_pos(&self, _area: Rect) -> Option<(u16, u16)> {
        None
    }
}

/// 灵活布局项（权重 + 可渲染对象）
pub struct FlexItem<'a> {
    pub weight: u16,
    pub renderable: &'a mut dyn Renderable,
}

/// 灵活布局容器
pub struct FlexRenderable<'a> {
    pub items: Vec<FlexItem<'a>>,
}

impl<'a> FlexRenderable<'a> {
    pub fn new(items: Vec<FlexItem<'a>>) -> Self {
        Self { items }
    }
}

impl<'a> Renderable for FlexRenderable<'a> {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        use ratatui::layout::{Constraint, Direction, Layout};

        let total_weight: u16 = self.items.iter().map(|i| i.weight).sum();
        if total_weight == 0 {
            return;
        }

        let constraints: Vec<Constraint> = self
            .items
            .iter()
            .map(|item| {
                let percentage = (item.weight as f32 / total_weight as f32) * 100.0;
                Constraint::Percentage(percentage as u16)
            })
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area);

        for (i, item) in self.items.iter_mut().enumerate() {
            item.renderable.render(chunks[i], buf);
        }
    }
}
