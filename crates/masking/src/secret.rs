//! Structure describing secret.

use std::{fmt, marker::PhantomData};

use crate::{strategy::Strategy, PeekInterface, StrongSecret};

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

    /// Convert to [`StrongSecret`]
    pub fn into_strong(self) -> StrongSecret<SecretValue, MaskingStrategy>
    where
        SecretValue: zeroize::DefaultIsZeroes,
    {
        StrongSecret::new(self.inner_secret)
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

    fn peek_mut(&mut self) -> &mut SecretValue {
        &mut self.inner_secret
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

// Required by base64-serde to serialize Secret of Vec<u8> which contains the base64 decoded value
impl AsRef<[u8]> for Secret<Vec<u8>> {
    fn as_ref(&self) -> &[u8] {
        self.peek().as_slice()
    }
}

/// Strategy for masking JSON values
pub enum JsonMaskStrategy {}

impl Strategy<serde_json::Value> for JsonMaskStrategy {
    fn fmt(value: &serde_json::Value, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match value {
            serde_json::Value::Object(map) => {
                write!(f, "{{")?;
                let mut first = true;
                for (key, val) in map {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "\"{}\":", key)?;
                    Self::fmt(val, f)?;
                }
                write!(f, "}}")
            }
            serde_json::Value::Array(arr) => {
                write!(f, "[")?;
                let mut first = true;
                for val in arr {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    Self::fmt(val, f)?;
                }
                write!(f, "]")
            }
            serde_json::Value::String(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::Bool(_) => {
                write!(f, "\"*** {} ***\"", std::any::type_name::<serde_json::Value>())
            }
            serde_json::Value::Null => write!(f, "null"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_mask_strategy() {
        let json_value = serde_json::json!({
            "name": "John Doe",
            "email": "john@example.com",
            "age": 30,
            "is_active": true,
            "address": {
                "street": "123 Main St",
                "city": "Anytown",
                "zip": "12345"
            },
            "phones": [
                "555-1234",
                "555-5678"
            ],
            "nullable": null
        });

        let secret = Secret::<_, JsonMaskStrategy>::new(json_value);
        let formatted = format!("{:?}", secret);
        
        // Check that the output has the expected structure
        assert!(formatted.contains("\"name\":"));
        assert!(formatted.contains("\"email\":"));
        assert!(formatted.contains("\"age\":"));
        assert!(formatted.contains("\"is_active\":"));
        assert!(formatted.contains("\"address\":"));
        assert!(formatted.contains("\"street\":"));
        assert!(formatted.contains("\"phones\":"));
        assert!(formatted.contains("\"nullable\":null"));
        
        // Verify that values are masked
        assert!(formatted.contains("\"*** serde_json::value::Value ***\""));
        assert!(!formatted.contains("John Doe"));
        assert!(!formatted.contains("john@example.com"));
        assert!(!formatted.contains("30"));
        assert!(!formatted.contains("true"));
        assert!(!formatted.contains("123 Main St"));
        assert!(!formatted.contains("555-1234"));
    }
}
