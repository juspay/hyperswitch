//!
//! This module holds traits for extending functionalities for existing datatypes
//! & inbuilt datatypes.
//!

use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, Secret, Strategy};
use serde::{Deserialize, Serialize};

use crate::errors::{self, CustomResult};

///
/// Encode interface
/// An interface for performing type conversions and serialization
///
pub trait Encode<'e, P>
where
    Self: 'e + std::fmt::Debug,
{
    // If needed get type information/custom error implementation.
    ///
    /// Converting `Self` into an intermediate representation `<P>`
    /// and then performing encoding operation using the `Serialize` trait from `serde`
    /// Specifically to convert into json, by using `serde_json`
    ///
    fn convert_and_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize;

    ///
    /// Converting `Self` into an intermediate representation `<P>`
    /// and then performing encoding operation using the `Serialize` trait from `serde`
    /// Specifically, to convert into urlencoded, by using `serde_urlencoded`
    ///
    fn convert_and_url_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize;

    ///
    /// Functionality, for specifically encoding `Self` into `String`
    /// after serialization by using `serde::Serialize`
    ///
    fn url_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize;

    ///
    /// Functionality, for specifically encoding `Self` into `String`
    /// after serialization by using `serde::Serialize`
    /// specifically, to convert into JSON `String`.
    ///
    fn encode_to_string_of_json(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize;

    ///
    /// Functionality, for specifically encoding `Self` into `serde_json::Value`
    /// after serialization by using `serde::Serialize`
    ///
    fn encode_to_value(&'e self) -> CustomResult<serde_json::Value, errors::ParsingError>
    where
        Self: Serialize;

    ///
    /// Functionality, for specifically encoding `Self` into `Vec<u8>`
    /// after serialization by using `serde::Serialize`
    ///
    fn encode_to_vec(&'e self) -> CustomResult<Vec<u8>, errors::ParsingError>
    where
        Self: Serialize;
}

impl<'e, P, A> Encode<'e, P> for A
where
    Self: 'e + std::fmt::Debug,
{
    fn convert_and_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize,
    {
        serde_json::to_string(&P::try_from(self).change_context(errors::ParsingError)?)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    fn convert_and_url_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize,
    {
        serde_urlencoded::to_string(&P::try_from(self).change_context(errors::ParsingError)?)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    // Check without two functions can we combine this
    fn url_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_urlencoded::to_string(self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    fn encode_to_string_of_json(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_string(self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    fn encode_to_value(&'e self) -> CustomResult<serde_json::Value, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_value(self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a value"))
    }

    fn encode_to_vec(&'e self) -> CustomResult<Vec<u8>, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_vec(self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a value"))
    }
}

///
/// Extending functionalities of `bytes::Bytes`
///
pub trait BytesExt<T> {
    ///
    /// Convert `bytes::Bytes` into type `<T>` using `serde::Deserialize`
    ///
    fn parse_struct<'de>(&'de self, type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>;
}

impl<T> BytesExt<T> for bytes::Bytes {
    fn parse_struct<'de>(&'de self, _type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>,
    {
        use bytes::Buf;

        serde_json::from_slice::<T>(self.chunk())
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| {
                let variable_type = std::any::type_name::<T>();
                format!("Unable to parse {variable_type} from bytes {self:?}")
            })
    }
}

///
/// Extending functionalities of `[u8]` for performing parsing
///
pub trait ByteSliceExt {
    ///
    /// Convert `[u8]` into type `<T>` by using `serde::Deserialize`
    ///
    fn parse_struct<'de, T>(&'de self, type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>;
}

impl ByteSliceExt for [u8] {
    fn parse_struct<'de, T>(&'de self, type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>,
    {
        serde_json::from_slice(self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to parse {type_name} from &[u8]"))
    }
}

///
/// Extending functionalities of `serde_json::Value` for performing parsing
///
pub trait ValueExt<T> {
    ///
    /// Convert `serde_json::Value` into type `<T>` by using `serde::Deserialize`
    ///
    fn parse_value(self, type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: serde::de::DeserializeOwned;
}

impl<T> ValueExt<T> for serde_json::Value {
    fn parse_value(self, type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: serde::de::DeserializeOwned,
    {
        let debug = format!(
            "Unable to parse {type_name} from serde_json::Value: {:?}",
            &self
        );
        serde_json::from_value::<T>(self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| debug)
    }
}

impl<T, MaskingStrategy> ValueExt<T> for Secret<serde_json::Value, MaskingStrategy>
where
    MaskingStrategy: Strategy<serde_json::Value>,
{
    fn parse_value(self, type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.expose().parse_value(type_name)
    }
}

///
/// Extending functionalities of `String` for performing parsing
///
pub trait StringExt<T> {
    ///
    /// Convert `String` into type `<T>` (which being an `enum`)
    ///
    fn parse_enum(self, enum_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: std::str::FromStr,
        // Requirement for converting the `Err` variant of `FromStr` to `Report<Err>`
        <T as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static;

    ///
    /// Convert `serde_json::Value` into type `<T>` by using `serde::Deserialize`
    ///
    fn parse_struct<'de>(&'de self, type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>;
}

impl<T> StringExt<T> for String {
    fn parse_enum(self, enum_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        T::from_str(&self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Invalid enum variant {self:?} for enum {enum_name}"))
    }

    fn parse_struct<'de>(&'de self, type_name: &str) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>,
    {
        serde_json::from_str::<T>(self)
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable_lazy(|| format!("Unable to parse {type_name} from string"))
    }
}

///
/// Extending functionalities of Wrapper types for idiomatic
///
#[async_trait::async_trait]
pub trait AsyncExt<A, B> {
    /// Output type of the map function
    type WrappedSelf<T>;
    ///
    /// Extending map by allowing functions which are async
    ///
    async fn async_map<F, Fut>(self, func: F) -> Self::WrappedSelf<B>
    where
        F: FnOnce(A) -> Fut + Send,
        Fut: futures::Future<Output = B> + Send;

    ///
    /// Extending the `and_then` by allowing functions which are async
    ///
    async fn async_and_then<F, Fut>(self, func: F) -> Self::WrappedSelf<B>
    where
        F: FnOnce(A) -> Fut + Send,
        Fut: futures::Future<Output = Self::WrappedSelf<B>> + Send;
}

#[async_trait::async_trait]
impl<A: Send, B, E: Send> AsyncExt<A, B> for Result<A, E> {
    type WrappedSelf<T> = Result<T, E>;
    async fn async_and_then<F, Fut>(self, func: F) -> Self::WrappedSelf<B>
    where
        F: FnOnce(A) -> Fut + Send,
        Fut: futures::Future<Output = Self::WrappedSelf<B>> + Send,
    {
        match self {
            Ok(a) => func(a).await,
            Err(err) => Err(err),
        }
    }

    async fn async_map<F, Fut>(self, func: F) -> Self::WrappedSelf<B>
    where
        F: FnOnce(A) -> Fut + Send,
        Fut: futures::Future<Output = B> + Send,
    {
        match self {
            Ok(a) => Ok(func(a).await),
            Err(err) => Err(err),
        }
    }
}

#[async_trait::async_trait]
impl<A: Send, B> AsyncExt<A, B> for Option<A> {
    type WrappedSelf<T> = Option<T>;
    async fn async_and_then<F, Fut>(self, func: F) -> Self::WrappedSelf<B>
    where
        F: FnOnce(A) -> Fut + Send,
        Fut: futures::Future<Output = Self::WrappedSelf<B>> + Send,
    {
        match self {
            Some(a) => func(a).await,
            None => None,
        }
    }

    async fn async_map<F, Fut>(self, func: F) -> Self::WrappedSelf<B>
    where
        F: FnOnce(A) -> Fut + Send,
        Fut: futures::Future<Output = B> + Send,
    {
        match self {
            Some(a) => Some(func(a).await),
            None => None,
        }
    }
}

/// Extension trait for validating application configuration. This trait provides utilities to
/// check whether the value is either the default value or is empty.
pub trait ConfigExt {
    /// Returns whether the value of `self` is the default value for `Self`.
    fn is_default(&self) -> bool
    where
        Self: Default + PartialEq<Self>,
    {
        *self == Self::default()
    }

    /// Returns whether the value of `self` is empty after trimming whitespace on both left and
    /// right ends.
    fn is_empty_after_trim(&self) -> bool;

    /// Returns whether the value of `self` is the default value for `Self` or empty after trimming
    /// whitespace on both left and right ends.
    fn is_default_or_empty(&self) -> bool
    where
        Self: Default + PartialEq<Self>,
    {
        self.is_default() || self.is_empty_after_trim()
    }
}

impl ConfigExt for String {
    fn is_empty_after_trim(&self) -> bool {
        self.trim().is_empty()
    }
}
