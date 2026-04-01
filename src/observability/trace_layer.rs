//! Tracing Layer 实现，将 spans 导出到 TraceCollector
//!
//! 提供与 tracing 生态系统的集成，自动捕获 spans 并发送到 TraceCollector

use crate::observability::trace_collector::TraceCollector;
use crate::observability::trace_types::{OperationKind, SpanStatus, SpanTrace};
use std::sync::Arc;
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

/// TraceCollectionLayer - 将 tracing spans 导出到 TraceCollector
pub struct TraceCollectionLayer {
    collector: Arc<TraceCollector>,
}

impl TraceCollectionLayer {
    /// 创建新的 TraceCollectionLayer
    pub fn new(collector: Arc<TraceCollector>) -> Self {
        Self { collector }
    }

    /// 从 target 字符串解析操作类型
    fn parse_operation_kind(target: &str) -> OperationKind {
        match target {
            t if t.contains("planner") => OperationKind::Planner,
            t if t.contains("critic") => OperationKind::Critic,
            t if t.contains("orchestrator") => OperationKind::Orchestrator,
            t if t.contains("llm") || t.contains("openai") || t.contains("deepseek") => {
                OperationKind::LlmCall
            }
            t if t.contains("tool") || t.contains("executor") => OperationKind::ToolExecution,
            t if t.contains("memory") => OperationKind::Memory,
            t if t.contains("rag") => OperationKind::RagRetrieval,
            t if t.contains("stream") || t.contains("response") => OperationKind::ResponseStream,
            t if t.contains("skill") => OperationKind::SkillSelection,
            t if t.contains("evolution") => OperationKind::EvolutionAnalysis,
            _ => OperationKind::Orchestrator,
        }
    }
}

impl<S> Layer<S> for TraceCollectionLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: Context<'_, S>,
    ) {
        let metadata = attrs.metadata();
        let span_name = metadata.name();
        let target = metadata.target();

        // 解析操作类型
        let operation = Self::parse_operation_kind(target);

        // 尝试从上下文中提取 request_id
        let request_id = extract_request_id_from_context(&ctx);

        if let Some(req_id) = request_id {
            let span_id = format!("{}-{:x}", span_name, id.into_u64());

            let mut span_trace = SpanTrace::new(req_id.clone(), operation, span_name);
            span_trace.span_id = span_id;

            // 将 span 存储到 extensions 中，以便在 close 时使用
            if let Some(span_ref) = ctx.span(id) {
                let mut extensions = span_ref.extensions_mut();
                extensions.insert(span_trace);
            }
        }
    }

    fn on_close(&self, id: tracing::span::Id, ctx: Context<'_, S>) {
        let span_ref = ctx.span(&id);
        if let Some(span_ref) = span_ref {
            let metadata = span_ref.metadata();
            let span_name = metadata.name();
            let level = metadata.level();

            // 提取 request_id
            let request_id = extract_request_id_from_context(&ctx);

            if let Some(_req_id) = request_id {
                let span_id = format!("{}-{:x}", span_name, id.into_u64());

                // 从 extensions 中获取 span
                let mut extensions = span_ref.extensions_mut();
                if let Some(stored_span) = extensions.get_mut::<SpanTrace>() {
                    stored_span.span_id = span_id;

                    // 根据级别设置状态
                    if level >= &tracing::Level::WARN {
                        stored_span.status = SpanStatus::Failure;
                    } else {
                        stored_span.status = SpanStatus::Success;
                    }
                }
                // span 会在 extensions 释放时自动被发送到 collector
            }
        }
    }
}

/// 从上下文中提取 request_id
fn extract_request_id_from_context<S>(ctx: &Context<'_, S>) -> Option<String>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    // 尝试从当前 span 树中提取 request_id
    ctx.lookup_current().and_then(|span| {
        // 首先检查当前 span 的 extensions
        let extensions = span.extensions();
        if let Some(id) = extensions.get::<RequestId>() {
            return Some(id.0.clone());
        }

        // 向上查找父 span
        let mut current = span.parent();
        while let Some(parent) = current {
            let parent_extensions = parent.extensions();
            if let Some(id) = parent_extensions.get::<RequestId>() {
                return Some(id.0.clone());
            }
            current = parent.parent();
        }

        None
    })
}

/// RequestId 包装器，用于存储在 span extensions 中
struct RequestId(String);

/// 初始化追踪收集层
pub fn init_trace_collection(collector: Arc<TraceCollector>) {
    let layer = TraceCollectionLayer::new(collector);
    let filter_layer = tracing_subscriber::EnvFilter::from_default_env();

    // 检查是否已经初始化过
    static INITIALIZED: std::sync::Once = std::sync::Once::new();
    INITIALIZED.call_once(|| {
        // 使用 builder pattern 初始化
        let subscriber = tracing_subscriber::registry()
            .with(layer)
            .with(filter_layer);
        subscriber.try_init().ok();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_operation_kind() {
        assert_eq!(
            TraceCollectionLayer::parse_operation_kind("bee::planner"),
            OperationKind::Planner
        );
        assert_eq!(
            TraceCollectionLayer::parse_operation_kind("bee::critic"),
            OperationKind::Critic
        );
        assert_eq!(
            TraceCollectionLayer::parse_operation_kind("bee::llm::openai"),
            OperationKind::LlmCall
        );
        assert_eq!(
            TraceCollectionLayer::parse_operation_kind("bee::tools::executor"),
            OperationKind::ToolExecution
        );
        assert_eq!(
            TraceCollectionLayer::parse_operation_kind("bee::memory"),
            OperationKind::Memory
        );
        assert_eq!(
            TraceCollectionLayer::parse_operation_kind("unknown"),
            OperationKind::Orchestrator
        );
    }
}
