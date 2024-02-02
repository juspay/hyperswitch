use core::fmt;

/// Debugging trait which is specialized for handling secret values
pub trait Strategy<T> {
    /// Format information about the secret's type.
    fn fmt(value: &T, fmt: &mut fmt::Formatter<'_>) -> fmt::Result;
}

/// Debug with type
pub enum WithType {}

impl<T> Strategy<T> for WithType {
        /// This method formats the given value by writing "*** " followed by the type name of the value, and then " ***" to the provided formatter.
    fn fmt(_: &T, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("*** ")?;
        fmt.write_str(std::any::type_name::<T>())?;
        fmt.write_str(" ***")
    }
}

/// Debug without type
pub enum WithoutType {}

impl<T> Strategy<T> for WithoutType {
        /// Formats the given value using the specified formatter.
    ///
    /// # Arguments
    ///
    /// * `_: &T` - The value to be formatted. Ignored in this implementation.
    /// * `fmt: &mut fmt::Formatter<'_>` - The formatter used to write the formatted value.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating the success or failure of the formatting operation.
    fn fmt(_: &T, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("*** ***")
    }
}
