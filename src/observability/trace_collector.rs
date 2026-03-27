//! TraceCollector 核心实现
//!
//! 提供追踪数据收集和管理功能，支持内存 + SQLite 双存储：
//! - DashMap 内存存储：最近 100 条快速访问，LRU 淘汰机制
//! - SqliteTraceStore：SQLite 持久化存储
//! - 支持按 request_id 查询、最近列表、清理旧数据

use crate::observability::trace_types::{
    OperationKind, RequestTrace, SpanStatus, SpanTrace, TraceStatus,
};
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// 追踪收集器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceCollectorConfig {
    /// 内存中保留的最大追踪数（LRU 淘汰）
    pub max_memory_traces: usize,
    /// SQLite 数据库路径（None 表示禁用持久化）
    pub sqlite_path: Option<String>,
    /// 是否启用异步事件处理
    pub enable_async_events: bool,
}

impl Default for TraceCollectorConfig {
    fn default() -> Self {
        Self {
            max_memory_traces: 100,
            sqlite_path: Some("bee_traces.db".to_string()),
            enable_async_events: true,
        }
    }
}

impl TraceCollectorConfig {
    /// 创建仅内存配置
    pub fn memory_only() -> Self {
        Self {
            max_memory_traces: 100,
            sqlite_path: None,
            enable_async_events: true,
        }
    }

    /// 创建自定义配置
    pub fn new(sqlite_path: impl Into<String>, max_traces: usize) -> Self {
        Self {
            max_memory_traces: max_traces,
            sqlite_path: Some(sqlite_path.into()),
            enable_async_events: true,
        }
    }
}

/// 追踪事件枚举 - 用于异步事件处理
#[derive(Debug, Clone)]
pub enum TraceEvent {
    /// 记录请求追踪
    RecordRequest(RequestTrace),
    /// 添加 Span 到请求
    AddSpan {
        request_id: String,
        span: SpanTrace,
    },
    /// 更新请求状态
    UpdateStatus {
        request_id: String,
        status: TraceStatus,
    },
    /// 清理旧数据
    Cleanup {
        keep_recent: usize,
    },
    /// 按 request_id 查询
    Query {
        request_id: String,
        response_tx: Sender<Option<RequestTrace>>,
    },
    /// 获取最近追踪列表
    GetRecent {
        limit: usize,
        response_tx: Sender<Vec<RequestTraceSummary>>,
    },
}

/// 请求追踪摘要 - 用于列表展示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestTraceSummary {
    /// 请求 ID
    pub request_id: String,
    /// 会话 ID
    pub session_id: Option<String>,
    /// 状态
    pub status: TraceStatus,
    /// 持续时间（毫秒）
    pub duration_ms: Option<u64>,
    /// Span 数量
    pub span_count: usize,
    /// 输入摘要
    pub input_summary: Option<String>,
    /// LLM 调用次数
    pub llm_calls_count: Option<u32>,
    /// 工具执行次数
    pub tool_executions_count: Option<u32>,
}

impl From<&RequestTrace> for RequestTraceSummary {
    fn from(trace: &RequestTrace) -> Self {
        Self {
            request_id: trace.request_id.clone(),
            session_id: trace.session_id.clone(),
            status: trace.status.clone(),
            duration_ms: trace.duration_ms,
            span_count: trace.spans.len(),
            input_summary: trace.input_summary.clone(),
            llm_calls_count: trace.llm_calls_count,
            tool_executions_count: trace.tool_executions_count,
        }
    }
}

/// SQLite 追踪存储
#[derive(Clone)]
pub struct SqliteTraceStore {
    conn: Arc<tokio::sync::Mutex<Connection>>,
}

impl SqliteTraceStore {
    /// 创建新的 SQLite 追踪存储
    pub fn new(path: impl AsRef<Path>) -> SqliteResult<Self> {
        let conn = Connection::open(path.as_ref())?;

        // 初始化表结构
        Self::init_tables(&conn)?;

        Ok(Self {
            conn: Arc::new(tokio::sync::Mutex::new(conn)),
        })
    }

    /// 创建内存中 SQLite 存储（用于测试）
    pub fn in_memory() -> SqliteResult<Self> {
        let conn = Connection::open(":memory:")?;

        // 初始化表结构
        Self::init_tables(&conn)?;

        Ok(Self {
            conn: Arc::new(tokio::sync::Mutex::new(conn)),
        })
    }

