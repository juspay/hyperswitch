//! Utilities for converting between foreign types

/// Trait for converting from one foreign type to another
pub trait ForeignFrom<F> {
    /// Convert from a foreign type to the current type
    fn foreign_from(from: F) -> Self;
}

/// Trait for converting from one foreign type to another
pub trait ForeignTryFrom<F>: Sized {
    /// Custom error for conversion failure
    type Error;
    /// Convert from a foreign type to the current type and return an error if the conversion fails
    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}
