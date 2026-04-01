//! 终端 ASCII 渲染器 - 将追踪数据渲染为 ASCII 树形结构
//!
//! 提供终端友好的追踪可视化，支持：
//! - ASCII 树形结构展示 Span 层级
//! - 时间线展示
//! - 状态标识（成功/失败/运行中）

use crate::observability::trace_collector::RequestTraceSummary;
use crate::observability::trace_types::{
    OperationKind, RequestTrace, SpanStatus, SpanTrace, TraceStatus,
};
use std::fmt::Write;

/// ASCII 渲染器配置
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// 是否显示时间戳
    pub show_timestamps: bool,
    /// 是否显示持续时间
    pub show_duration: bool,
    /// 是否显示属性
    pub show_attributes: bool,
    /// 是否使用颜色（如果终端支持）
    pub use_color: bool,
    /// 缩进宽度
    pub indent_width: usize,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            show_timestamps: false,
            show_duration: true,
            show_attributes: false,
            use_color: false,
            indent_width: 2,
        }
    }
}

/// ASCII 渲染器
pub struct AsciiRenderer {
    config: RenderConfig,
}

impl AsciiRenderer {
    /// 创建新的 ASCII 渲染器
    pub fn new(config: RenderConfig) -> Self {
        Self { config }
    }

    /// 创建默认渲染器
    pub fn default() -> Self {
        Self::new(RenderConfig::default())
    }

    /// 渲染 RequestTrace 为 ASCII 树
    pub fn render_trace(&self, trace: &RequestTrace) -> String {
        let mut output = String::new();

        // 请求头部信息
        writeln!(
            output,
            "╔══════════════════════════════════════════════════════════╗"
        )
        .unwrap();
        writeln!(
            output,
            "║  Request: {:<53} ║",
            truncate(&trace.request_id, 53)
        )
        .unwrap();
        writeln!(
            output,
            "╠══════════════════════════════════════════════════════════╣"
        )
        .unwrap();

        // 基本信息
        let status_str = format_trace_status(&trace.status);
        writeln!(output, "║  Status: {:<52} ║", status_str).unwrap();

        if let Some(ref session_id) = trace.session_id {
            writeln!(output, "║  Session: {:<51} ║", truncate(session_id, 51)).unwrap();
        }

        if let Some(duration) = trace.duration_ms {
            writeln!(
                output,
                "║  Duration: {:>4} ms                                         ║",
                duration
            )
            .unwrap();
        }

        if let Some(ref summary) = trace.input_summary {
            writeln!(output, "║  Input: {:<53} ║", truncate(summary, 53)).unwrap();
        }

        // 统计信息
        writeln!(
            output,
            "╠══════════════════════════════════════════════════════════╣"
        )
        .unwrap();
        writeln!(
            output,
            "║  Spans: {:<3}  LLM Calls: {:<3}  Tools: {:<3}  Tokens: {:<5}    ║",
            trace.spans.len(),
            trace.llm_calls_count.unwrap_or(0),
            trace.tool_executions_count.unwrap_or(0),
            trace.total_tokens.unwrap_or(0)
        )
        .unwrap();
        writeln!(
            output,
            "╚══════════════════════════════════════════════════════════╝"
        )
        .unwrap();

        // 渲染 Spans 树
        if !trace.spans.is_empty() {
            writeln!(output).unwrap();
            writeln!(output, "Spans:").unwrap();
            self.render_spans_tree(&trace.spans, &mut output, 0);
        }

        output
    }

