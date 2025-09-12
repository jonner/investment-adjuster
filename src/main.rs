use std::fmt::Display;

use anyhow::Context;
use clap::Parser;
use directories::ProjectDirs;
use tracing::debug;

// FIXME: handle dollar sign
type Dollar = f32;
// FIXME: handle dollar sign and plus/minus
type RelativeDollar = String;
// FIXME: handle percent sign
type Percent = f32;
// FIXME: handle percent sign and plus/minus
type RelativePercent = String;

#[derive(Debug)]
pub enum Action {
    Nothing,
    Sell(Dollar),
    Buy(Dollar),
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Nothing => write!(f, "no action needed"),
            Action::Sell(v) => write!(f, "sell ${v:.2}"),
            Action::Buy(v) => write!(f, "buy ${v:.2}"),
        }
    }
}

mod cli {
    use std::path::PathBuf;

    use clap::Parser;

    #[derive(Parser, Debug)]
    pub(crate) struct Cli {
        #[arg(short, long, help = "Target allocation")]
        pub target: Option<PathBuf>,
        #[arg(help = "Current allocation CSV downloaded from fidelity")]
        pub current_allocations: PathBuf,
    }
}

mod portfolio {
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
}

mod target {
    use anyhow::{anyhow, bail};
    use serde::Deserialize;
    use tracing::debug;

    use crate::{Action, Dollar, Percent};

    #[derive(Debug)]
    struct PositionAdjustment {
        symbol: String,
        current_value: Dollar,
        desired_percent: Percent,
    }

    #[derive(Debug)]
    pub struct AccountTarget {
        pub account_number: String,
        pub core_position: CorePosition,
        position_targets: Vec<PositionTarget>,
    }

    impl AccountTarget {
        pub(crate) fn targets(&self) -> Vec<PositionTarget> {
            self.position_targets.clone()
        }

        pub(crate) fn process(
            &self,
            current: Vec<crate::portfolio::Position>,
        ) -> anyhow::Result<Vec<(String, Action)>> {
            let core = current
                .iter()
                .find(|&pos| {
                    pos.symbol() == self.core_position.symbol
                        && pos.account_number == self.account_number
                })
                .ok_or_else(|| {
                    anyhow!(
                        "Failed to find an entry for core position {} for account {}",
                        self.core_position.symbol,
                        self.account_number
                    )
                })?;
            if !core.is_core_position() {
                bail!(
                    "Found position {} but it is not marked as the core position",
                    self.core_position.symbol
                )
            }
            if core.current_value < self.core_position.minimum {
                return Err(anyhow!(
                    "Core position is currently below the target minimum: {} < {}",
                    core.current_value,
                    self.core_position.minimum
                ));
            }
            let to_distribute = core.current_value - self.core_position.minimum;
            let mut adjustments: Vec<PositionAdjustment> = Vec::new();
            for target in self.position_targets.iter() {
                let portfolio_pos = current
                    .iter()
                    .find(|&p| {
                        p.symbol() == target.symbol && p.account_number == self.account_number
                    })
                    .ok_or_else(|| {
                        anyhow!(
                            "Couldn't find an entry for position {} in account {}",
                            target.symbol,
                            self.account_number
                        )
                    })?;
                adjustments.push(PositionAdjustment {
                    symbol: target.symbol.clone(),
                    current_value: portfolio_pos.current_value,
                    desired_percent: target.percent,
                });
            }

            let total_val =
                adjustments.iter().map(|p| p.current_value).sum::<Dollar>() + to_distribute;
            let mut actions: Vec<(String, Action)> = adjustments
                .into_iter()
                .map(|pos| {
                    let desired_val = total_val * (pos.desired_percent / 100.0);
                    let action = match desired_val - pos.current_value {
                        val if val > 0.0 => Action::Buy(val),
                        val if val < 0.0 => Action::Sell(val),
                        _ => Action::Nothing,
                    };
                    (pos.symbol, action)
                })
                .collect();
            actions.push((
                self.core_position.symbol.clone(),
                Action::Sell(to_distribute),
            ));
            debug!(?actions, "processed data");
            Ok(actions)
        }
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct AccountTargetBuilder {
        pub account_number: String,
        pub core_position: CorePosition,
        pub positions: Vec<PositionTarget>,
    }

    impl TryInto<AccountTarget> for AccountTargetBuilder {
        type Error = anyhow::Error;

        fn try_into(self) -> Result<AccountTarget, Self::Error> {
            self.validate()?;
            Ok(AccountTarget {
                account_number: self.account_number,
                core_position: self.core_position,
                position_targets: self.positions,
            })
        }
    }

    impl AccountTargetBuilder {
        fn validate(&self) -> anyhow::Result<()> {
            let total_percent: f32 = self.positions.iter().map(|position| position.percent).sum();
            anyhow::ensure!(
                total_percent == 100.0,
                "Target positions do not add up to 100%"
            );
            Ok(())
        }

        pub fn build(self) -> anyhow::Result<AccountTarget> {
            self.try_into()
        }
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct CorePosition {
        symbol: String,
        /// Minimum amount to retain in the core position in dollars
        minimum: f32,
    }

    #[derive(Debug, Deserialize, Clone)]
    #[serde(rename_all = "PascalCase")]
    pub struct PositionTarget {
        pub symbol: String,
        pub percent: Percent,
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let opts = cli::Cli::parse();

    let mut position_reader = csv::Reader::from_path(opts.current_allocations)?;
    debug!("created reader");
    let positions = position_reader
        .deserialize()
        .filter_map(|record| record.ok())
        .collect();
    debug!(?positions, "got positions");

    let Some(targets_path) =
        opts.target
            .or(ProjectDirs::from("org", "quotidian", "investment-adjuster")
                .map(|pdirs| pdirs.config_dir().join("target.yml")))
    else {
        anyhow::bail!("Failed to get target path");
    };
    let targets_file = std::fs::File::open(&targets_path)
        .with_context(|| format!("Failed to open file {targets_path:?}"))?;
    let account_targets: target::AccountTargetBuilder = serde_yaml::from_reader(targets_file)?;
    let account_targets = account_targets.build()?;
    debug!(?account_targets, "got targets");

    println!(
        "Allocation targets for account {}",
        account_targets.account_number
    );
    account_targets
        .targets()
        .into_iter()
        .for_each(|pos| println!(" - {}: {:.1}%", pos.symbol, pos.percent));
    println!("");

    let actions = account_targets.process(positions)?;
    println!("In order to maintain your target allocations, the following actions are necessary:");
    actions
        .iter()
        .for_each(|(symbol, action)| println!(" - {symbol}: {action}"));
    Ok(())
}
