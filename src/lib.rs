pub mod account;
pub mod provider;

pub type Dollar = f32;
pub type Percent = f32;

/// A description of what must be done to achieve a target allocation for an investment
#[derive(Debug, Default)]
pub enum Action {
    #[default]
    DoNothing,
    Sell(Dollar),
    Buy(Dollar),
}
