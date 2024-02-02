//! Functional programming utilities

/// The Applicative trait provides a pure behavior,
/// which can be used to create values of type f a from values of type a.
pub trait Applicative<R> {
    /// The Associative type acts as a (f a) wrapper for Self.
    type WrappedSelf<T>;

    /// Applicative::pure(_) is abstraction with lifts any arbitrary type to underlying higher
    /// order type
    fn pure(v: R) -> Self::WrappedSelf<R>;
}

impl<R> Applicative<R> for Option<R> {
    type WrappedSelf<T> = Option<T>;
        /// This method takes a value `v` and returns it wrapped in `Some`.
    fn pure(v: R) -> Self::WrappedSelf<R> {
        Some(v)
    }
}

impl<R, E> Applicative<R> for Result<R, E> {
    type WrappedSelf<T> = Result<T, E>;
        /// This method takes a value of type R and wraps it in a Result enum, returning it as a wrapped value of the same type. If successful, it returns Ok(v).
    fn pure(v: R) -> Self::WrappedSelf<R> {
        Ok(v)
    }
}

/// This function wraps the evaluated result of `f` into current context,
/// based on the condition provided into the `predicate`
pub fn when<W: Applicative<(), WrappedSelf<()> = W>, F>(predicate: bool, f: F) -> W
where
    F: FnOnce() -> W,
{
    if predicate {
        f()
    } else {
        W::pure(())
    }
}
