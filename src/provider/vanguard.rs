use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, bail};
use tracing::{debug, warn};

use crate::{
    Dollar,
    account::{Balance, Portfolio, Position},
    provider::Provider,
};

pub enum Columns {
    AccountNumber = 0,
    Symbol = 2,
    TotalValue = 5,
}

struct ProviderImpl;
pub fn provider() -> impl Provider {
    ProviderImpl
}

impl Provider for ProviderImpl {
    fn parse_accounts(&self, path: &Path) -> anyhow::Result<Portfolio> {
        let mut csv_reader = csv::ReaderBuilder::new().flexible(true).from_path(path)?;
        let headers = csv_reader.headers()?;
        if headers.get(Columns::AccountNumber as usize) != Some("Account Number")
            && headers.get(Columns::Symbol as usize) != Some("Symbol")
            && headers.get(Columns::TotalValue as usize) != Some("Total Value")
        {
            warn!(?headers, "Unexpected headers");
            bail!("Unexpected csv file format");
        }
        let mut accounts = HashMap::<String, Balance>::new();
        for row in csv_reader.records() {
            let row = row?;
            debug!(?row, "parsed row");
            if row.len() < Columns::TotalValue as usize {
                debug!(
                    ?row,
                    "Row doesn't have enough fields to be a position ({} < {})",
                    row.len(),
                    Columns::TotalValue as usize
                );
                break;
            }
            let Some(account_number) = row.get(Columns::AccountNumber as usize) else {
                bail!("failed to get account number for row");
            };
            let acct = accounts
                .entry(account_number.to_string())
                .or_insert(Balance {
                    account_number: account_number.to_string(),
                    ..Default::default()
                });
            let symbol = row
                .get(Columns::Symbol as usize)
                .ok_or_else(|| anyhow!("Failed to get symbol"))?;
            let total_value = row
                .get(Columns::TotalValue as usize)
                .and_then(|s| s.replace('$', "").parse::<Dollar>().ok())
                .ok_or_else(|| anyhow!("Failed to get symbol"))?;
            let pos = Position {
                symbol: symbol.trim_end_matches("**").to_string(),
                current_value: total_value,
                // FIXME: is this reasonable?
                is_core: symbol.eq_ignore_ascii_case("VMFXX"),
            };
            debug!(?acct, ?pos, "adding regular position");
            acct.positions.push(pos);
        }
        Ok(Portfolio {
            accounts: accounts.into_values().collect(),
        })
    }
}
