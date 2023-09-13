pub mod errors;
pub mod mandates;
pub mod payments;

// TODO: This decision about using KV mode or not,
// should be taken at a top level rather than pushing it down to individual functions via an enum.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MerchantStorageScheme {
    #[default]
    PostgresOnly,
    RedisKv,
}

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
)]
pub enum RemoteStorageObject<T: ForeignIDRef> {
    ForeignID(String),
    Object(T)
}

pub trait ForeignIDRef {
    fn foreign_id(&self) -> String;
}

impl<T: ForeignIDRef> RemoteStorageObject<T> {
    pub fn get_id(&self) -> String {
        match self {
            RemoteStorageObject::ForeignID(id) => id.clone(),
            RemoteStorageObject::Object(i) => i.foreign_id(),
        }
    }
}