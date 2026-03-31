use std::io::Cursor;

use driftfix::{
    Dollar,
    provider::{self, ProviderType},
};
const FIDELITY_CSV: &str = r#"
Account Number,Account Name,Symbol,Description,Quantity,Last Price,Last Price Change,Current Value,Today's Gain/Loss Dollar,Today's Gain/Loss Percent,Total Gain/Loss Dollar,Total Gain/Loss Percent,Percent Of Account,Cost Basis Total,Average Cost Basis,Type
Z12345678,INDIVIDUAL - TOD,AAPL,APPLE INC,10.000,$175.43,+$1.24,$1754.30,+$12.40,+0.71%,+$154.30,+9.64%,15.5%,$1600.00,$160.00,Cash
Z12345678,INDIVIDUAL - TOD,SPAXX**,FIDELITY GOVERNMENT MONEY MARKET,500.00,1.00,--,$500.00,--,--,--,--,4.4%,$500.00,$1.00,Cash
Y98765432,ROTH IRA,FXAIX,FIDELITY 500 INDEX FUND,150.444,$165.12,-$0.45,$24841.31,-$67.70,-0.27%,+$2144.12,+9.44%,80.1%,$22697.19,$150.87,Cash
Y98765432,ROTH IRA,FTIHX,FIDELITY INTL INDEX FUND,50.000,$42.10,+$0.12,$2105.00,+$6.00,+0.29%,-$100.00,-4.53%,10.0%,$2205.00,$44.10,Cash"#;
const VANGUARD_CSV: &str = r#"Account Number,Investment Name,Symbol,Shares,Share Price,Total Value
12345678,Vanguard Federal Money Market Fund,VMFXX,1000.00,1.00,1000.00
12345678,Vanguard Total Stock Market Index Fund Admiral Shares,VTSAX,250.500,118.42,29664.21
12345678,Vanguard Total Bond Market II Index Fund Investor Shares,VTBIX,100.000,10.25,1025.00
87654321,Vanguard S&P 500 ETF,VOO,15.250,465.10,7092.78
87654321,Vanguard Federal Money Market Fund,VMFXX,500.000,1.00,500.00"#;

#[test]
fn parse_fidelity() {
    let mut reader = Cursor::new(FIDELITY_CSV);
    let portfolio = provider::load_portfolio(&mut reader, Some(ProviderType::Fidelity))
        .expect("Failed to parse fidelity example");

    assert_eq!(portfolio.len(), 2);

    let individual = portfolio
        .iter()
        .find(|a| a.account_id == "Z12345678")
        .unwrap();
    assert_eq!(individual.holdings.len(), 2);
    assert_eq!(individual.holdings[0].symbol, "AAPL");
    assert_eq!(individual.holdings[0].current_value, Dollar(1754.30));
    assert!(!individual.holdings[0].is_cash);
    assert_eq!(individual.holdings[1].symbol, "SPAXX");
    assert_eq!(individual.holdings[1].current_value, Dollar(500.00));
    assert!(individual.holdings[1].is_cash);
}

#[test]
fn parse_vanguard() {
    let mut reader = Cursor::new(VANGUARD_CSV);
    let portfolio = provider::load_portfolio(&mut reader, Some(ProviderType::Vanguard))
        .expect("Failed to parse vanguard example");

    assert_eq!(portfolio.len(), 2);

    let acct1 = portfolio
        .iter()
        .find(|a| a.account_id == "12345678")
        .unwrap();
    assert_eq!(acct1.holdings.len(), 3);
    let vmfxx = acct1.holdings.iter().find(|p| p.symbol == "VMFXX").unwrap();
    assert_eq!(vmfxx.current_value, Dollar(1000.00));
    assert!(vmfxx.is_cash);
}

#[test]
fn parse_auto() {
    let mut reader = Cursor::new(FIDELITY_CSV);
    let portfolio =
        provider::load_portfolio(&mut reader, None).expect("Failed to parse fidelity example");
    assert_eq!(portfolio.len(), 2);
    let mut reader = Cursor::new(VANGUARD_CSV);
    let portfolio =
        provider::load_portfolio(&mut reader, None).expect("Failed to parse vanguard example");
    assert_eq!(portfolio.len(), 2);
}
