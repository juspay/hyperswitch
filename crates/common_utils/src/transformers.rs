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

/// Trait for converting to a foreign type
pub trait ForeignInto<T> {
    /// Convert to a foreign type
    fn foreign_into(self) -> T;
}

/// impl for ForeignFrom
impl<F, T> ForeignInto<T> for F
where
    T: ForeignFrom<F>,
{
    fn foreign_into(self) -> T {
        T::foreign_from(self)
    }
}
