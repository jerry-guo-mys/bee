# 四层工具治理落地复盘

## 背景

这一轮改造并不是从零开始设计一套全新的 Agent 架构，而是在持续修复真实问题的过程中，逐步把零散补丁收敛成一套可解释、可验证、可演进的工程方案。

最初暴露的问题看起来很分散：

- 外部 GitHub 仓库分析时误用本地 `ls`、`cat`、`code_read`
- “今天有什么新闻”返回旧日期内容
- “吉隆坡明天天气”先抓 Wikipedia，再漂到 `deep_search`
- `search`、`browser`、`github_repo_inspect`、`deep_search` 之间功能交叉
- Critic 经常像“杠精”，明明结果已经够了，还推动 Planner 继续搜索
- 前端思考步骤冗余，放大了系统绕路感

这些问题表面不同，内核却是一致的：工具选择缺少系统化治理。模型虽然能“猜到”一部分正确工具，但当工具数量增加、上下文变长、恢复提示叠加之后，路由会快速退化。

所以这一轮真正完成的，不是某几个 bugfix，而是把“工具调用”从 prompt 依赖型逻辑，升级成一个四层治理体系：

1. 分流层
2. 约束层
3. 反馈层
4. 守卫层

这篇复盘重点总结这四层是如何落地的、为什么这样设计、已经解决了哪些问题、还留下了哪些边界。

## 一、从个案修复到抽象治理

一开始，系统的修复方式更接近 case by case。

例如：

- GitHub 仓库分析出错，就给 GitHub URL 加一条 rewrite
- 天气抓错页面，就加一个天气专用工具
- X/Twitter 读不到内容，就加几个 mirror fallback

这些修复都有效，但如果继续沿着这条路走，系统会很快进入两个坏状态。

第一，逻辑分散。不同 case 的规则落在 `search`、`deep_search`、`planner prompt`、`loop_`、前端和配置文件里，系统行为越来越难预测。

第二，抽象层级不一致。有些问题在工具里修，有些在策略里修，有些在 Critic 里修，有些靠 prompt 硬压。这种状态下，即使单个问题被修掉，后面也很容易以另一种形式复发。

所以这轮改造最关键的判断是：不再继续“遇到问题就补一条 if”，而是把这类 if 背后的模式抽象出来。最终我们把它们归并为四类职责：

- 哪些请求应该绕过 Planner，直接走确定性工具
- 哪些工具在当前场景下根本不该给 Planner 看见
- 哪些结果需要 Critic 介入，哪些结果应该直接放行
- 如何保证模型升级后不会又退回老路径

有了这个抽象，后续每次扩展专用工具或修复新问题时，就不必再从零决定“逻辑该写在哪”，而是能稳定落在对应层上。

## 二、分流层：把高确定性请求从 Planner 手里拿出来

### 为什么要做

不是所有请求都值得让大模型先“思考一圈”再决定怎么做。

像天气、新闻、汇率、股价、体育比分这类问题，有几个共同特点：

- 任务意图强确定性
- 输入模式稳定
- 输出结构清晰
- 用户对时效性和正确性的期待很高

在这种场景下，如果还让 Planner 在一堆通用工具里挑选，实际上是在制造无意义的不确定性。

例如“今天有什么新闻，推荐 5 条”，理论上完全可以直接走 `news`；如果非要先让 Planner 判断，它就可能选 `search`，再抓到百度、Google、Wikipedia 之类中间页面，然后又引入 Critic、恢复和多轮重试。

### 这次是怎么落地的

