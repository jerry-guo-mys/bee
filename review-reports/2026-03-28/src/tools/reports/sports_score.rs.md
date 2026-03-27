# Rust 代码审查报告：sports_score.rs

## 业务场景和职责
- 获取体育比赛实时比分工具
- 支持多种联赛（NBA/NFL/MLB/NHL/EPL）
- 使用 ESPN API

---

## 问题

### 1. **Client 构建使用 unwrap_or_default**
**行号**: 19-24
```rust
client: Client::builder()
    .timeout(std::time::Duration::from_secs(timeout_secs))
    .user_agent("Bee/1.0")
    .build()
    .unwrap_or_default(),
```
**修复方案**: 使用 expect 提供更好错误信息：
```rust
.build()
.expect("Failed to create HTTP client")
```

### 2. **联赛映射硬编码**
**行号**: 29-37
```rust
fn league_path(league: &str) -> Option<(&'static str, &'static str)> {
    match league.trim().to_lowercase().as_str() {
        "nba" => Some(("basketball", "nba")),
        // ...
    }
}
```
**触发场景**: 新增联赛需要修改代码
**修复方案**: 移到配置中（可选）

### 3. **ESPN API URL 硬编码**
**行号**: 115-118
```rust
let url = format!(
    "https://site.api.espn.com/apis/site/v2/sports/{}/{}/scoreboard",
    sport, league_path
);
```
**修复方案**: 如果 ESPN API 变化，需要代码修改；考虑配置化

### 4. **JSON 解析脆弱**
**行号**: 140-169
```rust
let games = json["events"]
    .as_array()
    .cloned()
    .unwrap_or_default()
    .into_iter()
    .filter_map(|event| {
        let competition = event["competitions"].as_array()?.first()?.clone();
```
**触发场景**: ESPN API 响应格式变化可能导致解析失败
**修复方案**: 使用 serde 定义强类型结构：
```rust
#[derive(Deserialize)]
struct EspnResponse {
    events: Vec<Event>,
}
```

---

## 设计确认（非问题）
- 使用 ESPN API 是合理选择
- 支持团队过滤实用
- 输出结构化 JSON 便于消费

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ⚠️ JSON 解析脆弱 |
| Async | ✓ 异步 HTTP 请求 |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：2
- 💡 建议：2
