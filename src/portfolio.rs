use std::{fmt::Debug, path::PathBuf};

use clap::ValueEnum;

use crate::Dollar;

mod provider {
    pub(crate) mod fidelity {
        use std::{collections::HashMap, path::PathBuf};

        use anyhow::{anyhow, bail};
        use tracing::{debug, warn};

        use crate::{
            Dollar,
            portfolio::{AccountBalance, Position},
        };

        pub enum Columns {
            AccountNumber = 0,
            AccountName = 1,
            Symbol = 2,
            CurrentValue = 7,
        }

        pub(crate) fn parse_accounts(
            path: &PathBuf,
        ) -> Result<HashMap<String, AccountBalance>, anyhow::Error> {
            let mut csv_reader = csv::ReaderBuilder::new().flexible(true).from_path(path)?;
            let headers = csv_reader.headers()?;
            if headers.get(Columns::AccountNumber as usize) != Some("AccountNumber")
                && headers.get(Columns::AccountName as usize) != Some("Account Name")
                && headers.get(Columns::Symbol as usize) != Some("Symbol")
                && headers.get(Columns::CurrentValue as usize) != Some("Current Value")
            {
                warn!(?headers, "Unexpected headers");
                bail!("Unexpected csv file format");
            }
            let mut accounts = HashMap::<String, AccountBalance>::new();
            for row in csv_reader.records() {
                let row = row?;
                debug!(?row, "parsed row");
                if row.len() < Columns::CurrentValue as usize {
                    debug!(?row, "Row doesn't have enough fields to be a position");
                    break;
                }
                let Some(account_number) = row.get(Columns::AccountNumber as usize) else {
                    bail!("failed to get account number for row");
                };
                let acct = accounts
                    .entry(account_number.to_string())
                    .or_insert(AccountBalance {
                        account_number: account_number.to_string(),
                        positions: Default::default(),
                    });
                let symbol = row
                    .get(Columns::Symbol as usize)
                    .ok_or_else(|| anyhow!("Failed to get symbol"))?;
                let current_value = row
                    .get(Columns::CurrentValue as usize)
                    .and_then(|s| s.replace('$', "").parse::<Dollar>().ok())
                    .ok_or_else(|| anyhow!("Failed to get symbol"))?;
                if symbol == "Pending activity" {
                    debug!(?acct, "Adding pending activity to core position");
                    acct.positions
                        .iter_mut()
                        .find(|p| p.is_core)
                        .map(|p| p.current_value += current_value)
                        .ok_or_else(|| {
                            anyhow!("Failed to find core position for pending activity")
                        })?;
                } else {
                    let pos = Position {
                        symbol: symbol.trim_end_matches("**").to_string(),
                        current_value,
                        is_core: symbol.ends_with("**"),
                        ignored: false,
                    };
                    debug!(?acct, ?pos, "adding regular position");
                    acct.positions.push(pos);
                }
            }
            Ok(accounts)
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
                let accounts = provider::fidelity::parse_accounts(path)?;
                Ok(Self {
                    accounts: accounts.into_values().collect(),
                })
            }
        }
    }
}
