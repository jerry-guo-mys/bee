# Tool Routing

## 目标

降低工具误选，尤其是这些高频重叠场景：

- 外部 GitHub 仓库分析：`github_repo_inspect` vs `search` vs `browser`
- 本地文件读取：`cat` vs `code_read`
- 本地目录探测：`ls` vs 误用于外部仓库问题

## 当前策略

当前实现已经升级成四层：

- 分流层：高确定性请求直接命中专用工具
- 约束层：Metadata + capability/cost 排序收窄候选工具
- 反馈层：Critic 打分而不是直接发号施令
- 守卫层：Golden Dataset 固化预期路径

详见：

- [`TOOL_ROUTER_ARCHITECTURE.md`](/Users/g/Documents/GitHub/feature/org_20260321/docs/TOOL_ROUTER_ARCHITECTURE.md)

### 1. 先收窄候选工具，再让 Planner 选择

运行时会先根据用户输入做一层硬路由：

- 如果问题包含外部 GitHub 仓库 URL，并且在问架构、技术栈、源码结构、`package.json`、`Cargo.toml`、README 等内容：
  - 只保留 `github_repo_inspect`、`search`、`browser`、`deep_search`
  - 默认优先级：`github_repo_inspect` > `search` > `browser` > `deep_search`
- 这样会直接排除 `cat`、`code_read`、`ls`、`shell` 等本地工具

实现位置：

- [`tool_policy.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tool_policy.rs)
- [`agent.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/agent.rs)
- [`runtime.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/gateway/runtime.rs)
- [`web.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/bin/web.rs)

### 1.1 工具元数据

所有工具现在都支持统一元数据：

- `scope`
- `intents`
- `risk`
- `output_shape`
- `supports_freshness`
- `supports_side_effects`

这些元数据会进入：

- tool schema
- 路由过滤
- capability 分组
- 成本排序
- 执行前 guardrail
- 观测与审计

新增 metadata 字段包括：

- `capability_group`
- `capability_subgroup`
- `latency_class`
- `token_cost_class`
- `api_cost_class`
- `overall_cost_class`
- `preferred_rank`

实现位置：

- [`metadata.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tools/metadata.rs)
- [`registry.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tools/registry.rs)

### 2. 专用工具负责专用语义

GitHub 仓库的 `repo/blob/tree` 检查只由 `github_repo_inspect` 负责。

`search` 现在只负责通用网页文本抓取；如果传入 GitHub 仓库 URL，会直接返回错误并提示改用 `github_repo_inspect`。

实现位置：

- [`github_repo_inspect.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tools/github_repo_inspect.rs)
- [`search.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tools/search.rs)

### 3. 执行前 rewrite / guard

在 planner 产出工具调用后、真正执行前，会经过统一策略层：

- rewrite：如 `search + GitHub repo URL` 自动改写为 `github_repo_inspect`
- guard：如“直接解释型问题”禁止再调用 `ls`、`cat`、`code_read`、`shell`

实现位置：

- [`tool_policy.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tool_policy.rs)
- [`loop_.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/react/loop_.rs)

### 4. 工具返回后尽快收口

当 `github_repo_inspect` 已经返回 `repo_summary`、`detected_stack`、`top_level_directories`、`key_files_found` 或 `file_snippets` 时，Planner 会被明确引导直接回答，而不是继续试探本地工具。

实现位置：

- [`loop_.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/react/loop_.rs)
- [`system.md`](/Users/g/Documents/GitHub/feature/org_20260321/config/prompts/system.md)

### 5. 结构化输出与可观测性

关键工具现在会统一输出：

- `tool`
- `summary`
- `sufficient_to_answer`
- `data`

并新增策略层观测：

- `policy_rewrites`
- `policy_blocks`
- `direct_route_hits`
- `route_drift_count`

实现位置：

- [`output.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tools/output.rs)
- [`observability/mod.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/observability/mod.rs)

## 路由矩阵

| 用户问题类型 | 首选工具 | 次选工具 | 禁止/避免 |
| --- | --- | --- | --- |
| 外部 GitHub 仓库架构/技术栈/源码结构 | `github_repo_inspect` | `search`, `browser`, `deep_search` | `cat`, `code_read`, `ls`, `shell` |
| GitHub 仓库链接/开源地址 | 直接回答或 `github_repo_inspect` | `search` | `cat`, `ls` |
| 通用网页正文抓取 | `search` | `browser` | `cat`, `code_read` |
| 本地仓库文件阅读 | `cat`, `code_read` | `code_grep`, `ls` | `github_repo_inspect` |
| 本地代码结构分析 | `code_read`, `code_grep` | `cat` | `github_repo_inspect`, `search` |

## 设计原则

1. 不让一个工具同时承担“外部网页抓取”和“外部仓库结构分析”。
2. 不把“本地文件系统”和“远程仓库文件”混成同一类读取问题。
3. 优先通过运行时硬约束减少错误选择，而不是单纯依赖 prompt。
4. 工具结果尽量结构化，便于模型判断“信息已足够回答”。
