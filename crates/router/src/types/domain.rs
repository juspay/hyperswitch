use std::{
    fmt::{self, Debug},
    marker, ops,
};
pub(crate) mod payments;
pub use payments::*;
mod transformers;
///
/// This is a wrapper that acts as a extension to existing types
/// <T, I>: Where I is the inner data type & T is the Extension used
///
pub struct StorageWrapper<I, T: StorageExt<I>> {
    marker: marker::PhantomData<T>,
    inner: I,
}

impl<I, T: StorageExt<I>> StorageWrapper<I, T> {
    ///
    /// convert item of type I into StorageWrapper<I, T>
    ///
    pub fn new(item: I) -> Self {
        Self {
            marker: marker::PhantomData,
            inner: item,
        }
    }

    ///
    /// consumes self to return the internal value
    ///
    pub fn into_inner(self) -> I {
        self.inner
    }

    pub fn into(self) -> I {
        self.into_inner()
    }
}

impl<I, T: StorageExt<I>> From<I> for StorageWrapper<I, T> {
    fn from(item: I) -> Self {
        Self::new(item)
    }
}

impl<I: Debug, T: StorageExt<I>> fmt::Debug for StorageWrapper<I, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<I: fmt::Display, T: StorageExt<I>> fmt::Display for StorageWrapper<I, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<I, T: StorageExt<I>> ops::Deref for StorageWrapper<I, T> {
    type Target = I;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<I, T: StorageExt<I>> ops::DerefMut for StorageWrapper<I, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<I: PartialEq, T: StorageExt<I>> PartialEq for StorageWrapper<I, T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<I: Clone, T: StorageExt<I>> Clone for StorageWrapper<I, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: marker::PhantomData,
        }
    }
}

impl<I: serde::Serialize, T: StorageExt<I>> serde::Serialize for StorageWrapper<I, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        I::serialize(&self.inner, serializer)
    }
}

impl<'de, I: serde::Deserialize<'de>, T: StorageExt<I>> serde::Deserialize<'de>
    for StorageWrapper<I, T>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            inner: I::deserialize(deserializer)?,
            marker: marker::PhantomData,
        })
    }
}

pub trait StorageExt<I> {}

pub struct PaymentIdCover;
impl StorageExt<String> for PaymentIdCover {}

pub struct AttemptIdCover;
impl StorageExt<String> for AttemptIdCover {}

pub type PaymentId = StorageWrapper<String, PaymentIdCover>;
pub type AttemptId = StorageWrapper<String, AttemptIdCover>;