    /// 渲染 Spans 树形结构
    fn render_spans_tree(&self, spans: &[SpanTrace], output: &mut String, depth: usize) {
        let indent = "  ".repeat(depth * self.config.indent_width / 2);

        for (i, span) in spans.iter().enumerate() {
            let is_last = i == spans.len() - 1;
            let connector = if is_last { "└─" } else { "├─" };
            let status_icon = status_icon(span.status);

            let duration_str = if self.config.show_duration {
                if let Some(duration) = span.duration_ms {
                    format!(" [{}ms]", duration)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            writeln!(
                output,
                "{}{} {}{} {}{}",
                indent,
                connector,
                status_icon,
                span.operation_name,
                format_operation_kind(&span.operation_kind),
                duration_str
            )
            .unwrap();

            // 显示属性（如果启用）
            if self.config.show_attributes && !span.attributes.is_empty() {
                let attr_indent = "  ".repeat((depth * self.config.indent_width / 2) + 2);
                for (key, value) in span.attributes.iter().take(3) {
                    writeln!(output, "{}{}: {:?}", attr_indent, key, value).unwrap();
                }
            }
        }
    }

    /// 渲染最近的追踪列表
    pub fn render_summary_list(&self, summaries: &[RequestTraceSummary]) -> String {
        let mut output = String::new();

        writeln!(
            output,
            "╔════════════════════════════════════════════════════════════════════════════╗"
        )
        .unwrap();
        writeln!(
            output,
            "║  Recent Traces                                                             ║"
        )
        .unwrap();
        writeln!(
            output,
            "╠════════════════════════════════════════════════════════════════════════════╣"
        )
        .unwrap();
        writeln!(
            output,
            "║  {:<12} {:<10} {:>8} {:>6} {:>5} {:<35}  ║",
            "Request ID", "Status", "Time(ms)", "Spans", "LLM", "Input"
        )
        .unwrap();
        writeln!(
            output,
            "╠════════════════════════════════════════════════════════════════════════════╣"
        )
        .unwrap();

        for summary in summaries {
            let status_str = format_trace_status(&summary.status);
            let duration_str = summary
                .duration_ms
                .map(|d| format!("{}", d))
                .unwrap_or_else(|| "-".to_string());
            let input_summary = summary.input_summary.as_deref().unwrap_or("-");

            writeln!(
                output,
                "║  {:<12} {:<10} {:>8} {:>6} {:>5} {:<35}  ║",
                truncate(&summary.request_id, 12),
                status_str,
                duration_str,
                summary.span_count,
                summary.llm_calls_count.unwrap_or(0),
                truncate(input_summary, 35)
            )
            .unwrap();
        }

        writeln!(
            output,
            "╚════════════════════════════════════════════════════════════════════════════╝"
        )
        .unwrap();
        output
    }

    /// 渲染时间线视图
    pub fn render_timeline(&self, trace: &RequestTrace) -> String {
        let mut output = String::new();

        if trace.spans.is_empty() {
            return output;
        }

        // 找到最小和最大时间戳
        let min_time = trace
            .spans
            .iter()
            .map(|s| s.start_timestamp_ms)
            .min()
            .unwrap_or(0);
        let max_time = trace
            .spans
            .iter()
            .filter_map(|s| s.duration_ms.map(|d| s.start_timestamp_ms + d))
            .max()
            .unwrap_or(min_time);

        let total_duration = max_time - min_time;
        let timeline_width = 60;

        writeln!(output, "Timeline:").unwrap();
        writeln!(
            output,
            "0ms {} {}ms",
            "-".repeat(timeline_width / 2 - 10),
            total_duration
        )
        .unwrap();

        for span in &trace.spans {
            let relative_start = span.start_timestamp_ms - min_time;
            let span_duration = span.duration_ms.unwrap_or(0);

            let start_pos = if total_duration > 0 {
                (relative_start as f64 / total_duration as f64 * timeline_width as f64) as usize
            } else {
                0
            };

            let span_width = if total_duration > 0 {
                ((span_duration as f64 / total_duration as f64 * timeline_width as f64) as usize)
                    .max(1)
            } else {
                1
            };

            let bar = if span.status == SpanStatus::Failure {
                "█"
            } else {
                "─"
            };

            let indent = " ".repeat(start_pos);
            let bar_str = bar.repeat(span_width);

            writeln!(
                output,
                "  {:<20} {}{}",
                truncate(&span.operation_name, 20),
                indent,
                bar_str
            )
            .unwrap();
        }

        output
    }
}

/// 格式化操作类型
fn format_operation_kind(kind: &OperationKind) -> String {
    format!("[{}]", kind.as_str())
}

/// 获取状态图标
fn status_icon(status: SpanStatus) -> &'static str {
    match status {
        SpanStatus::Success => "✓",
        SpanStatus::Failure => "✗",
        SpanStatus::Running => "○",
    }
}

/// 格式化追踪状态
fn format_trace_status(status: &TraceStatus) -> String {
    match status {
        TraceStatus::Running => "🔄 Running".to_string(),
        TraceStatus::Success => "✅ Success".to_string(),
        TraceStatus::Failure => "❌ Failure".to_string(),
        TraceStatus::Cancelled => "🚫 Cancelled".to_string(),
    }
}

/// 截断字符串
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// 快速渲染追踪（使用默认配置）
pub fn render_trace_quick(trace: &RequestTrace) -> String {
    AsciiRenderer::default().render_trace(trace)
}

/// 快速渲染追踪列表
pub fn render_summary_list_quick(summaries: &[RequestTraceSummary]) -> String {
    AsciiRenderer::default().render_summary_list(summaries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::trace_types::AttributeValue;
    use std::collections::HashMap;
    use std::time::Duration;

    fn create_test_trace() -> RequestTrace {
        use crate::observability::trace_types::RequestTrace;

        let mut trace = RequestTrace::new("test-req-123".to_string());
        trace.session_id = Some("session-456".to_string());
        trace.input_summary = Some("Test user input".to_string());
        trace.status = TraceStatus::Success;
        trace.duration_ms = Some(150);
        trace.llm_calls_count = Some(2);
        trace.tool_executions_count = Some(1);

        // 添加一些 spans
        let mut span1 = SpanTrace::new(
            "test-req-123".to_string(),
            OperationKind::Planner,
            "Generate plan",
        );
        span1.status = SpanStatus::Success;
        span1.duration_ms = Some(50);
        trace.add_span(span1);

        let mut span2 = SpanTrace::new(
            "test-req-123".to_string(),
            OperationKind::LlmCall,
            "Call LLM",
        );
        span2.status = SpanStatus::Success;
        span2.duration_ms = Some(80);
        trace.add_span(span2);

        let mut span3 = SpanTrace::new(
            "test-req-123".to_string(),
            OperationKind::ToolExecution,
            "Execute tool",
        );
        span3.status = SpanStatus::Success;
        span3.duration_ms = Some(20);
        trace.add_span(span3);

        trace
    }

    #[test]
    fn test_render_trace() {
        let renderer = AsciiRenderer::default();
        let trace = create_test_trace();
        let output = renderer.render_trace(&trace);

        assert!(output.contains("Request:"));
        assert!(output.contains("test-req-123"));
        assert!(output.contains("Success"));
        assert!(output.contains("Spans:"));
        assert!(output.contains("Generate plan"));
        assert!(output.contains("Call LLM"));
        assert!(output.contains("Execute tool"));
    }

    #[test]
    fn test_render_summary_list() {
        let renderer = AsciiRenderer::default();

        let summaries = vec![
            RequestTraceSummary {
                request_id: "req-1".to_string(),
                session_id: Some("session-1".to_string()),
                status: TraceStatus::Success,
                duration_ms: Some(100),
                span_count: 3,
                input_summary: Some("Test input 1".to_string()),
                llm_calls_count: Some(2),
                tool_executions_count: Some(1),
            },
            RequestTraceSummary {
                request_id: "req-2".to_string(),
                session_id: Some("session-2".to_string()),
                status: TraceStatus::Failure,
                duration_ms: Some(50),
                span_count: 1,
                input_summary: Some("Test input 2".to_string()),
                llm_calls_count: Some(1),
                tool_executions_count: Some(0),
            },
        ];

        let output = renderer.render_summary_list(&summaries);
        assert!(output.contains("Recent Traces"));
        assert!(output.contains("req-1"));
        assert!(output.contains("req-2"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("hello", 5), "hello");
        assert_eq!(truncate("hello", 4), "h...");
    }

    #[test]
    fn test_status_icon() {
        assert_eq!(status_icon(SpanStatus::Success), "✓");
        assert_eq!(status_icon(SpanStatus::Failure), "✗");
        assert_eq!(status_icon(SpanStatus::Running), "○");
    }

    #[test]
    fn test_format_trace_status() {
        assert!(format_trace_status(&TraceStatus::Running).contains("Running"));
        assert!(format_trace_status(&TraceStatus::Success).contains("Success"));
        assert!(format_trace_status(&TraceStatus::Failure).contains("Failure"));
        assert!(format_trace_status(&TraceStatus::Cancelled).contains("Cancelled"));
    }
}
