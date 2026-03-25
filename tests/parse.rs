use investment_adjuster::provider::Provider;

#[test]
fn parse_fidelity() {
    let filename = "./tests/data/example-portfolio-fidelity.csv";
    Provider::Fidelity
        .load_portfolio(filename)
        .expect("Failed to parse fidelity example");
}

#[test]
fn parse_vanguard() {
    let filename = "./tests/data/example-portfolio-vanguard.csv";
    Provider::Vanguard
        .load_portfolio(filename)
        .expect("Failed to parse vanguard example");
}
