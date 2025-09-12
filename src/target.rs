use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;
use tracing::debug;

use crate::{Action, Dollar, Percent, portfolio::Account};

#[derive(Debug)]
struct PositionAdjustment {
    current_value: Dollar,
    desired_percent: Percent,
}

#[derive(Debug)]
pub struct AccountTarget {
    pub account_number: String,
    pub core_position: CorePosition,
    targets: HashMap<String, Percent>,
}

impl AccountTarget {
    pub(crate) fn load_from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let targets_file =
            std::fs::File::open(path).with_context(|| format!("Failed to open file {path:?}"))?;
        let account_targets: AccountTargetBuilder = serde_yaml::from_reader(targets_file)?;
        account_targets.build()
    }

    pub(crate) fn targets(&self) -> HashMap<String, Percent> {
        self.targets.clone()
    }

    pub(crate) fn process(
        &self,
        account: &Account,
        ignore: &[String],
    ) -> anyhow::Result<Vec<(String, Action)>> {
        let core = account
            .positions
            .iter()
            .find(|&pos| {
                pos.symbol == self.core_position.symbol
                    && account.account_number == self.account_number
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
                "Found position {} but it is not marked as the core position",
                self.core_position.symbol
            )
        }
        if core.current_value < self.core_position.minimum {
            bail!(
                "Core position is currently below the target minimum: {} < {}",
                core.current_value,
                self.core_position.minimum
            );
        }
        if self.targets.contains_key(&core.symbol) {
            bail!("Core position cannot be in target list");
        }
        for (symbol, _) in self.targets.iter() {
            if ignore.contains(symbol) {
                bail!("Can't ignore symbol '{symbol}': it is specified in the target allocation")
            }
        }
        let mut adjustments: HashMap<String, PositionAdjustment> = HashMap::new();
        for (target_symbol, &target_percent) in self.targets.iter() {
            adjustments
                .entry(target_symbol.clone())
                .and_modify(|e| e.desired_percent = target_percent)
                .or_insert(PositionAdjustment {
                    current_value: 0.0,
                    desired_percent: target_percent,
                });
        }
        for pos in account.positions.iter() {
            adjustments
                .entry(pos.symbol.to_owned())
                .and_modify(|e| e.current_value = pos.current_value)
                .or_insert(PositionAdjustment {
                    current_value: pos.current_value,
                    desired_percent: 0.0,
                });
        }

        let total_val = adjustments
            .iter()
            .filter_map(|(sym, adj)| match ignore.contains(sym) {
                true => None,
                false => Some(adj.current_value),
            })
            .sum::<Dollar>();
        let to_distribute = total_val - self.core_position.minimum;
        if to_distribute < 0.0 {
            bail!("Not enough money to maintain core position minimum");
        }
        let actions: Vec<(String, Action)> = adjustments
            .into_iter()
            .map(|(symbol, pos)| {
                let action = match ignore.contains(&symbol) {
                    true => Action::Ignore,
                    false => {
                        if symbol == self.core_position.symbol {
                            if pos.current_value > self.core_position.minimum {
                                Action::Sell(pos.current_value - self.core_position.minimum)
                            } else {
                                Action::Nothing
                            }
                        } else {
                            let desired_val = to_distribute * (pos.desired_percent / 100.0);
                            match desired_val - pos.current_value {
                                val if val > 0.0 => Action::Buy(val.abs()),
                                val if val < 0.0 => Action::Sell(val.abs()),
                                _ => Action::Nothing,
                            }
                        }
                    }
                };
                (symbol, action)
            })
            .collect();
        debug!(?actions, "processed data");
        Ok(actions)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AccountTargetBuilder {
    pub account_number: String,
    pub core_position: CorePosition,
    pub positions: HashMap<String, Percent>,
}

impl TryInto<AccountTarget> for AccountTargetBuilder {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<AccountTarget, Self::Error> {
        self.validate()?;
        Ok(AccountTarget {
            account_number: self.account_number,
            core_position: self.core_position,
            targets: self.positions,
        })
    }
}

impl AccountTargetBuilder {
    fn validate(&self) -> anyhow::Result<()> {
        let total_percent: f32 = self.positions.values().sum();
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
    pub symbol: String,
    /// Minimum amount to retain in the core position in dollars
    pub minimum: f32,
}
