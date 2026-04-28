use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProfileAcquirerCreate {
    /// The merchant id assigned by the acquirer
    #[schema(value_type= String,example = "M123456789")]
    pub acquirer_assigned_merchant_id: String,
    /// merchant name
    #[schema(value_type= String,example = "NewAge Retailer")]
    pub merchant_name: String,
    /// Network provider
    #[schema(value_type= String,example = "VISA")]
    pub network: common_enums::enums::CardNetwork,
    /// Acquirer bin
    #[schema(value_type= String,example = "456789")]
    pub acquirer_bin: String,
    /// Acquirer ica provided by acquirer
    #[schema(value_type= Option<String>,example = "401288")]
    pub acquirer_ica: Option<String>,
    /// Fraud rate for the particular acquirer configuration
    #[schema(value_type= Option<f64>,example = 0.01)]
    pub acquirer_fraud_rate: Option<f64>,
    /// Acquirer country code
    #[schema(value_type= Option<String>,example = "US")]
    pub acquirer_country_code: Option<String>,
    /// Parent profile id to link the acquirer account with
    #[schema(value_type= String,example = "pro_ky0yNyOXXlA5hF8JzE5q")]
    pub profile_id: common_utils::id_type::ProfileId,
    /// Whether this configuration bucket is the default fallback for the profile.
    pub is_default: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct ProfileAcquirerResponse {
    /// The unique identifier of the profile acquirer
    #[schema(value_type= String,example = "pro_acq_LCRdERuylQvNQ4qh3QE0")]
    pub profile_acquirer_id: common_utils::id_type::ProfileAcquirerId,
    /// The merchant id assigned by the acquirer
    #[schema(value_type= Option<String>,example = "M123456789")]
    pub acquirer_assigned_merchant_id: Option<String>,
    /// Merchant name
    #[schema(value_type= Option<String>,example = "NewAge Retailer")]
    pub merchant_name: Option<String>,
    /// Network provider
    #[schema(value_type= Option<String>,example = "VISA")]
    pub network: Option<common_enums::enums::CardNetwork>,
    /// Acquirer bin
    #[schema(value_type= Option<String>,example = "456789")]
    pub acquirer_bin: Option<String>,
    /// Acquirer ica provided by acquirer
    #[schema(value_type= Option<String>,example = "401288")]
    pub acquirer_ica: Option<String>,
    /// Fraud rate for the particular acquirer configuration
    #[schema(value_type= Option<f64>,example = 0.01)]
    pub acquirer_fraud_rate: Option<f64>,
    /// Acquirer country code
    #[schema(value_type= Option<String>,example = "US")]
    pub acquirer_country_code: Option<String>,
    /// Parent profile id to link the acquirer account with
    #[schema(value_type= String,example = "pro_ky0yNyOXXlA5hF8JzE5q")]
    pub profile_id: common_utils::id_type::ProfileId,
    /// Whether this configuration bucket is the default fallback for the profile.
    pub is_default: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct AcquirerBucketConfigResponse {
    /// The merchant id assigned by the acquirer
    #[schema(value_type= Option<String>,example = "M123456789")]
    pub acquirer_assigned_merchant_id: Option<String>,
    /// Merchant name
    #[schema(value_type= Option<String>,example = "NewAge Retailer")]
    pub merchant_name: Option<String>,
    /// Network provider
    #[schema(value_type= String,example = "VISA")]
    pub network: common_enums::enums::CardNetwork,
    /// Acquirer bin
    #[schema(value_type= Option<String>,example = "456789")]
    pub acquirer_bin: Option<String>,
    /// Acquirer ica provided by acquirer
    #[schema(value_type= Option<String>,example = "401288")]
    pub acquirer_ica: Option<String>,
    /// Fraud rate for the particular acquirer configuration
    #[schema(value_type= Option<f64>,example = 0.01)]
    pub acquirer_fraud_rate: Option<f64>,
    /// Acquirer country code
    #[schema(value_type= Option<String>,example = "US")]
    pub acquirer_country_code: Option<String>,
}

impl common_utils::events::ApiEventMetric for ProfileAcquirerCreate {}
impl common_utils::events::ApiEventMetric for ProfileAcquirerResponse {}
impl common_utils::events::ApiEventMetric for AcquirerBucketConfigResponse {}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ProfileAcquirerConfigsResponse {
    /// The default bucket for acquirer configurations
    #[schema(value_type= Option<String>,example = "pro_acq_LCRdERuylQvNQ4qh3QE0")]
    pub default_acquirer_config: Option<common_utils::id_type::ProfileAcquirerId>,
    /// Flattened map of acquirer configuration buckets
    pub configs: std::collections::HashMap<
        common_utils::id_type::ProfileAcquirerId,
        Vec<AcquirerBucketConfigResponse>,
    >,
}

impl
    From<(
        common_utils::id_type::ProfileAcquirerId,
        &common_utils::id_type::ProfileId,
        Option<&common_types::domain::AcquirerConfig>,
        bool,
    )> for ProfileAcquirerResponse
{
    fn from(
        (profile_acquirer_id, profile_id, acquirer_config, is_default): (
            common_utils::id_type::ProfileAcquirerId,
            &common_utils::id_type::ProfileId,
            Option<&common_types::domain::AcquirerConfig>,
            bool,
        ),
    ) -> Self {
        Self {
            profile_acquirer_id,
            profile_id: profile_id.clone(),
            acquirer_assigned_merchant_id: acquirer_config
                .and_then(|c| c.acquirer_assigned_merchant_id.clone()),
            merchant_name: acquirer_config.and_then(|c| c.merchant_name.clone()),
            network: acquirer_config.map(|c| c.network.clone()),
            acquirer_bin: acquirer_config.and_then(|c| c.acquirer_bin.clone()),
            acquirer_ica: acquirer_config.and_then(|c| c.acquirer_ica.clone()),
            acquirer_fraud_rate: acquirer_config.and_then(|c| c.acquirer_fraud_rate),
            acquirer_country_code: acquirer_config.and_then(|c| c.acquirer_country_code.clone()),
            is_default,
        }
    }
}

impl From<&common_types::domain::AcquirerConfig> for AcquirerBucketConfigResponse {
    fn from(acquirer_config: &common_types::domain::AcquirerConfig) -> Self {
        Self {
            acquirer_assigned_merchant_id: acquirer_config.acquirer_assigned_merchant_id.clone(),
            merchant_name: acquirer_config.merchant_name.clone(),
            network: acquirer_config.network.clone(),
            acquirer_bin: acquirer_config.acquirer_bin.clone(),
            acquirer_ica: acquirer_config.acquirer_ica.clone(),
            acquirer_fraud_rate: acquirer_config.acquirer_fraud_rate,
            acquirer_country_code: acquirer_config.acquirer_country_code.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileAcquirerUpdate {
    #[schema(value_type = Option<String>, example = "M987654321")]
    pub acquirer_assigned_merchant_id: Option<String>,
    #[schema(value_type = Option<String>, example = "Updated Retailer Name")]
    pub merchant_name: Option<String>,
    /// The card network this configuration entry targets — optional if updating just the default.
    #[schema(value_type = Option<String>, example = "MASTERCARD")]
    pub network: Option<common_enums::enums::CardNetwork>,
    #[schema(value_type = Option<String>, example = "987654")]
    pub acquirer_bin: Option<String>,
    #[schema(value_type = Option<String>, example = "501299")]
    pub acquirer_ica: Option<String>,
    #[schema(value_type = Option<f64>, example = "0.02")]
    pub acquirer_fraud_rate: Option<f64>,
    #[schema(value_type = Option<String>, example = "US")]
    pub acquirer_country_code: Option<String>,
    /// Whether this configuration bucket is the default fallback for the profile.
    pub is_default: Option<bool>,
}

impl common_utils::events::ApiEventMetric for ProfileAcquirerUpdate {}
