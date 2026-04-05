//! Type definitions for Superposition integration

use std::collections::HashMap;

use aws_smithy_types::Document;
use common_utils::{errors::CustomResult, fp_utils::when};
use masking::{ExposeInterface, Secret};

/// Trait for converting Rust types to Superposition Document for write operations
pub trait ToDocument {
    /// Convert the value to a Document for storage in Superposition
    fn to_document(&self) -> Document;
}

impl ToDocument for String {
    fn to_document(&self) -> Document {
        Document::String(self.clone())
    }
}

impl ToDocument for bool {
    fn to_document(&self) -> Document {
        Document::Bool(*self)
    }
}

impl ToDocument for i64 {
    fn to_document(&self) -> Document {
        if *self >= 0 {
            let converted = u64::try_from(*self).unwrap_or_else(|e| {
                router_env::logger::warn!("Failed to convert i64 {} to u64: {}", self, e);
                u64::default()
            });
            Document::Number(aws_smithy_types::Number::PosInt(converted))
        } else {
            Document::Number(aws_smithy_types::Number::NegInt(*self))
        }
    }
}

impl ToDocument for i32 {
    fn to_document(&self) -> Document {
        if *self >= 0 {
            let converted = u64::try_from(*self).unwrap_or_else(|e| {
                router_env::logger::warn!("Failed to convert i32 {} to u64: {}", self, e);
                u64::default()
            });
            Document::Number(aws_smithy_types::Number::PosInt(converted))
        } else {
            // let converted = i64::try_from(*self).unwrap_or_else(|e| {
            //     router_env::logger::warn!("Failed to convert i32 {} to i64: {}", self, e);
            //     i64::default()
            // });
            let converted = i64::from(*self);
            Document::Number(aws_smithy_types::Number::NegInt(converted))
        }
    }
}

impl ToDocument for serde_json::Value {
    fn to_document(&self) -> Document {
        match self {
            Self::String(s) => Document::String(s.clone()),
            Self::Bool(b) => Document::Bool(*b),
            Self::Number(n) => {
                if let Some(n) = n.as_i64() {
                    if n >= 0 {
                        let converted = u64::try_from(n).unwrap_or_else(|e| {
                            router_env::logger::warn!(
                                "Failed to convert i64 {} to u64 in JSON value: {}",
                                n,
                                e
                            );
                            u64::default()
                        });
                        Document::Number(aws_smithy_types::Number::PosInt(converted))
                    } else {
                        Document::Number(aws_smithy_types::Number::NegInt(n))
                    }
                } else if let Some(n) = n.as_u64() {
                    Document::Number(aws_smithy_types::Number::PosInt(n))
                } else {
                    // For floats, serialize as string since Document Number doesn't support floats directly
                    Document::String(n.to_string())
                }
            }
            Self::Object(obj) => Document::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.to_document()))
                    .collect(),
            ),
            Self::Array(arr) => Document::Array(arr.iter().map(|v| v.to_document()).collect()),
            Self::Null => Document::Null,
        }
    }
}

/// Wrapper type for JSON values from Superposition
#[derive(Debug, Clone)]
pub struct JsonValue(serde_json::Value);

impl JsonValue {
    /// Consume the wrapper and return the inner JSON value
    pub(super) fn into_inner(self) -> serde_json::Value {
        self.0
    }
}

impl TryFrom<open_feature::StructValue> for JsonValue {
    type Error = String;

    fn try_from(sv: open_feature::StructValue) -> Result<Self, Self::Error> {
        let capacity = sv.fields.len();
        let map: serde_json::Map<String, serde_json::Value> =
            sv.fields.into_iter().try_fold(
                serde_json::Map::with_capacity(capacity),
                |mut map, (k, v)| -> Result<_, String> {
                    let value = super::convert_open_feature_value(v)?;
                    map.insert(k, value);
                    Ok(map)
                },
            )?;

        // When the CAC value is a JSON array, the OpenFeature provider encodes it as a
        // StructValue with consecutive integer string keys ("0", "1", ...). Detect this
        // and convert back to a proper JSON array so downstream deserialization works.
        let is_array = !map.is_empty()
            && (0..map.len()).all(|i| map.contains_key(&i.to_string()));

        if is_array {
            let arr = (0..map.len())
                .map(|i| map[&i.to_string()].clone())
                .collect();
            Ok(Self(serde_json::Value::Array(arr)))
        } else {
            Ok(Self(serde_json::Value::Object(map)))
        }
    }
}

