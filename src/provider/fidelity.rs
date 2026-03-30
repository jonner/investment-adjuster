use std::{collections::HashMap, io::Read};

use anyhow::{anyhow, bail};
use tracing::{debug, warn};

use crate::{
    Dollar,
    account::{Balance, Holding},
    provider::Provider,
};

pub enum Columns {
    AccountNumber = 0,
    AccountName = 1,
    Symbol = 2,
    CurrentValue = 7,
}

pub fn provider() -> impl Provider {
    ProviderImpl
}

struct ProviderImpl;

impl Provider for ProviderImpl {
    fn parse_portfolio(&self, reader: &mut dyn Read) -> anyhow::Result<Vec<Balance>> {
        if !self.detect(reader)? {
            bail!("Portfolio file does not appear to be a valid Fidelity CSV file.");
        }
        let mut csv_reader = csv::ReaderBuilder::new().flexible(true).from_reader(reader);
        let mut accounts = HashMap::<String, Balance>::new();
        for row in csv_reader.records() {
            let row = row?;
            debug!(?row, "parsed row");
            if row.len() < Columns::CurrentValue as usize {
                debug!(?row, "Row doesn't have enough fields to be a position");
                break;
            }
            let Some(account_id) = row.get(Columns::AccountNumber as usize) else {
                bail!("failed to get account number for row");
            };
            let Some(account_name) = row.get(Columns::AccountName as usize) else {
                bail!("failed to get account name for row");
            };
            let acct = accounts.entry(account_id.to_string()).or_insert(Balance {
                account_id: account_id.to_string(),
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
                if let Some(core) = acct.holdings.iter_mut().find(|p| p.is_cash) {
                    core.current_value += current_value;
                } else {
                    warn!(
                        "Account '{}' has ${current_value} in pending activity but cannot find core position.",
                        acct.account_id
                    );
                }
            } else {
                let pos = Holding {
                    symbol: symbol.trim_end_matches("**").to_string(),
                    current_value,
                    is_cash: symbol.ends_with("**"),
                };
                debug!(?acct, ?pos, "adding regular position");
                acct.holdings.push(pos);
            }
        }
        Ok(accounts.into_values().collect())
    }

    fn detect(&self, reader: &mut dyn Read) -> anyhow::Result<bool> {
        let mut csv_reader = csv::ReaderBuilder::new().flexible(true).from_reader(reader);
        let headers = csv_reader.headers()?;
        let mut iter = headers.iter();

        let valid = iter.next() == Some("Account Number")
            && iter.next() == Some("Account Name")
            && iter.next() == Some("Symbol")
            && iter.next() == Some("Description")
            && iter.next() == Some("Quantity")
            && iter.next() == Some("Last Price")
            && iter.next() == Some("Last Price Change")
            && iter.next() == Some("Current Value")
            && iter.next() == Some("Today's Gain/Loss Dollar")
            && iter.next() == Some("Today's Gain/Loss Percent")
            && iter.next() == Some("Total Gain/Loss Dollar")
            && iter.next() == Some("Total Gain/Loss Percent")
            && iter.next() == Some("Percent Of Account")
            && iter.next() == Some("Cost Basis Total")
            && iter.next() == Some("Average Cost Basis")
            && iter.next() == Some("Type")
            && iter.next().is_none();
        Ok(valid)
    }
}
