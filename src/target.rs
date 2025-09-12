use std::collections::HashMap;

use anyhow::{anyhow, bail};
use serde::Deserialize;
use tracing::debug;

use crate::{Action, Dollar, Percent};

#[derive(Debug)]
struct PositionAdjustment {
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
            bail!(
                "Core position is currently below the target minimum: {} < {}",
                core.current_value,
                self.core_position.minimum
            );
        }
        if self
            .position_targets
            .iter()
            .any(|p| p.symbol == core.symbol())
        {
            bail!("Core position cannot be in target list");
        }
        let mut adjustments: HashMap<String, PositionAdjustment> = HashMap::new();
        for target in self.position_targets.iter() {
            adjustments
                .entry(target.symbol.clone())
                .and_modify(|e| e.desired_percent = target.percent)
                .or_insert(PositionAdjustment {
                    current_value: 0.0,
                    desired_percent: target.percent,
                });
        }
        for pos in current.iter() {
            adjustments
                .entry(pos.symbol().to_owned())
                .and_modify(|e| e.current_value = pos.current_value)
                .or_insert(PositionAdjustment {
                    current_value: pos.current_value,
                    desired_percent: 0.0,
                });
        }

        let total_val = adjustments
            .values()
            .map(|pos| pos.current_value)
            .sum::<Dollar>();
        let to_distribute = total_val - self.core_position.minimum;
        if to_distribute < 0.0 {
            bail!("Not enough money to maintain core position minimum");
        }
        let actions: Vec<(String, Action)> = adjustments
            .into_iter()
            .map(|(symbol, pos)| {
                let action = if symbol == self.core_position.symbol {
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
    pub symbol: String,
    /// Minimum amount to retain in the core position in dollars
    pub minimum: f32,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PositionTarget {
    pub symbol: String,
    pub percent: Percent,
}
