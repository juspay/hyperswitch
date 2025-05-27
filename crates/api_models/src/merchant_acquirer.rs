use common_utils::custom_serde;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MerchantAcquirerCreate {
    pub acquirer_assigned_merchant_id: String,
    pub merchant_name: String,
    pub mcc: String,
    pub merchant_country_code: String,
    pub network: common_enums::enums::CardNetwork,
    pub acquirer_bin: String,
    pub acquirer_ica: Option<String>,
    pub acquirer_fraud_rate: f64,
    pub profile_id: common_utils::id_type::ProfileId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MerchantAcquirerResponse {
    pub merchant_acquirer_id: common_utils::id_type::MerchantAcquirerId,
    pub acquirer_assigned_merchant_id: String,
    pub merchant_name: String,
    pub mcc: String,
    pub merchant_country_code: String,
    pub network: common_enums::enums::CardNetwork,
    pub acquirer_bin: String,
    pub acquirer_ica: Option<String>,
    pub acquirer_fraud_rate: f64,
    pub profile_id: common_utils::id_type::ProfileId,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
}

impl common_utils::events::ApiEventMetric for MerchantAcquirerCreate {}
impl common_utils::events::ApiEventMetric for MerchantAcquirerResponse {}
