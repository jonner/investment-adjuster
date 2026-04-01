use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, AddAssign, Deref, Div, Mul, Sub},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

pub mod account;
pub mod provider;

/// A type that represents dollar values
#[derive(Debug, Deserialize, Serialize, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Dollar(pub f32);

impl Display for Dollar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${:.2}", self.0)
    }
}

impl Deref for Dollar {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Sub for Dollar {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Add for Dollar {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
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

impl AddAssign for Dollar {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Div for Dollar {
    type Output = f32;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl FromStr for Dollar {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        f32::from_str(s).map(Self)
    }
}

impl Sum for Dollar {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|element| element.0).sum())
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
#[derive(Debug, Deserialize, Serialize, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Percent(pub f32);

impl Deref for Percent {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Sub for Percent {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
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

/// A description of what must be done to achieve a target allocation for an investment
#[derive(Debug, Default)]
pub enum Action {
    #[default]
    DoNothing,
    Sell(Dollar),
    Buy(Dollar),
}
