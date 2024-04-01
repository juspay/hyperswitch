pub mod errors;
pub mod mandates;
pub mod payments;
#[cfg(feature = "payouts")]
pub mod payouts;

#[cfg(not(feature = "payouts"))]
pub trait PayoutAttemptInterface {}

#[cfg(not(feature = "payouts"))]
pub trait PayoutsInterface {}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub enum RemoteStorageObject<T: ForeignIDRef> {
    ForeignID(String),
    Object(T),
}

impl<T: ForeignIDRef> From<T> for RemoteStorageObject<T> {
    fn from(value: T) -> Self {
        Self::Object(value)
    }
}

pub trait ForeignIDRef {
    fn foreign_id(&self) -> String;
}

impl<T: ForeignIDRef> RemoteStorageObject<T> {
    pub fn get_id(&self) -> String {
        match self {
            Self::ForeignID(id) => id.clone(),
            Self::Object(i) => i.foreign_id(),
        }
    }
}
