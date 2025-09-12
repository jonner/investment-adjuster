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
pub struct AllocationTargets {
    pub account_number: String,
    pub core_position: CorePosition,
    allocations: HashMap<String, Percent>,
}

impl AllocationTargets {
    pub(crate) fn load_from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let targets_file =
            std::fs::File::open(path).with_context(|| format!("Failed to open file {path:?}"))?;
        let builder: AllocationTargetsBuilder = serde_yaml::from_reader(targets_file)
            .with_context(|| format!("Failed to parse config file {path:?}"))?;
        builder.build()
    }

    pub(crate) fn targets(&self) -> HashMap<String, Percent> {
        self.allocations.clone()
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
        if self.allocations.contains_key(&core.symbol) {
            bail!("Core position cannot be in target list");
        }
        for (symbol, _) in self.allocations.iter() {
            if ignore.iter().any(|i| symbol.eq_ignore_ascii_case(i)) {
                bail!("Can't ignore symbol '{symbol}': it is specified in the target allocation")
            }
        }
        let mut adjustments: HashMap<String, PositionAdjustment> = HashMap::new();
        for (target_symbol, &target_percent) in self.allocations.iter() {
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
            .filter_map(
                |(sym, adj)| match ignore.iter().any(|i| sym.eq_ignore_ascii_case(i)) {
                    true => None,
                    false => Some(adj.current_value),
                },
            )
            .sum::<Dollar>();
        let to_distribute = total_val - self.core_position.minimum;
        if to_distribute < 0.0 {
            bail!("Not enough money to maintain core position minimum");
        }
        let actions: Vec<(String, Action)> = adjustments
            .into_iter()
            .map(|(symbol, pos)| {
                let action = match ignore.iter().any(|i| i.eq_ignore_ascii_case(&symbol)) {
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
struct AllocationTargetsBuilder {
    pub account_number: String,
    pub core_position: CorePosition,
    pub allocations: HashMap<String, Percent>,
}

impl TryInto<AllocationTargets> for AllocationTargetsBuilder {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<AllocationTargets, Self::Error> {
        self.validate()?;
        Ok(AllocationTargets {
            account_number: self.account_number,
            core_position: self.core_position,
            allocations: self.allocations,
        })
    }
}

impl AllocationTargetsBuilder {
    fn validate(&self) -> anyhow::Result<()> {
        let total_percent: f32 = self.allocations.values().sum();
        anyhow::ensure!(
            total_percent == 100.0,
            "Target positions do not add up to 100%"
        );
        Ok(())
    }

    pub fn build(self) -> anyhow::Result<AllocationTargets> {
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
