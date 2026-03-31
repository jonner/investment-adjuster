use std::io::Cursor;

use driftfix::provider::{self, ProviderType};

fn main() {
    divan::main()
}

#[divan::bench]
fn parse_fidelity() {
    let testdata = r#"Account Number,Account Name,Symbol,Description,Quantity,Last Price,Last Price Change,Current Value,Today's Gain/Loss Dollar,Today's Gain/Loss Percent,Total Gain/Loss Dollar,Total Gain/Loss Percent,Percent Of Account,Cost Basis Total,Average Cost Basis,Type
X12345678,Individual,FZFXX**,HELD IN MONEY MARKET,,,,$16092.87,,,,,32.27%,,,Cash
X12345678,Individual,BND,VANGUARD BD INDEX FDS TOTAL BND MRKT,84.503,$74.355,+$0.095,$6283.22,+$8.02,+0.12%,+$4.96,+0.07%,12.60%,$6278.26,$74.30,Cash
X12345678,Individual,IBM,INTERNATIONAL BUS MACH CORP COM USD0.20,14.273,$289.33,-$0.62,$4129.60,-$8.85,-0.22%,+$1559.24,+60.66%,8.28%,$2570.36,$180.09,Cash
X12345678,Individual,VTI,VANGUARD INDEX FDS VANGUARD TOTAL STK MKT ETF,39.649,$324.4466,+$0.2266,$12863.98,+$8.98,+0.06%,+$22.13,+0.17%,25.80%,$12841.85,$323.89,Cash
X12345678,Individual,VXUS,VANGUARD TOTAL INTERNATIONAL STOCK INDEX FUND,143.709,$73.02,-$0.33,$10493.63,-$47.43,-0.45%,-$20.23,-0.20%,21.04%,$10513.86,$73.16,Cash
87654321,ROTH IRA,SPAXX**,HELD IN MONEY MARKET,,,,$6.54,,,,,0.10%,,,Cash
87654321,ROTH IRA,BND,VANGUARD BD INDEX FDS TOTAL BND MRKT,18.767,$74.355,+$0.095,$1395.42,+$1.78,+0.12%,-$4.41,-0.32%,20.41%,$1399.83,$74.59,Cash
87654321,ROTH IRA,VTI,VANGUARD INDEX FDS VANGUARD TOTAL STK MKT ETF,10,$324.4466,+$0.2266,$3244.46,+$2.26,+0.06%,-$105.54,-3.16%,47.45%,$3350.00,$335.00,Cash
87654321,ROTH IRA,VXUS,VANGUARD TOTAL INTERNATIONAL STOCK INDEX FUND,30,$73.02,-$0.33,$2190.60,-$9.90,-0.45%,-$58.31,-2.60%,32.04%,$2248.91,$74.96,Cash

"The data and information in this spreadsheet is provided to you solely for your use and is not for distribution. The spreadsheet is provided for informational purposes only, and is not intended to provide advice, nor should it be construed as an offer to sell, a solicitation of an offer to buy or a recommendation for any security by Fidelity or any third party. Data and information shown is based on information known to Fidelity as of the date it was exported and is subject to change. It should not be used in place of your account statements or trade confirmations and is not intended for tax reporting purposes. For more information on the data included in this spreadsheet, including any limitations thereof, go to Fidelity.com."

"Brokerage services are provided by Fidelity Brokerage Services LLC (FBS), 900 Salem Street, Smithfield, RI 02917. Custody and other services provided by National Financial Services LLC (NFS). Both are Fidelity Investment companies and members SIPC, NYSE. Neither FBS nor NFS offer crypto as a direct investment nor provide trading or custody services for such assets."

"Date downloaded Nov-19-2025 12:11 p.m ET""#;

    let mut r = Cursor::new(testdata);
    provider::load_portfolio(&mut r, Some(ProviderType::Fidelity)).expect("Failed to parse");
}
