//!
//! Secret strings
//!
//! There is not alias type by design.

use alloc::{
    str::FromStr,
    string::{String, ToString},
};

use super::{Secret, Strategy};
use crate::StrongSecret;

impl<I> FromStr for Secret<String, I>
where
    I: Strategy<String>,
{
    type Err = core::convert::Infallible;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(src.to_string()))
    }
}

impl<I> FromStr for StrongSecret<String, I>
where
    I: Strategy<String>,
{
    type Err = core::convert::Infallible;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(src.to_string()))
    }
}
