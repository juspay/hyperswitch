use diesel::{prelude::Identifiable, AsChangeset, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::merchant_acquirer};

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = merchant_acquirer, primary_key(merchant_acquirer_id), check_for_backend(diesel::pg::Pg))]
pub struct MerchantAcquirer {
    pub merchant_acquirer_id: common_utils::id_type::MerchantAcquirerId,
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
#[diesel(table_name = merchant_acquirer)]
pub struct MerchantAcquirerNew {
    pub merchant_acquirer_id: common_utils::id_type::MerchantAcquirerId,
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

#[derive(
    Clone, Debug, AsChangeset, router_derive::DebugAsDisplay, serde::Deserialize, serde::Serialize,
)]
#[diesel(table_name = merchant_acquirer)]
pub struct MerchantAcquirerUpdate {
    pub merchant_acquirer_id: Option<common_utils::id_type::MerchantAcquirerId>,
    pub acquirer_assigned_merchant_id: Option<String>,
    pub merchant_name: Option<String>,
    pub mcc: Option<String>,
    pub merchant_country_code: Option<storage_enums::CountryAlpha2>,
    pub network: Option<storage_enums::CardNetwork>,
    pub acquirer_bin: Option<String>,
    pub acquirer_ica: Option<String>,
    pub acquirer_fraud_rate: Option<f64>,
    pub profile_id: Option<common_utils::id_type::ProfileId>,
}

impl MerchantAcquirerUpdate {
    pub fn apply_changeset(self, source: MerchantAcquirer) -> MerchantAcquirer {
        let Self {
            merchant_acquirer_id,
            acquirer_assigned_merchant_id,
            merchant_name,
            mcc,
            merchant_country_code,
            network,
            acquirer_bin,
            acquirer_ica,
            acquirer_fraud_rate,
            profile_id,
        } = self;

        MerchantAcquirer {
            merchant_acquirer_id: merchant_acquirer_id.unwrap_or(source.merchant_acquirer_id),
            acquirer_assigned_merchant_id: acquirer_assigned_merchant_id
                .unwrap_or(source.acquirer_assigned_merchant_id),
            merchant_name: merchant_name.unwrap_or(source.merchant_name),
            mcc: mcc.unwrap_or(source.mcc),
            merchant_country_code: merchant_country_code.unwrap_or(source.merchant_country_code),
            network: network.unwrap_or(source.network),
            acquirer_bin: acquirer_bin.unwrap_or(source.acquirer_bin),
            acquirer_ica: acquirer_ica.map_or(source.acquirer_ica, Some),
            acquirer_fraud_rate: acquirer_fraud_rate.unwrap_or(source.acquirer_fraud_rate),
            profile_id: profile_id.unwrap_or(source.profile_id),
            created_at: source.created_at,
            last_modified_at: common_utils::date_time::now(),
        }
    }
}
