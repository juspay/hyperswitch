use common_utils::custom_serde;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, schema::merchant_key_store};

#[derive(
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    Identifiable,
    Queryable,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchant_key_store)]
#[diesel(primary_key(merchant_id))]
pub struct MerchantKeyStore {
    pub merchant_id: String,
    pub key: Encryption,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
}

#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, Insertable, router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchant_key_store)]
pub struct MerchantKeyStoreNew {
    pub merchant_id: String,
    pub key: Encryption,
    pub created_at: PrimitiveDateTime,
}

#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, AsChangeset, router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchant_key_store)]
pub struct MerchantKeyStoreUpdateInternal {
    pub merchant_id: String,
    pub key: Encryption,
}
