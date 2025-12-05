use investment_adjuster::portfolio::{Portfolio, Provider};

fn main() {
    divan::main()
}

#[divan::bench]
fn parse_fidelity() {
    Portfolio::load_from_file("benches/fidelity.csv", Provider::Fidelity).expect("Failed to parse");
}
