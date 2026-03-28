use std::{fmt::Debug, path::Path};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::account::Portfolio;

mod fidelity;
mod vanguard;

/// Brokerage providers supported by this tool
#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    #[default]
    Fidelity,
    Vanguard,
}

fn provider(t: ProviderType) -> Box<dyn Provider> {
    match t {
        ProviderType::Fidelity => Box::new(fidelity::provider()),
        ProviderType::Vanguard => Box::new(vanguard::provider()),
    }
}

/// Load a portfolio from the given file path that conforms to the expected format for the given `ProviderType`
pub fn load_portfolio<P: AsRef<Path>>(path: P, ptype: ProviderType) -> anyhow::Result<Portfolio> {
    provider(ptype).parse_accounts(path.as_ref())
}

/// a trait that must be implemented by providers in order to be supported by this tool
trait Provider {
    fn parse_accounts(&self, path: &Path) -> anyhow::Result<Portfolio>;
}