/// Configuration for Superposition integration
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct SuperpositionClientConfig {
    /// Whether Superposition is enabled
    pub enabled: bool,
    /// Superposition API endpoint
    pub endpoint: String,
    /// Authentication token for Superposition
    pub token: Secret<String>,
    /// Organization ID in Superposition
    pub org_id: String,
    /// Workspace ID in Superposition
    pub workspace_id: String,
    /// Polling interval in seconds for configuration updates
    pub polling_interval: u64,
    /// Request timeout in seconds for Superposition API calls (None = no timeout)
    pub request_timeout: Option<u64>,
}

impl Default for SuperpositionClientConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: String::new(),
            token: Secret::new(String::new()),
            org_id: String::new(),
            workspace_id: String::new(),
            polling_interval: 15,
            request_timeout: None,
        }
    }
}

/// Errors that can occur when using Superposition
#[derive(Debug, thiserror::Error)]
pub enum SuperpositionError {
    /// Error initializing the Superposition client
    #[error("Failed to initialize Superposition client: {0}")]
    ClientInitError(String),
    /// Error from the Superposition client
    #[error("Superposition client error: {0}")]
    ClientError(String),
    /// Invalid configuration provided
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

/// Context for configuration requests
#[derive(Debug, Clone, Default)]
pub struct ConfigContext {
    /// Key-value pairs for configuration context
    pub(super) values: HashMap<String, String>,
}

impl SuperpositionClientConfig {
    /// Validate the Superposition configuration
    pub fn validate(&self) -> Result<(), SuperpositionError> {
        if !self.enabled {
            return Ok(());
        }

        when(self.endpoint.is_empty(), || {
            Err(SuperpositionError::InvalidConfiguration(
                "Superposition endpoint cannot be empty".to_string(),
            ))
        })?;

        when(url::Url::parse(&self.endpoint).is_err(), || {
            Err(SuperpositionError::InvalidConfiguration(
                "Superposition endpoint must be a valid URL".to_string(),
            ))
        })?;

        when(self.token.clone().expose().is_empty(), || {
            Err(SuperpositionError::InvalidConfiguration(
                "Superposition token cannot be empty".to_string(),
            ))
        })?;

        when(self.org_id.is_empty(), || {
            Err(SuperpositionError::InvalidConfiguration(
                "Superposition org_id cannot be empty".to_string(),
            ))
        })?;

        when(self.workspace_id.is_empty(), || {
            Err(SuperpositionError::InvalidConfiguration(
                "Superposition workspace_id cannot be empty".to_string(),
            ))
        })?;

        Ok(())
    }
}

impl ConfigContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a key-value pair to the context. Replaces existing value if key exists.
    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.values.insert(key.to_string(), value.to_string());
        self
    }
}

#[cfg(feature = "superposition")]
#[async_trait::async_trait]
impl hyperswitch_interfaces::secrets_interface::secret_handler::SecretsHandler
    for SuperpositionClientConfig
{
    async fn convert_to_raw_secret(
        value: hyperswitch_interfaces::secrets_interface::secret_state::SecretStateContainer<
            Self,
            hyperswitch_interfaces::secrets_interface::secret_state::SecuredSecret,
        >,
        secret_management_client: &dyn hyperswitch_interfaces::secrets_interface::SecretManagementInterface,
    ) -> CustomResult<
        hyperswitch_interfaces::secrets_interface::secret_state::SecretStateContainer<
            Self,
            hyperswitch_interfaces::secrets_interface::secret_state::RawSecret,
        >,
        hyperswitch_interfaces::secrets_interface::SecretsManagementError,
    > {
        let superposition_config = value.get_inner();
        let token = if superposition_config.enabled {
            secret_management_client
                .get_secret(superposition_config.token.clone())
                .await?
        } else {
            superposition_config.token.clone()
        };

        Ok(value.transition_state(|superposition_config| Self {
            token,
            ..superposition_config
        }))
    }
}
