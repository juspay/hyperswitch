use common_utils::{custom_serde, encryption::Encryption};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::schema::merchant_key_store;

#[derive(
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    Identifiable,
    Queryable,
    Selectable,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchant_key_store, primary_key(merchant_id), check_for_backend(diesel::pg::Pg))]
pub struct MerchantKeyStore {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub key: Encryption,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
}

#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, Insertable, router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchant_key_store)]
pub struct MerchantKeyStoreNew {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub key: Encryption,
    pub created_at: PrimitiveDateTime,
}

#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, AsChangeset, router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchant_key_store)]
pub struct MerchantKeyStoreUpdateInternal {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub key: Encryption,
}
