use std::path::PathBuf;

use clap::Parser;

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
}
