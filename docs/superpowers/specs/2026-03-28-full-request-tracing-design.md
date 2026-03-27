# 全链路追踪系统设计规范

## 1. 概述

### 1.1 目标

构建一个完整的请求追踪系统，让开发者和用户能够清晰地看到每次用户请求从进入到退出的完整执行过程，包括：
- 经过了哪些组件（Orchestrator → Planner → Critic → Executor）
- 调用了哪些 LLM（模型、Token 数、延迟）
- 执行了哪些工具（工具名、参数、执行时间）
- 每个步骤的成功/失败状态

### 1.2 设计原则

1. **低开销** - 追踪不应显著影响系统性能（目标 <5% 延迟增加）
2. **结构化** - 所有 trace 数据为结构化格式，支持查询和分析
3. **渐进式** - P0 实现核心追踪，P1 扩展可视化
4. **可观测性统一** - 与现有 tracing/metrics 系统无缝集成

---

## 2. 架构设计

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        Application Code                          │
│  (Orchestrator, Planner, Critic, ToolExecutor, LLM clients)     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ tracing spans
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    tracing-subscriber                            │
│         (JSON output → 日志文件 / 终端 / 外部采集器)               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ async channel (bounded)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      TraceCollector                              │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Memory Store (DashMap)                                   │  │
│  │  - 最近 100 条请求快速访问                                  │  │
│  │  - LRU 淘汰机制                                            │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  SQLite Store                                             │  │
│  │  - 全量持久化                                              │  │
│  │  - 支持 SQL 查询（按时间/状态/操作类型筛选）                 │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
┌─────────────────────────┐     ┌─────────────────────────┐
│   Terminal ASCII View   │     │      Web UI Dashboard   │
│   (精简树状图)           │     │   (火焰图/时序图/查询)    │
└─────────────────────────┘     └─────────────────────────┘
```

### 2.2 组件职责

| 组件 | 职责 |
|------|------|
| **tracing spans** | 在代码中埋点，自动继承父子关系 |
| **TraceCollector** | 异步收集 spans，构建完整 trace |
| **Memory Store** | 热数据快速访问 |
| **SQLite Store** | 冷数据持久化，支持历史查询 |
| **Terminal ASCII** | 开发调试时快速查看 |
| **Web UI** | 生产环境可视化分析 |

---

## 3. 数据模型

### 3.1 RequestTrace（请求级追踪）

```rust
pub struct RequestTrace {
    /// 唯一请求 ID（UUID v4）
    pub request_id: String,
    /// 用户 ID（可选，用于多用户场景）
    pub user_id: Option<String>,
    /// 会话 ID
    pub session_id: String,
    /// 请求开始时间
    pub start_time: SystemTime,
    /// 请求结束时间
    pub end_time: Option<SystemTime>,
    /// 请求状态
    pub status: TraceStatus,
    /// 所有 span 的集合
    pub spans: Vec<SpanTrace>,
    /// 根 span ID（用于重建树状结构）
    pub root_span_id: Option<String>,
    /// 元数据
    pub metadata: TraceMetadata,
}

pub enum TraceStatus {
    Running,
    Completed,
    Failed { error: String },
    Timeout,
}

pub struct TraceMetadata {
    /// 用户原始输入
    pub user_input: Option<String>,
    /// 最终响应
    pub final_response: Option<String>,
    /// ReAct 循环次数
    pub react_steps: u32,
    /// 总 Token 消耗
    pub total_tokens: u64,
    /// 总延迟（毫秒）
    pub total_latency_ms: Option<u64>,
}
```

### 3.2 SpanTrace（操作级追踪）

```rust
pub struct SpanTrace {
    /// Span 唯一 ID
    pub span_id: String,
    /// 父 Span ID（用于构建树状结构）
    pub parent_span_id: Option<String>,
    /// 操作类型
    pub operation: OperationKind,
    /// 操作名称（如 "read_file", "deepseek-chat"）
    pub name: String,
    /// 开始时间
    pub start_time: SystemTime,
    /// 结束时间
    pub end_time: Option<SystemTime>,
    /// 持续时间（毫秒）
    pub duration_ms: Option<u64>,
    /// Span 状态
    pub status: SpanStatus,
    /// 属性集合
    pub attributes: HashMap<String, AttributeValue>,
}

