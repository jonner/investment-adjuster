use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub(crate) struct Cli {
    #[arg(short, long, help = "Target allocation")]
    pub target: Option<PathBuf>,
    #[arg(help = "Current allocation CSV downloaded from fidelity")]
    pub current_allocations: PathBuf,
}
