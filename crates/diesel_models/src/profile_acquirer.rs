use diesel::{prelude::Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::profile_acquirer};

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = profile_acquirer, primary_key(profile_acquirer_id), check_for_backend(diesel::pg::Pg))]
pub struct ProfileAcquirer {
    pub profile_acquirer_id: common_utils::id_type::ProfileAcquirerId,
    pub acquirer_assigned_merchant_id: String,
    pub merchant_name: String,
    pub mcc: String,
    pub merchant_country_code: storage_enums::CountryAlpha2,
    pub network: storage_enums::CardNetwork,
    pub acquirer_bin: String,
    pub acquirer_ica: Option<String>,
    pub acquirer_fraud_rate: f64,
    pub profile_id: common_utils::id_type::ProfileId,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Insertable,
    serde::Serialize,
    serde::Deserialize,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = profile_acquirer)]
pub struct ProfileAcquirerNew {
    pub profile_acquirer_id: common_utils::id_type::ProfileAcquirerId,
    pub acquirer_assigned_merchant_id: String,
    pub merchant_name: String,
    pub mcc: String,
    pub merchant_country_code: storage_enums::CountryAlpha2,
    pub network: storage_enums::CardNetwork,
    pub acquirer_bin: String,
    pub acquirer_ica: Option<String>,
    pub acquirer_fraud_rate: f64,
    pub profile_id: common_utils::id_type::ProfileId,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_modified_at: Option<PrimitiveDateTime>,
}
