//!
//! Abstract data types.
//!

use crate::Secret;

/// Interface to expose a reference to an inner secret
pub trait PeekInterface<S> {
    /// Only method providing access to the secret value.
    fn peek(&self) -> &S;
}

/// Interface that consumes a option secret and returns the value.
pub trait ExposeOptionInterface<S> {
    /// Expose option.
    fn expose_option(self) -> S;
}

/// Interface that consumes a secret and returns the inner value.
pub trait ExposeInterface<S> {
    /// Consume the secret and return the inner value
    fn expose(self) -> S;
}

impl<S, I> ExposeOptionInterface<Option<S>> for Option<Secret<S, I>>
where
    S: Clone,
    I: crate::Strategy<S>,
{
    fn expose_option(self) -> Option<S> {
        self.map(ExposeInterface::expose)
    }
}

impl<S, I> ExposeInterface<S> for Secret<S, I>
where
    I: crate::Strategy<S>,
{
    fn expose(self) -> S {
        self.inner_secret
    }
}
