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
    #[command(about = "Calculate adjustments needed to acheive configured target allocations")]
    Plan(PlanArgs),
    #[command(about = "Configure account target allocations")]
    Edit,
    #[command(about = "Manage account balance data")]
    Data(DataArgs),
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
    Add(DataAddArgs),
    List,
    Show {
        #[arg(help = "Show data for the given account ID")]
        account: String,
    },
    Remove {
        #[arg(help = "Remove latest balance for the given account ID")]
        account: String,
    },
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
