use std::path::PathBuf;

use clap::Parser;

use crate::portfolio::Provider;

#[derive(Parser, Debug)]
pub(crate) struct Cli {
    #[arg(
        short,
        long,
        value_name = "CONFIG_FILE",
        help = "Override default target allocation configuration file"
    )]
    pub target: Option<PathBuf>,
    #[arg(
        value_name = "ACCOUNT_BALANCES",
        help = "A file containing account balances"
    )]
    pub account_balance: PathBuf,
    #[arg(
        short,
        long,
        value_delimiter = ',',
        value_name = "SYMBOL",
        help = "Ignore the specified holdings when calculating target allocations"
    )]
    pub ignore: Vec<String>,
    #[arg(
        long,
        value_name = "VALUE",
        help = "Amount to keeep in core position (overrides target allocation configuration for all accounts)"
    )]
    pub core_minimum: Option<f32>,
    #[arg(
        short,
        long,
        value_enum,
        value_name = "PROVIDER_ID",
        default_value_t = Provider::Fidelity,
        help = "Investment provider associated with account balances file")]
    pub provider: Provider,
}
