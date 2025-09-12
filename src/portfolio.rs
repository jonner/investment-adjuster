use serde::{Deserialize, Deserializer};

use crate::{Dollar, Percent, RelativeDollar, RelativePercent};

pub fn deserialize_optional_dollar<'de, D>(deserializer: D) -> Result<Option<Dollar>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.trim().is_empty() {
        return Ok(None);
    }
    let cleaned = s.replace('$', "");
    let value = cleaned.parse().map_err(serde::de::Error::custom)?;
    Ok(Some(value))
}

pub fn deserialize_dollar<'de, D>(deserializer: D) -> Result<Dollar, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let cleaned = s.replace('$', "");
    cleaned.parse().map_err(serde::de::Error::custom)
}

pub fn deserialize_percent<'de, D>(deserializer: D) -> Result<Percent, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let cleaned = s.replace('%', "");
    cleaned.parse().map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
pub(crate) struct Position {
    #[serde(rename = "Account Number")]
    pub account_number: String,
    #[serde(rename = "Account Name")]
    account_name: String,
    #[serde(rename = "Symbol")]
    symbol: String,
    #[serde(rename = "Description")]
    description: String,
    #[serde(rename = "Quantity")]
    quantity: Option<f32>,
    #[serde(
        rename = "Last Price",
        deserialize_with = "deserialize_optional_dollar"
    )]
    last_price: Option<Dollar>,
    #[serde(rename = "Last Price Change")]
    last_price_change: Option<RelativeDollar>,
    #[serde(rename = "Current Value", deserialize_with = "deserialize_dollar")]
    pub current_value: Dollar,
    #[serde(rename = "Today's Gain/Loss Dollar")]
    today_change_dollar: Option<RelativeDollar>,
    #[serde(rename = "Today's Gain/Loss Percent")]
    today_change_percent: Option<RelativePercent>,
    #[serde(rename = "Total Gain/Loss Dollar")]
    total_change_dollar: Option<RelativeDollar>,
    #[serde(rename = "Total Gain/Loss Percent")]
    total_change_percent: Option<RelativePercent>,
    #[serde(
        rename = "Percent Of Account",
        deserialize_with = "deserialize_percent"
    )]
    percent_of_account: Percent,
    #[serde(
        rename = "Cost Basis Total",
        deserialize_with = "deserialize_optional_dollar"
    )]
    cost_basis_total: Option<Dollar>,
    #[serde(
        rename = "Average Cost Basis",
        deserialize_with = "deserialize_optional_dollar"
    )]
    cost_basis_average: Option<Dollar>,
    #[serde(rename = "Type")]
    position_type: String,
}

impl Position {
    pub fn symbol(&self) -> &str {
        self.symbol.trim_end_matches("**")
    }

    pub fn is_core_position(&self) -> bool {
        self.symbol.ends_with("**")
    }
}
