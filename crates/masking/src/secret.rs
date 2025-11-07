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

    /// Convert to [`Secret`] with a reference to the inner secret
    pub fn as_ref(&self) -> Secret<&SecretValue, MaskingStrategy>
    where
        MaskingStrategy: for<'a> Strategy<&'a SecretValue>,
    {
        Secret::new(self.peek())
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

impl<SecretValue, MaskingStrategy> std::hash::Hash for Secret<SecretValue, MaskingStrategy>
where
    Self: PeekInterface<SecretValue>,
    SecretValue: std::hash::Hash,
    MaskingStrategy: Strategy<SecretValue>,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.peek().hash(state);
    }
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
#[cfg(feature = "serde")]
pub enum JsonMaskStrategy {}

#[cfg(feature = "serde")]
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
                    write!(f, "\"{key}\":")?;
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
            serde_json::Value::String(s) => {
                // For strings, we show a masked version that gives a hint about the content
                let masked = if s.len() <= 2 {
                    "**".to_string()
                } else if s.len() <= 6 {
                    format!("{}**", &s[0..1])
                } else {
                    // For longer strings, show first and last character with length in between
                    format!(
                        "{}**{}**{}",
                        &s[0..1],
                        s.len() - 2,
                        &s[s.len() - 1..s.len()]
                    )
                };
                write!(f, "\"{masked}\"")
            }
            serde_json::Value::Number(n) => {
                // For numbers, we can show the order of magnitude
                if n.is_i64() || n.is_u64() {
                    let num_str = n.to_string();
                    let masked_num = "*".repeat(num_str.len());
                    write!(f, "{masked_num}")
                } else if n.is_f64() {
                    // For floats, just use a generic mask
                    write!(f, "**.**")
                } else {
                    write!(f, "0")
                }
            }
            serde_json::Value::Bool(b) => {
                // For booleans, we can show a hint about which one it is
                write!(f, "{}", if *b { "**true" } else { "**false" })
            }
            serde_json::Value::Null => write!(f, "null"),
        }
    }
}

