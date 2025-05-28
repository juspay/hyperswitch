use common_utils::custom_serde;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MerchantAcquirerCreate {
    #[schema(value_type= String,example = "dugduwhqdiuhqw_123")]
    pub acquirer_assigned_merchant_id: String,
    #[schema(value_type= String,example = "NewAge Retailer")]
    pub merchant_name: String,
    #[schema(value_type= String,example = "5812")]
    pub mcc: String,
    #[schema(value_type= String,example = "US")]
    pub merchant_country_code: String,
    #[schema(value_type= String,example = "VISA")]
    pub network: common_enums::enums::CardNetwork,
    #[schema(value_type= String,example = "123456")]
    pub acquirer_bin: String,
    #[schema(value_type= String,example = "123456")]
    pub acquirer_ica: Option<String>,
    #[schema(value_type= f64,example = "0.01")]
    pub acquirer_fraud_rate: f64,
    #[schema(value_type= String,example = "pro_ky0yNyOXXlA5hF8JzE5q")]
    pub profile_id: common_utils::id_type::ProfileId,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MerchantAcquirerResponse {
    #[schema(value_type= String,example = "mer_acq_LCRdERuylQvNQ4qh3QE0")]
    pub merchant_acquirer_id: common_utils::id_type::MerchantAcquirerId,
    #[schema(value_type= String,example = "dugduwhqdiuhqw_123")]
    pub acquirer_assigned_merchant_id: String,
    #[schema(value_type= String,example = "NewAge Retailer")]
    pub merchant_name: String,
    #[schema(value_type= String,example = "5812")]
    pub mcc: String,
    #[schema(value_type= String,example = "US")]
    pub merchant_country_code: String,
    #[schema(value_type= String,example = "VISA")]
    pub network: common_enums::enums::CardNetwork,
    #[schema(value_type= String,example = "123456")]
    pub acquirer_bin: String,
    #[schema(value_type= String,example = "123456")]
    pub acquirer_ica: Option<String>,
    #[schema(value_type= f64,example = "0.01")]
    pub acquirer_fraud_rate: f64,
    #[schema(value_type= String,example = "pro_ky0yNyOXXlA5hF8JzE5q")]
    pub profile_id: common_utils::id_type::ProfileId,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
}

impl common_utils::events::ApiEventMetric for MerchantAcquirerCreate {}
impl common_utils::events::ApiEventMetric for MerchantAcquirerResponse {}
