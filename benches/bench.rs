use investment_adjuster::provider::Provider;

fn main() {
    divan::main()
}

#[divan::bench]
fn parse_fidelity() {
    Provider::Fidelity
        .load_portfolio("benches/fidelity.csv")
        .expect("Failed to parse");
}
