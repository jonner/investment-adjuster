use anyhow::anyhow;
use clap::Parser;
use directories::ProjectDirs;
use tabled::{
    Table, Tabled,
    settings::{Alignment, Style, object::Columns},
};

use crate::{portfolio::Portfolio, target::AccountTarget};

type Dollar = f32;
// FIXME: handle dollar sign and plus/minus
type RelativeDollar = String;
type Percent = f32;
// FIXME: handle percent sign and plus/minus
type RelativePercent = String;

#[derive(Debug)]
pub enum Action {
    Nothing,
    Sell(Dollar),
    Buy(Dollar),
}

mod cli;
mod portfolio;
mod target;

fn display_optional_dollar(val: &Option<Dollar>) -> String {
    if let Some(val) = val {
        format!("${val:.2}")
    } else {
        "".to_string()
    }
}

fn display_dollar(val: &Dollar) -> String {
    format!("${val:.2}")
}

fn display_percentage(val: &Percent) -> String {
    format!("{val:.1}%")
}

fn display_optional_percentage(val: &Option<Percent>) -> String {
    if let Some(val) = val {
        display_percentage(val)
    } else {
        "".to_string()
    }
}

#[derive(Debug, Tabled)]
struct AllocationTableRow {
    #[tabled(rename = "Symbol")]
    symbol: String,
    #[tabled(rename = "Value", display = "display_dollar")]
    current_value: Dollar,
    #[tabled(rename = "Percent", display = "display_percentage")]
    current_percentage: Percent,
    #[tabled(rename = "Target", display = "display_optional_percentage")]
    target: Option<Percent>,
    #[tabled(rename = "Retain", display = "display_optional_dollar")]
    minimum: Option<Dollar>,
    #[tabled(rename = "Sell", display = "display_optional_dollar")]
    sell: Option<Dollar>,
    #[tabled(rename = "Buy", display = "display_optional_dollar")]
    buy: Option<Dollar>,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let opts = cli::Cli::parse();

    let Some(targets_path) =
        opts.target
            .or(ProjectDirs::from("org", "quotidian", "investment-adjuster")
                .map(|pdirs| pdirs.config_dir().join("target.yml")))
    else {
        anyhow::bail!("Failed to get target path");
    };
    let account_targets = AccountTarget::load_from_file(&targets_path)?;
    let portfolio = Portfolio::load_from_file(opts.current_allocations)?;
    let account = portfolio
        .accounts
        .iter()
        .find(|a| a.account_number == account_targets.account_number)
        .ok_or_else(|| {
            anyhow!(
                "Failed to find any positions for account {}",
                account_targets.account_number
            )
        })?;

    let actions = account_targets.process(account)?;

    println!("Account {}", account_targets.account_number);
    let total: f32 = account.positions.iter().map(|pos| pos.current_value).sum();
    let rows: Vec<AllocationTableRow> = account
        .positions
        .iter()
        .map(|pos| AllocationTableRow {
            symbol: pos.symbol.clone(),
            current_value: pos.current_value,
            current_percentage: pos.current_value / total * 100.0,
            target: account_targets.targets().get(&pos.symbol).copied(),
            minimum: match pos.is_core && account_targets.core_position.minimum > 0.0 {
                true => Some(account_targets.core_position.minimum),
                false => None,
            },
            buy: actions
                .iter()
                .find(|(symbol, _)| symbol == &pos.symbol)
                .and_then(|(_, action)| match action {
                    Action::Buy(val) => Some(*val),
                    _ => None,
                }),
            sell: actions
                .iter()
                .find(|(symbol, _)| symbol == &pos.symbol)
                .and_then(|(_, action)| match action {
                    Action::Sell(val) => Some(*val),
                    _ => None,
                }),
        })
        .collect();
    let mut table = Table::new(rows);
    table.with(Style::rounded());
    table.modify(Columns::new(..), Alignment::right());
    println!("{table}");
    println!("[ To change allocation targets, edit the file {targets_path:?} ]");
    Ok(())
}
