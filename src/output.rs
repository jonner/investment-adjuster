use driftfix::{Action, Dollar, Percent, account::PositionAdjustment};
use tabled::{
    Table, Tabled,
    derive::display,
    settings::{
        Alignment, Color, Style,
        object::{Columns, Rows},
    },
};

#[derive(Debug, Tabled)]
#[tabled(display(Option, "display::option", ""))]
struct AllocationTableRow {
    #[tabled(rename = "Symbol")]
    symbol: String,
    #[tabled(rename = "Value")]
    current_value: Dollar,
    #[tabled(rename = "Percent")]
    current_percentage: Percent,
    #[tabled(rename = "Target")]
    target: Option<Percent>,
    #[tabled(rename = "Sell")]
    sell: Option<Dollar>,
    #[tabled(rename = "Buy")]
    buy: Option<Dollar>,
    #[tabled(rename = "Result")]
    result: Option<Dollar>,
    #[tabled(skip)]
    ignore: bool,
}

pub fn format_adjustments(adjustments: Vec<PositionAdjustment>) -> Table {
    let total: Dollar = adjustments
        .iter()
        .map(|adj| adj.holding.current_value)
        .sum();
    let rows: Vec<AllocationTableRow> = adjustments
        .iter()
        .map(|adj| AllocationTableRow {
            symbol: adj.holding.symbol.clone(),
            current_value: adj.holding.current_value,
            current_percentage: Percent(adj.holding.current_value / total * 100.0),
            target: Some(adj.target),
            buy: match adj.action {
                Action::Buy(val) => Some(val),
                _ => None,
            },
            sell: match adj.action {
                Action::Sell(val) => Some(val),
                _ => None,
            },
            result: Some(adj.holding.current_value + &adj.action),
            ignore: adj.ignored,
        })
        .collect();
    let mut table = Table::new(rows.iter());
    table.with(Style::rounded());
    table.modify(Columns::new(..), Alignment::right());
    for (i, row) in rows.iter().enumerate() {
        if row.ignore {
            // header row is technically the first row
            table.modify(Rows::one(i + 1), Color::rgb_fg(150, 150, 150));
        }
    }
    table
}
