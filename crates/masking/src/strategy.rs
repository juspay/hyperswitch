use core::fmt;

/// Debugging trait which is specialized for handling secret values
pub trait Strategy<T> {
    /// Format information about the secret's type.
    fn fmt(value: &T, fmt: &mut fmt::Formatter<'_>) -> fmt::Result;
}

/// Debug with type
pub struct WithType;

impl<T> Strategy<T> for WithType {
    fn fmt(_: &T, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("*** ")?;
        fmt.write_str(std::any::type_name::<T>())?;
        fmt.write_str(" ***")
    }
}

/// Debug without type
pub struct WithoutType;

impl<T> Strategy<T> for WithoutType {
    fn fmt(_: &T, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("*** ***")
    }
}

/// ApiKey strategy
#[derive(Debug)]
pub struct ApiKey;

impl<T> Strategy<T> for ApiKey
where
    T: AsRef<str>,
{
    fn fmt(_value: &T, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, " *** api-key *** ")
    }
}
