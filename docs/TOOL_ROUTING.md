# Tool Routing

## 目标

降低工具误选，尤其是这些高频重叠场景：

- 外部 GitHub 仓库分析：`github_repo_inspect` vs `search` vs `browser`
- 本地文件读取：`cat` vs `code_read`
- 本地目录探测：`ls` vs 误用于外部仓库问题

## 当前策略

### 1. 先收窄候选工具，再让 Planner 选择

运行时会先根据用户输入做一层硬路由：

- 如果问题包含外部 GitHub 仓库 URL，并且在问架构、技术栈、源码结构、`package.json`、`Cargo.toml`、README 等内容：
  - 只保留 `github_repo_inspect`、`search`、`browser`、`deep_search`
  - 默认优先级：`github_repo_inspect` > `search` > `browser` > `deep_search`
- 这样会直接排除 `cat`、`code_read`、`ls`、`shell` 等本地工具

实现位置：

- [`tool_routing.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tool_routing.rs)
- [`agent.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/agent.rs)
- [`runtime.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/gateway/runtime.rs)
- [`web.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/bin/web.rs)

### 2. 专用工具负责专用语义

GitHub 仓库的 `repo/blob/tree` 检查只由 `github_repo_inspect` 负责。

`search` 现在只负责通用网页文本抓取；如果传入 GitHub 仓库 URL，会直接返回错误并提示改用 `github_repo_inspect`。

实现位置：

- [`github_repo_inspect.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tools/github_repo_inspect.rs)
- [`search.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tools/search.rs)

### 3. 工具返回后尽快收口

当 `github_repo_inspect` 已经返回 `repo_summary`、`detected_stack`、`top_level_directories`、`key_files_found` 或 `file_snippets` 时，Planner 会被明确引导直接回答，而不是继续试探本地工具。

实现位置：

- [`loop_.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/react/loop_.rs)
- [`system.md`](/Users/g/Documents/GitHub/feature/org_20260321/config/prompts/system.md)

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
