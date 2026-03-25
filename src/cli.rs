use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::provider::ProviderType;

#[derive(Parser, Debug)]
pub(crate) struct Cli {
    #[arg(
        short,
        long,
        value_name = "CONFIG_FILE",
        help = "Override default target allocation configuration file"
    )]
    pub config: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Debug)]
pub(crate) struct AdjustArgs {
    #[arg(
        value_name = "ACCOUNT_BALANCES",
        help = "A file containing account balances"
    )]
    pub(crate) account_balances: PathBuf,
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
        help = "Amount to keeep in core position (overrides target allocation configuration for all accounts)"
    )]
    pub(crate) core_minimum: Option<f32>,
    #[arg(
        short,
        long,
        value_enum,
        value_name = "PROVIDER_ID",
        default_value_t = ProviderType::Fidelity,
        help = "Investment provider associated with account balances file",
    )]
    pub(crate) provider: ProviderType,
    #[arg(short, long, help = "Only show targets for the given account id")]
    pub(crate) account: Option<String>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    #[command(about = "Calculate adjustments needed to acheive configured target allocations")]
    Adjust(AdjustArgs),
    #[command(about = "Edit the target allocation configuration file")]
    Edit,
}
