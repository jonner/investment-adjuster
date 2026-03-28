use std::{
    collections::HashMap,
    fs::File,
    io::{ErrorKind, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::anyhow;
use directories::ProjectDirs;
use driftfix::{
    account::{self, Balance},
    provider,
};
use tracing::{debug, trace, warn};

use crate::{
    backup::{self, BackupFile},
    cli::{self, DataAddArgs, DataArgs, PlanArgs},
    output,
};

#[derive(Debug)]
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
            cli::MainCommands::Edit => self.edit_command(),
            cli::MainCommands::Plan(plan_args) => self.plan_command(plan_args),
            cli::MainCommands::Data(data_args) => self.data_command(data_args),
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
        let accounts = self.load_balances()?;
        let mut accounts_with_config =
            HashMap::<String, (account::Balance, account::Config)>::new();
        for account in accounts {
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

    fn data_command(&self, args: &DataArgs) -> anyhow::Result<()> {
        match &args.command {
            cli::DataCommands::Add(data_add_args) => self.data_add_command(data_add_args),
            cli::DataCommands::Remove { account } => self.data_remove_command(account),
        }
    }

    fn data_add_command(&self, args: &DataAddArgs) -> anyhow::Result<()> {
        let portfolio = provider::load_portfolio(&args.account_balances, args.provider)?;
        for account in portfolio.accounts {
            self.import_account_balance(account)?;
        }
        Ok(())
    }

    fn import_account_balance(&self, balance: Balance) -> anyhow::Result<()> {
        let mut balances = self.load_balances()?;
        trace!(?balances, "before adding new account balance");
        balances.retain(|b| b.account_number != balance.account_number);
        balances.push(balance);
        trace!(?balances, "after adding new account balance");
        self.save_balances(&balances)?;
        Ok(())
    }

    #[tracing::instrument(ret, level = "trace")]
    fn save_balances(&self, balances: &[Balance]) -> anyhow::Result<()> {
        let path = self.dirs.data_dir().join("balances.yml");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        let contents = serde_yaml::to_string(balances)?;
        file.write_all(contents.as_bytes())?;
        Ok(())
    }

    #[tracing::instrument(ret, level = "trace")]
    fn load_balances(&self) -> anyhow::Result<Vec<Balance>> {
        let path = self.dirs.data_dir().join("balances.yml");
        trace!(?path);
        match File::open(path) {
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e.into()),
            Ok(mut file) => {
                let mut contents = String::new();
                file.read_to_string(&mut contents)?;
                trace!(contents, "read file");
                serde_yaml::from_str(&contents).map_err(Into::into)
            }
        }
    }

    fn data_remove_command(&self, account: &str) -> anyhow::Result<()> {
        let mut balances = self.load_balances()?;
        balances.retain(|b| b.account_number != account);
        self.save_balances(&balances)?;
        Ok(())
    }
}

fn ensure_dir_exists(dir: &Path) -> anyhow::Result<()> {
    debug!(?dir, "ensuring dir exists");
    match std::fs::create_dir_all(dir) {
        Err(e) if e.kind() != std::io::ErrorKind::AlreadyExists => {
            Err(anyhow!("Failed to initialize data directory: {e}"))
        }
        _ => Ok(()),
    }
}
