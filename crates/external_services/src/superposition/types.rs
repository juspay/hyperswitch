//! Type definitions for Superposition integration

use std::{collections::HashMap, sync::Arc};

use aws_smithy_types::Document;
use common_utils::{errors::CustomResult, fp_utils::when};
use error_stack::ResultExt;
use hyperswitch_masking::{ExposeInterface, Secret};

use super::SuperpositionClient;

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
        match *self {
            n if n >= 0 => Document::Number(aws_smithy_types::Number::PosInt(n.unsigned_abs())),
            n => Document::Number(aws_smithy_types::Number::NegInt(n)),
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
        sv.fields
            .into_iter()
            .try_fold(
                serde_json::Map::with_capacity(capacity),
                |mut map, (k, v)| {
                    let value = super::convert_open_feature_value(v)?;
                    map.insert(k, value);
                    Ok(map)
                },
            )
            .map(|map| Self(serde_json::Value::Object(map)))
    }
}

/// Polling interval used when neither `polling_interval_milliseconds` nor the deprecated
/// `polling_interval` is configured
const DEFAULT_POLLING_INTERVAL_MILLISECONDS: u64 = 15_000;

/// Converts the deprecated seconds-based `polling_interval` and `request_timeout`
const MILLISECONDS_PER_SECOND: u64 = 1_000;

/// Configuration for Superposition integration
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct SuperpositionClientConfig {
    /// Superposition API endpoint
    pub endpoint: String,
    /// Authentication token for Superposition
    pub token: Secret<String>,
    /// Organization ID in Superposition
    pub org_id: String,
    /// Workspace ID in Superposition
    pub workspace_id: String,
    /// Polling interval in milliseconds for configuration updates (default: 15000)
    pub polling_interval_milliseconds: Option<u64>,
    /// Request timeout in milliseconds for Superposition API calls (None = no timeout)
    pub request_timeout_milliseconds: Option<u64>,
    /// Deprecated: polling interval in seconds. Only read when
    /// `polling_interval_milliseconds` is not set.
    pub polling_interval: Option<u64>,
    /// Deprecated: request timeout in seconds. Only read when
    /// `request_timeout_milliseconds` is not set.
    pub request_timeout: Option<u64>,
    /// Path to a TOML backup config file on PVC/EFS.
    /// Used as fallback if the primary HTTP init fails at startup.
    pub backup_file_path: Option<std::path::PathBuf>,
}

impl Default for SuperpositionClientConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            token: Secret::new(String::new()),
            org_id: String::new(),
            workspace_id: String::new(),
            polling_interval_milliseconds: None,
            request_timeout_milliseconds: None,
            polling_interval: None,
            request_timeout: None,
            backup_file_path: None,
        }
    }
}

/// Errors that can occur when using Superposition
#[derive(Debug, thiserror::Error)]
// Deja records the WHOLE `CustomResult<T, SuperpositionError>` at the read
// boundary ("recording threw ⇒ replay throws"), so the error must round-trip.
// Every variant is `Variant(String)` → trivially (de)serializable. Feature-gated
// so the release build (deja off) keeps the error type dependency-lean.
#[cfg_attr(feature = "deja", derive(serde::Serialize, serde::Deserialize))]
pub enum SuperpositionError {
    /// Error initializing the Superposition client
    #[error("Failed to initialize Superposition client: {0}")]
    ClientInitError(String),
    /// Error from the Superposition client
    #[error("Superposition client error: {0}")]
    ClientError(String),
    /// Bad request rejected by Superposition
    #[error("Superposition bad request: {0}")]
    BadRequest(String),
    /// Resource not found in Superposition (404)
    #[error("Superposition resource not found: {0}")]
    NotFound(String),
    /// Invalid configuration provided
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    /// Error from the Superposition provider
    #[error("Superposition provider error: {0}")]
    ProviderError(String),
}

/// Context for configuration requests
#[derive(Debug, Clone, Default)]
pub struct ConfigContext {
    /// Key-value pairs for configuration context
    pub(super) values: HashMap<String, String>,
}

