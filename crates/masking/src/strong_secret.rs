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

impl<Secret: ZeroizableSecret, MaskingStrategy> StrongSecret<Secret, MaskingStrategy> {
    /// Take ownership of a secret value
    pub fn new(secret: Secret) -> Self {
        Self {
            inner_secret: secret,
            masking_strategy: PhantomData,
        }
    }
}

impl<Secret: ZeroizableSecret, MaskingStrategy> PeekInterface<Secret>
    for StrongSecret<Secret, MaskingStrategy>
{
        /// Returns a reference to the inner secret value without consuming the Secret object.
    fn peek(&self) -> &Secret {
        &self.inner_secret
    }
}

impl<Secret: ZeroizableSecret, MaskingStrategy> From<Secret>
    for StrongSecret<Secret, MaskingStrategy>
{
        /// Creates a new instance of Self using the provided Secret.
    fn from(secret: Secret) -> Self {
        Self::new(secret)
    }
}

impl<Secret: Clone + ZeroizableSecret, MaskingStrategy> Clone
    for StrongSecret<Secret, MaskingStrategy>
{
        /// Creates a deep copy of the current instance, including cloning the inner secret data.
    fn clone(&self) -> Self {
        Self {
            inner_secret: self.inner_secret.clone(),
            masking_strategy: PhantomData,
        }
    }
}

impl<Secret, MaskingStrategy> PartialEq for StrongSecret<Secret, MaskingStrategy>
where
    Self: PeekInterface<Secret>,
    Secret: ZeroizableSecret + StrongEq,
{
        /// Compares the current value of the data structure with another value of the same type.
    /// Returns true if the two values are equal, false otherwise.
    fn eq(&self, other: &Self) -> bool {
        StrongEq::strong_eq(self.peek(), other.peek())
    }
}

impl<Secret, MaskingStrategy> Eq for StrongSecret<Secret, MaskingStrategy>
where
    Self: PeekInterface<Secret>,
    Secret: ZeroizableSecret + StrongEq,
{
}

impl<Secret: ZeroizableSecret, MaskingStrategy: Strategy<Secret>> fmt::Debug
    for StrongSecret<Secret, MaskingStrategy>
{
        /// Formats the inner secret using the specified masking strategy and writes the result to the provided formatter.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            MaskingStrategy::fmt(&self.inner_secret, f)
        }
}

impl<Secret: ZeroizableSecret, MaskingStrategy: Strategy<Secret>> fmt::Display
    for StrongSecret<Secret, MaskingStrategy>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        MaskingStrategy::fmt(&self.inner_secret, f)
    }
}

impl<Secret: ZeroizableSecret, MaskingStrategy> Default for StrongSecret<Secret, MaskingStrategy>
where
    Secret: ZeroizableSecret + Default,
{
        /// Returns a new instance of the current type with default values.
    fn default() -> Self {
        Secret::default().into()
    }
}

impl<Secret: ZeroizableSecret, MaskingStrategy> Drop for StrongSecret<Secret, MaskingStrategy> {
        /// This method is used to securely zeroize the inner secret data contained within the object to prevent it from being accessed or leaked after the object is dropped.
    fn drop(&mut self) {
        self.inner_secret.zeroize();
    }
}

trait StrongEq {
    fn strong_eq(&self, other: &Self) -> bool;
}

impl StrongEq for String {
        /// Compares two instances of the same type using constant-time equality comparison to mitigate timing attacks.
    fn strong_eq(&self, other: &Self) -> bool {
            let lhs = self.as_bytes();
            let rhs = other.as_bytes();
    
            bool::from(lhs.ct_eq(rhs))
        }
}
