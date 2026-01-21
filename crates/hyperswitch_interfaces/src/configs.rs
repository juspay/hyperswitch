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
