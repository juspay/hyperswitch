use common_enums::{AuthenticationType, CountryAlpha2};
use common_utils::{self};
use time::PrimitiveDateTime;

use crate::enums::Connector;

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct SampleDataRequest {
    pub record: Option<usize>,
    pub connector: Option<Vec<Connector>>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub end_time: Option<PrimitiveDateTime>,
    // The amount for each sample will be between min_amount and max_amount (in dollars)
    pub min_amount: Option<i64>,
    pub max_amount: Option<i64>,
    pub currency: Option<Vec<common_enums::Currency>>,
    pub auth_type: Option<Vec<AuthenticationType>>,
    pub business_country: Option<CountryAlpha2>,
    pub business_label: Option<String>,
    pub profile_id: Option<String>,
}
