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
/// Use the [`crate::strategy::Strategy`] trait to implement a masking strategy on a zero-variant
/// enum and pass this enum as a second generic parameter to [`Secret`] while defining it.
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
/// enum MyStrategy {}
///
/// impl<T> Strategy<T> for MyStrategy
/// where
///     T: fmt::Display
/// {
///     fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "{}", val.to_string().to_ascii_lowercase())
///     }
/// }
///
/// let my_secret: Secret<String, MyStrategy> = Secret::new("HELLO".to_string());
///
/// assert_eq!("hello", &format!("{:?}", my_secret));
/// ```
///
pub struct Secret<Secret, MaskingStrategy = crate::WithType>
where
    MaskingStrategy: Strategy<Secret>,
{
    pub(crate) inner_secret: Secret,
    pub(crate) masking_strategy: PhantomData<MaskingStrategy>,
}

impl<SecretValue, MaskingStrategy> Secret<SecretValue, MaskingStrategy>
where
    MaskingStrategy: Strategy<SecretValue>,
{
    /// Take ownership of a secret value
    pub fn new(secret: SecretValue) -> Self {
        Self {
            inner_secret: secret,
            masking_strategy: PhantomData,
        }
    }

    /// Zip 2 secrets with the same masking strategy into one
    pub fn zip<OtherSecretValue>(
        self,
        other: Secret<OtherSecretValue, MaskingStrategy>,
    ) -> Secret<(SecretValue, OtherSecretValue), MaskingStrategy>
    where
        MaskingStrategy: Strategy<OtherSecretValue> + Strategy<(SecretValue, OtherSecretValue)>,
    {
        (self.inner_secret, other.inner_secret).into()
    }

    /// consume self and modify the inner value
    pub fn map<OtherSecretValue>(
        self,
        f: impl FnOnce(SecretValue) -> OtherSecretValue,
    ) -> Secret<OtherSecretValue, MaskingStrategy>
    where
        MaskingStrategy: Strategy<OtherSecretValue>,
    {
        f(self.inner_secret).into()
    }
}

impl<SecretValue, MaskingStrategy> PeekInterface<SecretValue>
    for Secret<SecretValue, MaskingStrategy>
where
    MaskingStrategy: Strategy<SecretValue>,
{
    fn peek(&self) -> &SecretValue {
        &self.inner_secret
    }
}

impl<SecretValue, MaskingStrategy> From<SecretValue> for Secret<SecretValue, MaskingStrategy>
where
    MaskingStrategy: Strategy<SecretValue>,
{
    fn from(secret: SecretValue) -> Self {
        Self::new(secret)
    }
}

impl<SecretValue, MaskingStrategy> Clone for Secret<SecretValue, MaskingStrategy>
where
    SecretValue: Clone,
    MaskingStrategy: Strategy<SecretValue>,
{
    fn clone(&self) -> Self {
        Self {
            inner_secret: self.inner_secret.clone(),
            masking_strategy: PhantomData,
        }
    }
}

impl<SecretValue, MaskingStrategy> PartialEq for Secret<SecretValue, MaskingStrategy>
where
    Self: PeekInterface<SecretValue>,
    SecretValue: PartialEq,
    MaskingStrategy: Strategy<SecretValue>,
{
    fn eq(&self, other: &Self) -> bool {
        self.peek().eq(other.peek())
    }
}

impl<SecretValue, MaskingStrategy> Eq for Secret<SecretValue, MaskingStrategy>
where
    Self: PeekInterface<SecretValue>,
    SecretValue: Eq,
    MaskingStrategy: Strategy<SecretValue>,
{
}

impl<SecretValue, MaskingStrategy> fmt::Debug for Secret<SecretValue, MaskingStrategy>
where
    MaskingStrategy: Strategy<SecretValue>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        MaskingStrategy::fmt(&self.inner_secret, f)
    }
}

impl<SecretValue, MaskingStrategy> Default for Secret<SecretValue, MaskingStrategy>
where
    SecretValue: Default,
    MaskingStrategy: Strategy<SecretValue>,
{
    fn default() -> Self {
        SecretValue::default().into()
    }
}

impl<SecretValue, MaskingStrategy, T> AsRef<T> for Secret<SecretValue, MaskingStrategy>
where
    SecretValue: AsRef<T>,
    MaskingStrategy: Strategy<SecretValue>,
    T: ?Sized,
{
    fn as_ref(&self) -> &T {
        self.peek().as_ref()
    }
}
