use crate::{Action, Dollar, Percent, account::PositionAdjustment};
use tabled::{
    Table, Tabled,
    settings::{
        Alignment, Color, Style,
        object::{Columns, Rows},
    },
};

fn display_optional_dollar(val: &Option<Dollar>) -> String {
    if let Some(val) = val {
        format!("${val:.2}")
    } else {
        "".to_string()
    }
}

fn display_dollar(val: &Dollar) -> String {
    format!("${val:.2}")
}

fn display_percentage(val: &Percent) -> String {
    format!("{val:.1}%")
}

fn display_optional_percentage(val: &Option<Percent>) -> String {
    if let Some(val) = val {
        display_percentage(val)
    } else {
        "".to_string()
    }
}

#[derive(Debug, Tabled)]
#[tabled(display(Dollar, "display_dollar"))]
#[tabled(display(Option<Dollar>, "display_optional_dollar"))]
#[tabled(display(Percent, "display_percentage"))]
#[tabled(display(Option<Percent>, "display_optional_percentage"))]
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
    let total: f32 = adjustments
        .iter()
        .map(|adj| adj.holding.current_value)
        .sum();
    let rows: Vec<AllocationTableRow> = adjustments
        .iter()
        .map(|adj| AllocationTableRow {
            symbol: adj.holding.symbol.clone(),
            current_value: adj.holding.current_value,
            current_percentage: adj.holding.current_value / total * 100.0,
            target: Some(adj.target),
            buy: match adj.action {
                Action::Buy(val) => Some(val),
                _ => None,
            },
            sell: match adj.action {
                Action::Sell(val) => Some(val),
                _ => None,
            },
            result: Some(
                adj.holding.current_value
                    + match adj.action {
                        Action::Sell(val) => -val,
                        Action::Buy(val) => val,
                        _ => 0.0,
                    },
            ),
            ignore: matches!(adj.action, Action::Ignored),
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
