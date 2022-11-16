//!
//! Abstract data types.
//!

use crate::Secret;

/// Interface to expose a reference to an inner secret
pub trait PeekInterface<S> {
    /// Only method providing access to the secret value.
    fn peek(&self) -> &S;
}

/// Interface to expose a clone of secret
pub trait PeekOptionInterface<S> {
    /// Expose option.
    fn peek_cloning(&self) -> S;
}

/// Interface that consumes a secret and returns the inner value.
pub trait ExposeInterface<S> {
    /// Consume the secret and return the inner value
    fn expose(self) -> S;
}

impl<S, I> PeekOptionInterface<Option<S>> for Option<Secret<S, I>>
where
    S: Clone,
    I: crate::Strategy<S>,
{
    fn peek_cloning(&self) -> Option<S> {
        self.as_ref().map(|val| val.peek().clone())
    }
}

impl<S, I> PeekOptionInterface<S> for Secret<S, I>
where
    S: Clone,
    I: crate::Strategy<S>,
{
    fn peek_cloning(&self) -> S {
        self.peek().clone()
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

// impl<S> ExposeInterface<Option<&S>> for Option<Secret<S>>
// where
//     S: ZeroizableSecret,
//     Secret<S> : ExposeInterface<S>,
// {
//     fn expose(&self) -> &Option<&S> {
//       if let Some( ref val ) = self {
//           &Some( val.peek() )
//       } else {
//           &None
//       }
//       // &None
//     }
// }
