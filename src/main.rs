use anyhow::anyhow;
use clap::Parser;
use directories::ProjectDirs;

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
    let account_targets = AccountTarget::load_from_file(targets_path)?;
    println!(
        "Allocation targets for account {}",
        account_targets.account_number
    );
    account_targets
        .targets()
        .into_iter()
        .for_each(|(symbol, percent)| println!(" - {}: {:.1}%", symbol, percent));
    println!(
        " - Core position({}): ${} Minimum",
        account_targets.core_position.symbol, account_targets.core_position.minimum
    );
    println!();

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
    println!("In order to maintain your target allocations, the following actions are necessary.");
    println!("Sell:");
    actions.iter().for_each(|(symbol, action)| {
        if let Action::Sell(val) = action {
            println!(" - {symbol}: {val:.2}");
        }
    });
    println!("Buy:");
    actions.iter().for_each(|(symbol, action)| {
        if let Action::Buy(val) = action {
            println!(" - {symbol}: {val:.2}")
        }
    });
    Ok(())
}
