use investment_adjuster::{Action, Dollar, Percent};
use std::{collections::HashMap, io::Write, path::Path};
use tracing::warn;

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

use crate::{
    cli::AdjustArgs,
    portfolio::{AccountBalance, Portfolio, Position},
    target::AllocationTargets,
};

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

    match opts.command {
        cli::Command::Edit => {
            edit_targets(&targets_path)?;
        }
        cli::Command::Adjust(args) => {
            let targets = AllocationTargets::load_from_file(&targets_path)?;
            calculate_adjustments(args, targets)?
        }
    }
    Ok(())
}

fn edit_targets<P: AsRef<Path>>(targets_path: P) -> Result<(), anyhow::Error> {
    let path = targets_path.as_ref();
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    let mut try_again = true;
    let mut command = std::process::Command::new(editor);
    command.arg(path);
    while try_again {
        let exit_status = command.status()?;
        if !exit_status.success() {
            warn!("Failed to edit target file '{}'", path.display());
        } else {
            match AllocationTargets::load_from_file(&targets_path) {
                Ok(_) => {
                    println!("Updated configuration file '{}'", path.display());
                    try_again = false;
                }
                Err(e) => {
                    println!("Failed to validate configuration file: {e}");
                    print!("Would you like to try again? [y/N] ");
                    std::io::stdout().flush().unwrap();
                    let mut input = String::new();
                    if std::io::stdin().read_line(&mut input).is_ok() {
                        let line = input.trim().to_lowercase();
                        if !(line == "y" || line == "yes") {
                            try_again = false
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn calculate_adjustments(
    args: AdjustArgs,
    mut targets: Vec<AllocationTargets>,
) -> Result<(), anyhow::Error> {
    if let Some(acct) = args.account {
        targets.retain(|acc| acc.account_number == acct)
    }
    if let Some(keep) = args.core_minimum {
        if targets.len() != 1 {
            anyhow::bail!(
                "--core-minimum can only be used with a single account. Try specifying --account."
            );
        }
        targets[0].core_position.minimum = keep;
    }
    let portfolio = Portfolio::load_from_file(&args.account_balances, args.provider)?;
    let mut accounts_with_targets = HashMap::<String, (AccountBalance, AllocationTargets)>::new();
    for account in portfolio.accounts {
        if let Some(target) = targets
            .iter()
            .find(|t| t.account_number == account.account_number)
        {
            accounts_with_targets.insert(account.account_number.clone(), (account, target.clone()));
        }
    }
    if accounts_with_targets.is_empty() {
        return Err(anyhow!(
            "Failed to find any accounts with allocation targets",
        ));
    }
    for (_, (mut account, mut targets)) in accounts_with_targets {
        targets.ignored.extend(args.ignore.iter().cloned());
        account.set_ignored(&targets.ignored);

        let actions = targets.adjust_allocations(&account)?;

        // make sure that the account positions contain rows for the target allocations even if they don't yet exist in the account.
        for (sym, _) in targets.targets() {
            if !account.positions.iter().any(|e| e.symbol == sym) {
                account.positions.push(Position {
                    symbol: sym,
                    current_value: 0.0,
                    is_core: false,
                    ignored: false,
                })
            }
        }
        println!(
            "Account {}: {}",
            account.account_number, account.account_name
        );
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
    }
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