我们新增了 [`src/tool_router.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tool_router.rs)，引入 deterministic router。

当前已经支持直接分流的任务：

- 天气 -> `weather`
- 新闻 -> `news`
- 汇率 -> `exchange_rate`
- 行情 -> `market_quote`
- 比分 -> `sports_score`

运行方式不是“给 Planner 一个更强提示”，而是直接在运行时判断：

- 如果命中高确定性场景
- 且对应专用工具在当前 allowlist 内
- 就直接调用工具并返回结果
- 跳过 Planner

这样做有几个直接收益：

1. 降低 token 消耗。很多请求不需要进入完整 ReAct 循环。
2. 降低延迟。省掉了一轮甚至多轮模型规划。
3. 降低路径漂移。模型根本没有机会在这些场景里乱选工具。
4. 降低恢复链复杂度。错误从“Planner 选错 -> Critic 指正 -> 重试”变成“工具是否正常返回”。

### 经验

一个成熟的 Agent 系统，不应该把所有请求都交给 Planner。Planner 应该保留给那些：

- 问题结构复杂
- 工具组合不确定
- 需要多轮推理或跨信息源整合

而不是把天气和新闻这类高度结构化请求也交给它自由发挥。

## 三、约束层：Metadata 不只是描述信息，而是决策输入

### 为什么 metadata 之前还不够

此前系统里已经有一版 `ToolMetadata`，包含 scope、intent、risk、freshness 等字段，但它更像是“记录信息”，还不是真正的治理输入。

问题在于：

- 只知道工具是 `RemoteWeb`，不足以决定它是“新闻工具”还是“通用网页抓取”
- 只知道 risk 高低，不能帮助 Planner 在两个都可用的低风险工具里做成本更优的选择
- 缺少 capability hierarchy，就只能平铺所有工具给 Planner，看哪个名字更像

所以我们进一步扩展了 metadata。

### 这次新增了什么

在 [`src/tools/metadata.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/tools/metadata.rs) 里新增了：

- `capability_group`
- `capability_subgroup`
- `latency_class`
- `token_cost_class`
- `api_cost_class`
- `overall_cost_class`
- `preferred_rank`

这些字段的作用不是展示，而是进入约束层排序逻辑。

例如：

- `weather`、`news`、`exchange_rate`、`market_quote`、`sports_score` 都属于 `RealtimeData`
- `github_repo_inspect` 属于 `RepositoryAnalysis`
- `search` 属于 `WebResearch`
- `cat`、`ls`、`code_read` 属于 `LocalWorkspace`
- `shell` 属于 `SystemExecution`

这样 `tool_policy` 就不再是简单的“这个工具能不能用”，而是能进一步回答：

- 当前 query 更适合哪个 capability group
- 在同组内谁成本更低
- 在都满足条件时，谁的默认优先级更高

### 候选集收敛的意义

这一轮把 `tool_policy` 的收敛逻辑改成了：

1. 先按 query kind 做适配性过滤
2. 再按 capability 和成本排序
3. 最后只保留最多 5 个候选工具给 Planner

这一步非常关键。因为很多路由错误，本质上不是 Planner“理解错了”，而是它面对太多看起来都差不多的工具时，用了最脆弱的词面匹配策略。

换句话说，减少候选集，比增强 prompt 更有效。

### 经验

Metadata 的真正价值，不是让工具 schema 更漂亮，而是把“工具选择”从纯文本匹配提升成半结构化决策。只要这个层面做好，后续新增工具也不会立刻把系统稳定性拉垮。

## 四、反馈层：Critic 必须从“命令者”降级为“打分器”

### 之前的问题

原先 Critic 的工作方式是：

- 工具结果出来后
- Critic 判断“是不是有问题”
- 如果有问题，直接生成一句 correction suggestion
- 系统再把这句建议塞回上下文，推动 Planner 继续搜索

这个机制在少量高风险场景里是有用的，但一旦用得过宽，就会带来两个副作用。

第一，它会把“还可以更好”和“根本不够回答”混为一谈。结果是工具其实已经够用了，但 Critic 仍然推动系统继续搜。

第二，它没有预算约束。只要 Planner 和 Critic 互相强化，就很容易出现无意义的继续搜索。

### 这次怎么改

