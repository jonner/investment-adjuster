use anyhow::Context;
use clap::Parser;
use directories::ProjectDirs;
use tracing::debug;

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

    let mut position_reader = csv::Reader::from_path(opts.current_allocations)?;
    debug!("created reader");
    let positions = position_reader
        .deserialize()
        .filter_map(|record| record.ok())
        .collect();
    debug!(?positions, "got positions");

    let Some(targets_path) =
        opts.target
            .or(ProjectDirs::from("org", "quotidian", "investment-adjuster")
                .map(|pdirs| pdirs.config_dir().join("target.yml")))
    else {
        anyhow::bail!("Failed to get target path");
    };
    let targets_file = std::fs::File::open(&targets_path)
        .with_context(|| format!("Failed to open file {targets_path:?}"))?;
    let account_targets: target::AccountTargetBuilder = serde_yaml::from_reader(targets_file)?;
    let account_targets = account_targets.build()?;
    debug!(?account_targets, "got targets");

    println!(
        "Allocation targets for account {}",
        account_targets.account_number
    );
    account_targets
        .targets()
        .into_iter()
        .for_each(|pos| println!(" - {}: {:.1}%", pos.symbol, pos.percent));
    println!(
        " - Core position({}): ${} Minimum",
        account_targets.core_position.symbol, account_targets.core_position.minimum
    );
    println!();

    let actions = account_targets.process(positions)?;
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
