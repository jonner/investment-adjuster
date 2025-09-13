use anyhow::anyhow;
use clap::Parser;
use directories::ProjectDirs;
use tabled::{
    Table, Tabled,
    settings::{
        Alignment, Color, Style,
        object::{Columns, Rows},
    },
};

use crate::{portfolio::Portfolio, target::AllocationTargets};

type Dollar = f32;
// FIXME: handle dollar sign and plus/minus
type RelativeDollar = String;
type Percent = f32;
// FIXME: handle percent sign and plus/minus
type RelativePercent = String;

#[derive(Debug)]
pub enum Action {
    Nothing,
    Ignore,
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
#[tabled(display(Dollar, "display_dollar"))]
#[tabled(display(Option<Dollar>, "display_optional_dollar"))]
#[tabled(display(Percent, "display_percentage"))]
#[tabled(display(Option<Percent>, "display_optional_percentage"))]
struct AllocationTableRow {
    #[tabled(rename = "Symbol")]
    symbol: String,
    #[tabled(rename = "Value")]
    current_value: Dollar,
    #[tabled(rename = "Percent")]
    current_percentage: Percent,
    #[tabled(rename = "Target")]
    target: Option<Percent>,
    #[tabled(rename = "Retain")]
    minimum: Option<Dollar>,
    #[tabled(rename = "Sell")]
    sell: Option<Dollar>,
    #[tabled(rename = "Buy")]
    buy: Option<Dollar>,
    #[tabled(skip)]
    ignore: bool,
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
    let targets = AllocationTargets::load_from_file(&targets_path)?;
    let portfolio = Portfolio::load_from_file(&opts.account_balance, opts.provider)?;
    let mut account = portfolio
        .accounts
        .into_iter()
        .find(|a| a.account_number == targets.account_number)
        .ok_or_else(|| {
            anyhow!(
                "Failed to find any positions for account {}",
                targets.account_number
            )
        })?;
    account.set_ignored(&opts.ignore);

    let actions = targets.adjust_allocations(&account)?;

    println!("Account {}", targets.account_number);
    let total: f32 = account.positions.iter().map(|pos| pos.current_value).sum();
    let rows: Vec<AllocationTableRow> = account
        .positions
        .into_iter()
        .map(|pos| AllocationTableRow {
            symbol: pos.symbol.clone(),
            current_value: pos.current_value,
            current_percentage: pos.current_value / total * 100.0,
            target: targets.targets().get(&pos.symbol).copied(),
            minimum: match pos.is_core && targets.core_position.minimum > 0.0 {
                true => Some(targets.core_position.minimum),
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
            ignore: actions
                .iter()
                .find(|(symbol, _)| symbol == &pos.symbol)
                .map(|(_, action)| matches!(action, Action::Ignore))
                .unwrap_or(false),
        })
        .collect();
    let ignored_rows = find_ignored_rows(&rows);
    let mut table = Table::new(rows);
    table.with(Style::rounded());
    table.modify(Columns::new(..), Alignment::right());
    for r in ignored_rows {
        table.modify(Rows::one(r), Color::rgb_fg(150, 150, 150));
    }
    println!("{table}");
    println!("[ To change allocation targets, edit the file {targets_path:?} ]");
    Ok(())
}

fn find_ignored_rows(rows: &[AllocationTableRow]) -> Vec<usize> {
    let mut ignored_rows = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        if row.ignore {
            // header row is techincally the first row
            ignored_rows.push(i + 1)
        }
    }
    ignored_rows
}
