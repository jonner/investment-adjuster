use std::path::PathBuf;

use clap::Parser;

use crate::portfolio::Provider;

#[derive(Parser, Debug)]
pub(crate) struct Cli {
    #[arg(
        short,
        long,
        help = "Override global target allocation configuration file"
    )]
    pub target: Option<PathBuf>,
    #[arg(
        value_name = "CSV FILE",
        help = "A CSV file containing account balances"
    )]
    pub account_balance: PathBuf,
    #[arg(
        short,
        long,
        value_delimiter = ',',
        help = "Ignore the specified holdings when calculating target allocations"
    )]
    pub ignore: Vec<String>,
    #[arg(short, long, value_enum, default_value_t = Provider::Fidelity, help = "Investment provider")]
    pub provider: Provider,
}
