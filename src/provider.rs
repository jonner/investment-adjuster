use std::{fmt::Debug, path::Path};

use clap::ValueEnum;

use crate::account::Portfolio;

mod fidelity;
mod vanguard;

#[derive(Clone, Debug, ValueEnum)]
pub enum ProviderType {
    Fidelity,
    Vanguard,
}

fn provider(t: ProviderType) -> Box<dyn Provider> {
    match t {
        ProviderType::Fidelity => Box::new(fidelity::provider()),
        ProviderType::Vanguard => Box::new(vanguard::provider()),
    }
}
pub fn load_portfolio<P: AsRef<Path>>(path: P, ptype: ProviderType) -> anyhow::Result<Portfolio> {
    provider(ptype).parse_accounts(path.as_ref())
}

trait Provider {
    fn parse_accounts(&self, path: &Path) -> anyhow::Result<Portfolio>;
}
