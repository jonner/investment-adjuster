use driftfix::{Action, Dollar, Percent, account, provider};
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
};
use tracing::{debug, warn};

use anyhow::{anyhow, bail};
use clap::Parser;
use directories::ProjectDirs;

use crate::{backup::BackupFile, cli::PlanArgs};

mod backup;
mod cli;
mod output;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let opts = cli::Cli::parse();
    let config_dir = ProjectDirs::from("org", "quotidian", "driftfix")
        .map(|pdirs| pdirs.config_dir().to_path_buf())
        .ok_or_else(|| anyhow!("Failed to get configuration directory"))?;

    // Ensure the config directory exists
    match std::fs::create_dir_all(&config_dir) {
        Err(e) if e.kind() != std::io::ErrorKind::AlreadyExists => {
            bail!("Failed to initialize config directory: {e}");
        }
        _ => (),
    }

    let config_file = opts.config.unwrap_or(config_dir.join("target.yml"));

    match opts.command {
        cli::Command::Edit => {
            edit_command(config_file)?;
        }
        cli::Command::Plan(args) => plan_command(args, config_file)?,
    }
    Ok(())
}

fn edit_command(config_path: PathBuf) -> Result<(), anyhow::Error> {
    let backup = BackupFile::new(config_path, Some(account::Config::example_config()?))?;
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    debug!(editor, "Using editor");
    let mut command = std::process::Command::new(editor);
    command.stdin(Stdio::piped()).arg(backup.path());

    loop {
        let exit_status = command.status()?;
        if !exit_status.success() {
            warn!(
                "Failed to edit configuration file '{}'",
                backup.path().display()
            );
        } else {
            match account::Config::load_from_file(backup.path()) {
                Ok(_) => match backup.finish() {
                    Ok(p) => {
                        println!("Updated configuration file '{}'", p.display());
                        return Ok(());
                    }
                    Err(backup::Error::NotModified) => {
                        println!("Configuration file not updated");
                        return Ok(());
                    }
                    Err(e) => return Err(e.into()),
                },
                Err(e) => {
                    println!("Failed to validate configuration file: {e}");
                    print!("Would you like to try again? [y/N] ");
                    std::io::stdout().flush().unwrap();
                    let mut input = String::new();
                    if std::io::stdin().read_line(&mut input).is_ok() {
                        let line = input.trim().to_lowercase();
                        if !(line == "y" || line == "yes") {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}

fn plan_command<P: AsRef<Path>>(args: PlanArgs, config_path: P) -> Result<(), anyhow::Error> {
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
        account_configs[0].cash_sweep.minimum = keep;
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
        config.ignored_holdings.extend(args.ignore.iter().cloned());

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
