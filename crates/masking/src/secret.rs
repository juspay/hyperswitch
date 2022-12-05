//!
//! Structure describing secret.
//!

use std::{fmt, marker::PhantomData};

use crate::{strategy::Strategy, PeekInterface};

///
/// Secret thing.
///
/// To get access to value use method `expose()` of trait [`crate::ExposeInterface`].
///
/// ## Masking
/// Use the [`crate::strategy::Strategy`] trait to implement a masking strategy on a unit struct
/// and pass the unit struct as a second generic parameter to [`Secret`] while defining it.
/// [`Secret`] will take care of applying the masking strategy on the inner secret when being
/// displayed.
///
/// ## Masking Example
///
/// ```
/// use masking::Strategy;
/// use masking::Secret;
/// use std::fmt;
///
/// struct MyStrategy;
///
/// impl<T> Strategy<T> for MyStrategy
/// where
///     T: fmt::Display
/// {
///     fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         f.write_str(
///             &format!("{}", val).to_ascii_lowercase()
///         )
///     }
/// }
///
/// let my_secret: Secret<String, MyStrategy> = Secret::new("HELLO".to_string());
///
/// assert_eq!("hello", &format!("{:?}", my_secret));
/// ```
///
pub struct Secret<S, I = crate::WithType>
where
    I: Strategy<S>,
{
    /// Inner secret value
    pub(crate) inner_secret: S,
    pub(crate) marker: PhantomData<I>,
}

impl<S, I> Secret<S, I>
where
    I: Strategy<S>,
{
    /// Take ownership of a secret value
    pub fn new(secret: S) -> Self {
        Secret {
            inner_secret: secret,
            marker: PhantomData,
        }
    }
}

impl<S, I> PeekInterface<S> for Secret<S, I>
where
    I: Strategy<S>,
{
    fn peek(&self) -> &S {
        &self.inner_secret
    }
}

impl<S, I> From<S> for Secret<S, I>
where
    I: Strategy<S>,
{
    fn from(secret: S) -> Secret<S, I> {
        Self::new(secret)
    }
}

impl<S, I> Clone for Secret<S, I>
where
    S: Clone,
    I: Strategy<S>,
{
    fn clone(&self) -> Self {
        Secret {
            inner_secret: self.inner_secret.clone(),
            marker: PhantomData,
        }
    }
}

impl<S, I> PartialEq for Secret<S, I>
where
    Self: PeekInterface<S>,
    S: PartialEq,
    I: Strategy<S>,
{
    fn eq(&self, other: &Self) -> bool {
        self.peek().eq(other.peek())
    }
}

impl<S, I> Eq for Secret<S, I>
where
    Self: PeekInterface<S>,
    S: Eq,
    I: Strategy<S>,
{
}

impl<S, I> fmt::Debug for Secret<S, I>
where
    I: Strategy<S>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        I::fmt(&self.inner_secret, f)
    }
}

impl<S, I> Default for Secret<S, I>
where
    S: Default,
    I: Strategy<S>,
{
    fn default() -> Self {
        S::default().into()
    }
}
