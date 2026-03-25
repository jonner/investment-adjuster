use investment_adjuster::provider::{self, ProviderType};

#[test]
fn parse_fidelity() {
    let filename = "./tests/data/example-portfolio-fidelity.csv";
    let portfolio = provider::load_portfolio(filename, ProviderType::Fidelity)
        .expect("Failed to parse fidelity example");

    assert_eq!(portfolio.accounts.len(), 2);

    let individual = portfolio
        .accounts
        .iter()
        .find(|a| a.account_number == "Z12345678")
        .unwrap();
    assert_eq!(individual.positions.len(), 2);
    assert_eq!(individual.positions[0].symbol, "AAPL");
    assert_eq!(individual.positions[0].current_value, 1754.30);
    assert!(!individual.positions[0].is_core);
    assert_eq!(individual.positions[1].symbol, "SPAXX");
    assert_eq!(individual.positions[1].current_value, 500.00);
    assert!(individual.positions[1].is_core);
}

#[test]
fn parse_vanguard() {
    let filename = "./tests/data/example-portfolio-vanguard.csv";
    let portfolio = provider::load_portfolio(filename, ProviderType::Vanguard)
        .expect("Failed to parse vanguard example");

    assert_eq!(portfolio.accounts.len(), 2);

    let acct1 = portfolio
        .accounts
        .iter()
        .find(|a| a.account_number == "12345678")
        .unwrap();
    assert_eq!(acct1.positions.len(), 3);
    let vmfxx = acct1
        .positions
        .iter()
        .find(|p| p.symbol == "VMFXX")
        .unwrap();
    assert_eq!(vmfxx.current_value, 1000.00);
    assert!(vmfxx.is_core);
}
