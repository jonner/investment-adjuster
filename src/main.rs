use clap::Parser;

use crate::app::App;

mod app;
mod backup;
mod cli;
mod output;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let opts = cli::Cli::parse();
    let app = App::new(opts)?;
    app.run()
}
