use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::anyhow;
use directories::ProjectDirs;
use driftfix::{account, provider};
use tracing::{debug, warn};

use crate::{
    backup::{self, BackupFile},
    cli::{self, PlanArgs},
    output,
};

pub struct App {
    dirs: ProjectDirs,
    config_file: PathBuf,
    args: cli::Cli,
}

impl App {
    pub fn new(args: cli::Cli) -> anyhow::Result<Self> {
        let dirs = ProjectDirs::from("org", "quotidian", "driftfix")
            .ok_or_else(|| anyhow!("Unable to determine project directories"))?;

        // Ensure the app directories exist
        ensure_dir_exists(dirs.config_dir())?;
        ensure_dir_exists(dirs.data_dir())?;

        Ok(Self {
            config_file: args
                .config
                .clone()
                .unwrap_or(dirs.config_dir().join("target.yml")),
            dirs,
            args,
        })
    }

    pub fn run(&self) -> anyhow::Result<()> {
        match &self.args.command {
            cli::Command::Edit => self.edit_command(),
            cli::Command::Plan(plan_args) => self.plan_command(plan_args),
        }
    }

    fn edit_command(&self) -> anyhow::Result<()> {
        let backup = BackupFile::new(
            self.config_file.clone(),
            Some(account::Config::example_config()?),
        )?;
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

    fn plan_command(&self, args: &PlanArgs) -> anyhow::Result<()> {
        let mut account_configs = account::Config::load_from_file(&self.config_file)?;
        if let Some(acct) = &args.account {
            account_configs.retain(|acc| acc.account_number == *acct)
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
        let mut accounts_with_config =
            HashMap::<String, (account::Balance, account::Config)>::new();
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
}

fn ensure_dir_exists(dir: &Path) -> anyhow::Result<()> {
    match std::fs::create_dir_all(dir) {
        Err(e) if e.kind() != std::io::ErrorKind::AlreadyExists => {
            Err(anyhow!("Failed to initialize data directory: {e}"))
        }
        _ => Ok(()),
    }
}
