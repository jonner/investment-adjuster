use std::{
    fs::File,
    io::{ErrorKind, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::{anyhow, bail};
use directories::ProjectDirs;
use driftfix::{
    account::{self, Balance},
    provider::{self, ProviderType},
};
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

use crate::{
    backup::{self, BackupFile},
    cli::{self, DataAddArgs, DataArgs, PlanArgs},
    output,
};

#[derive(Default, Debug, Serialize, Deserialize)]
struct Config {
    default_provider: ProviderType,
}

#[derive(Debug)]
pub struct App {
    dirs: ProjectDirs,
    target_config_file: PathBuf,
    config: Config,
    args: cli::Cli,
}

impl App {
    pub fn new(args: cli::Cli) -> anyhow::Result<Self> {
        let dirs = ProjectDirs::from("org", "quotidian", "driftfix")
            .ok_or_else(|| anyhow!("Unable to determine project directories"))?;

        // Ensure the app directories exist
        ensure_dir_exists(dirs.config_dir())?;
        ensure_dir_exists(dirs.data_dir())?;

        let config = Self::load_config(&app_config_dir(&dirs))?;
        Ok(Self {
            target_config_file: args
                .target_config
                .clone()
                .unwrap_or(dirs.data_dir().join("target.yml")),
            config,
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
            self.target_config_file.clone(),
            Some(account::AllocationConfig::example_config()?),
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
                match account::AllocationConfig::load_from_file(backup.path()) {
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
        let mut account_configs =
            account::AllocationConfig::load_from_file(&self.target_config_file)?;
        if let Some(acct) = &args.account {
            account_configs.retain(|acc| acc.account_number == *acct);
            if account_configs.is_empty() {
                bail!("No allocation targets are configured for that account");
            }
        }
        if account_configs.is_empty() {
            bail!("No allocation targets are configured. See help for more information.");
        }
        if let Some(keep) = args.cash_minimum {
            if account_configs.len() != 1 {
                anyhow::bail!(
                    "--cash-minimum can only be used with a single account. Try specifying --account."
                );
            }
            account_configs[0].cash_sweep.minimum = keep;
        }
        let accounts = self.load_balances()?;
        if accounts.is_empty() {
            bail!("Please import account balance data first. See help for more information.")
        }
        let naccounts = accounts.len();
        let mut accounts_with_config = Vec::<(account::Balance, account::AllocationConfig)>::new();
        for account in accounts {
            if let Some(cfg) = account_configs
                .iter()
                .find(|t| t.account_number == account.account_number)
            {
                accounts_with_config.push((account, cfg.clone()));
            }
        }
        if accounts_with_config.is_empty() {
            bail!(
                "Balance data has been imported for {naccounts} accounts, but no target allocation configuration exists for any of these accounts."
            );
        }
        accounts_with_config.sort_by(|a, b| {
            match a.0.total_value().partial_cmp(&b.0.total_value()) {
                Some(std::cmp::Ordering::Equal) | None => {
                    a.1.account_number.cmp(&b.1.account_number)
                }
                Some(x) => x.reverse(),
            }
        });
        for (account, mut config) in accounts_with_config {
            config.ignored_holdings.extend(args.ignore.iter().cloned());

            let adjustments = config.adjust_allocations(&account)?;
            let table = output::format_adjustments(adjustments);

            if !account.account_name.is_empty() {
                println!("{}", account.account_name);
            }
            println!("Account number: {}", account.account_number);
            println!("Total balance: {}", account.total_value());
            println!("{table}");
            println!();
        }
        Ok(())
    }

    fn data_command(&self, args: &DataArgs) -> anyhow::Result<()> {
        match &args.command {
            cli::DataCommands::Add(data_add_args) => self.data_add_command(data_add_args),
            cli::DataCommands::Remove { account } => self.data_remove_command(account),
            cli::DataCommands::Reset => self.data_reset_command(),
        }
    }

    fn data_add_command(&self, args: &DataAddArgs) -> anyhow::Result<()> {
        let portfolio = provider::load_portfolio(
            &args.account_balances,
            args.provider.unwrap_or(self.config.default_provider),
        )?;
        self.import_account_balances(portfolio.accounts)?;
        Ok(())
    }

    fn import_account_balances(&self, new_balances: Vec<Balance>) -> anyhow::Result<()> {
        let new_account_ids: Vec<_> = new_balances
            .iter()
            .map(|b| b.account_number.clone())
            .collect();
        let mut existing = self.load_balances()?;
        trace!(?existing, "before adding new account balance");
        existing.retain(|b| !new_account_ids.contains(&b.account_number));
        existing.extend(new_balances);
        trace!(?existing, "after adding new account balance");
        self.save_balances(&existing)?;
        Ok(())
    }

    fn cached_balance_file(&self) -> PathBuf {
        self.dirs.data_dir().join("balances.yml")
    }

    #[tracing::instrument(ret, level = "trace")]
    fn save_balances(&self, balances: &[Balance]) -> anyhow::Result<()> {
        let path = self.cached_balance_file();
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
        let path = self.cached_balance_file();
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

    fn data_reset_command(&self) -> Result<(), anyhow::Error> {
        self.save_balances(&Vec::new())?;
        Ok(())
    }

    #[tracing::instrument(ret, level = "trace")]
    fn load_config(path: &Path) -> anyhow::Result<Config> {
        match File::open(path) {
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e.into()),
            Ok(mut file) => {
                let mut contents = String::new();
                file.read_to_string(&mut contents)?;
                trace!(contents, "read file");
                serde_yaml::from_str(&contents).map_err(Into::into)
            }
        }
    }

    #[allow(dead_code)]
    #[tracing::instrument(ret, level = "trace")]
    fn save_config(config: &Config, path: &Path) -> anyhow::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        file.write_all(serde_yaml::to_string(config)?.as_bytes())?;
        Ok(())
    }
}

fn app_config_dir(dirs: &ProjectDirs) -> PathBuf {
    dirs.config_dir().join("config.yml")
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