    /// 初始化数据库表和索引
    fn init_tables(conn: &Connection) -> SqliteResult<()> {
        // 创建请求追踪表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS request_traces (
                request_id TEXT PRIMARY KEY,
                session_id TEXT,
                start_timestamp_ms INTEGER NOT NULL,
                end_timestamp_ms INTEGER,
                duration_ms INTEGER,
                status TEXT NOT NULL,
                input_summary TEXT,
                output_summary TEXT,
                metadata_json TEXT,
                error_message TEXT,
                react_steps_total INTEGER,
                llm_calls_count INTEGER,
                tool_executions_count INTEGER,
                total_tokens INTEGER,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            )",
            [],
        )?;

        // 创建 Span 追踪表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS span_traces (
                span_id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                parent_span_id TEXT,
                operation_kind TEXT NOT NULL,
                operation_name TEXT NOT NULL,
                start_timestamp_ms INTEGER NOT NULL,
                duration_ms INTEGER,
                status TEXT NOT NULL,
                attributes_json TEXT,
                error_message TEXT,
                react_step INTEGER,
                created_at INTEGER DEFAULT (strftime('%s', 'now')),
                FOREIGN KEY (request_id) REFERENCES request_traces(request_id)
            )",
            [],
        )?;

        // 创建索引以提高查询性能
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_traces_session ON request_traces(session_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_traces_status ON request_traces(status)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_traces_created ON request_traces(created_at)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_span_traces_request_id ON span_traces(request_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_span_traces_operation_kind ON span_traces(operation_kind)",
            [],
        )?;

        debug!("SQLite trace store initialized with tables and indexes");
        Ok(())
    }

    /// 保存请求追踪
    pub async fn save_request(&self, trace: &RequestTrace) -> SqliteResult<()> {
        let conn = self.conn.lock().await;

        let metadata_json = trace.metadata.as_ref()
            .map(|m| serde_json::to_string(m).ok())
            .flatten();

        conn.execute(
            "INSERT OR REPLACE INTO request_traces (
                request_id, session_id, start_timestamp_ms, end_timestamp_ms,
                duration_ms, status, input_summary, output_summary, metadata_json,
                error_message, react_steps_total, llm_calls_count,
                tool_executions_count, total_tokens
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                trace.request_id,
                trace.session_id,
                trace.start_timestamp_ms,
                trace.end_timestamp_ms,
                trace.duration_ms,
                Self::serialize_trace_status(&trace.status),
                trace.input_summary,
                trace.output_summary,
                metadata_json,
                trace.error_message,
                trace.react_steps_total,
                trace.llm_calls_count,
                trace.tool_executions_count,
                trace.total_tokens,
            ],
        )?;

        // 保存所有 Spans
        for span in &trace.spans {
            self.save_span_internal_sync(&conn, span)?;
        }

        debug!("Saved request trace: {}", trace.request_id);
        Ok(())
    }

    /// 保存单个 Span（同步版本，在事务内部调用）
    fn save_span_internal_sync(&self, conn: &Connection, span: &SpanTrace) -> SqliteResult<()> {
        let attributes_json = if !span.attributes.is_empty() {
            Some(serde_json::to_string(&span.attributes).ok())
        } else {
            None
        };

        conn.execute(
            "INSERT OR REPLACE INTO span_traces (
                span_id, request_id, parent_span_id, operation_kind,
                operation_name, start_timestamp_ms, duration_ms, status,
                attributes_json, error_message, react_step
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                span.span_id,
                span.request_id,
                span.parent_span_id,
                Self::serialize_operation_kind(&span.operation_kind),
                span.operation_name,
                span.start_timestamp_ms,
                span.duration_ms,
                Self::serialize_span_status(&span.status),
                attributes_json,
                span.error_message,
                span.react_step,
            ],
        )?;

        Ok(())
    }

    /// 保存 Span（公开方法）
    pub async fn save_span(&self, span: &SpanTrace) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        self.save_span_internal_sync(&conn, span)?;
        Ok(())
    }

    /// 按 request_id 查询追踪
    pub async fn get_by_request_id(&self, request_id: &str) -> SqliteResult<Option<RequestTrace>> {
        let conn = self.conn.lock().await;

        // 查询请求追踪
        let mut stmt = conn.prepare(
            "SELECT request_id, session_id, start_timestamp_ms, end_timestamp_ms,
                    duration_ms, status, input_summary, output_summary, metadata_json,
                    error_message, react_steps_total, llm_calls_count,
                    tool_executions_count, total_tokens
             FROM request_traces
             WHERE request_id = ?1",
        )?;

        let trace_opt = match stmt.query_row(params![request_id], |row| {
            let status = Self::deserialize_trace_status(&row.get::<_, String>(5)?);

            Ok(RequestTrace {
                request_id: row.get(0)?,
                session_id: row.get(1)?,
                start_timestamp_ms: row.get(2)?,
                end_timestamp_ms: row.get(3)?,
                duration_ms: row.get(4)?,
                status,
                input_summary: row.get(6)?,
                output_summary: row.get(7)?,
                metadata: row.get::<_, Option<String>>(8)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                error_message: row.get(9)?,
                react_steps_total: row.get(10)?,
                llm_calls_count: row.get(11)?,
                tool_executions_count: row.get(12)?,
                total_tokens: row.get(13)?,
                spans: Vec::new(), // 稍后填充
            })
        }) {
            Ok(trace) => Some(trace),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(e),
        };

        if let Some(mut trace) = trace_opt {
            // 查询所有 Spans
            let mut span_stmt = conn.prepare(
                "SELECT span_id, request_id, parent_span_id, operation_kind,
                        operation_name, start_timestamp_ms, duration_ms, status,
                        attributes_json, error_message, react_step
                 FROM span_traces
                 WHERE request_id = ?1
                 ORDER BY start_timestamp_ms",
            )?;

            let span_rows = span_stmt.query_map(params![request_id], |row| {
                let operation_kind = Self::deserialize_operation_kind(&row.get::<_, String>(3)?);
                let status = Self::deserialize_span_status(&row.get::<_, String>(7)?);

                let attributes: std::collections::HashMap<String, crate::observability::trace_types::AttributeValue> =
                    row.get::<_, Option<String>>(8)?
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default();

                Ok(SpanTrace {
                    span_id: row.get(0)?,
                    request_id: row.get(1)?,
                    parent_span_id: row.get(2)?,
                    operation_kind,
                    operation_name: row.get(4)?,
                    start_timestamp_ms: row.get(5)?,
                    duration_ms: row.get(6)?,
                    status,
                    attributes,
                    error_message: row.get(9)?,
                    react_step: row.get(10)?,
                })
            })?;

            for span_result in span_rows {
                if let Ok(span) = span_result {
                    trace.spans.push(span);
                }
            }

            Ok(Some(trace))
        } else {
            Ok(None)
        }
    }

    /// 获取最近的追踪摘要列表
    pub async fn get_recent_summaries(&self, limit: usize) -> SqliteResult<Vec<RequestTraceSummary>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT request_id, session_id, status, duration_ms,
                    input_summary, llm_calls_count, tool_executions_count
             FROM request_traces
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;

        let summaries = stmt.query_map(params![limit], |row| {
            let status = Self::deserialize_trace_status(&row.get::<_, String>(2)?);

            Ok(RequestTraceSummary {
                request_id: row.get(0)?,
                session_id: row.get(1)?,
                status,
                duration_ms: row.get(3)?,
                span_count: 0, // 稍后查询
                input_summary: row.get(4)?,
                llm_calls_count: row.get(5)?,
                tool_executions_count: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for summary_result in summaries {
            if let Ok(mut summary) = summary_result {
                // 查询 Span 数量
                let span_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM span_traces WHERE request_id = ?1",
                    params![summary.request_id],
                    |row| row.get(0),
                ).unwrap_or(0);
                summary.span_count = span_count as usize;
                result.push(summary);
            }
        }

        Ok(result)
    }

    /// 清理旧数据，保留最近的 N 条记录
    pub async fn cleanup_old(&self, keep_recent: usize) -> SqliteResult<usize> {
        let conn = self.conn.lock().await;

        // 获取需要删除的 request_ids
        let mut stmt = conn.prepare(
            "SELECT request_id FROM request_traces
             ORDER BY created_at DESC
             LIMIT -1 OFFSET ?1",
        )?;

        let ids_to_delete: Vec<String> = stmt
            .query_map(params![keep_recent], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        let deleted_count = ids_to_delete.len();

        if !ids_to_delete.is_empty() {
            // 删除 Spans（先外键）
            for id in &ids_to_delete {
                conn.execute("DELETE FROM span_traces WHERE request_id = ?1", params![id])?;
            }

            // 删除请求追踪
            let placeholders: Vec<&str> = ids_to_delete.iter().map(|_| "?").collect();
            let in_clause = placeholders.join(", ");
            conn.execute(
                &format!("DELETE FROM request_traces WHERE request_id IN ({})", in_clause),
                rusqlite::params_from_iter(ids_to_delete.iter()),
            )?;

            info!("Cleaned up {} old trace records", deleted_count);
        }

        Ok(deleted_count)
    }

    /// 序列化 TraceStatus 为字符串
    fn serialize_trace_status(status: &TraceStatus) -> &'static str {
        match status {
            TraceStatus::Running => "running",
            TraceStatus::Success => "success",
            TraceStatus::Failure => "failure",
            TraceStatus::Cancelled => "cancelled",
        }
    }

    /// 反序列化 TraceStatus
    fn deserialize_trace_status(s: &str) -> TraceStatus {
        parse_trace_status(s).unwrap_or(TraceStatus::Running)
    }

    /// 序列化 OperationKind 为字符串
    fn serialize_operation_kind(kind: &OperationKind) -> String {
        kind.as_str().to_string()
    }

    /// 反序列化 OperationKind
    fn deserialize_operation_kind(s: &str) -> OperationKind {
        parse_operation_kind(s).unwrap_or(OperationKind::Custom(s.to_string()))
    }

    /// 序列化 SpanStatus 为字符串
    fn serialize_span_status(status: &SpanStatus) -> &'static str {
        match status {
            SpanStatus::Running => "running",
            SpanStatus::Success => "success",
            SpanStatus::Failure => "failure",
        }
    }

    /// 反序列化 SpanStatus
    fn deserialize_span_status(s: &str) -> SpanStatus {
        parse_span_status(s).unwrap_or(SpanStatus::Running)
    }
}