#[cfg(feature = "proto_tonic")]
impl<T> prost::Message for Secret<T, crate::WithType>
where
    T: prost::Message + Default + Clone,
{
    fn encode_raw(&self, buf: &mut impl bytes::BufMut) {
        self.peek().encode_raw(buf);
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut impl bytes::Buf,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        if tag == 1 {
            self.peek_mut().merge_field(tag, wire_type, buf, ctx)
        } else {
            prost::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        self.peek().encoded_len()
    }

    fn clear(&mut self) {
        self.peek_mut().clear();
    }
}

#[cfg(test)]
mod hash_tests {
    use std::hash::{DefaultHasher, Hash, Hasher};

    use super::*;

    #[test]
    fn test_secret_hash_implementation() {
        let secret1: Secret<String> = Secret::new("test_string".to_string());
        let secret2: Secret<String> = Secret::new("test_string".to_string());
        let secret3: Secret<String> = Secret::new("different_string".to_string());

        // Test that equal secrets hash to the same value
        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        secret1.hash(&mut hasher1);
        secret2.hash(&mut hasher2);
        assert_eq!(hasher1.finish(), hasher2.finish());

        // Test that different secrets hash to different values (usually)
        let mut hasher3 = DefaultHasher::new();
        secret3.hash(&mut hasher3);
        assert_ne!(hasher1.finish(), hasher3.finish());
    }
}

#[cfg(test)]
#[cfg(feature = "serde")]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_json_mask_strategy() {
        // Create a sample JSON with different types for testing
        let original = json!({ "user": { "name": "John Doe", "email": "john@example.com", "age": 35, "verified": true }, "card": { "number": "4242424242424242", "cvv": 123, "amount": 99.99 }, "tags": ["personal", "premium"], "null_value": null, "short": "hi" });

        // Apply the JsonMaskStrategy
        let secret = Secret::<_, JsonMaskStrategy>::new(original.clone());
        let masked_str = format!("{secret:?}");

        // Get specific values from original
        let original_obj = original.as_object().expect("Original should be an object");
        let user_obj = original_obj["user"]
            .as_object()
            .expect("User should be an object");
        let name = user_obj["name"].as_str().expect("Name should be a string");
        let email = user_obj["email"]
            .as_str()
            .expect("Email should be a string");
        let age = user_obj["age"].as_i64().expect("Age should be a number");
        let verified = user_obj["verified"]
            .as_bool()
            .expect("Verified should be a boolean");

        let card_obj = original_obj["card"]
            .as_object()
            .expect("Card should be an object");
        let card_number = card_obj["number"]
            .as_str()
            .expect("Card number should be a string");
        let cvv = card_obj["cvv"].as_i64().expect("CVV should be a number");

        let tags = original_obj["tags"]
            .as_array()
            .expect("Tags should be an array");
        let tag1 = tags
            .first()
            .and_then(|v| v.as_str())
            .expect("First tag should be a string");

        // Now explicitly verify the masking patterns for each value type

        // 1. String masking - pattern: first char + ** + length - 2 + ** + last char
        let expected_name_mask = format!(
            "\"{}**{}**{}\"",
            &name[0..1],
            name.len() - 2,
            &name[name.len() - 1..]
        );
        let expected_email_mask = format!(
            "\"{}**{}**{}\"",
            &email[0..1],
            email.len() - 2,
            &email[email.len() - 1..]
        );
        let expected_card_mask = format!(
            "\"{}**{}**{}\"",
            &card_number[0..1],
            card_number.len() - 2,
            &card_number[card_number.len() - 1..]
        );
        let expected_tag1_mask = if tag1.len() <= 2 {
            "\"**\"".to_string()
        } else if tag1.len() <= 6 {
            format!("\"{}**\"", &tag1[0..1])
        } else {
            format!(
                "\"{}**{}**{}\"",
                &tag1[0..1],
                tag1.len() - 2,
                &tag1[tag1.len() - 1..]
            )
        };
        let expected_short_mask = "\"**\"".to_string(); // For "hi"

        // 2. Number masking
        let expected_age_mask = "*".repeat(age.to_string().len()); // Repeat * for the number of digits
        let expected_cvv_mask = "*".repeat(cvv.to_string().len());

        // 3. Boolean masking
        let expected_verified_mask = if verified { "**true" } else { "**false" };

        // Check that the masked output includes the expected masked patterns
        assert!(
            masked_str.contains(&expected_name_mask),
            "Name not masked correctly. Expected: {expected_name_mask}"
        );
        assert!(
            masked_str.contains(&expected_email_mask),
            "Email not masked correctly. Expected: {expected_email_mask}",
        );
        assert!(
            masked_str.contains(&expected_card_mask),
            "Card number not masked correctly. Expected: {expected_card_mask}",
        );
        assert!(
            masked_str.contains(&expected_tag1_mask),
            "Tag not masked correctly. Expected: {expected_tag1_mask}",
        );
        assert!(
            masked_str.contains(&expected_short_mask),
            "Short string not masked correctly. Expected: {expected_short_mask}",
        );

        assert!(
            masked_str.contains(&expected_age_mask),
            "Age not masked correctly. Expected: {expected_age_mask}",
        );
        assert!(
            masked_str.contains(&expected_cvv_mask),
            "CVV not masked correctly. Expected: {expected_cvv_mask}",
        );

        assert!(
            masked_str.contains(expected_verified_mask),
            "Boolean not masked correctly. Expected: {expected_verified_mask}",
        );

        // Check structure preservation
        assert!(
            masked_str.contains("\"user\""),
            "Structure not preserved - missing user object"
        );
        assert!(
            masked_str.contains("\"card\""),
            "Structure not preserved - missing card object"
        );
        assert!(
            masked_str.contains("\"tags\""),
            "Structure not preserved - missing tags array"
        );
        assert!(
            masked_str.contains("\"null_value\":null"),
            "Null value not preserved correctly"
        );

        // Additional security checks to ensure no original values are exposed
        assert!(
            !masked_str.contains(name),
            "Original name value exposed in masked output"
        );
        assert!(
            !masked_str.contains(email),
            "Original email value exposed in masked output"
        );
        assert!(
            !masked_str.contains(card_number),
            "Original card number exposed in masked output"
        );
        assert!(
            !masked_str.contains(&age.to_string()),
            "Original age value exposed in masked output"
        );
        assert!(
            !masked_str.contains(&cvv.to_string()),
            "Original CVV value exposed in masked output"
        );
        assert!(
            !masked_str.contains(tag1),
            "Original tag value exposed in masked output"
        );
        assert!(
            !masked_str.contains("hi"),
            "Original short string value exposed in masked output"
        );
    }
}
