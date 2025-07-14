//! Module for tokenization-related functionality
//!
//! This module provides types and functions for handling tokenized payment data,
//! including response structures and token generation utilities.

use crate::consts::TOKEN_LENGTH;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
/// Generates a new token string
///
/// # Returns
/// A randomly generated token string of length `TOKEN_LENGTH`
pub fn generate_token() -> String {
    use nanoid::nanoid;
    nanoid!(TOKEN_LENGTH)
}
