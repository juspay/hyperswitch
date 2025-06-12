use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::enums;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProfileAcquirerCreate {
    /// The merchant id assigned by the acquirer
    #[schema(value_type= String,example = "M123456789")]
    pub acquirer_assigned_merchant_id: String,
    /// merchant name
    #[schema(value_type= String,example = "NewAge Retailer")]
    pub merchant_name: String,
    /// Merchant country code assigned by acquirer
    #[schema(value_type= String,example = "US")]
    pub merchant_country_code: enums::CountryAlpha2,
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
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
    /// Merchant country code assigned by acquirer
    #[schema(value_type= String,example = "US")]
    pub merchant_country_code: enums::CountryAlpha2,
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
