use common_utils::{
    consts,
    link_utils::{
        EnabledPaymentMethod, GenericLinkStatus, GenericLinkUiConfig, PaymentMethodCollectStatus,
        PayoutLinkData, PayoutLinkStatus,
    },
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::{Duration, PrimitiveDateTime};

use crate::{enums as storage_enums, schema::generic_link};

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Serialize, Deserialize,
)]
#[diesel(table_name = generic_link, primary_key(link_id), check_for_backend(diesel::pg::Pg))]
pub struct GenericLink {
    pub link_id: String,
    pub primary_reference: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expiry: PrimitiveDateTime,
    pub link_data: serde_json::Value,
    pub link_status: GenericLinkStatus,
    pub link_type: storage_enums::GenericLinkType,
    pub url: Secret<String>,
    pub return_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenericLinkState {
    pub link_id: String,
    pub primary_reference: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expiry: PrimitiveDateTime,
    pub link_data: GenericLinkData,
    pub link_status: GenericLinkStatus,
    pub link_type: storage_enums::GenericLinkType,
    pub url: Secret<String>,
    pub return_url: Option<String>,
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
    pub merchant_id: common_utils::id_type::MerchantId,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_modified_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expiry: PrimitiveDateTime,
    pub link_data: serde_json::Value,
    pub link_status: GenericLinkStatus,
    pub link_type: storage_enums::GenericLinkType,
    pub url: Secret<String>,
    pub return_url: Option<String>,
}

impl Default for GenericLinkNew {
    fn default() -> Self {
        let now = common_utils::date_time::now();

        Self {
            link_id: String::default(),
            primary_reference: String::default(),
            merchant_id: common_utils::id_type::MerchantId::default(),
            created_at: Some(now),
            last_modified_at: Some(now),
            expiry: now + Duration::seconds(consts::DEFAULT_SESSION_EXPIRY),
            link_data: serde_json::Value::default(),
            link_status: GenericLinkStatus::default(),
            link_type: common_enums::GenericLinkType::default(),
            url: Secret::default(),
            return_url: Option::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GenericLinkData {
    PaymentMethodCollect(PaymentMethodCollectLinkData),
    PayoutLink(PayoutLinkData),
}

impl GenericLinkData {
    pub fn get_payment_method_collect_data(&self) -> Result<&PaymentMethodCollectLinkData, String> {
        match self {
            Self::PaymentMethodCollect(pm) => Ok(pm),
            _ => Err("Invalid link type for fetching payment method collect data".to_string()),
        }
    }
    pub fn get_payout_link_data(&self) -> Result<&PayoutLinkData, String> {
        match self {
            Self::PayoutLink(pl) => Ok(pl),
            _ => Err("Invalid link type for fetching payout link data".to_string()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentMethodCollectLink {
    pub link_id: String,
    pub primary_reference: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expiry: PrimitiveDateTime,
    pub link_data: PaymentMethodCollectLinkData,
    pub link_status: PaymentMethodCollectStatus,
    pub link_type: storage_enums::GenericLinkType,
    pub url: Secret<String>,
    pub return_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodCollectLinkData {
    pub pm_collect_link_id: String,
    pub customer_id: common_utils::id_type::CustomerId,
    pub link: Secret<String>,
    pub client_secret: Secret<String>,
    pub session_expiry: u32,
    #[serde(flatten)]
    pub ui_config: GenericLinkUiConfig,
    pub enabled_payment_methods: Option<Vec<EnabledPaymentMethod>>,
}

#[derive(Clone, Debug, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = generic_link)]
#[diesel(primary_key(link_id))]
pub struct PayoutLink {
    pub link_id: String,
    pub primary_reference: common_utils::id_type::PayoutId,
    pub merchant_id: common_utils::id_type::MerchantId,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expiry: PrimitiveDateTime,
    pub link_data: PayoutLinkData,
    pub link_status: PayoutLinkStatus,
    pub link_type: storage_enums::GenericLinkType,
    pub url: Secret<String>,
    pub return_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PayoutLinkUpdate {
    StatusUpdate { link_status: PayoutLinkStatus },
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = generic_link)]
pub struct GenericLinkUpdateInternal {
    pub link_status: Option<GenericLinkStatus>,
}

impl From<PayoutLinkUpdate> for GenericLinkUpdateInternal {
    fn from(generic_link_update: PayoutLinkUpdate) -> Self {
        match generic_link_update {
            PayoutLinkUpdate::StatusUpdate { link_status } => Self {
                link_status: Some(GenericLinkStatus::PayoutLink(link_status)),
            },
        }
    }
}
