use std::{fmt::Debug, path::Path};

use clap::ValueEnum;

use crate::account::Portfolio;

mod fidelity;
mod vanguard;

#[derive(Clone, Debug, ValueEnum)]
pub enum Provider {
    Fidelity,
    Vanguard,
}

impl Provider {
    pub fn load_portfolio<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<Portfolio> {
        match self {
            Provider::Fidelity => {
                let accounts = fidelity::parse_accounts(path)?;
                Ok(Portfolio { accounts })
            }
            Provider::Vanguard => {
                let accounts = vanguard::parse_accounts(path)?;
                Ok(Portfolio { accounts })
            }
        }
    }
}
