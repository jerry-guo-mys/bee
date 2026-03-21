use async_trait::async_trait;
use chrono::Local;
use reqwest::Client;
use serde_json::Value;

use crate::tools::output;
use crate::tools::{
    Tool, ToolCapabilityGroup, ToolCapabilitySubgroup, ToolCostClass, ToolCriticMode,
    ToolFreshness, ToolIntent, ToolMetadata, ToolOutputShape, ToolRisk, ToolScope, ToolUseCase,
};

pub struct SportsScoreTool {
    client: Client,
    timeout_secs: u64,
}

impl SportsScoreTool {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .user_agent("Bee/1.0")
                .build()
                .unwrap_or_default(),
            timeout_secs,
        }
    }

    fn league_path(league: &str) -> Option<(&'static str, &'static str)> {
        match league.trim().to_lowercase().as_str() {
            "nba" => Some(("basketball", "nba")),
            "nfl" => Some(("football", "nfl")),
            "mlb" => Some(("baseball", "mlb")),
            "nhl" => Some(("hockey", "nhl")),
            "epl" => Some(("soccer", "eng.1")),
            _ => None,
        }
    }
}

#[async_trait]
impl Tool for SportsScoreTool {
    fn name(&self) -> &str {
        "sports_score"
    }

    fn description(&self) -> &str {
        "Get live sports scores. Args: {\"league\": \"nba|nfl|mlb|nhl|epl\", \"team\": \"optional\"}"
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(
            ToolScope::RemoteWeb,
            vec![ToolIntent::FetchWebPage, ToolIntent::Research],
        )
        .with_risk(ToolRisk::Low)
        .with_output_shape(ToolOutputShape::StructuredJson)
        .with_freshness(ToolFreshness::Live)
        .with_preferred_use_cases(vec![
            ToolUseCase::SportsScore,
            ToolUseCase::TimeSensitiveCurrent,
        ])
        .with_capability(
            ToolCapabilityGroup::RealtimeData,
            ToolCapabilitySubgroup::SportsScore,
        )
        .with_costs(
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Low,
        )
        .with_preferred_rank(1)
        .with_critic_mode(ToolCriticMode::Skip)
    }

    fn timeout_secs(&self) -> Option<u64> {
        Some(self.timeout_secs)
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "league": {
                    "type": "string",
                    "description": "League code like nba, nfl, mlb, nhl, epl"
                },
                "team": {
                    "type": "string",
                    "description": "Optional team filter"
                }
            },
            "required": ["league"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let league = args
            .get("league")
            .and_then(|v| v.as_str())
            .ok_or("Missing league")?;
        let team_filter = args
            .get("team")
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase());
        let date = args
            .get("date")
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| Local::now().format("%Y%m%d").to_string());

        let (sport, league_path) =
            Self::league_path(league).ok_or_else(|| format!("Unsupported league: {}", league))?;
        let url = format!(
            "https://site.api.espn.com/apis/site/v2/sports/{}/{}/scoreboard",
            sport, league_path
        );

        let response = self
            .client
            .get(url)
            .query(&[("dates", date.as_str())])
            .send()
            .await
            .map_err(|e| format!("Sports-score request failed: {}", e))?;
        let response = response
            .error_for_status()
            .map_err(|e| format!("Sports-score request failed: {}", e))?;
        let json: Value = response
            .json()
            .await
            .map_err(|e| format!("Sports-score parse failed: {}", e))?;

        let games = json["events"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|event| {
                let competition = event["competitions"].as_array()?.first()?.clone();
                let competitors = competition["competitors"].as_array()?.clone();
                let names = competitors
                    .iter()
                    .filter_map(|team| team["team"]["displayName"].as_str())
                    .collect::<Vec<_>>();
                if let Some(filter) = team_filter.as_deref() {
                    let matches_team = names
                        .iter()
                        .any(|name| name.to_lowercase().contains(filter));
                    if !matches_team {
                        return None;
                    }
                }

                Some(serde_json::json!({
                    "name": event["name"],
                    "short_name": event["shortName"],
                    "status": competition["status"]["type"]["description"],
                    "detail": competition["status"]["type"]["detail"],
                    "competitors": competitors.iter().map(|team| serde_json::json!({
                        "team": team["team"]["displayName"],
                        "score": team["score"],
                        "home_away": team["homeAway"],
                        "winner": team["winner"],
                    })).collect::<Vec<_>>(),
                }))
            })
            .collect::<Vec<_>>();

        output::structured(
            self.name(),
            format!("Fetched {} {} game(s)", games.len(), league.to_uppercase()),
            !games.is_empty(),
            serde_json::json!({
                "league": league,
                "date": date,
                "games": games,
                "source": "ESPN scoreboard"
            }),
        )
    }
}
