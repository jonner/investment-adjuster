use std::{fmt::Debug, path::Path};

use clap::ValueEnum;

use crate::account::Portfolio;

pub(crate) mod fidelity {
    use std::{collections::HashMap, path::Path};

    use anyhow::{anyhow, bail};
    use tracing::{debug, warn};

    use crate::{
        Dollar,
        account::{AccountBalance, Position},
    };

    pub enum Columns {
        AccountNumber = 0,
        AccountName = 1,
        Symbol = 2,
        CurrentValue = 7,
    }

    pub fn parse_accounts<P: AsRef<Path>>(
        path: P,
    ) -> Result<HashMap<String, AccountBalance>, anyhow::Error> {
        let mut csv_reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_path(path.as_ref())?;
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
            let Some(account_name) = row.get(Columns::AccountName as usize) else {
                bail!("failed to get account name for row");
            };
            let acct = accounts
                .entry(account_number.to_string())
                .or_insert(AccountBalance {
                    account_number: account_number.to_string(),
                    account_name: account_name.to_string(),
                    ..Default::default()
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
                if let Some(core) = acct.positions.iter_mut().find(|p| p.is_core) {
                    core.current_value += current_value;
                } else {
                    warn!(
                        "Account '{}' has ${current_value} in pending activity but cannot find core position.",
                        acct.account_number
                    );
                }
            } else {
                let pos = Position {
                    symbol: symbol.trim_end_matches("**").to_string(),
                    current_value,
                    is_core: symbol.ends_with("**"),
                };
                debug!(?acct, ?pos, "adding regular position");
                acct.positions.push(pos);
            }
        }
        Ok(accounts)
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum Provider {
    Fidelity,
}

impl Provider {
    pub fn load_portfolio<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<Portfolio> {
        match self {
            Provider::Fidelity => {
                let accounts = fidelity::parse_accounts(path)?;
                Ok(Portfolio {
                    accounts: accounts.into_values().collect(),
                })
            }
        }
    }
}
