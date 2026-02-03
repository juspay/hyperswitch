use base64::Engine;
use common_utils::{
    consts::BASE64_ENGINE,
    errors::{CustomResult, ValidationError},
    id_type,
};
use error_stack::ResultExt;

/// SDK authorization data for client-side SDK authentication
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SdkAuthorization {
    /// The profile ID associated with this payment
    pub profile_id: id_type::ProfileId,

    /// The publishable key of the processor merchant (connected or standard)
    pub publishable_key: String,

    /// Platform publishable key (only for platform-initiated flows)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub platform_publishable_key: Option<String>,

    /// Client secret for the payment (if available)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub client_secret: Option<String>,

    /// Customer ID for the payment (if available)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub customer_id: Option<id_type::CustomerId>,
}

impl SdkAuthorization {
    /// Encodes SdkAuthorization into a base64-encoded URL-encoded string
    ///
    /// Returns base64-encoded string in format: `base64(key1=value1&key2=value2&...)`
    pub fn encode(&self) -> CustomResult<String, ValidationError> {
        let url_encoded =
            serde_urlencoded::to_string(self).change_context(ValidationError::InvalidValue {
                message: "Failed to URL-encode SDK authorization".to_string(),
            })?;

        Ok(BASE64_ENGINE.encode(url_encoded))
    }

    /// Decodes a base64-encoded URL-encoded string into SdkAuthorization
    ///
    /// # Arguments
    /// * `encoded` - Base64-encoded string containing URL-encoded key-value pairs
    pub fn decode(encoded: &str) -> CustomResult<Self, ValidationError> {
        let decoded_bytes =
            BASE64_ENGINE
                .decode(encoded)
                .change_context(ValidationError::InvalidValue {
                    message: "Failed to base64-decode SDK authorization".to_string(),
                })?;

        let url_encoded =
            String::from_utf8(decoded_bytes).change_context(ValidationError::InvalidValue {
                message: "SDK authorization is not valid UTF-8".to_string(),
            })?;

        serde_urlencoded::from_str(&url_encoded).change_context(ValidationError::InvalidValue {
            message: "Failed to URL-decode SDK authorization".to_string(),
        })
    }
}
