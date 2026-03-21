use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::tools::output;
use crate::tools::{
    Tool, ToolCriticMode, ToolFreshness, ToolIntent, ToolMetadata, ToolOutputShape, ToolRisk,
    ToolScope, ToolUseCase,
};

pub struct ExchangeRateTool {
    client: Client,
    timeout_secs: u64,
}

impl ExchangeRateTool {
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
impl Tool for ExchangeRateTool {
    fn name(&self) -> &str {
        "exchange_rate"
    }

    fn description(&self) -> &str {
        "Get live exchange rates. Args: {\"base\": \"USD\", \"quote\": \"CNY\", \"amount\": 1}"
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
            ToolUseCase::ExchangeRate,
            ToolUseCase::TimeSensitiveCurrent,
        ])
        .with_critic_mode(ToolCriticMode::Skip)
    }

    fn timeout_secs(&self) -> Option<u64> {
        Some(self.timeout_secs)
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "base": { "type": "string", "description": "Base currency code, e.g. USD" },
                "quote": { "type": "string", "description": "Quote currency code, e.g. CNY" },
                "amount": { "type": "number", "description": "Optional amount to convert", "default": 1.0 }
            },
            "required": ["base", "quote"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let base = args
            .get("base")
            .and_then(|v| v.as_str())
            .unwrap_or("USD")
            .trim()
            .to_uppercase();
        let quote = args
            .get("quote")
            .and_then(|v| v.as_str())
            .unwrap_or("CNY")
            .trim()
            .to_uppercase();
        let amount = args.get("amount").and_then(|v| v.as_f64()).unwrap_or(1.0);

        let url = format!("https://open.er-api.com/v6/latest/{}", base);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Exchange-rate request failed: {}", e))?;
        let response = response
            .error_for_status()
            .map_err(|e| format!("Exchange-rate request failed: {}", e))?;
        let json: Value = response
            .json()
            .await
            .map_err(|e| format!("Exchange-rate parse failed: {}", e))?;

        let rate = json["rates"][quote.as_str()]
            .as_f64()
            .ok_or_else(|| format!("Quote currency not found: {}", quote))?;

        output::structured(
            self.name(),
            format!("Live exchange rate {} -> {}", base, quote),
            true,
            serde_json::json!({
                "base": base,
                "quote": quote,
                "rate": rate,
                "amount": amount,
                "converted_amount": amount * rate,
                "updated_at": json["time_last_update_utc"],
                "source": "open.er-api.com"
            }),
        )
    }
}