pub enum OperationKind {
    /// Orchestrator 处理
    Orchestrator,
    /// Planner 思考
    Planner,
    /// Critic 审查
    Critic,
    /// LLM 调用
    LlmCall,
    /// 工具执行
    ToolExecution,
    /// 内存操作
    Memory,
    /// 响应流式输出
    ResponseStream,
}

pub enum SpanStatus {
    Ok,
    Error { message: String },
    Timeout { timeout_ms: u64 },
}

pub enum AttributeValue {
    String(String),
    Int(i64),
    Float(f64),
    Json(serde_json::Value),
}
```

### 3.3 关键属性定义

**LLM Call Span** 属性：
| 属性名 | 类型 | 描述 |
|--------|------|------|
| `model` | String | 模型名称（如 "deepseek-chat"） |
| `prompt_tokens` | Int | Prompt Token 数 |
| `completion_tokens` | Int | Completion Token 数 |
| `total_tokens` | Int | 总 Token 数 |
| `latency_ms` | Int | 延迟（毫秒） |
| `streaming` | Bool | 是否流式输出 |

**Tool Execution Span** 属性：
| 属性名 | 类型 | 描述 |
|--------|------|------|
| `tool_name` | String | 工具名称 |
| `parameters` | Json | 输入参数 |
| `result_size` | Int | 结果大小（字节） |
| `execution_time_ms` | Int | 执行时间 |
| `success` | Bool | 是否成功 |

---

## 4. 实现设计

### 4.1 TraceCollector

```rust
pub struct TraceCollector {
    /// 内存存储（最近 N 条）
    memory_store: Arc<DashMap<String, RequestTrace>>,
    /// SQLite 存储
    sqlite_store: Arc<SqliteTraceStore>,
    /// 配置
    config: TraceCollectorConfig,
    /// 异步通道发送端
    tx: mpsc::Sender<TraceEvent>,
}

pub struct TraceCollectorConfig {
    /// 内存中保留的最大请求数
    pub max_memory_traces: usize,
    /// 是否启用 SQLite 持久化
    pub enable_persistence: bool,
    /// SQLite 数据库路径
    pub db_path: PathBuf,
    /// Trace 保留天数（0 = 永久）
    pub retention_days: u32,
}

/// Trace 事件（通过 channel 异步处理）
pub enum TraceEvent {
    /// 请求开始
    RequestStarted {
        request_id: String,
        session_id: String,
        user_id: Option<String>,
    },
    /// Span 开始
    SpanStarted {
        request_id: String,
        span: SpanTrace,
    },
    /// Span 结束
    SpanEnded {
        request_id: String,
        span_id: String,
        duration_ms: u64,
        status: SpanStatus,
        attributes: HashMap<String, AttributeValue>,
    },
    /// 请求结束
    RequestCompleted {
        request_id: String,
        status: TraceStatus,
        metadata: TraceMetadata,
    },
}

impl TraceCollector {
    /// 创建新的 TraceCollector
    pub async fn new(config: TraceCollectorConfig) -> Self;

    /// 获取请求追踪（优先内存，未找到则查 SQLite）
    pub async fn get_trace(&self, request_id: &str) -> Option<RequestTrace>;

    /// 获取最近的追踪列表
    pub async fn get_recent_traces(&self, limit: usize) -> Vec<RequestTraceSummary>;

    /// 查询追踪（支持条件筛选）
    pub async fn query_traces(&self, filter: TraceFilter) -> Vec<RequestTraceSummary>;

    /// 删除旧的追踪
    pub async fn cleanup_old_traces(&self, older_than: Duration) -> usize;
}
```

### 4.2 与 tracing 集成

使用 `tracing-subscriber` 的 Layer trait，将 spans 导出到 TraceCollector：

```rust
pub struct TraceCollectionLayer {
    tx: mpsc::Sender<TraceEvent>,
}

impl<S: Subscriber> Layer<S> for TraceCollectionLayer {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        // Span 开始，发送事件到 TraceCollector
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        // Span 结束，计算持续时间并发送事件
    }
}

