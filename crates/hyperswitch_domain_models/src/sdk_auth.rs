use base64::Engine;
use common_utils::{
    consts::BASE64_ENGINE,
    errors::{CustomResult, ValidationError},
    id_type,
};
use error_stack::{report, ResultExt};

/// SDK authorization data for client-side SDK authentication
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SdkAuthorization {
    /// The profile ID associated with this payment
    pub profile_id: id_type::ProfileId,

    /// The publishable key of the processor merchant (connected or standard)
    pub publishable_key: String,

    /// Platform publishable key (only for platform-initiated flows)
    pub platform_publishable_key: Option<String>,

    /// Client secret for the payment (if available)
    pub client_secret: Option<String>,

    /// Customer ID for the payment (if available)
    pub customer_id: Option<id_type::CustomerId>,
}

impl SdkAuthorization {
    /// Encodes SdkAuthorization into base64-encoded comma-separated key-value pairs
    ///
    /// Returns base64-encoded string in format: `base64(key1=value1,key2=value2,...)`
    pub fn encode(&self) -> CustomResult<String, ValidationError> {
        // Use functional style to build parts without mutations
        let parts: Vec<String> = [
            Some(format!("profile_id={}", self.profile_id.get_string_repr())),
            Some(format!("publishable_key={}", self.publishable_key)),
            self.platform_publishable_key
                .as_ref()
                .map(|k| format!("platform_publishable_key={}", k)),
            self.client_secret
                .as_ref()
                .map(|s| format!("client_secret={}", s)),
            self.customer_id
                .as_ref()
                .map(|id| format!("customer_id={}", id.get_string_repr())),
        ]
        .into_iter()
        .flatten() // Remove None values
        .collect();

        let comma_separated = parts.join(",");
        Ok(BASE64_ENGINE.encode(comma_separated))
    }

    /// Decodes base64 string to SdkAuthorization
    ///
    /// # Arguments
    /// * `encoded` - Base64-encoded string containing comma-separated key-value pairs
    ///
    /// # Returns
    /// Decoded and validated SdkAuthorization instance
    pub fn decode(encoded: &str) -> CustomResult<Self, ValidationError> {
        let decoded_bytes =
            BASE64_ENGINE
                .decode(encoded)
                .change_context(ValidationError::InvalidValue {
                    message: "Failed to decode SDK authorization".to_string(),
                })?;

        let comma_separated =
            String::from_utf8(decoded_bytes).change_context(ValidationError::InvalidValue {
                message: "SDK authorization is not valid UTF-8".to_string(),
            })?;

        let mut profile_id = None;
        let mut publishable_key = None;
        let mut platform_publishable_key = None;
        let mut client_secret = None;
        let mut customer_id = None;

        for part in comma_separated.split(',') {
            let (key, value) = part.split_once('=').ok_or_else(|| {
                report!(ValidationError::InvalidValue {
                    message: "Invalid SDK authorization format: missing '=' separator".to_string()
                })
            })?;

            match key.trim() {
                "profile_id" => profile_id = Some(value.trim().to_string()),
                "publishable_key" => publishable_key = Some(value.trim().to_string()),
                "platform_publishable_key" => {
                    platform_publishable_key = Some(value.trim().to_string())
                }
                "client_secret" => client_secret = Some(value.trim().to_string()),
                "customer_id" => customer_id = Some(value.trim().to_string()),
                _ => {
                    // Ignore unknown keys for forward compatibility
                }
            }
        }

        Ok(Self {
            profile_id: profile_id
                .ok_or_else(|| {
                    report!(ValidationError::InvalidValue {
                        message: "Missing required field: profile_id".to_string()
                    })
                })
                .and_then(|id_str| {
                    id_type::ProfileId::try_from(std::borrow::Cow::from(id_str)).change_context(
                        ValidationError::InvalidValue {
                            message: "Invalid profile_id format".to_string(),
                        },
                    )
                })?,
            publishable_key: publishable_key.ok_or_else(|| {
                report!(ValidationError::InvalidValue {
                    message: "Missing required field: publishable_key".to_string()
                })
            })?,
            platform_publishable_key,
            client_secret,
            customer_id: customer_id
                .map(|id_str| {
                    id_type::CustomerId::try_from(std::borrow::Cow::from(id_str)).change_context(
                        ValidationError::InvalidValue {
                            message: "Invalid customer_id format".to_string(),
                        },
                    )
                })
                .transpose()?,
        })
    }
}
