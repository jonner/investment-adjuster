use std::{collections::HashMap, path::Path};

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;

use crate::{Action, Dollar, Percent};

#[derive(Debug, Default)]
pub struct Balance {
    pub account_number: String,
    pub account_name: String,
    pub holdings: Vec<Holding>,
}

#[derive(Debug, Default, Clone)]
pub struct Holding {
    pub symbol: String,
    pub current_value: Dollar,
    pub is_cash: bool,
}

#[derive(Debug)]
pub struct Portfolio {
    pub accounts: Vec<Balance>,
}

#[derive(Debug, Default)]
pub struct PositionAdjustment {
    pub holding: Holding,
    pub target: Percent,
    pub ignored: bool,
    pub action: Action,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct CashConfig {
    pub symbol: String,
    /// Minimum amount to retain in the core position in dollars
    pub minimum: Dollar,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct Config {
    pub account_number: String,
    pub cash_sweep: CashConfig,
    pub targets: HashMap<String, Percent>,
    #[serde(default)]
    pub ignored_holdings: Vec<String>,
}

impl Config {
    fn validate(&self) -> anyhow::Result<()> {
        let total_percent: f32 = self.targets.values().sum();
        anyhow::ensure!(
            total_percent == 100.0,
            "Target allocations for account {} do not add up to 100%",
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

    pub fn adjust_allocations(&self, balance: &Balance) -> anyhow::Result<Vec<PositionAdjustment>> {
        let core = balance
            .holdings
            .iter()
            .find(|&pos| {
                pos.symbol == self.cash_sweep.symbol
                    && balance.account_number == self.account_number
            })
            .ok_or_else(|| {
                anyhow!(
                    "Failed to find an entry for core position {} for account {}",
                    self.cash_sweep.symbol,
                    self.account_number
                )
            })?;
        if !core.is_cash {
            bail!(
                "Found position {} but it was not marked as the core position in the provided data file",
                self.cash_sweep.symbol
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
                    target: target_percent,
                    holding: Holding {
                        symbol: target_symbol.clone(),
                        ..Default::default()
                    },
                    ..Default::default()
                });
        }
        for holding in balance.holdings.iter() {
            adjustments
                .entry(holding.symbol.to_owned())
                .and_modify(|e| e.holding.current_value = holding.current_value)
                .or_insert(PositionAdjustment {
                    holding: holding.clone(),
                    ignored: self.ignored_holdings.contains(&holding.symbol),
                    ..Default::default()
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
                    Some(adj.holding.current_value)
                }
            })
            .sum::<Dollar>();
        let to_distribute = total_val - self.cash_sweep.minimum;
        if to_distribute < 0.0 {
            bail!(
                "Not enough value to maintain core position minimum. Sell all investments or transfer more into account."
            );
        }

        let mut adjustments: Vec<_> = adjustments
            .into_values()
            .map(|mut adj| {
                let action = if adj.ignored {
                    Action::Ignored
                } else if adj.holding.symbol == self.cash_sweep.symbol {
                    if adj.holding.current_value > self.cash_sweep.minimum {
                        Action::Sell(adj.holding.current_value - self.cash_sweep.minimum)
                    } else if adj.holding.current_value < self.cash_sweep.minimum {
                        Action::Buy(self.cash_sweep.minimum - adj.holding.current_value)
                    } else {
                        Action::DoNothing
                    }
                } else {
                    let desired_val = to_distribute * (adj.target / 100.0);
                    match desired_val - adj.holding.current_value {
                        val if val > 0.0 => Action::Buy(val.abs()),
                        val if val < 0.0 => Action::Sell(val.abs()),
                        _ => Action::DoNothing,
                    }
                };
                adj.action = action;
                adj
            })
            .collect();
        // sort core position first, then by current value, then by symbol name
        adjustments.sort_by(|a, b| match b.holding.is_cash.cmp(&a.holding.is_cash) {
            std::cmp::Ordering::Equal => match a
                .holding
                .current_value
                .partial_cmp(&b.holding.current_value)
                .unwrap_or(std::cmp::Ordering::Equal)
                .reverse()
            {
                std::cmp::Ordering::Equal => a.holding.symbol.cmp(&b.holding.symbol),
                res => res,
            },
            res => res,
        });
        Ok(adjustments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Action;

    #[test]
    fn test_config_validate() {
        let mut targets = HashMap::new();
        targets.insert("A".to_string(), 50.0);
        targets.insert("B".to_string(), 50.0);
        let config = Config {
            account_number: "123".to_string(),
            cash_sweep: CashConfig {
                symbol: "CORE".to_string(),
                minimum: 100.0,
            },
            targets,
            ignored_holdings: vec![],
        };
        assert!(config.validate().is_ok());

        let mut targets = HashMap::new();
        targets.insert("A".to_string(), 50.0);
        targets.insert("B".to_string(), 40.0);
        let config = Config {
            account_number: "123".to_string(),
            cash_sweep: CashConfig {
                symbol: "CORE".to_string(),
                minimum: 100.0,
            },
            targets,
            ignored_holdings: vec![],
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_adjust_allocations_basic() {
        let mut targets = HashMap::new();
        targets.insert("A".to_string(), 50.0);
        targets.insert("B".to_string(), 50.0);
        let config = Config {
            account_number: "123".to_string(),
            cash_sweep: CashConfig {
                symbol: "CORE".to_string(),
                minimum: 1000.0,
            },
            targets,
            ignored_holdings: vec![],
        };

        let balance = Balance {
            account_number: "123".to_string(),
            account_name: "Test Account".to_string(),
            holdings: vec![
                Holding {
                    symbol: "CORE".to_string(),
                    current_value: 5000.0,
                    is_cash: true,
                },
                Holding {
                    symbol: "A".to_string(),
                    current_value: 1000.0,
                    is_cash: false,
                },
                Holding {
                    symbol: "B".to_string(),
                    current_value: 1000.0,
                    is_cash: false,
                },
            ],
        };

        let adjustments = config.adjust_allocations(&balance).unwrap();
        assert_eq!(adjustments.len(), 3);

        let core_adj = adjustments
            .iter()
            .find(|a| a.holding.symbol == "CORE")
            .unwrap();
        if let Action::Sell(amount) = core_adj.action {
            assert_eq!(amount, 4000.0);
        } else {
            panic!(
                "CORE action should be Sell(4000.0), but was {:?}",
                core_adj.action
            );
        }

        let a_adj = adjustments
            .iter()
            .find(|a| a.holding.symbol == "A")
            .unwrap();
        if let Action::Buy(amount) = a_adj.action {
            assert_eq!(amount, 2000.0);
        } else {
            panic!("A action should be Buy(2000.0), but was {:?}", a_adj.action);
        }

        let b_adj = adjustments
            .iter()
            .find(|a| a.holding.symbol == "B")
            .unwrap();
        if let Action::Buy(amount) = b_adj.action {
            assert_eq!(amount, 2000.0);
        } else {
            panic!("B action should be Buy(2000.0), but was {:?}", b_adj.action);
        }
    }

    #[test]
    fn test_adjust_allocations_with_ignored() {
        let mut targets = HashMap::new();
        targets.insert("A".to_string(), 100.0);
        let config = Config {
            account_number: "123".to_string(),
            cash_sweep: CashConfig {
                symbol: "CORE".to_string(),
                minimum: 1000.0,
            },
            targets,
            ignored_holdings: vec!["IGNORED".to_string()],
        };

        let balance = Balance {
            account_number: "123".to_string(),
            account_name: "Test Account".to_string(),
            holdings: vec![
                Holding {
                    symbol: "CORE".to_string(),
                    current_value: 5000.0,
                    is_cash: true,
                },
                Holding {
                    symbol: "A".to_string(),
                    current_value: 1000.0,
                    is_cash: false,
                },
                Holding {
                    symbol: "IGNORED".to_string(),
                    current_value: 2000.0,
                    is_cash: false,
                },
            ],
        };

        let adjustments = config.adjust_allocations(&balance).unwrap();

        assert_eq!(adjustments.len(), 3);

        let ignored_adj = adjustments
            .iter()
            .find(|a| a.holding.symbol == "IGNORED")
            .unwrap();
        matches!(ignored_adj.action, Action::Ignored);

        // the total value to consider for distribution is 6000 (5000 core +
        // 1000 A), since IGNORED is ignored
        let a_adj = adjustments
            .iter()
            .find(|a| a.holding.symbol == "A")
            .unwrap();
        if let Action::Buy(amount) = a_adj.action {
            assert_eq!(amount, 4000.0);
        } else {
            panic!("A action should be Buy(4000.0), but was {:?}", a_adj.action);
        }
        let core_adj = adjustments
            .iter()
            .find(|a| a.holding.symbol == "CORE")
            .unwrap();
        if let Action::Sell(amount) = core_adj.action {
            assert_eq!(amount, 4000.0);
        } else {
            panic!(
                "CORE action should be Sell(4000.0), but was {:?}",
                core_adj.action
            );
        }
    }
}
