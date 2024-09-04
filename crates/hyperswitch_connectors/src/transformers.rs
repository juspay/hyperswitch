//! Transformer traits for converting between foreign types and native types.

/// ForeignInto trait
pub trait ForeignInto<T> {
    /// Convert from a native type to a foreign type.
    fn foreign_into(self) -> T;
}

/// ForeignTryInto trait
pub trait ForeignTryInto<T> {
    /// The error type that can be returned when converting.
    type Error;
    /// Convert from a foreign type to a native type.
    fn foreign_try_into(self) -> Result<T, Self::Error>;
}

/// ForeignFrom trait
pub trait ForeignFrom<F> {
    /// Convert from a foreign type to a native type.
    fn foreign_from(from: F) -> Self;
}

/// ForeignTryFrom trait
pub trait ForeignTryFrom<F>: Sized {
    /// The error type that can be returned when converting.
    type Error;
    /// Convert from a foreign type to a native type.
    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

/// ForeignInto implementation
impl<F, T> ForeignInto<T> for F
where
    T: ForeignFrom<F>,
{
    fn foreign_into(self) -> T {
        T::foreign_from(self)
    }
}

/// ForeignTryInto implementation
impl<F, T> ForeignTryInto<T> for F
where
    T: ForeignTryFrom<F>,
{
    type Error = <T as ForeignTryFrom<F>>::Error;

    fn foreign_try_into(self) -> Result<T, Self::Error> {
        T::foreign_try_from(self)
    }
}
