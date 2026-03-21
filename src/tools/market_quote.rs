use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::tools::output;
use crate::tools::{
    Tool, ToolCapabilityGroup, ToolCapabilitySubgroup, ToolCostClass, ToolCriticMode,
    ToolFreshness, ToolIntent, ToolMetadata, ToolOutputShape, ToolRisk, ToolScope, ToolUseCase,
};

pub struct MarketQuoteTool {
    client: Client,
    timeout_secs: u64,
}

impl MarketQuoteTool {
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
}

#[async_trait]
impl Tool for MarketQuoteTool {
    fn name(&self) -> &str {
        "market_quote"
    }

    fn description(&self) -> &str {
        "Get live stock, index, or crypto quotes. Args: {\"symbols\": [\"AAPL\", \"BTC-USD\"]}"
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
            ToolUseCase::MarketQuote,
            ToolUseCase::TimeSensitiveCurrent,
        ])
        .with_capability(
            ToolCapabilityGroup::RealtimeData,
            ToolCapabilitySubgroup::FinancialRealtime,
        )
        .with_costs(
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Medium,
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
                "symbols": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Ticker symbols like AAPL, BTC-USD, ^GSPC"
                }
            },
            "required": ["symbols"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let symbols = if let Some(arr) = args.get("symbols").and_then(|v| v.as_array()) {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        } else if let Some(symbol) = args.get("symbol").and_then(|v| v.as_str()) {
            vec![symbol.trim().to_string()]
        } else {
            Vec::new()
        };

        if symbols.is_empty() {
            return Err("Missing symbols".to_string());
        }

        let response = self
            .client
            .get("https://query1.finance.yahoo.com/v7/finance/quote")
            .query(&[("symbols", symbols.join(","))])
            .send()
            .await
            .map_err(|e| format!("Market quote request failed: {}", e))?;
        let response = response
            .error_for_status()
            .map_err(|e| format!("Market quote request failed: {}", e))?;
        let json: Value = response
            .json()
            .await
            .map_err(|e| format!("Market quote parse failed: {}", e))?;

        let quotes = json["quoteResponse"]["result"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|quote| {
                serde_json::json!({
                    "symbol": quote["symbol"],
                    "short_name": quote["shortName"],
                    "currency": quote["currency"],
                    "market_price": quote["regularMarketPrice"],
                    "market_change": quote["regularMarketChange"],
                    "market_change_percent": quote["regularMarketChangePercent"],
                    "market_time": quote["regularMarketTime"],
                    "market_state": quote["marketState"],
                })
            })
            .collect::<Vec<_>>();

        output::structured(
            self.name(),
            format!("Fetched {} market quote(s)", quotes.len()),
            !quotes.is_empty(),
            serde_json::json!({
                "requested_symbols": symbols,
                "quotes": quotes,
                "source": "Yahoo Finance"
            }),
        )
    }
}
