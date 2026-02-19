use std::collections::HashMap;

use base64::Engine;
use common_utils::{
    consts::BASE64_ENGINE,
    errors::{CustomResult, ValidationError},
    id_type,
};
use error_stack::{report, ResultExt};

#[cfg(feature = "v2")]
use crate::platform::Initiator;
/// Input for constructing SdkAuthorization from a Platform context (V2 only)
#[cfg(feature = "v2")]
pub struct SdkAuthorizationContext {
    pub platform: crate::platform::Platform,
    pub profile_id: id_type::ProfileId,
    pub client_secret: String,
    pub customer_id: Option<id_type::GlobalCustomerId>,
    pub payment_method_session_id: Option<id_type::GlobalPaymentMethodSessionId>,
}

#[cfg(feature = "v2")]
impl From<SdkAuthorizationContext> for Option<SdkAuthorization> {
    fn from(input: SdkAuthorizationContext) -> Self {
        match input.platform.get_initiator()? {
            Initiator::Api {
                merchant_account_type,
                publishable_key,
                ..
            } => {
                let platform_publishable_key = if matches!(
                    merchant_account_type,
                    common_enums::MerchantAccountType::Platform
                ) {
                    Some(publishable_key.clone())
                } else {
                    None
                };
                Some(SdkAuthorization {
                    profile_id: input.profile_id,
                    publishable_key: input
                        .platform
                        .get_processor()
                        .get_account()
                        .publishable_key
                        .clone(),
                    platform_publishable_key,
                    client_secret: input.client_secret,
                    customer_id: input.customer_id,
                    payment_method_session_id: input.payment_method_session_id,
                })
            }
            Initiator::Admin | Initiator::Jwt { .. } | Initiator::EmbeddedToken { .. } => None, // SDK authorization is only applicable for API initiators
        }
    }
}

/// SDK authorization data for client-side SDK authentication
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SdkAuthorization {
    /// The profile ID associated with this payment/session
    pub profile_id: id_type::ProfileId,

    /// The publishable key of the processor merchant (connected or standard)
    pub publishable_key: String,

    /// Platform publishable key (only for platform-initiated flows)
    pub platform_publishable_key: Option<String>,

    /// Client secret for the payment/session (required for SDK authorization)
    pub client_secret: String,

    /// Customer ID for the payment/session (if available)
    #[cfg(feature = "v1")]
    pub customer_id: Option<id_type::CustomerId>,

    /// Customer ID for the payment/session (if available)
    #[cfg(feature = "v2")]
    pub customer_id: Option<id_type::GlobalCustomerId>,

    #[cfg(feature = "v2")]
    pub payment_method_session_id: Option<id_type::GlobalPaymentMethodSessionId>,
}

impl SdkAuthorization {
    /// Encodes SdkAuthorization into base64-encoded comma-separated key-value pairs
    ///
    /// Returns base64-encoded string in format: `base64(key1=value1,key2=value2,...)`
    pub fn encode(&self) -> CustomResult<String, ValidationError> {
        let comma_separated = [
            Some(format!("profile_id={}", self.profile_id.get_string_repr())),
            Some(format!("publishable_key={}", self.publishable_key)),
            self.platform_publishable_key
                .as_ref()
                .map(|platform_publishable_key| {
                    format!("platform_publishable_key={}", platform_publishable_key)
                }),
            Some(format!("client_secret={}", self.client_secret)),
            self.customer_id
                .as_ref()
                .map(|id| format!("customer_id={}", id.get_string_repr())),
            #[cfg(feature = "v2")]
            self.payment_method_session_id
                .as_ref()
                .map(|id| format!("payment_method_session_id={}", id.get_string_repr())),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(",");

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

        let parts: HashMap<&str, &str> = comma_separated
            .split(',')
            .map(|part| {
                part.split_once('=')
                    .map(|(key, value)| (key.trim(), value.trim()))
                    .ok_or_else(|| {
                        report!(ValidationError::InvalidValue {
                            message: "Invalid SDK authorization format: missing '=' separator"
                                .to_string()
                        })
                    })
            })
            .collect::<CustomResult<HashMap<_, _>, _>>()?;

        Ok(Self {
            profile_id: id_type::ProfileId::try_from(std::borrow::Cow::from(
                parts
                    .get("profile_id")
                    .ok_or_else(|| {
                        report!(ValidationError::InvalidValue {
                            message: "Missing required field: profile_id".to_string()
                        })
                    })?
                    .to_string(),
            ))
            .change_context(ValidationError::InvalidValue {
                message: "Invalid profile_id format".to_string(),
            })?,
            publishable_key: parts
                .get("publishable_key")
                .ok_or_else(|| {
                    report!(ValidationError::InvalidValue {
                        message: "Missing required field: publishable_key".to_string()
                    })
                })?
                .to_string(),
            platform_publishable_key: parts
                .get("platform_publishable_key")
                .map(|platform_publishable_key| platform_publishable_key.to_string()),
            client_secret: parts
                .get("client_secret")
                .ok_or_else(|| {
                    report!(ValidationError::InvalidValue {
                        message: "Missing required field: client_secret".to_string()
                    })
                })?
                .to_string(),
            #[cfg(feature = "v1")]
            customer_id: parts
                .get("customer_id")
                .map(|customer_id| {
                    id_type::CustomerId::try_from(std::borrow::Cow::from(customer_id.to_string()))
                        .change_context(ValidationError::InvalidValue {
                            message: "Invalid customer_id format".to_string(),
                        })
                })
                .transpose()?,
            #[cfg(feature = "v2")]
            customer_id: parts.get("customer_id").map(|customer_id| {
                id_type::GlobalCustomerId::new_unchecked(customer_id.to_string())
            }),
            #[cfg(feature = "v2")]
            payment_method_session_id: parts.get("payment_method_session_id").map(
                |payment_method_session_id| {
                    id_type::GlobalPaymentMethodSessionId::new_unchecked(
                        payment_method_session_id.to_string(),
                    )
                },
            ),
        })
    }
}
