use crate::secrets_interface::secret_handler::SecretsHandler;
use crate::secrets_interface::secret_state::RawSecret;
use crate::secrets_interface::secret_state::SecretStateContainer;
use crate::secrets_interface::secret_state::SecuredSecret;
use crate::secrets_interface::SecretManagementInterface;
use crate::secrets_interface::SecretsManagementError;
use common_enums::enums::ApplicationError;
use common_utils::ext_traits::ConfigExt;
use common_utils::{crypto::Encryptable, errors::CustomResult, id_type};
pub use hyperswitch_domain_models::{
    connector_endpoints::Connectors, errors::api_error_response, merchant_connector_account,
};
use masking::{PeekInterface, Secret};
use serde::Deserialize;

#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct Tenant {
    pub tenant_id: id_type::TenantId,
    pub base_url: String,
    pub schema: String,
    pub accounts_schema: String,
    pub redis_key_prefix: String,
    pub clickhouse_database: String,
    pub user: TenantUserConfig,
}

#[allow(missing_docs)]
#[derive(Debug, Deserialize, Clone)]
pub struct TenantUserConfig {
    pub control_center_url: String,
}

impl common_utils::types::TenantConfig for Tenant {
    fn get_tenant_id(&self) -> &id_type::TenantId {
        &self.tenant_id
    }
    fn get_accounts_schema(&self) -> &str {
        self.accounts_schema.as_str()
    }
    fn get_schema(&self) -> &str {
        self.schema.as_str()
    }
    fn get_redis_key_prefix(&self) -> &str {
        self.redis_key_prefix.as_str()
    }
    fn get_clickhouse_database(&self) -> &str {
        self.clickhouse_database.as_str()
    }
}

#[allow(missing_docs)]
// Todo: Global tenant should not be part of tenant config(https://github.com/juspay/hyperswitch/issues/7237)
#[derive(Debug, Deserialize, Clone)]
pub struct GlobalTenant {
    #[serde(default = "id_type::TenantId::get_default_global_tenant_id")]
    pub tenant_id: id_type::TenantId,
    pub schema: String,
    pub redis_key_prefix: String,
    pub clickhouse_database: String,
}

// Todo: Global tenant should not be part of tenant config
impl common_utils::types::TenantConfig for GlobalTenant {
    fn get_tenant_id(&self) -> &id_type::TenantId {
        &self.tenant_id
    }
    fn get_accounts_schema(&self) -> &str {
        self.schema.as_str()
    }
    fn get_schema(&self) -> &str {
        self.schema.as_str()
    }
    fn get_redis_key_prefix(&self) -> &str {
        self.redis_key_prefix.as_str()
    }
    fn get_clickhouse_database(&self) -> &str {
        self.clickhouse_database.as_str()
    }
}

impl Default for GlobalTenant {
    fn default() -> Self {
        Self {
            tenant_id: id_type::TenantId::get_default_global_tenant_id(),
            schema: String::from("global"),
            redis_key_prefix: String::from("global"),
            clickhouse_database: String::from("global"),
        }
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct InternalMerchantIdProfileIdAuthSettings {
    pub enabled: bool,
    pub internal_api_key: Secret<String>,
}

#[allow(missing_docs)]
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct InternalServicesConfig {
    pub payments_base_url: String,
}

#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub enum MerchantConnectorAccountType {
    DbVal(Box<merchant_connector_account::MerchantConnectorAccount>),
    CacheVal(api_models::admin::MerchantConnectorDetails),
}

#[allow(missing_docs)]
impl MerchantConnectorAccountType {
    pub fn get_metadata(&self) -> Option<Secret<serde_json::Value>> {
        match self {
            Self::DbVal(val) => val.metadata.to_owned(),
            Self::CacheVal(val) => val.metadata.to_owned(),
        }
    }

    pub fn get_connector_account_details(&self) -> serde_json::Value {
        match self {
            Self::DbVal(val) => val.connector_account_details.peek().to_owned(),
            Self::CacheVal(val) => val.connector_account_details.peek().to_owned(),
        }
    }

    pub fn get_connector_wallets_details(&self) -> Option<Secret<serde_json::Value>> {
        match self {
            Self::DbVal(val) => val.connector_wallets_details.as_deref().cloned(),
            Self::CacheVal(_) => None,
        }
    }

    pub fn is_disabled(&self) -> bool {
        match self {
            Self::DbVal(ref inner) => inner.disabled.unwrap_or(false),
            // Cached merchant connector account, only contains the account details,
            // the merchant connector account must only be cached if it's not disabled
            Self::CacheVal(_) => false,
        }
    }

    #[cfg(feature = "v1")]
    pub fn is_test_mode_on(&self) -> Option<bool> {
        match self {
            Self::DbVal(val) => val.test_mode,
            Self::CacheVal(_) => None,
        }
    }

    #[cfg(feature = "v2")]
    pub fn is_test_mode_on(&self) -> Option<bool> {
        None
    }

    pub fn get_mca_id(&self) -> Option<id_type::MerchantConnectorAccountId> {
        match self {
            Self::DbVal(db_val) => Some(db_val.get_id()),
            Self::CacheVal(_) => None,
        }
    }

