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
        /// Maps the `Option` to an `Option` of the exposed value by calling the `expose` method on the inner type, if it exists.
    fn expose_option(self) -> Option<S> {
        self.map(ExposeInterface::expose)
    }
}

impl<S, I> ExposeInterface<S> for Secret<S, I>
where
    I: crate::Strategy<S>,
{
        /// This method returns the inner secret value that is contained within the current instance.
    fn expose(self) -> S {
        self.inner_secret
    }
}

/// Interface that consumes a secret and converts it to a secret with a different masking strategy.
pub trait SwitchStrategy<FromStrategy, ToStrategy> {
    /// The type returned by `switch_strategy()`.
    type Output;

    /// Consumes the secret and converts it to a secret with a different masking strategy.
    fn switch_strategy(self) -> Self::Output;
}

impl<S, FromStrategy, ToStrategy> SwitchStrategy<FromStrategy, ToStrategy>
    for Secret<S, FromStrategy>
where
    FromStrategy: crate::Strategy<S>,
    ToStrategy: crate::Strategy<S>,
{
    type Output = Secret<S, ToStrategy>;

        /// This method creates a new instance of the `Secret` struct using the inner secret value of the current instance.
    fn switch_strategy(self) -> Self::Output {
        Secret::new(self.inner_secret)
    }
}