我们把 Critic 改成了 score-based 模式，在 [`src/react/critic.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/react/critic.rs) 中返回：

- `score`
- `reason`
- `retry_recommended`
- `blocking_risk`

同时在配置和 prompt 中引入：

- `score_threshold`
- `max_self_corrections`

运行时逻辑变成：

- 分数高于阈值，直接通过
- 分数低于阈值，但 `retry_recommended=false`，也不强制重试
- 分数低于阈值且建议重试，才会真正介入
- 单次对话重试次数受预算限制

这让 Critic 的角色从“二元裁判”变成“风险信号源”。

### 这个变化为什么重要

因为 Critic 的最优定位，不是替 Planner 做第二遍规划，而是在高风险、低信心、明显偏题时给系统一个“值得再看一眼”的信号。

如果把 Critic 放得太前，它会放大系统不稳定性；如果把它收敛成一个带阈值的评分器，它才更像工程系统里的质量守卫。

### 经验

Agent 里的 Critic 不是越强越好，而是越克制越好。好的 Critic 应该减少无意义重试，而不是制造更多“看起来在努力”的搜索动作。

## 五、守卫层：没有 Golden Dataset，就没有真正的稳定性

### 为什么单元测试不够

如果只做单工具测试，我们能验证：

- `weather` 是否返回正常 JSON
- `github_repo_inspect` 是否能抓 repo 树
- `search` 是否能抓页面

但这些都不能回答一个更关键的问题：

对于给定用户输入，系统是否走了正确路径？

真正容易退化的，往往不是单个工具的内部逻辑，而是整个“输入 -> 分类 -> 候选工具 -> rewrite -> guard -> 最终执行”的链路。

### 这次怎么做

我们引入了 Golden Dataset：

- [`tests/golden/tool_routing_cases.json`](/Users/g/Documents/GitHub/feature/org_20260321/tests/golden/tool_routing_cases.json)
- [`tests/tool_routing_golden.rs`](/Users/g/Documents/GitHub/feature/org_20260321/tests/tool_routing_golden.rs)

每条 case 不只包含输入，还包含：

- `expected_query_kind`
- `expected_direct_tool`
- `expected_allowed_tools`
- `forbidden_tools`
- `should_use_memory`

这意味着我们开始正式测试“路径”，而不只是测试“输出”。

例如：

- “吉隆坡明天天气”必须直达 `weather`
- “今天有什么新闻，推荐 5 条”必须直达 `news`
- 外部 GitHub 仓库分析不能出现 `ls`、`cat`、`code_read`、`shell`
- 直接解释型问题不能被本地工具污染

### 经验

一旦系统开始依赖 query classification、tool rewrite、critic gating 这类中间层逻辑，Golden Dataset 就不是加分项，而是必需品。没有它，模型升级、prompt 变动、工具新增后，路径漂移几乎不可避免。

## 六、观测：只有能看见，才谈得上持续优化

这一轮在 [`src/observability/mod.rs`](/Users/g/Documents/GitHub/feature/org_20260321/src/observability/mod.rs) 里补充了几个关键指标：

- `direct_route_hits`
- `route_drift_count`
- `policy_rewrites`
- `policy_blocks`

这几个指标分别对应：

- 专用工具直达命中率
- 路径偏移和 Critic 低分信号
- Planner 和策略层不一致的频率
- 明显不合理工具调用被阻断的频率

虽然当前还没有进一步做 score histogram 和更细粒度的 drift 统计，但第一步已经建立了方向：我们不再只从用户抱怨里感知路由问题，而是能从指标里直接看到系统行为是否在漂。

经验是，Agent 系统的很多问题都不是“突然坏了”，而是“慢慢漂了”。观测就是防止这类慢性退化的基础。

## 七、这轮最关键的设计取舍

### 1. 为什么 GitHub 分析没有完全 direct route

GitHub repo 分析已经有专用工具 `github_repo_inspect`，但这轮没有把它也做成完全绕过 Planner 的 direct route。

原因是 GitHub 场景虽然抓取确定，但用户问题往往带有较强的组织和总结需求，例如：

- “技术架构是什么”
- “系统设计如何”
- “跟 OpenAI Codex 有什么差异”

这些回答通常仍然适合让 Planner 结合结构化结果组织语言。因此目前策略是：

- GitHub 保留给专用工具抓取
- 再通过 system hint 和结构化输出，引导 Planner 直接收口

这是一种相对保守的取舍，优点是风险低，缺点是还没有把 GitHub 路由进一步压缩到极致。

### 2. 为什么没有先全面引入 embedding router

从长期看，embedding router 或 classifier router 当然值得做，但这一轮优先选择了 deterministic router + metadata policy。

原因很现实：

- 现有 bad case 足够规律，用规则就能覆盖大部分问题
- 规则式路由更容易测试、更容易解释、更容易快速迭代
- 在没有 Golden Dataset 之前，先引入 embedding router 会让排查更复杂

所以这轮的优先级是先把系统从“全靠模型选工具”拉回到“有运行时硬治理”，而不是立刻追求更智能的 router。

### 3. 为什么先做专用工具，再谈通用搜索质量

通用搜索当然还可以继续优化，但对天气、新闻、汇率、股价、比分这类任务来说，专用工具的收益远大于继续雕刻搜索逻辑。

因为这些任务的目标不是“搜到尽可能多网页”，而是“快速、稳定、低噪音地给出当前结果”。专用接口天生更适合这个目标。

经验是：如果一个问题域存在稳定 API，优先做专用工具；只有在没有稳定 API 时，才让通用搜索承担主角色。

## 八、这轮已经解决了什么

从系统行为上看，这轮已经实质性解决了几类高频问题：

1. 天气、新闻、汇率、行情、比分不再默认绕到通用搜索。
2. 工具 metadata 不再只是说明书，而是正式参与候选收敛和排序。
3. Critic 不再轻易把系统逼入继续搜索。
4. 核心坏路径开始有 Golden Dataset 守卫。
5. 路由质量开始具备可观测性。
6. `cargo check` 层面的历史 warning 也一并清掉了，降低了后续噪音。

这意味着 Bee 的工具调用体系，已经从“问题发生后局部补救”，进入了“有统一边界和统一策略”的阶段。

## 九、还没完成但值得继续推进的方向

这轮并不意味着问题已经彻底终结，后面仍然有几个清晰方向值得继续推进。

### 1. 扩展 capability hierarchy

现在已经有 `capability_group` 和 `capability_subgroup`，但还可以继续细化，例如：

- `SocialContent`
- `ArticlePage`
- `SearchResultsPage`
- `FinancialRealtime`
- `RepositoryFile`

一旦类型足够稳定，就可以把很多“站点特判”进一步抽象到 source adapter 层。

### 2. 给 GitHub 和社交内容做 source adapter

目前 GitHub 已经有专用工具，X/Twitter 也有 mirror fallback，但两者还没有统一抽象成 source adapter。

如果后续把：

- GitHub
- Social Post
- Search Engine Results
- Dynamic Web Page

作为标准内容源类型来处理，后面扩展 Reddit、Substack、Medium 之类站点就会轻松很多。

### 3. 继续扩充 Golden Dataset

现在的 Golden Dataset 还偏小，更像是第一批核心坏路径守卫。下一步最值得做的是把历史 bad case 系统化沉淀进去，尤其是：

- direct answer 退化成本地探测
- 外部 repo 退化成本地路径读取
- 时效性请求被旧记忆污染
- 专用工具失效后错误回退到通用搜索

### 4. 增加 Critic score 分布观测

现在已经有 `route_drift_count`，但如果能进一步记录：

- Critic score 分布
- 低分 query kind 分布
- 低分工具对分布

就能更快发现哪些场景的路由还在持续不稳。

## 十、这轮最值得保留的经验

如果要把这轮复盘压缩成几条最值得长期保留的原则，我认为有这些：

1. 先分流，再规划。不是所有请求都值得进入 ReAct。
2. Metadata 要进入决策，而不只是进入 schema。
3. Planner 只应该看到少量真正相关的工具。
4. Critic 应该打分，不应该过度发号施令。
5. Golden Dataset 必须覆盖“路径”，不只覆盖“输出”。
6. 观测要围绕漂移来设计，而不只是围绕错误来设计。

## 结语

这轮改造真正的价值，不在于“又多了几个工具”，也不在于“修好了几个 warning”，而在于我们第一次把工具调用系统明确拆成了分层治理问题。

在这之前，Bee 更像是“依赖模型临场发挥的工具调用器”；在这之后，它开始具备一个更成熟 Agent 系统应有的特征：

- 有确定性分流
- 有 metadata 约束
- 有克制的反馈层
- 有路径级回归守卫
- 有面向漂移的观测

这并不意味着以后不会再出现工具误选，但意味着再出现同类问题时，我们已经不需要从零猜测“该往哪修”，而是可以沿着一套已经验证过的工程边界继续演进。
