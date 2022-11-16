//!
//! Structure describing secret.
//!

use std::{fmt, marker::PhantomData};

use zeroize::{self, Zeroize as ZeroizableSecret};

use crate::{strategy::Strategy, PeekInterface};

///
/// Secret thing.
///
/// To get access to value use method `expose()` of trait [`ExposeInterface`].
///

pub struct StrongSecret<S: ZeroizableSecret, I = crate::WithType> {
    /// Inner secret value
    pub(crate) inner_secret: S,
    pub(crate) marker: PhantomData<I>,
}

impl<S: ZeroizableSecret, I> StrongSecret<S, I> {
    /// Take ownership of a secret value
    pub fn new(secret: S) -> Self {
        StrongSecret {
            inner_secret: secret,
            marker: PhantomData,
        }
    }
}

impl<S: ZeroizableSecret, I> PeekInterface<S> for StrongSecret<S, I> {
    fn peek(&self) -> &S {
        &self.inner_secret
    }
}

impl<S: ZeroizableSecret, I> From<S> for StrongSecret<S, I> {
    fn from(secret: S) -> StrongSecret<S, I> {
        Self::new(secret)
    }
}

impl<S: Clone + ZeroizableSecret, I> Clone for StrongSecret<S, I> {
    fn clone(&self) -> Self {
        StrongSecret {
            inner_secret: self.inner_secret.clone(),
            marker: PhantomData,
        }
    }
}

impl<S: ZeroizableSecret, I> PartialEq for StrongSecret<S, I>
where
    Self: PeekInterface<S>,
    S: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.peek().eq(other.peek())
    }
}

impl<S: ZeroizableSecret, I> Eq for StrongSecret<S, I>
where
    Self: PeekInterface<S>,
    S: Eq,
{
}

impl<S: ZeroizableSecret, I: Strategy<S>> fmt::Debug for StrongSecret<S, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        I::fmt(&self.inner_secret, f)
    }
}

impl<S: ZeroizableSecret, I: Strategy<S>> fmt::Display for StrongSecret<S, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        I::fmt(&self.inner_secret, f)
    }
}

impl<S: ZeroizableSecret, I> Default for StrongSecret<S, I>
where
    S: ZeroizableSecret + Default,
{
    fn default() -> Self {
        S::default().into()
    }
}

impl<T: ZeroizableSecret, S> Drop for StrongSecret<T, S> {
    fn drop(&mut self) {
        self.inner_secret.zeroize();
    }
}
