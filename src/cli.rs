use std::path::PathBuf;

use clap::Parser;

use crate::portfolio::Provider;

#[derive(Parser, Debug)]
pub(crate) struct Cli {
    #[arg(short, long, help = "Target allocation")]
    pub target: Option<PathBuf>,
    #[arg(help = "Current allocation CSV downloaded from fidelity")]
    pub current_allocations: PathBuf,
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
