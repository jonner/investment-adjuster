use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, Div, Mul},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

pub mod account;
pub mod provider;

/// A type that represents dollar values
#[derive(
    Debug,
    Deserialize,
    Serialize,
    Default,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    derive_more::Add,
    derive_more::AddAssign,
    derive_more::Sub,
    derive_more::Mul,
    derive_more::Div,
    derive_more::Sum,
)]
pub struct Dollar(pub f32);

impl Display for Dollar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${:.2}", self.0)
    }
}

impl Add<&Action> for Dollar {
    type Output = Self;

    fn add(self, rhs: &Action) -> Self::Output {
        Self(
            self.0
                + match rhs {
                    Action::DoNothing => 0.0,
                    Action::Sell(dollar) => -dollar.0,
                    Action::Buy(dollar) => dollar.0,
                },
        )
    }
}

impl FromStr for Dollar {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        f32::from_str(s).map(Self)
    }
}

impl From<Dollar> for f32 {
    fn from(val: Dollar) -> Self {
        val.0
    }
}

impl Mul<Percent> for Dollar {
    type Output = Self;

    fn mul(self, rhs: Percent) -> Self::Output {
        Self(self.0 * (rhs.0 / 100.0))
    }
}

impl Dollar {
    fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    fn max(&self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }
}

/// A type that represents percentage values
#[derive(
    Debug,
    Deserialize,
    Serialize,
    Default,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    derive_more::Add,
    derive_more::Sub,
    derive_more::Sum,
)]
pub struct Percent(pub f32);

impl Percent {
    pub fn new<N, D>(numerator: N, denominator: D) -> Self
    where
        N: Into<f32>,
        D: Into<f32>,
    {
        Self(numerator.into() / denominator.into() * 100.0_f32)
    }
}

impl From<Percent> for f32 {
    fn from(val: Percent) -> Self {
        val.0
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}%", self.0)
    }
}

impl<'a> Sum<&'a Percent> for Percent {
    fn sum<I: Iterator<Item = &'a Percent>>(iter: I) -> Self {
        Self(iter.map(|element| element.0).sum())
    }
}

impl Mul for Percent {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(100.0 * (self.0 / 100.0) * (rhs.0 / 100.0))
    }
}

impl Div for Percent {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(100.0 * (self.0 / 100.0) / (rhs.0 / 100.0))
    }
}

/// A description of what must be done to achieve a target allocation for an investment
#[derive(Debug, Default)]
pub enum Action {
    #[default]
    DoNothing,
    Sell(Dollar),
    Buy(Dollar),
}
