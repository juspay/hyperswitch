//!
//! Structure describing secret.
//!

use std::{fmt, marker::PhantomData};

use zeroize::{self, Zeroize as ZeroizableSecret};

use crate::{strategy::Strategy, PeekInterface};

// FIXME(kos) : It would be convenient to implement Deref<Target=Thing> for Secret<Thing>
// This would allow getting rid of `peek()`.

// FIXME(kos): Documentation should explain how it differs from the `Secret`
// type and why it's called "strong". Save the reader's time and
// free him from wondering and parsing the `impl`s of this type to
// understand the difference.
///
/// Secret thing.
///
/// To get access to value use method `expose()` of trait [`crate::ExposeInterface`].
///
// TODO(kos): Consider renaming these types. Instead of having `Secret` and
// `StrongSecret`, it's better to have `WeakSecret` and `Secret`.
// When it comes to security, emphasizing weaknesses has much more
// sense than emphasizing strengths (which are default expectations).
pub struct StrongSecret<S: ZeroizableSecret, I = crate::WithType> {
    /// Inner secret value
    pub(crate) inner_secret: S,
    // FIXME(kos): `marker` is too generic naming here, being non-obvious about
    // what kind of metadata it stores. We should name it directly after
    // its purpose: `masking_strategy`.
    // Also I. Not obvious.
    // Also, consider renaming in similar places over the codebase.
    pub(crate) marker: PhantomData<I>,
}

impl<S: ZeroizableSecret, I> StrongSecret<S, I> {
    /// Take ownership of a secret value
    pub fn new(secret: S) -> Self {
        // FIXME(kos): Use `Self` syntax here.
        // `#![warn(clippy::use_self)]` on crate level should help
        // with this.
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
    // FIXME(kos): Usual comparison is not safe in cryptography as is lazy
    // (fail-fast) and makes the code potentially vulnerable to
    // timing attacks:
    // https://www.chosenplaintext.ca/articles/beginners-guide-constant-time-cryptography.html
    // Use crates like `subtle` for constant-time comparison of
    // secret values:
    // https://docs.rs/subtle
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
