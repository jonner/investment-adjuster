use std::{collections::HashMap, path::Path};

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;
use tracing::debug;

use crate::{Action, Dollar, Percent};

#[derive(Debug, Default)]
pub struct AccountBalance {
    pub account_number: String,
    pub account_name: String,
    pub positions: Vec<Position>,
}

#[derive(Debug)]
pub struct Position {
    pub symbol: String,
    pub current_value: Dollar,
    pub is_core: bool,
}

#[derive(Debug)]
pub struct Portfolio {
    pub accounts: Vec<AccountBalance>,
}

#[derive(Debug)]
struct PositionAdjustment {
    current_value: Dollar,
    target: Percent,
    ignored: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct AccountConfig {
    pub account_number: String,
    pub core_position: CorePosition,
    targets: HashMap<String, Percent>,
    #[serde(default)]
    pub ignored: Vec<String>,
}

impl AccountConfig {
    fn validate(&self) -> anyhow::Result<()> {
        let total_percent: f32 = self.targets.values().sum();
        anyhow::ensure!(
            total_percent == 100.0,
            "Target positions for account {} do not add up to 100%",
            self.account_number
        );
        Ok(())
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<Self>> {
        let targets_file = std::fs::File::open(path.as_ref())
            .with_context(|| format!("Failed to open file {:?}", path.as_ref()))?;
        let targets: Vec<Self> = serde_yaml::from_reader(targets_file)?;
        targets
            .into_iter()
            .map(|t| {
                t.validate()?;
                Ok(t)
            })
            .collect()
    }

    pub fn targets(&self) -> HashMap<String, Percent> {
        self.targets.clone()
    }

    pub fn adjust_allocations(
        &self,
        balance: &AccountBalance,
    ) -> anyhow::Result<Vec<(String, Action)>> {
        let core = balance
            .positions
            .iter()
            .find(|&pos| {
                pos.symbol == self.core_position.symbol
                    && balance.account_number == self.account_number
            })
            .ok_or_else(|| {
                anyhow!(
                    "Failed to find an entry for core position {} for account {}",
                    self.core_position.symbol,
                    self.account_number
                )
            })?;
        if !core.is_core {
            bail!(
                "Found position {} but it was not marked as the core position in the provided data file",
                self.core_position.symbol
            )
        }
        if self.targets.contains_key(&core.symbol) {
            bail!("Core position cannot be in target list");
        }
        let mut adjustments: HashMap<String, PositionAdjustment> = HashMap::new();
        for (target_symbol, &target_percent) in self.targets.iter() {
            adjustments
                .entry(target_symbol.clone())
                .and_modify(|e| e.target = target_percent)
                .or_insert(PositionAdjustment {
                    current_value: 0.0,
                    target: target_percent,
                    ignored: false,
                });
        }
        for pos in balance.positions.iter() {
            adjustments
                .entry(pos.symbol.to_owned())
                .and_modify(|e| e.current_value = pos.current_value)
                .or_insert(PositionAdjustment {
                    current_value: pos.current_value,
                    target: 0.0,
                    ignored: self.ignored.contains(&pos.symbol),
                });
        }

        for (symbol, adj) in adjustments.iter() {
            if adj.ignored && adj.target != 0.0 {
                bail!("Can't ignore symbol '{symbol}': it is specified in the target allocation")
            }
        }

        let total_val = adjustments
            .iter()
            .filter_map(|(_, adj)| {
                if adj.ignored {
                    None
                } else {
                    Some(adj.current_value)
                }
            })
            .sum::<Dollar>();
        let to_distribute = total_val - self.core_position.minimum;
        if to_distribute < 0.0 {
            bail!(
                "Not enough value to maintain core position minimum. Sell all investments or transfer more into account."
            );
        }

        let actions: Vec<(String, Action)> = adjustments
            .into_iter()
            .map(|(symbol, adj)| {
                let action = if adj.ignored {
                    Action::Ignore
                } else if symbol == self.core_position.symbol {
                    if adj.current_value > self.core_position.minimum {
                        Action::Sell(adj.current_value - self.core_position.minimum)
                    } else if adj.current_value < self.core_position.minimum {
                        Action::Buy(self.core_position.minimum - adj.current_value)
                    } else {
                        Action::Nothing
                    }
                } else {
                    let desired_val = to_distribute * (adj.target / 100.0);
                    match desired_val - adj.current_value {
                        val if val > 0.0 => Action::Buy(val.abs()),
                        val if val < 0.0 => Action::Sell(val.abs()),
                        _ => Action::Nothing,
                    }
                };
                (symbol, action)
            })
            .collect();
        debug!(?actions, "processed data");
        Ok(actions)
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct CorePosition {
    pub symbol: String,
    /// Minimum amount to retain in the core position in dollars
    pub minimum: Dollar,
}
