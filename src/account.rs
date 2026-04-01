use std::{collections::HashMap, io::ErrorKind, path::Path};

use anyhow::{Context, anyhow, bail};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{Action, Dollar, Percent};

/// A representation of the balance of a brokerage account
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Balance {
    /// The unique ID number associated with the account
    pub account_id: String,
    /// A user-friendly name for the account
    pub account_name: String,
    /// A list of investments in the account
    pub holdings: Vec<Holding>,
}

impl Balance {
    /// Calculates the total value of this account
    pub fn total_value(&self) -> Dollar {
        self.holdings
            .iter()
            .map(|holding| holding.current_value)
            .sum()
    }
}

/// A single investment within a brokerage account
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Holding {
    /// the string representing the stock or fund (e.g. 'GOOG')
    pub symbol: String,
    /// The current value of the investment
    pub current_value: Dollar,
    /// Whether this investment represents cash within the account. Different
    /// brokerages have different terms for this. Fidelity calls it your 'Core
    /// position', Vanguard calls it your 'Settlement fund'. It is generally cash or a
    /// money market fund.
    pub is_cash: bool,
}

/// A description of a current holding and what needs to be done to align it
/// with a given target allocation
#[derive(Debug, Default)]
pub struct PositionAdjustment {
    pub holding: Holding,
    pub target: Percent,
    pub ignored: bool,
    pub action: Action,
}

/// A definition of the desired state of the cash sweep within a given brokerage account
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct CashConfig {
    /// The fund that represents the cash sweep (perhaps a money market fund)
    pub symbol: String,
    /// Minimum amount to retain in the core position in dollars
    pub minimum: Dollar,
}

/// A definition of the desired allocations for a given brokerage account
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct AllocationConfig {
    /// The account that is being configured
    pub account_id: String,
    /// A nickname for the account
    pub nickname: Option<String>,
    /// the desired state of the cash sweep for this account
    #[serde(default)]
    pub cash_sweep: Option<CashConfig>,
    /// The desired target allocation for specific holdings within this account.
    /// The percentages for all targets should add up to exactly 100%
    pub targets: HashMap<String, Percent>,
    /// Any symbols listed here will be ignored from all analysis
    #[serde(default)]
    pub ignored_holdings: Vec<String>,
}

impl AllocationConfig {
    /// Ensure that the target allocations are reasonable
    fn validate(&self) -> anyhow::Result<()> {
        let total_percent: Percent = self.targets.values().sum();
        anyhow::ensure!(
            total_percent == Percent(100.0),
            "Target allocations for account {} do not add up to 100%",
            self.account_id
        );
        Ok(())
    }

    /// Load a series of [Config] objects from the given yaml file path
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<Self>> {
        let targets_file = match std::fs::File::open(path.as_ref()) {
            Ok(f) => Ok(f),
            Err(e) if e.kind() == ErrorKind::NotFound => Err(anyhow!(
                "Please configure target allocations first. See help for more information."
            )),
            e => e.with_context(|| format!("Failed to open file {:?}", path.as_ref())),
        }?;
        let targets: Vec<Self> = serde_yaml::from_reader(targets_file)?;
        targets
            .into_iter()
            .map(|t| {
                t.validate()?;
                Ok(t)
            })
            .collect()
    }

