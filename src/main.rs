use investment_adjuster::{Action, Dollar, Percent};
use std::{collections::HashMap, io::Write, path::Path};
use tracing::warn;

use anyhow::anyhow;
use clap::Parser;
use directories::ProjectDirs;

use crate::cli::AdjustArgs;

mod account;
mod cli;
mod output;
mod provider;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let opts = cli::Cli::parse();

    let Some(config_path) =
        opts.config
            .or(ProjectDirs::from("org", "quotidian", "investment-adjuster")
                .map(|pdirs| pdirs.config_dir().join("target.yml")))
    else {
        anyhow::bail!("Failed to get target path");
    };

    match opts.command {
        cli::Command::Edit => {
            edit_command(&config_path)?;
        }
        cli::Command::Adjust(args) => adjust_command(args, config_path)?,
    }
    Ok(())
}

fn edit_command<P: AsRef<Path>>(config_path: P) -> Result<(), anyhow::Error> {
    let path = config_path.as_ref();
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    let mut try_again = true;
    let mut command = std::process::Command::new(editor);
    command.arg(path);
    while try_again {
        let exit_status = command.status()?;
        if !exit_status.success() {
            warn!("Failed to edit configuration file '{}'", path.display());
        } else {
            match account::Config::load_from_file(&config_path) {
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

fn adjust_command<P: AsRef<Path>>(args: AdjustArgs, config_path: P) -> Result<(), anyhow::Error> {
    let mut account_configs = account::Config::load_from_file(config_path.as_ref())?;
    if let Some(acct) = args.account {
        account_configs.retain(|acc| acc.account_number == acct)
    }
    if let Some(keep) = args.core_minimum {
        if account_configs.len() != 1 {
            anyhow::bail!(
                "--core-minimum can only be used with a single account. Try specifying --account."
            );
        }
        account_configs[0].core_position.minimum = keep;
    }
    let portfolio = provider::load_portfolio(&args.account_balances, args.provider)?;
    let mut accounts_with_config = HashMap::<String, (account::Balance, account::Config)>::new();
    for account in portfolio.accounts {
        if let Some(cfg) = account_configs
            .iter()
            .find(|t| t.account_number == account.account_number)
        {
            accounts_with_config.insert(account.account_number.clone(), (account, cfg.clone()));
        }
    }
    if accounts_with_config.is_empty() {
        return Err(anyhow!(
            "Failed to find any accounts with allocation targets",
        ));
    }
    for (_, (account, mut config)) in accounts_with_config {
        config.ignored.extend(args.ignore.iter().cloned());

        let adjustments = config.adjust_allocations(&account)?;
        let table = output::format_adjustments(adjustments);

        println!(
            "Account {}: {}",
            account.account_number, account.account_name
        );
        println!("{table}\n");
    }
    Ok(())
}
