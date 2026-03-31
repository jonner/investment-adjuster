use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use driftfix::{Dollar, provider::ProviderType};

#[derive(Parser, Debug)]
pub(crate) struct Cli {
    #[arg(
        short,
        long,
        global = true,
        value_name = "TARGET_CONFIG_FILE",
        help = "Override default target allocation configuration file"
    )]
    pub target_config: Option<PathBuf>,
    #[command(subcommand)]
    pub command: MainCommands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum MainCommands {
    #[command(about = "Configure account target allocations", alias = "edit")]
    Configure,
    #[command(about = "Calculate adjustments needed to acheive configured target allocations")]
    Plan(PlanArgs),
    #[command(about = "Manage account balance data")]
    Data(DataArgs),
    #[command(about = "Generate shell autocompletion script")]
    Completion { shell: clap_complete::Shell },
}

#[derive(Args, Debug)]
pub(crate) struct PlanArgs {
    #[arg(
        short,
        long,
        value_delimiter = ',',
        value_name = "SYMBOL",
        help = "Ignore the specified space-separated holdings when calculating allocation adjustments"
    )]
    pub(crate) ignore: Vec<String>,
    #[arg(
        long,
        value_name = "VALUE",
        help = "Amount to keep in cash sweep (overrides target allocation configuration)"
    )]
    pub(crate) cash_minimum: Option<Dollar>,
    #[arg(short, long, help = "Only show targets for the given account id")]
    pub(crate) account: Option<String>,
}

#[derive(Args, Debug)]
pub(crate) struct DataArgs {
    #[command(subcommand)]
    pub command: DataCommands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum DataCommands {
    #[command(about = "Add or update account balances by importing a portfolio file")]
    Add(DataAddArgs),
    #[command(about = "List data that has already been imported")]
    List,
    #[command(about = "Show detailed information about a specific account")]
    Show {
        #[arg(help = "an account ID")]
        account: String,
    },
    #[command(about = "Remove account balance data for a given account")]
    Remove {
        #[arg(help = "an account ID")]
        account: String,
    },
    #[command(about = "Remove account balance data for all accounts")]
    Reset,
}

#[derive(Args, Debug)]
pub(crate) struct DataAddArgs {
    #[arg(
        value_name = "ACCOUNT_BALANCES",
        help = "A file containing account balances"
    )]
    pub(crate) account_balances: PathBuf,
    #[arg(
        short,
        long,
        value_enum,
        value_name = "PROVIDER_ID",
        help = "Investment provider associated with account balances file"
    )]
    pub(crate) provider: Option<ProviderType>,
}