/// 解析 TraceStatus 字符串
pub fn parse_trace_status(s: &str) -> Option<TraceStatus> {
    match s.to_lowercase().as_str() {
        "running" => Some(TraceStatus::Running),
        "success" => Some(TraceStatus::Success),
        "failure" | "failed" => Some(TraceStatus::Failure),
        "cancelled" | "canceled" => Some(TraceStatus::Cancelled),
        _ => None,
    }
}

/// 解析 OperationKind 字符串
pub fn parse_operation_kind(s: &str) -> Option<OperationKind> {
    match s.to_lowercase().as_str() {
        "orchestrator" => Some(OperationKind::Orchestrator),
        "planner" => Some(OperationKind::Planner),
        "critic" => Some(OperationKind::Critic),
        "llm_call" => Some(OperationKind::LlmCall),
        "tool_execution" => Some(OperationKind::ToolExecution),
        "memory" => Some(OperationKind::Memory),
        "rag_retrieval" => Some(OperationKind::RagRetrieval),
        "response_stream" => Some(OperationKind::ResponseStream),
        "skill_selection" => Some(OperationKind::SkillSelection),
        "evolution_analysis" => Some(OperationKind::EvolutionAnalysis),
        _ => Some(OperationKind::Custom(s.to_string())),
    }
}