// 初始化时注册 Layer
pub fn init_trace_collection(collector: Arc<TraceCollector>) {
    let layer = TraceCollectionLayer::new(collector);
    tracing_subscriber::registry()
        .with(layer)
        .with(EnvFilter::from_default_env())
        .init();
}
```

### 4.3 SQLite Schema

```sql
-- 请求追踪表
CREATE TABLE IF NOT EXISTS request_traces (
    request_id TEXT PRIMARY KEY,
    user_id TEXT,
    session_id TEXT NOT NULL,
    start_time INTEGER NOT NULL,  -- Unix timestamp (ms)
    end_time INTEGER,
    status TEXT NOT NULL,
    user_input TEXT,
    final_response TEXT,
    react_steps INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    total_latency_ms INTEGER,
    created_at INTEGER DEFAULT (strftime('%s', 'now') * 1000)
);

-- Span 表
CREATE TABLE IF NOT EXISTS span_traces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id TEXT NOT NULL,
    span_id TEXT NOT NULL,
    parent_span_id TEXT,
    operation TEXT NOT NULL,
    name TEXT NOT NULL,
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    duration_ms INTEGER,
    status TEXT NOT NULL,
    attributes TEXT,  -- JSON 格式
    FOREIGN KEY (request_id) REFERENCES request_traces(request_id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_request_traces_session ON request_traces(session_id);
CREATE INDEX IF NOT EXISTS idx_request_traces_start_time ON request_traces(start_time DESC);
CREATE INDEX IF NOT EXISTS idx_span_traces_request ON span_traces(request_id);
CREATE INDEX IF NOT EXISTS idx_span_traces_operation ON span_traces(operation);
```

---

## 5. 可视化设计

### 5.1 终端 ASCII 树

**格式示例**：

```
Request: 550e8400-e29b-41d4-a716-446655440000
Session: default
Status: ✓ Completed (2.3s)
Tokens: 3.2k | React Steps: 4
├─ [350ms] Planner::think
│  └─ [320ms] LLM::chat (deepseek-chat, 1.2k tokens) ✓
├─ [180ms] Critic::review
│  └─ [160ms] LLM::chat (deepseek-chat, 800 tokens) ✓
├─ [1.5s] ToolExecutor::execute
│  ├─ [200ms] read_file (config.toml) ✓
│  └─ [1.2s] shell (cargo build) ✓
└─ [100ms] Response::stream ✓
```

**实现**：

```rust
pub fn render_ascii_tree(trace: &RequestTrace) -> String {
    let mut output = Vec::new();

    // Header
    output.push(format!("Request: {}", trace.request_id));
    output.push(format!("Status: {} ({})",
        status_icon(&trace.status),
        format_duration(trace.total_latency_ms())
    ));

    // Build tree
    let root = trace.root_span_id.as_ref().unwrap();
    render_span_tree(trace, root, &mut output, "", true);

    output.join("\n")
}

fn render_span_tree(
    trace: &RequestTrace,
    span_id: &str,
    output: &mut Vec<String>,
    prefix: &str,
    is_last: bool,
) {
    // 递归渲染子 span
}
```

### 5.2 Web UI Dashboard

**页面结构**：

```
┌─────────────────────────────────────────────────────────────┐
│  Request Traces Dashboard                     [Auto-refresh] │
├─────────────────────────────────────────────────────────────┤
│  Filters:                                                   │
│  [Status: All ▼] [Operation: All ▼] [Time range: 1h ▼]      │
│  [Search by request_id / session_id]                        │
├─────────────────────────────────────────────────────────────┤
│  Recent Requests                                            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ ✓ 550e8400...  | default | 2.3s  | 4 steps | 3.2k  │   │
│  │ ✗ 661f9511...  | default | 1.1s  | 2 steps | 1.8k  │   │
│  │ ⏱ 772a0622...  | user123 | 30s   | 1 step  | 500   │   │
│  └─────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│  [Click to expand details → Flame Graph / Timeline View]   │
└─────────────────────────────────────────────────────────────┘
```

**火焰图（Flame Graph）**：

```

  ┌────────────────────────────────────────────────────────────┐
  │  Orchestrator::process_message [2300ms]                   │
  │  ┌──────────────────────────────────────────────────────┐ │
  │  │  Planner::think [350ms]                              │ │
  │  │  ┌────────────────────────────────────────────────┐  │ │
  │  │  │  LLM::chat (deepseek-chat) [320ms]            │  │ │
  │  │  └────────────────────────────────────────────────┘  │ │
  │  └──────────────────────────────────────────────────────┘ │
  │  ┌──────────────────────────────────────────────────────┐ │
  │  │  Critic::review [180ms]                              │ │
  │  │  ┌────────────────────────────────────────────────┐  │ │
  │  │  │  LLM::chat (deepseek-chat) [160ms]            │  │ │
  │  │  └────────────────────────────────────────────────┘  │ │
  │  └──────────────────────────────────────────────────────┘ │
  │  ┌──────────────────────────────────────────────────────┐ │
  │  │  ToolExecutor::execute [1500ms]                      │ │
  │  │  ┌───────────────┐ ┌───────────────────────────────┐ │ │
  │  │  │  read_file    │ │  shell (cargo build)          │ │ │
  │  │  │  [200ms]      │ │  [1200ms]                     │ │ │
  │  │  └───────────────┘ └───────────────────────────────┘ │ │
  │  └──────────────────────────────────────────────────────┘ │
  └────────────────────────────────────────────────────────────┘
```

**API 设计**：

```rust
// GET /api/traces?limit=50&status=completed&session=default
pub async fn list_traces(
    Query(filter): Query<TraceFilter>,
    Extension(collector): Extension<Arc<TraceCollector>>,
) -> Json<Vec<RequestTraceSummary>>;

// GET /api/traces/:request_id
pub async fn get_trace(
    Path(request_id): Path<String>,
    Extension(collector): Extension<Arc<TraceCollector>>,
) -> Json<RequestTrace>;

// DELETE /api/traces/:request_id
pub async fn delete_trace(
    Path(request_id): Path<String>,
    Extension(collector): Extension<Arc<TraceCollector>>,
) -> StatusCode;
```

---

## 6. 实施计划

### P0 - 核心追踪（1-2 天）

- [ ] 定义数据模型（RequestTrace, SpanTrace, OperationKind 等）
- [ ] 实现 TraceCollector（内存存储 + SQLite 持久化）
- [ ] 实现 TraceCollectionLayer（与 tracing 集成）
- [ ] 在关键路径添加 span 埋点：
  - Orchestrator::process_message
  - Planner::think
  - Critic::review
  - ToolExecutor::execute
  - LLM clients

### P1 - 终端可视化（0.5 天）

- [ ] 实现 `render_ascii_tree()` 函数
- [ ] 添加 CLI 命令：`/traces` 查看最近追踪
- [ ] 添加 CLI 命令：`/trace <request_id>` 查看单个追踪详情

### P2 - Web UI（1 天）

- [ ] 扩展 metrics.html 或新建 traces.html
- [ ] 实现 API 端点（list_traces, get_trace）
- [ ] 实现请求列表和火焰图/时序图组件
- [ ] 添加筛选和搜索功能

### P3 - 增强功能（可选）

- [ ] 慢查询分析（找出超过阈值的请求）
- [ ] Trace 导出（JSON, Chrome Performance 格式）
- [ ] 告警集成（错误率/延迟超阈值）

---

## 7. 测试计划

### 单元测试

- [ ] TraceCollector 的内存存储测试
- [ ] SQLite 持久化测试
- [ ] ASCII 树渲染测试

### 集成测试

- [ ] 完整请求链路追踪测试
- [ ] 并发请求追踪测试
- [ ] 大量 spans 性能测试

---

## 8. 依赖变更

### 新增依赖

```toml
[dependencies]
# 已有的 tracing 相关
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"

# SQLite (如果尚未使用)
rusqlite = { version = "0.31", features = ["bundled"] }

# 并发数据结构
dashmap = "5.5"
```

---

## 9. 验收标准

1. **功能完整** - 每个请求都有完整的追踪记录
2. **性能开销** - 延迟增加 <5%
3. **终端可视化** - `/traces` 命令可正常显示
4. **Web 可视化** - 可查看火焰图/时序图
5. **查询能力** - 可按 session_id、状态、时间范围筛选
6. **持久化** - 重启后历史 traces 不丢失

---

## 10. 未来扩展

1. **分布式追踪** - 导出到 Jaeger/Zipkin
2. **Profiling 集成** - CPU/Memory 热点分析
3. **智能分析** - 自动识别性能瓶颈和异常模式
4. **告警系统** - 错误率/延迟超阈值通知
