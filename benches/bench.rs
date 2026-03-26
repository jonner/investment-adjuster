use driftfix::provider::{self, ProviderType};

fn main() {
    divan::main()
}

#[divan::bench]
fn parse_fidelity() {
    provider::load_portfolio("benches/fidelity.csv", ProviderType::Fidelity)
        .expect("Failed to parse");
}