/// 解析 SpanStatus 字符串
pub fn parse_span_status(s: &str) -> Option<SpanStatus> {
    match s.to_lowercase().as_str() {
        "running" => Some(SpanStatus::Running),
        "success" => Some(SpanStatus::Success),
        "failure" | "failed" => Some(SpanStatus::Failure),
        _ => None,
    }
}

/// TraceCollector 主结构体 - 内存 + SQLite 双存储
pub struct TraceCollector {
    /// 配置
    config: TraceCollectorConfig,
    /// 内存存储 - 最近追踪（使用 VecDeque 实现 LRU）
    memory_store: Arc<RwLock<VecDeque<RequestTrace>>>,
    /// SQLite 持久化存储（可选）
    sqlite_store: Option<SqliteTraceStore>,
    /// 异步事件发送器
    event_tx: Option<Sender<TraceEvent>>,
}

/// 全局 TraceCollector 实例
static GLOBAL_TRACE_COLLECTOR: std::sync::OnceLock<Arc<TraceCollector>> = std::sync::OnceLock::new();

impl TraceCollector {
    /// 获取全局 TraceCollector 实例
    pub fn get_global() -> Option<Arc<TraceCollector>> {
        GLOBAL_TRACE_COLLECTOR.get().cloned()
    }

    /// 设置全局 TraceCollector 实例
    pub fn set_global(collector: Arc<TraceCollector>) -> Result<(), ()> {
        GLOBAL_TRACE_COLLECTOR.set(collector).map_err(|_| ())
    }

