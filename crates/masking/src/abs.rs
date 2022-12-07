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
    // FIXED(kos): That should be improved.
    // Yes, of course, it improves ergonomics for the cases where we
    // want to clone the inner value. Doing so, we simplify the way to accidentally leak the
    // secret value: if it's easy to clone it, the user will tend to
    // do it more often, and quite probably without care about the
    // exposed secret to be handled properly (zeroizing memory, etc).
    /// Expose option.
    fn expose_option(self) -> S;
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
