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
    #[schema(value_type= f64,example = 0.01)]
    pub acquirer_fraud_rate: f64,
    /// Parent profile id to link the acquirer account with
    #[schema(value_type= String,example = "pro_ky0yNyOXXlA5hF8JzE5q")]
    pub profile_id: common_utils::id_type::ProfileId,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct ProfileAcquirerResponse {
    /// The unique identifier of the profile acquirer
    #[schema(value_type= String,example = "pro_acq_LCRdERuylQvNQ4qh3QE0")]
    pub profile_acquirer_id: common_utils::id_type::ProfileAcquirerId,
    /// The merchant id assigned by the acquirer
    #[schema(value_type= String,example = "M123456789")]
    pub acquirer_assigned_merchant_id: String,
    /// Merchant name
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
    #[schema(value_type= f64,example = 0.01)]
    pub acquirer_fraud_rate: f64,
    /// Parent profile id to link the acquirer account with
    #[schema(value_type= String,example = "pro_ky0yNyOXXlA5hF8JzE5q")]
    pub profile_id: common_utils::id_type::ProfileId,
}

impl common_utils::events::ApiEventMetric for ProfileAcquirerCreate {}
impl common_utils::events::ApiEventMetric for ProfileAcquirerResponse {}

impl
    From<(
        common_utils::id_type::ProfileAcquirerId,
        &common_utils::id_type::ProfileId,
        &common_types::domain::AcquirerConfig,
    )> for ProfileAcquirerResponse
{
    fn from(
        (profile_acquirer_id, profile_id, acquirer_config): (
            common_utils::id_type::ProfileAcquirerId,
            &common_utils::id_type::ProfileId,
            &common_types::domain::AcquirerConfig,
        ),
    ) -> Self {
        Self {
            profile_acquirer_id,
            profile_id: profile_id.clone(),
            acquirer_assigned_merchant_id: acquirer_config.acquirer_assigned_merchant_id.clone(),
            merchant_name: acquirer_config.merchant_name.clone(),
            network: acquirer_config.network.clone(),
            acquirer_bin: acquirer_config.acquirer_bin.clone(),
            acquirer_ica: acquirer_config.acquirer_ica.clone(),
            acquirer_fraud_rate: acquirer_config.acquirer_fraud_rate,
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
    #[schema(value_type = Option<String>, example = "MASTERCARD")]
    pub network: Option<common_enums::enums::CardNetwork>,
    #[schema(value_type = Option<String>, example = "987654")]
    pub acquirer_bin: Option<String>,
    #[schema(value_type = Option<String>, example = "501299")]
    pub acquirer_ica: Option<String>,
    #[schema(value_type = Option<f64>, example = "0.02")]
    pub acquirer_fraud_rate: Option<f64>,
}

impl common_utils::events::ApiEventMetric for ProfileAcquirerUpdate {}