    /// 创建新的 TraceCollector
    pub async fn new(config: TraceCollectorConfig) -> Result<Self, String> {
        let memory_store = Arc::new(RwLock::new(VecDeque::with_capacity(config.max_memory_traces)));

        let sqlite_store = if let Some(ref path) = config.sqlite_path {
            match SqliteTraceStore::new(path) {
                Ok(store) => {
                    info!("SQLite trace store initialized: {}", path);
                    Some(store)
                }
                Err(e) => {
                    warn!("Failed to initialize SQLite trace store: {}. Using memory only.", e);
                    None
                }
            }
        } else {
            None
        };

        let event_tx = if config.enable_async_events {
            let (tx, rx) = mpsc::channel::<TraceEvent>(100);
            // 启动事件处理循环
            let memory = Arc::clone(&memory_store);
            let sqlite = sqlite_store.clone();
            let max_traces = config.max_memory_traces;

            tokio::spawn(async move {
                Self::event_loop(rx, memory, sqlite, max_traces).await;
            });

            Some(tx)
        } else {
            None
        };

        Ok(Self {
            config,
            memory_store,
            sqlite_store,
            event_tx,
        })
    }

    /// 创建仅内存的 TraceCollector（用于测试）
    pub async fn memory_only() -> Result<Self, String> {
        Self::new(TraceCollectorConfig::memory_only()).await
    }

    /// 创建带 SQLite 的 TraceCollector
    pub async fn with_sqlite(path: impl Into<String>) -> Result<Self, String> {
        Self::new(TraceCollectorConfig::new(path, 100)).await
    }

    /// 异步事件处理循环
    async fn event_loop(
        mut rx: mpsc::Receiver<TraceEvent>,
        memory: Arc<RwLock<VecDeque<RequestTrace>>>,
        sqlite: Option<SqliteTraceStore>,
        max_traces: usize,
    ) {
        while let Some(event) = rx.recv().await {
            match event {
                TraceEvent::RecordRequest(trace) => {
                    // 添加到内存存储
                    let mut mem = memory.write().await;
                    mem.push_back(trace);
                    while mem.len() > max_traces {
                        mem.pop_front();
                    }

                    // 持久化到 SQLite
                    if let Some(ref store) = sqlite {
                        if let Err(e) = store.save_request(&mem.back().unwrap()).await {
                            error!("Failed to save trace to SQLite: {}", e);
                        }
                    }
                }
                TraceEvent::AddSpan { request_id, span } => {
                    // 在内存中查找并添加 Span
                    let mut mem = memory.write().await;
                    if let Some(trace) = mem.iter_mut().find(|t| t.request_id == request_id) {
                        trace.add_span(span.clone());

                        // 持久化 Span
                        if let Some(ref store) = sqlite {
                            if let Err(e) = store.save_span(&span).await {
                                error!("Failed to save span to SQLite: {}", e);
                            }
                        }
                    }
                }
                TraceEvent::UpdateStatus { request_id, status } => {
                    let mut mem = memory.write().await;
                    if let Some(trace) = mem.iter_mut().find(|t| t.request_id == request_id) {
                        trace.status = status;
                    }
                }
                TraceEvent::Cleanup { keep_recent } => {
                    // 内存清理
                    let mut mem = memory.write().await;
                    while mem.len() > keep_recent {
                        mem.pop_front();
                    }

                    // SQLite 清理
                    if let Some(ref store) = sqlite {
                        if let Err(e) = store.cleanup_old(keep_recent).await {
                            error!("Failed to cleanup old traces: {}", e);
                        }
                    }
                }
                TraceEvent::Query { request_id, response_tx } => {
                    let mem = memory.read().await;
                    let result = mem.iter()
                        .find(|t| t.request_id == request_id)
                        .cloned();

                    let _ = response_tx.send(result).await;
                }
                TraceEvent::GetRecent { limit, response_tx } => {
                    let mem = memory.read().await;
                    let summaries: Vec<RequestTraceSummary> = mem.iter()
                        .rev()
                        .take(limit)
                        .map(RequestTraceSummary::from)
                        .collect();

                    let _ = response_tx.send(summaries).await;
                }
            }
        }
    }