    #[cfg(feature = "v1")]
    pub fn get_connector_name(&self) -> Option<String> {
        match self {
            Self::DbVal(db_val) => Some(db_val.connector_name.to_string()),
            Self::CacheVal(_) => None,
        }
    }

    #[cfg(feature = "v2")]
    pub fn get_connector_name(&self) -> Option<common_enums::connector_enums::Connector> {
        match self {
            Self::DbVal(db_val) => Some(db_val.connector_name),
            Self::CacheVal(_) => None,
        }
    }

    pub fn get_additional_merchant_data(&self) -> Option<Encryptable<Secret<serde_json::Value>>> {
        match self {
            Self::DbVal(db_val) => db_val.additional_merchant_data.clone(),
            Self::CacheVal(_) => None,
        }
    }

    pub fn get_webhook_details(
        &self,
    ) -> CustomResult<Option<&Secret<serde_json::Value>>, api_error_response::ApiErrorResponse>
    {
        match self {
            Self::DbVal(db_val) => Ok(db_val.connector_webhook_details.as_ref()),
            Self::CacheVal(_) => Ok(None),
        }
    }
}

#[allow(missing_docs)]
#[derive(Debug, Deserialize, Clone, Default)]
pub enum DecryptionScheme {
    #[default]
    #[serde(rename = "RSA-OAEP")]
    RsaOaep,
    #[serde(rename = "RSA-OAEP-256")]
    RsaOaep256,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(missing_docs)]
#[serde(default)]
pub struct Locker {
    pub host: String,
    pub host_rs: String,
    pub mock_locker: bool,
    pub basilisk_host: String,
    pub locker_signing_key_id: String,
    pub locker_enabled: bool,
    pub ttl_for_storage_in_secs: i64,
    pub decryption_scheme: DecryptionScheme,
}

impl Default for Locker {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            host_rs: "localhost".into(),
            mock_locker: true,
            basilisk_host: "localhost".into(),
            locker_signing_key_id: "1".into(),
            //true or false
            locker_enabled: true,
            //Time to live for storage entries in locker
            ttl_for_storage_in_secs: 60 * 60 * 24 * 365 * 7,
            decryption_scheme: Default::default(),
        }
    }
}

#[allow(missing_docs)]
impl Locker {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(!self.mock_locker && self.host.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "locker host must not be empty when mock locker is disabled".into(),
            ))
        })?;

        when(
            !self.mock_locker && self.basilisk_host.is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "basilisk host must not be empty when mock locker is disabled".into(),
                ))
            },
        )
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[allow(missing_docs)]
#[serde(default)]
pub struct Jwekey {
    pub vault_encryption_key: Secret<String>,
    pub rust_locker_encryption_key: Secret<String>,
    pub vault_private_key: Secret<String>,
    pub tunnel_private_key: Secret<String>,
}

#[async_trait::async_trait]
impl SecretsHandler for Jwekey {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let jwekey = value.get_inner();
        let (
            vault_encryption_key,
            rust_locker_encryption_key,
            vault_private_key,
            tunnel_private_key,
        ) = tokio::try_join!(
            secret_management_client.get_secret(jwekey.vault_encryption_key.clone()),
            secret_management_client.get_secret(jwekey.rust_locker_encryption_key.clone()),
            secret_management_client.get_secret(jwekey.vault_private_key.clone()),
            secret_management_client.get_secret(jwekey.tunnel_private_key.clone())
        )?;
        Ok(value.transition_state(|_| Self {
            vault_encryption_key,
            rust_locker_encryption_key,
            vault_private_key,
            tunnel_private_key,
        }))
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(missing_docs)]
pub struct NetworkTokenizationService {
    pub generate_token_url: url::Url,
    pub fetch_token_url: url::Url,
    pub token_service_api_key: Secret<String>,
    pub public_key: Secret<String>,
    pub private_key: Secret<String>,
    pub key_id: String,
    pub delete_token_url: url::Url,
    pub check_token_status_url: url::Url,
    pub webhook_source_verification_key: Secret<String>,
}

#[allow(missing_docs)]
impl NetworkTokenizationService {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.token_service_api_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "token_service_api_key must not be empty".into(),
            ))
        })?;

        when(self.public_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "public_key must not be empty".into(),
            ))
        })?;

        when(self.key_id.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "key_id must not be empty".into(),
            ))
        })?;

        when(self.private_key.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "private_key must not be empty".into(),
            ))
        })?;

        when(
            self.webhook_source_verification_key.is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "webhook_source_verification_key must not be empty".into(),
                ))
            },
        )
    }
}

#[async_trait::async_trait]
impl SecretsHandler for NetworkTokenizationService {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let network_tokenization = value.get_inner();
        let token_service_api_key = secret_management_client
            .get_secret(network_tokenization.token_service_api_key.clone())
            .await?;
        let public_key = secret_management_client
            .get_secret(network_tokenization.public_key.clone())
            .await?;
        let private_key = secret_management_client
            .get_secret(network_tokenization.private_key.clone())
            .await?;
        let webhook_source_verification_key = secret_management_client
            .get_secret(network_tokenization.webhook_source_verification_key.clone())
            .await?;

        Ok(value.transition_state(|network_tokenization| Self {
            public_key,
            private_key,
            token_service_api_key,
            webhook_source_verification_key,
            ..network_tokenization
        }))
    }
}
