pub mod account;
pub mod provider;
pub type Dollar = f32;
pub type Percent = f32;

#[derive(Debug)]
pub enum Action {
    Nothing,
    Ignore,
    Sell(Dollar),
    Buy(Dollar),
}