    /// 记录请求追踪
    pub async fn record(&self, trace: RequestTrace) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(TraceEvent::RecordRequest(trace)).await;
        } else {
            // 同步模式：直接添加到内存存储
            let mut mem = self.memory_store.write().await;
            mem.push_back(trace);
            while mem.len() > self.config.max_memory_traces {
                mem.pop_front();
            }
        }
    }

    /// 添加 Span 到请求
    pub async fn add_span(&self, request_id: &str, span: SpanTrace) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(TraceEvent::AddSpan {
                request_id: request_id.to_string(),
                span,
            }).await;
        } else {
            // 同步模式
            let mut mem = self.memory_store.write().await;
            if let Some(trace) = mem.iter_mut().find(|t| t.request_id == request_id) {
                trace.add_span(span);
            }
        }
    }

    /// 更新请求状态
    pub async fn update_status(&self, request_id: &str, status: TraceStatus) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(TraceEvent::UpdateStatus {
                request_id: request_id.to_string(),
                status,
            }).await;
        } else {
            // 同步模式
            let mut mem = self.memory_store.write().await;
            if let Some(trace) = mem.iter_mut().find(|t| t.request_id == request_id) {
                trace.status = status;
            }
        }
    }

    /// 按 request_id 查询追踪
    pub async fn get_by_request_id(&self, request_id: &str) -> Option<RequestTrace> {
        if let Some(ref tx) = self.event_tx {
            let (response_tx, mut response_rx) = mpsc::channel(1);
            let _ = tx.send(TraceEvent::Query {
                request_id: request_id.to_string(),
                response_tx,
            }).await;

            // 先从内存查找
            let mem = self.memory_store.read().await;
            if let Some(trace) = mem.iter().find(|t| t.request_id == request_id) {
                return Some(trace.clone());
            }

            // 从 SQLite 查找
            if let Some(ref store) = self.sqlite_store {
                if let Ok(Some(trace)) = store.get_by_request_id(request_id).await {
                    return Some(trace);
                }
            }

            response_rx.recv().await.flatten()
        } else {
            let mem = self.memory_store.read().await;
            mem.iter().find(|t| t.request_id == request_id).cloned()
        }
    }

    /// 获取最近的追踪摘要列表
    pub async fn get_recent_summaries(&self, limit: usize) -> Vec<RequestTraceSummary> {
        if let Some(ref tx) = self.event_tx {
            let (response_tx, mut response_rx) = mpsc::channel(1);
            let _ = tx.send(TraceEvent::GetRecent { limit, response_tx }).await;

            match response_rx.recv().await {
                Some(summaries) => summaries,
                None => Vec::new(),
            }
        } else {
            let mem = self.memory_store.read().await;
            mem.iter().rev().take(limit).map(RequestTraceSummary::from).collect()
        }
    }

    /// 清理旧数据
    pub async fn cleanup(&self, keep_recent: usize) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(TraceEvent::Cleanup { keep_recent }).await;
        } else {
            let mut mem = self.memory_store.write().await;
            while mem.len() > keep_recent {
                mem.pop_front();
            }
        }
    }

    /// 获取内存中的追踪数量
    pub async fn memory_count(&self) -> usize {
        self.memory_store.read().await.len()
    }

    /// 获取配置
    pub fn config(&self) -> &TraceCollectorConfig {
        &self.config
    }

    /// 检查 SQLite 存储是否可用
    pub fn has_sqlite(&self) -> bool {
        self.sqlite_store.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::trace_types::{SpanTrace, TraceMetadata};
    use std::time::Duration;

    #[tokio::test]
    async fn test_trace_collector_memory_only() {
        let collector = TraceCollector::memory_only().await.unwrap();

        assert!(collector.event_tx.is_some());
        assert!(!collector.has_sqlite());
        assert_eq!(collector.config().max_memory_traces, 100);
    }

    #[tokio::test]
    async fn test_trace_collector_record_and_query() {
        let collector = TraceCollector::memory_only().await.unwrap();

        let mut trace = RequestTrace::new("test-req-1".to_string());
        trace.session_id = Some("test-session-1".to_string());
        trace.input_summary = Some("Test input".to_string());
        trace.output_summary = Some("Test output".to_string());

        // 记录追踪
        collector.record(trace.clone()).await;

        // 等待异步事件处理
        tokio::time::sleep(Duration::from_millis(10)).await;

        // 查询追踪
        let result = collector.get_by_request_id("test-req-1").await;
        assert!(result.is_some());

        let queried = result.unwrap();
        assert_eq!(queried.request_id, "test-req-1");
        assert_eq!(queried.session_id, Some("test-session-1".to_string()));
        assert_eq!(queried.input_summary, Some("Test input".to_string()));
    }

    #[tokio::test]
    async fn test_trace_collector_add_span() {
        let collector = TraceCollector::memory_only().await.unwrap();

        let trace = RequestTrace::new("test-req-2".to_string());
        collector.record(trace).await;

        tokio::time::sleep(Duration::from_millis(10)).await;

        let span = SpanTrace::new(
            "test-req-2".to_string(),
            OperationKind::Planner,
            "Planning phase",
        )
        .with_attribute("step", 1i64);

        collector.add_span("test-req-2", span.clone()).await;

        tokio::time::sleep(Duration::from_millis(10)).await;

        let result = collector.get_by_request_id("test-req-2").await;
        assert!(result.is_some());

        let trace = result.unwrap();
        assert_eq!(trace.spans.len(), 1);
        assert_eq!(trace.spans[0].operation_kind, OperationKind::Planner);
    }

    #[tokio::test]
    async fn test_trace_collector_update_status() {
        let collector = TraceCollector::memory_only().await.unwrap();

        let trace = RequestTrace::new("test-req-3".to_string());
        collector.record(trace).await;

        tokio::time::sleep(Duration::from_millis(10)).await;

        collector.update_status("test-req-3", TraceStatus::Success).await;

        tokio::time::sleep(Duration::from_millis(10)).await;

        let result = collector.get_by_request_id("test-req-3").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().status, TraceStatus::Success);
    }

    #[tokio::test]
    async fn test_trace_collector_get_recent_summaries() {
        let collector = TraceCollector::memory_only().await.unwrap();

        // 记录多个追踪
        for i in 0..5 {
            let trace = RequestTrace::new(format!("test-req-{}", i))
                .with_input_summary(format!("Input {}", i));
            collector.record(trace).await;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;

        let summaries = collector.get_recent_summaries(3).await;
        assert_eq!(summaries.len(), 3);

        // 验证摘要内容
        for summary in &summaries {
            assert!(summary.request_id.starts_with("test-req-"));
        }
    }

    #[tokio::test]
    async fn test_trace_collector_lru_eviction() {
        let config = TraceCollectorConfig {
            max_memory_traces: 3,
            sqlite_path: None,
            enable_async_events: true,
        };
        let collector = TraceCollector::new(config).await.unwrap();

        // 记录 5 个追踪（超过限制）
        for i in 0..5 {
            let trace = RequestTrace::new(format!("test-req-{}", i));
            collector.record(trace).await;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;

        // 验证内存中只保留最近的 3 个
        assert_eq!(collector.memory_count().await, 3);

        // 最早的请求应该被淘汰
        assert!(collector.get_by_request_id("test-req-0").await.is_none());
        assert!(collector.get_by_request_id("test-req-1").await.is_none());

        // 最近的请求应该还在
        assert!(collector.get_by_request_id("test-req-2").await.is_some());
        assert!(collector.get_by_request_id("test-req-3").await.is_some());
        assert!(collector.get_by_request_id("test-req-4").await.is_some());
    }

    #[tokio::test]
    async fn test_trace_collector_cleanup() {
        let collector = TraceCollector::memory_only().await.unwrap();

        // 记录 5 个追踪
        for i in 0..5 {
            let trace = RequestTrace::new(format!("test-req-{}", i));
            collector.record(trace).await;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;

        assert_eq!(collector.memory_count().await, 5);

        // 清理，只保留最近的 2 个
        collector.cleanup(2).await;

        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(collector.memory_count().await, 2);
    }

    #[tokio::test]
    async fn test_sqlite_trace_store_in_memory() {
        let store = SqliteTraceStore::in_memory().unwrap();

        let mut trace = RequestTrace::new("sqlite-test-1".to_string());
        trace.session_id = Some("session-1".to_string());
        trace.metadata = Some(
            TraceMetadata::new()
                .with_user_id("user-1")
                .with_source("Test")
        );

        // 保存追踪
        store.save_request(&trace).await.unwrap();

        // 查询追踪
        let result = store.get_by_request_id("sqlite-test-1").await.unwrap();
        assert!(result.is_some());

        let queried = result.unwrap();
        assert_eq!(queried.request_id, "sqlite-test-1");
        assert_eq!(queried.session_id, Some("session-1".to_string()));
    }

    #[tokio::test]
    async fn test_sqlite_trace_store_spans() {
        let store = SqliteTraceStore::in_memory().unwrap();

        let mut trace = RequestTrace::new("sqlite-test-2".to_string());

        // 添加多个 Spans
        trace.add_span(SpanTrace::new(
            "sqlite-test-2".to_string(),
            OperationKind::Planner,
            "Planning",
        ));
        trace.add_span(SpanTrace::new(
            "sqlite-test-2".to_string(),
            OperationKind::LlmCall,
            "LLM Call 1",
        ));
        trace.add_span(SpanTrace::new(
            "sqlite-test-2".to_string(),
            OperationKind::ToolExecution,
            "Tool execution",
        ));

        // 保存追踪
        store.save_request(&trace).await.unwrap();

        // 查询并验证 Spans
        let result = store.get_by_request_id("sqlite-test-2").await.unwrap();
        assert!(result.is_some());

        let queried = result.unwrap();
        assert_eq!(queried.spans.len(), 3);

        // 验证操作类型
        let operation_kinds: Vec<_> = queried.spans.iter()
            .map(|s| s.operation_kind.as_str())
            .collect();
        assert!(operation_kinds.contains(&"planner"));
        assert!(operation_kinds.contains(&"llm_call"));
        assert!(operation_kinds.contains(&"tool_execution"));
    }

    #[tokio::test]
    async fn test_sqlite_trace_store_get_recent() {
        let store = SqliteTraceStore::in_memory().unwrap();

        // 保存多个追踪
        for i in 0..5 {
            let trace = RequestTrace::new(format!("sqlite-req-{}", i))
                .with_input_summary(format!("Input {}", i));
            store.save_request(&trace).await.unwrap();
        }

        // 获取最近的 3 个摘要
        let summaries = store.get_recent_summaries(3).await.unwrap();
        assert_eq!(summaries.len(), 3);

        // 验证摘要内容
        for summary in &summaries {
            assert!(summary.request_id.starts_with("sqlite-req-"));
            assert!(summary.span_count >= 0);
        }
    }

    #[test]
    fn test_parse_trace_status() {
        assert_eq!(parse_trace_status("running"), Some(TraceStatus::Running));
        assert_eq!(parse_trace_status("success"), Some(TraceStatus::Success));
        assert_eq!(parse_trace_status("failure"), Some(TraceStatus::Failure));
        assert_eq!(parse_trace_status("failed"), Some(TraceStatus::Failure));
        assert_eq!(parse_trace_status("cancelled"), Some(TraceStatus::Cancelled));
        assert_eq!(parse_trace_status("canceled"), Some(TraceStatus::Cancelled));
        assert_eq!(parse_trace_status("unknown"), None);
    }

    #[test]
    fn test_parse_operation_kind() {
        assert_eq!(parse_operation_kind("orchestrator"), Some(OperationKind::Orchestrator));
        assert_eq!(parse_operation_kind("planner"), Some(OperationKind::Planner));
        assert_eq!(parse_operation_kind("critic"), Some(OperationKind::Critic));
        assert_eq!(parse_operation_kind("llm_call"), Some(OperationKind::LlmCall));
        assert_eq!(parse_operation_kind("tool_execution"), Some(OperationKind::ToolExecution));
        assert_eq!(parse_operation_kind("memory"), Some(OperationKind::Memory));
        assert_eq!(parse_operation_kind("rag_retrieval"), Some(OperationKind::RagRetrieval));
        assert_eq!(parse_operation_kind("custom_op"), Some(OperationKind::Custom("custom_op".to_string())));
    }

    #[test]
    fn test_parse_span_status() {
        assert_eq!(parse_span_status("running"), Some(SpanStatus::Running));
        assert_eq!(parse_span_status("success"), Some(SpanStatus::Success));
        assert_eq!(parse_span_status("failure"), Some(SpanStatus::Failure));
        assert_eq!(parse_span_status("failed"), Some(SpanStatus::Failure));
        assert_eq!(parse_span_status("unknown"), None);
    }

    #[test]
    fn test_request_trace_summary_from_trace() {
        let mut trace = RequestTrace::new("summary-test".to_string());
        trace.session_id = Some("session-1".to_string());
        trace.input_summary = Some("Test input".to_string());

        trace.record_llm_call();
        trace.record_llm_call();
        trace.record_tool_execution();
        trace.mark_success(Duration::from_millis(100));

        let summary = RequestTraceSummary::from(&trace);

        assert_eq!(summary.request_id, "summary-test");
        assert_eq!(summary.session_id, Some("session-1".to_string()));
        assert_eq!(summary.status, TraceStatus::Success);
        assert!(summary.duration_ms.is_some());
        assert_eq!(summary.llm_calls_count, Some(2));
        assert_eq!(summary.tool_executions_count, Some(1));
    }
}