    fn cash_minimum(&self) -> Dollar {
        self.cash_sweep
            .as_ref()
            .map(|cash| cash.minimum)
            .unwrap_or_default()
    }
    /// Compare this configuration with the given `balance` and calculate what adjustments need to be
    /// made in order to align the balance with the desired target allocations
    pub fn adjust_allocations(&self, balance: &Balance) -> anyhow::Result<Vec<PositionAdjustment>> {
        anyhow::ensure!(
            self.account_id == balance.account_id,
            "The target configuration doesn't apply to this account"
        );
        let cash_fallback = self.cash_sweep.as_ref().map(|sweep| Holding {
            symbol: sweep.symbol.clone(),
            current_value: Dollar(0.0),
            is_cash: true,
        });
        let cash_sweep = balance
            .holdings
            .iter()
            .find(|&pos| Some(&pos.symbol) == self.cash_sweep.as_ref().map(|s| &s.symbol))
            .or(cash_fallback.as_ref());
        debug!(?cash_sweep, ?balance);

        // make sure the output contains information about all holdings in the
        // account balance
        let mut adjustments: HashMap<String, PositionAdjustment> = HashMap::new();
        for holding in balance.holdings.iter() {
            let ignored = self.ignored_holdings.contains(&holding.symbol);
            adjustments.insert(
                holding.symbol.clone(),
                PositionAdjustment {
                    holding: holding.clone(),
                    ignored,
                    ..Default::default()
                },
            );
        }

        // make sure the output contains information about all targets in the
        // allocation configuration
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

        if let Some(cash_sweep) = cash_sweep {
            // make sure the output contains a cash sweep if a cash minimum is specified
            adjustments
                .entry(cash_sweep.symbol.to_owned())
                .and_modify(|e| e.holding.current_value = cash_sweep.current_value)
                .or_insert(PositionAdjustment {
                    holding: cash_sweep.clone(),
                    ignored: self.ignored_holdings.contains(&cash_sweep.symbol),
                    ..Default::default()
                });
        }

        for (symbol, adj) in adjustments.iter() {
            if adj.ignored && adj.target != Percent(0.0) {
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
        let cash_target = adjustments
            .values()
            .find(|v| v.holding.is_cash)
            .map(|adj| adj.target);
        let cash_desired = self
            .cash_minimum()
            .max(cash_target.map(|t| total_val * t).unwrap_or_default());
        debug!(?cash_target, ?cash_desired);

        // make sure there is only a single cash holding in the list
        anyhow::ensure!(
            adjustments
                .iter()
                .filter(|(_k, v)| v.holding.is_cash)
                .count()
                <= 1,
            "Account should contain at most 1 cash holding"
        );

        let mut adjustments: Vec<_> = adjustments
            .into_values()
            .map(|mut adj| {
                let action = if adj.ignored {
                    Action::DoNothing
                } else {
                    let mut desired_val = total_val * adj.target;
                    debug!(?desired_val, ?adj);
                    if adj.holding.is_cash {
                        desired_val = cash_desired;
                        debug!(?desired_val, "Setting cash val");
                    } else {
                        // if the minimum cash position was enforced, that
                        // leaves less to allocate for other holdings, so we
                        // allocate the rest of the holdings proportionally to
                        // their targets, even if they can't be acheived
                        if cash_desired == self.cash_minimum() {
                            let remainder = total_val - cash_desired;
                            let noncash_pct = Percent(100.0) - cash_target.unwrap_or_default();
                            let pct = adj.target / noncash_pct;
                            desired_val = remainder * pct;
                            debug!(
                                ?remainder,
                                ?noncash_pct,
                                ?pct,
                                ?desired_val,
                                ?adj.target,
                                "Setting adjusted non-cash val"
                            );
                        } else {
                            desired_val = total_val * adj.target;
                            debug!(?desired_val, "Setting non-cash val");
                        }
                    }
                    debug!(?desired_val, ?adj.holding.current_value, "setting action");
                    match desired_val - adj.holding.current_value {
                        val if val > Dollar(0.0) => Action::Buy(val.abs()),
                        val if val < Dollar(0.0) => Action::Sell(val.abs()),
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

    #[doc(hidden)]
    pub fn example_config() -> anyhow::Result<String> {
        let mut targets = HashMap::new();
        targets.insert("SYMBOL1".to_string(), Percent(75.0));
        targets.insert("SYMBOL2".to_string(), Percent(25.0));
        let ignored_holdings = vec!["SYMBOL3".to_string()];
        let config = Self {
            account_id: "<ACCOUNT_ID>".to_string(),
            cash_sweep: Some(CashConfig {
                symbol: "CASH_SYMBOL".to_string(),
                minimum: Dollar(1000.0),
            }),
            targets,
            ignored_holdings,
            nickname: None,
        };
        let s = serde_yaml::to_string(&vec![config])?;
        let comment = r#"# This is an example configuration.
# Modify the following lines to suit your needs
"#
        .to_string();
        Ok(comment + &s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Action;

    #[test]
    fn test_config_validate() {
        let mut targets = HashMap::new();
        targets.insert("A".to_string(), Percent(50.0));
        targets.insert("B".to_string(), Percent(50.0));
        let config = AllocationConfig {
            account_id: "123".to_string(),
            cash_sweep: Some(CashConfig {
                symbol: "CORE".to_string(),
                minimum: Dollar(100.0),
            }),
            targets,
            ignored_holdings: vec![],
            nickname: None,
        };
        assert!(config.validate().is_ok());

        let mut targets = HashMap::new();
        targets.insert("A".to_string(), Percent(50.0));
        targets.insert("B".to_string(), Percent(40.0));
        let config = AllocationConfig {
            account_id: "123".to_string(),
            cash_sweep: Some(CashConfig {
                symbol: "CORE".to_string(),
                minimum: Dollar(100.0),
            }),
            targets,
            ignored_holdings: vec![],
            nickname: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_adjust_allocations_basic() {
        let mut targets = HashMap::new();
        targets.insert("A".to_string(), Percent(50.0));
        targets.insert("B".to_string(), Percent(50.0));
        let config = AllocationConfig {
            account_id: "123".to_string(),
            cash_sweep: Some(CashConfig {
                symbol: "CORE".to_string(),
                minimum: Dollar(1000.0),
            }),
            targets,
            ignored_holdings: vec![],
            nickname: None,
        };

        let balance = Balance {
            account_id: "123".to_string(),
            account_name: "Test Account".to_string(),
            holdings: vec![
                Holding {
                    symbol: "CORE".to_string(),
                    current_value: Dollar(5000.0),
                    is_cash: true,
                },
                Holding {
                    symbol: "A".to_string(),
                    current_value: Dollar(1000.0),
                    is_cash: false,
                },
                Holding {
                    symbol: "B".to_string(),
                    current_value: Dollar(1000.0),
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
            assert_eq!(amount, Dollar(4000.0));
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
            assert_eq!(amount, Dollar(2000.0));
        } else {
            panic!("A action should be Buy(2000.0), but was {:?}", a_adj.action);
        }

        let b_adj = adjustments
            .iter()
            .find(|a| a.holding.symbol == "B")
            .unwrap();
        if let Action::Buy(amount) = b_adj.action {
            assert_eq!(amount, Dollar(2000.0));
        } else {
            panic!("B action should be Buy(2000.0), but was {:?}", b_adj.action);
        }
    }

    #[test]
    fn test_adjust_allocations_with_ignored() {
        let mut targets = HashMap::new();
        targets.insert("A".to_string(), Percent(100.0));
        let config = AllocationConfig {
            account_id: "123".to_string(),
            cash_sweep: Some(CashConfig {
                symbol: "CORE".to_string(),
                minimum: Dollar(1000.0),
            }),
            targets,
            ignored_holdings: vec!["IGNORED".to_string()],
            nickname: None,
        };

        let balance = Balance {
            account_id: "123".to_string(),
            account_name: "Test Account".to_string(),
            holdings: vec![
                Holding {
                    symbol: "CORE".to_string(),
                    current_value: Dollar(5000.0),
                    is_cash: true,
                },
                Holding {
                    symbol: "A".to_string(),
                    current_value: Dollar(1000.0),
                    is_cash: false,
                },
                Holding {
                    symbol: "IGNORED".to_string(),
                    current_value: Dollar(2000.0),
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
        assert!(ignored_adj.ignored);

        // the total value to consider for distribution is 6000 (5000 core +
        // 1000 A), since IGNORED is ignored
        let a_adj = adjustments
            .iter()
            .find(|a| a.holding.symbol == "A")
            .unwrap();
        if let Action::Buy(amount) = a_adj.action {
            assert_eq!(amount, Dollar(4000.0));
        } else {
            panic!("A action should be Buy(4000.0), but was {:?}", a_adj.action);
        }
        let core_adj = adjustments
            .iter()
            .find(|a| a.holding.symbol == "CORE")
            .unwrap();
        if let Action::Sell(amount) = core_adj.action {
            assert_eq!(amount, Dollar(4000.0));
        } else {
            panic!(
                "CORE action should be Sell(4000.0), but was {:?}",
                core_adj.action
            );
        }
    }
}
