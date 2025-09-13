use std::{collections::HashMap, fmt::Debug, path::PathBuf};

use clap::ValueEnum;
use tracing::{debug, info};

use crate::{Dollar, portfolio::provider::fidelity::FidelityCsvRow};

mod provider {
    pub(crate) mod fidelity {
        use crate::{Dollar, Percent, RelativeDollar, RelativePercent, portfolio::Position};
        use serde::{Deserialize, Deserializer};

        fn deserialize_optional_dollar<'de, D>(deserializer: D) -> Result<Option<Dollar>, D::Error>
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

        fn deserialize_dollar<'de, D>(deserializer: D) -> Result<Dollar, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            let cleaned = s.replace('$', "");
            cleaned.parse().map_err(serde::de::Error::custom)
        }

        fn deserialize_percent<'de, D>(deserializer: D) -> Result<Percent, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            let cleaned = s.replace('%', "");
            cleaned.parse().map_err(serde::de::Error::custom)
        }

        #[derive(Debug, Deserialize, Clone)]
        pub struct FidelityCsvRow {
            #[serde(rename = "Account Number")]
            pub account_number: String,
            #[serde(rename = "Account Name")]
            _account_name: String,
            #[serde(rename = "Symbol")]
            symbol: String,
            #[serde(rename = "Description")]
            _description: String,
            #[serde(rename = "Quantity")]
            _quantity: Option<f32>,
            #[serde(
                rename = "Last Price",
                deserialize_with = "deserialize_optional_dollar"
            )]
            _last_price: Option<Dollar>,
            #[serde(rename = "Last Price Change")]
            _last_price_change: Option<RelativeDollar>,
            #[serde(rename = "Current Value", deserialize_with = "deserialize_dollar")]
            pub current_value: Dollar,
            #[serde(rename = "Today's Gain/Loss Dollar")]
            _today_change_dollar: Option<RelativeDollar>,
            #[serde(rename = "Today's Gain/Loss Percent")]
            _today_change_percent: Option<RelativePercent>,
            #[serde(rename = "Total Gain/Loss Dollar")]
            _total_change_dollar: Option<RelativeDollar>,
            #[serde(rename = "Total Gain/Loss Percent")]
            _total_change_percent: Option<RelativePercent>,
            #[serde(
                rename = "Percent Of Account",
                deserialize_with = "deserialize_percent"
            )]
            _percent_of_account: Percent,
            #[serde(
                rename = "Cost Basis Total",
                deserialize_with = "deserialize_optional_dollar"
            )]
            _cost_basis_total: Option<Dollar>,
            #[serde(
                rename = "Average Cost Basis",
                deserialize_with = "deserialize_optional_dollar"
            )]
            _cost_basis_average: Option<Dollar>,
            #[serde(rename = "Type")]
            _position_type: String,
        }

        impl FidelityCsvRow {
            pub fn symbol(&self) -> &str {
                self.symbol.trim_end_matches("**")
            }

            pub fn is_core_position(&self) -> bool {
                self.symbol.ends_with("**")
            }
        }

        impl From<FidelityCsvRow> for Position {
            fn from(row: FidelityCsvRow) -> Self {
                Position {
                    symbol: row.symbol().to_owned(),
                    current_value: row.current_value,
                    is_core: row.is_core_position(),
                    ignored: false,
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct AccountBalance {
    pub account_number: String,
    pub positions: Vec<Position>,
}

impl AccountBalance {
    pub fn set_ignored(&mut self, ignored: &[String]) {
        for pos in self.positions.iter_mut() {
            if ignored.iter().any(|i| i.eq_ignore_ascii_case(&pos.symbol)) {
                pos.ignored = true;
            }
        }
    }
}

#[derive(Debug)]
pub struct Position {
    pub symbol: String,
    pub current_value: Dollar,
    pub is_core: bool,
    pub ignored: bool,
}

#[derive(Debug)]
pub struct Portfolio {
    pub accounts: Vec<AccountBalance>,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum Provider {
    Fidelity,
}

impl Portfolio {
    pub fn load_from_file(path: &PathBuf, provider: Provider) -> anyhow::Result<Self> {
        match provider {
            Provider::Fidelity => {
                let mut csv_reader = csv::Reader::from_path(path)?;
                debug!("created reader");
                let rows: Vec<FidelityCsvRow> = csv_reader
                    .deserialize()
                    .filter_map(|record| record.inspect_err(|e| info!("{e}")).ok())
                    .collect();
                debug!(?rows, "deserialized rows");
                let mut accounts = HashMap::<String, AccountBalance>::new();
                for row in rows {
                    accounts
                        .entry(row.account_number.clone())
                        .and_modify(|acc| acc.positions.push(row.clone().into()))
                        .or_insert(AccountBalance {
                            account_number: row.account_number.clone(),
                            positions: vec![row.into()],
                        });
                }
                Ok(Self {
                    accounts: accounts.into_values().collect(),
                })
            }
        }
    }
}
