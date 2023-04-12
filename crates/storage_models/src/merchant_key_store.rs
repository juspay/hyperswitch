use diesel::{AsChangeset, Identifiable, Insertable, Queryable};

use crate::{encryption::Encryption, schema::merchantkeystore};

#[derive(
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    Identifiable,
    Queryable,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchantkeystore)]
pub struct MerchantKeyStore {
    pub id: i32,
    pub merchant_id: String,
    pub key: Encryption,
}

#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, Insertable, router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchantkeystore)]
pub struct MerchantKeyStoreNew {
    pub merchant_id: String,
    pub key: Encryption,
}

#[derive(
    Clone, Debug, serde::Serialize, serde::Deserialize, AsChangeset, router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchantkeystore)]
pub struct MerchantKeyStoreUpdateInternal {
    pub merchant_id: String,
    pub key: Encryption,
}
