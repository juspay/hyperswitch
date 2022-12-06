//!
//! Structure describing secret.
//!

use std::{fmt, marker::PhantomData};

use subtle::ConstantTimeEq;
use zeroize::{self, Zeroize as ZeroizableSecret};

use crate::{strategy::Strategy, PeekInterface};

///
/// Secret thing.
///
/// To get access to value use method `expose()` of trait [`crate::ExposeInterface`].
///
pub struct StrongSecret<Secret: ZeroizableSecret, MaskingStrategy = crate::WithType> {
    /// Inner secret value
    pub(crate) inner_secret: Secret,
    pub(crate) masking_strategy: PhantomData<MaskingStrategy>,
}

impl<S: ZeroizableSecret, I> StrongSecret<S, I> {
    /// Take ownership of a secret value
    pub fn new(secret: S) -> Self {
        StrongSecret {
            inner_secret: secret,
            masking_strategy: PhantomData,
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
            masking_strategy: PhantomData,
        }
    }
}

impl<S: ZeroizableSecret, I> PartialEq for StrongSecret<S, I>
where
    Self: PeekInterface<S>,
    S: StrongEq,
{
    fn eq(&self, other: &Self) -> bool {
        StrongEq::eq(self.peek(), other.peek())
    }
}

impl<S: ZeroizableSecret, I> Eq for StrongSecret<S, I>
where
    Self: PeekInterface<S>,
    S: StrongEq,
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

trait StrongEq {
    fn eq(&self, other: &Self) -> bool;
}

impl StrongEq for String {
    fn eq(&self, other: &Self) -> bool {
        let lhs_bytes = self.as_bytes();
        let rhs_bytes = other.as_bytes();

        bool::from(lhs_bytes.ct_eq(rhs_bytes))
    }
}
