# Tool Router Architecture

## 目标

把“工具选择”从 prompt 技巧升级成运行时治理系统，降低以下退化：

- 明明可以直接回答，却先去 `ls`、`cat`、`search`
- 明明有专用工具，却绕到通用搜索或深搜
- Critic 过度介入，导致无意义重试
- 模型升级后路径漂移，但没有回归基准

## 四层结构

### 1. 分流层

位置：

- [`src/tool_router.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tool_router.rs)
- [`src/agent.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/agent.rs)
- [`src/gateway/runtime.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/gateway/runtime.rs)

职责：

- 对确定性极强的请求直接分发到专用工具
- 跳过 Planner，减少 token 和耗时

当前直达场景：

- 天气 -> `weather`
- 新闻 -> `news`
- 汇率 -> `exchange_rate`
- 行情 -> `market_quote`
- 比分 -> `sports_score`

直达执行会直接产出工具调用、工具结果和最终回复事件，并写回上下文。

### 2. 约束层

位置：

- [`src/tool_policy.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tool_policy.rs)
- [`src/tools/metadata.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tools/metadata.rs)

职责：

- 查询分类
- 候选工具过滤
- capability group / cost-to-benefit 排序
- rewrite / guard

Metadata 现在不仅描述风险和新鲜度，还描述：

- `capability_group`
- `capability_subgroup`
- `latency_class`
- `token_cost_class`
- `api_cost_class`
- `overall_cost_class`
- `preferred_rank`

约束层会先做适配性过滤，再按“适配度 + 成本 + 默认优先级”排序，最后只给 Planner 最多 5 个候选工具。

### 3. 反馈层

位置：

- [`src/react/critic.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/react/critic.rs)
- [`src/react/loop_.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/react/loop_.rs)
- [`config/prompts/critic.md`](/Users/g/Documents/GitHub/feature/org_20260321/config/prompts/critic.md)
- [`config/default.toml`](/Users/g/Documents/GitHub/feature/org_20260321/config/default.toml)

职责：

- Critic 不再直接输出“重做命令”
- 改为输出结构化评分：
  - `score`
  - `reason`
  - `retry_recommended`
  - `blocking_risk`

运行时行为：

- 只有低于阈值时才考虑纠偏
- 只有 `retry_recommended=true` 才注入修正建议
- 单次对话有 `max_self_corrections` 预算，避免死循环

### 4. 守卫层

位置：

- [`tests/golden/tool_routing_cases.json`](/Users/g/Documents/GitHub/feature/org_20260321/tests/golden/tool_routing_cases.json)
- [`tests/tool_routing_golden.rs`](/Users/g/Documents/GitHub/feature/org_20260321/tests/tool_routing_golden.rs)

职责：

- 用 Golden Dataset 固化关键路径
- 不只验证输出，还验证：
  - query kind
  - expected direct route
  - expected candidate tools
  - forbidden tools
  - 是否允许长期记忆

这能直接监控“从直接回答退化成绕路搜索”的路径漂移。

## 可观测性

位置：

- [`src/observability/mod.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/observability/mod.rs)

新增指标：

- `direct_route_hits`
- `route_drift_count`
- `policy_rewrites`
- `policy_blocks`

其中：

- `direct_route_hits` 用于观察专用工具的零样本命中率
- `route_drift_count` 用于观察 Critic 低分或路径偏移的频率

## 当前边界

当前 deterministic router 只直达“结构化专用工具”，没有把 GitHub 仓库分析也完全跳过 Planner。原因是 GitHub 仓库分析虽然工具抓取确定，但最终总结仍然适合让 Planner 组织语言。

如果后续要继续推进，可以考虑：

1. 为 GitHub 和社交帖文增加独立 source adapter
2. 为 direct answer 增加轻量模板化回答器
3. 在 observability 中加入 Critic score 分布直方图
4. 扩充 Golden Dataset，覆盖更多历史 bad cases
