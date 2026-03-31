use std::{
    fmt::Debug,
    io::{BufRead, BufReader, Read},
};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::account::Balance;

mod fidelity;
mod vanguard;

/// Brokerage providers supported by this tool
#[derive(Serialize, Deserialize, Clone, Copy, Debug, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Fidelity,
    Vanguard,
}
const PROVIDERS: &[ProviderType] = &[ProviderType::Fidelity, ProviderType::Vanguard];

fn provider(t: ProviderType) -> Box<dyn Provider> {
    match t {
        ProviderType::Fidelity => Box::new(fidelity::provider()),
        ProviderType::Vanguard => Box::new(vanguard::provider()),
    }
}

/// Load a portfolio from the given file path that conforms to the expected format for the given `ProviderType`
pub fn load_portfolio(
    reader: &mut dyn Read,
    ptype: Option<ProviderType>,
) -> anyhow::Result<Vec<Balance>> {
    let mut buffered = BufReader::new(reader);
    if let Some(ptype) = ptype {
        provider(ptype).parse_portfolio(&mut buffered)
    } else {
        let sample = buffered.fill_buf()?;
        for p in PROVIDERS {
            let prov = provider(*p);
            if prov.detect(sample).unwrap_or(false) {
                return prov.parse_portfolio(&mut buffered);
            }
        }
        Err(anyhow::anyhow!(
            "Couldn't find a provider to parse this portfolio file"
        ))
    }
}

/// a trait that must be implemented by providers in order to be supported by this tool
trait Provider {
    fn parse_portfolio(&self, reader: &mut dyn BufRead) -> anyhow::Result<Vec<Balance>>;
    fn detect(&self, sample: &[u8]) -> anyhow::Result<bool>;
}
