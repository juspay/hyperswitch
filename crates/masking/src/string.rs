//!
//! Secret strings
//!
//! There is not alias type by design.

use alloc::{
    str::FromStr,
    string::{String, ToString},
};

#[cfg(feature = "serde")]
use super::SerializableSecret;
use super::{Secret, Strategy};
use crate::StrongSecret;

#[cfg(feature = "serde")]
impl SerializableSecret for String {}

impl<I> FromStr for Secret<String, I>
where
    I: Strategy<String>,
{
    type Err = core::convert::Infallible;

        /// Attempts to create a new instance of Self from the given string slice.
    /// 
    /// # Arguments
    /// * `src` - The string slice to create a new instance from
    /// 
    /// # Returns
    /// * If successful, returns a Result containing the new instance of Self
    /// * If an error occurs, returns a Result containing the error type associated with Self
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
