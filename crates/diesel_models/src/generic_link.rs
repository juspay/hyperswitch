use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::{Duration, PrimitiveDateTime};

use crate::{enums as storage_enums, schema::generic_link};

#[derive(Clone, Debug, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = generic_link)]
#[diesel(primary_key(link_id))]
pub struct GenericLink {
    pub link_id: String,
    pub primary_reference: String,
    pub merchant_id: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expiry: PrimitiveDateTime,
    pub link_data: serde_json::Value,
    pub link_status: String,
    pub link_type: storage_enums::GenericLinkType,
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenericLinkS {
    pub link_id: String,
    pub primary_reference: String,
    pub merchant_id: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expiry: PrimitiveDateTime,
    pub link_data: GenericLinkData,
    pub link_status: storage_enums::GenericLinkStatus,
    pub link_type: storage_enums::GenericLinkType,
    pub url: String,
}

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Insertable,
    serde::Serialize,
    serde::Deserialize,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = generic_link)]
pub struct GenericLinkNew {
    pub link_id: String,
    pub primary_reference: String,
    pub merchant_id: String,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_modified_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub expiry: Option<PrimitiveDateTime>,
    pub link_data: serde_json::Value,
    pub link_status: String,
    pub link_type: storage_enums::GenericLinkType,
    pub url: String,
}

impl Default for GenericLinkNew {
    fn default() -> Self {
        let now = common_utils::date_time::now();

        Self {
            link_id: String::default(),
            primary_reference: String::default(),
            merchant_id: String::default(),
            created_at: Some(now),
            last_modified_at: Some(now),
            expiry: Some(now + Duration::minutes(15)),
            link_data: serde_json::Value::default(),
            link_status: common_enums::GenericLinkStatus::default().to_string(),
            link_type: common_enums::GenericLinkType::default(),
            url: String::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GenericLinkData {
    PaymentMethodCollect(PaymentMethodCollectLinkData),
}

impl GenericLinkData {
    pub fn get_payment_method_collect_data(&self) -> Result<&PaymentMethodCollectLinkData, String> {
        match self {
            Self::PaymentMethodCollect(pm) => Ok(pm),
            _ => Err("invalid variant for fetching payment_method_collect data".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodCollectLinkData {
    pub pm_collect_link_id: String,
    pub customer_id: String,
    pub sdk_host: String,
    pub link: String,
    pub client_secret: String,
}
