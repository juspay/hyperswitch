//!
//! Serde-related.
//!

pub use serde::{de, ser, Deserialize, Serialize};

use crate::{PeekInterface, Secret, Strategy, StrongSecret, ZeroizableSecret};

/// Marker trait for secret types which can be [`Serialize`]-d by [`serde`].
///
/// When the `serde` feature of this crate is enabled and types are marked with
/// this trait, they receive a [`Serialize` impl] for `Secret<T>`.
/// (NOTE: all types which impl `DeserializeOwned` receive a [`Deserialize`]
/// impl)
///
/// This is done deliberately to prevent accidental exfiltration of secrets
/// via `serde` serialization.
///

#[cfg_attr(docsrs, cfg(feature = "serde"))]
pub trait SerializableSecret: Serialize {}
// #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
// pub trait NonSerializableSecret: Serialize {}

impl SerializableSecret for serde_json::Value {}
impl SerializableSecret for u8 {}
impl SerializableSecret for u16 {}

impl<'de, T, I> Deserialize<'de> for Secret<T, I>
where
    T: Clone + de::DeserializeOwned + Sized,
    I: Strategy<T>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::new)
    }
}

impl<T, I> Serialize for Secret<T, I>
where
    T: SerializableSecret + Serialize + Sized,
    I: Strategy<T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        self.peek().serialize(serializer)
    }
}

impl<'de, T, I> Deserialize<'de> for StrongSecret<T, I>
where
    T: Clone + de::DeserializeOwned + Sized + ZeroizableSecret,
    I: Strategy<T>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::new)
    }
}

impl<T, I> Serialize for StrongSecret<T, I>
where
    T: SerializableSecret + Serialize + ZeroizableSecret + Sized,
    I: Strategy<T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.peek().serialize(serializer)
    }
}