impl SuperpositionClientConfig {
    /// Polling interval in milliseconds. `polling_interval_milliseconds` wins, then the
    /// deprecated seconds-based `polling_interval`, then the default.
    pub fn effective_polling_interval_milliseconds(&self) -> u64 {
        self.polling_interval_milliseconds
            .or_else(|| {
                self.polling_interval
                    .map(|seconds| seconds.saturating_mul(MILLISECONDS_PER_SECOND))
            })
            .unwrap_or(DEFAULT_POLLING_INTERVAL_MILLISECONDS)
    }

    /// Request timeout in milliseconds. `request_timeout_milliseconds` wins, then the
    /// deprecated seconds-based `request_timeout`.
    pub fn effective_request_timeout_milliseconds(&self) -> Option<u64> {
        self.request_timeout_milliseconds.or_else(|| {
            self.request_timeout
                .map(|seconds| seconds.saturating_mul(MILLISECONDS_PER_SECOND))
        })
    }

    /// Warn about each deprecated seconds-based key that is set
    pub fn warn_on_deprecated_keys(&self) {
        if self.polling_interval.is_some() {
            router_env::logger::warn!(
                polling_interval_milliseconds_set = self.polling_interval_milliseconds.is_some(),
                "superposition.polling_interval (seconds) is deprecated, use superposition.polling_interval_milliseconds"
            );
        }
        if self.request_timeout.is_some() {
            router_env::logger::warn!(
                request_timeout_milliseconds_set = self.request_timeout_milliseconds.is_some(),
                "superposition.request_timeout (seconds) is deprecated, use superposition.request_timeout_milliseconds"
            );
        }
    }

    /// Create and return a Superposition client.
    pub async fn get_superposition_client(
        &self,
        service_name: &'static str,
    ) -> CustomResult<Arc<SuperpositionClient>, SuperpositionError> {
        let client = SuperpositionClient::new(self.clone(), service_name)
            .await
            .change_context(SuperpositionError::ClientInitError(
                "Failed to create Superposition client".to_string(),
            ))?;
        Ok(Arc::new(client))
    }

    /// Validate the Superposition configuration
    pub fn validate(&self) -> Result<(), SuperpositionError> {
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

    /// Get a value from the context by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
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
        let token = secret_management_client
            .get_secret(superposition_config.token.clone())
            .await?;

        Ok(value.transition_state(|superposition_config| Self {
            token,
            ..superposition_config
        }))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn parse(json: &str) -> SuperpositionClientConfig {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn defaults_to_fifteen_seconds_and_no_timeout() {
        let config = SuperpositionClientConfig::default();
        assert_eq!(config.effective_polling_interval_milliseconds(), 15_000);
        assert_eq!(config.effective_request_timeout_milliseconds(), None);
    }

    #[test]
    fn millisecond_keys_are_used_as_is() {
        let config = parse(
            r#"{"polling_interval_milliseconds": 2500, "request_timeout_milliseconds": 750}"#,
        );
        assert_eq!(config.effective_polling_interval_milliseconds(), 2_500);
        assert_eq!(config.effective_request_timeout_milliseconds(), Some(750));
    }

    #[test]
    fn deprecated_second_keys_are_converted_to_milliseconds() {
        let config = parse(r#"{"polling_interval": 300, "request_timeout": 60}"#);
        assert_eq!(config.effective_polling_interval_milliseconds(), 300_000);
        assert_eq!(
            config.effective_request_timeout_milliseconds(),
            Some(60_000)
        );
    }

    #[test]
    fn millisecond_keys_win_over_deprecated_second_keys() {
        let config = parse(
            r#"{
                "polling_interval": 300,
                "request_timeout": 60,
                "polling_interval_milliseconds": 2500,
                "request_timeout_milliseconds": 750
            }"#,
        );
        assert_eq!(config.effective_polling_interval_milliseconds(), 2_500);
        assert_eq!(config.effective_request_timeout_milliseconds(), Some(750));
    }

    #[test]
    fn deprecated_second_keys_do_not_overflow() {
        let config = parse(&format!(
            r#"{{"polling_interval": {}, "request_timeout": {}}}"#,
            u64::MAX,
            u64::MAX
        ));
        assert_eq!(config.effective_polling_interval_milliseconds(), u64::MAX);
        assert_eq!(
            config.effective_request_timeout_milliseconds(),
            Some(u64::MAX)
        );
    }
}
