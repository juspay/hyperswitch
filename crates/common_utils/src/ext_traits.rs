//!
//! This module holds traits for extending functionalities for existing datatypes
//! & inbuilt datatypes.
//!

use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface, Secret, Strategy};
use quick_xml::de;
use serde::{Deserialize, Serialize};

use crate::{
    crypto,
    errors::{self, CustomResult},
    fp_utils::when,
};

///
/// Encode interface
/// An interface for performing type conversions and serialization
///
pub trait Encode<'e>
where
    Self: 'e + std::fmt::Debug,
{
    // If needed get type information/custom error implementation.
    ///
    /// Converting `Self` into an intermediate representation `<P>`
    /// and then performing encoding operation using the `Serialize` trait from `serde`
    /// Specifically to convert into json, by using `serde_json`
    ///
    fn convert_and_encode<P>(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize;

    ///
    /// Converting `Self` into an intermediate representation `<P>`
    /// and then performing encoding operation using the `Serialize` trait from `serde`
    /// Specifically, to convert into urlencoded, by using `serde_urlencoded`
    ///
    fn convert_and_url_encode<P>(&'e self) -> CustomResult<String, errors::ParsingError>
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
    /// Functionality, for specifically encoding `Self` into `String`
    /// after serialization by using `serde::Serialize`
    /// specifically, to convert into XML `String`.
    ///
    fn encode_to_string_of_xml(&'e self) -> CustomResult<String, errors::ParsingError>
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

impl<'e, A> Encode<'e> for A
where
    Self: 'e + std::fmt::Debug,
{
    fn convert_and_encode<P>(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize,
    {
        serde_json::to_string(
            &P::try_from(self).change_context(errors::ParsingError::UnknownError)?,
        )
        .change_context(errors::ParsingError::EncodeError("string"))
        .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    fn convert_and_url_encode<P>(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize,
    {
        serde_urlencoded::to_string(
            &P::try_from(self).change_context(errors::ParsingError::UnknownError)?,
        )
        .change_context(errors::ParsingError::EncodeError("url-encoded"))
        .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    // Check without two functions can we combine this
    fn url_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_urlencoded::to_string(self)
            .change_context(errors::ParsingError::EncodeError("url-encoded"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    fn encode_to_string_of_json(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_string(self)
            .change_context(errors::ParsingError::EncodeError("json"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    fn encode_to_string_of_xml(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize,
    {
        quick_xml::se::to_string(self)
            .change_context(errors::ParsingError::EncodeError("xml"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    fn encode_to_value(&'e self) -> CustomResult<serde_json::Value, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_value(self)
            .change_context(errors::ParsingError::EncodeError("json-value"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a value"))
    }

    fn encode_to_vec(&'e self) -> CustomResult<Vec<u8>, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_vec(self)
            .change_context(errors::ParsingError::EncodeError("byte-vec"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a value"))
    }
}

///
/// Extending functionalities of `bytes::Bytes`
///
pub trait BytesExt {
    ///
    /// Convert `bytes::Bytes` into type `<T>` using `serde::Deserialize`
    ///
    fn parse_struct<'de, T>(
        &'de self,
        type_name: &'static str,
    ) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>;
}

impl BytesExt for bytes::Bytes {
    fn parse_struct<'de, T>(
        &'de self,
        type_name: &'static str,
    ) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>,
    {
        use bytes::Buf;

        serde_json::from_slice::<T>(self.chunk())
            .change_context(errors::ParsingError::StructParseFailure(type_name))
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
    fn parse_struct<'de, T>(
        &'de self,
        type_name: &'static str,
    ) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>;
}

impl ByteSliceExt for [u8] {
    #[track_caller]
    fn parse_struct<'de, T>(
        &'de self,
        type_name: &'static str,
    ) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>,
    {
        serde_json::from_slice(self)
            .change_context(errors::ParsingError::StructParseFailure(type_name))
            .attach_printable_lazy(|| format!("Unable to parse {type_name} from &[u8] {:?}", &self))
    }
}

///
/// Extending functionalities of `serde_json::Value` for performing parsing
///
pub trait ValueExt {
    ///
    /// Convert `serde_json::Value` into type `<T>` by using `serde::Deserialize`
    ///
    fn parse_value<T>(self, type_name: &'static str) -> CustomResult<T, errors::ParsingError>
    where
        T: serde::de::DeserializeOwned;
}

impl ValueExt for serde_json::Value {
    fn parse_value<T>(self, type_name: &'static str) -> CustomResult<T, errors::ParsingError>
    where
        T: serde::de::DeserializeOwned,
    {
        let debug = format!(
            "Unable to parse {type_name} from serde_json::Value: {:?}",
            &self
        );
        serde_json::from_value::<T>(self)
            .change_context(errors::ParsingError::StructParseFailure(type_name))
            .attach_printable_lazy(|| debug)
    }
}

impl<MaskingStrategy> ValueExt for Secret<serde_json::Value, MaskingStrategy>
where
    MaskingStrategy: Strategy<serde_json::Value>,
{
    fn parse_value<T>(self, type_name: &'static str) -> CustomResult<T, errors::ParsingError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.expose().parse_value(type_name)
    }
}

impl<E: ValueExt + Clone> ValueExt for crypto::Encryptable<E> {
    fn parse_value<T>(self, type_name: &'static str) -> CustomResult<T, errors::ParsingError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.into_inner().parse_value(type_name)
    }
}

///
/// Extending functionalities of `String` for performing parsing
///
pub trait StringExt<T> {
    ///
    /// Convert `String` into type `<T>` (which being an `enum`)
    ///
    fn parse_enum(self, enum_name: &'static str) -> CustomResult<T, errors::ParsingError>
    where
        T: std::str::FromStr,
        // Requirement for converting the `Err` variant of `FromStr` to `Report<Err>`
        <T as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static;

    ///
    /// Convert `serde_json::Value` into type `<T>` by using `serde::Deserialize`
    ///
    fn parse_struct<'de>(
        &'de self,
        type_name: &'static str,
    ) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>;
}

impl<T> StringExt<T> for String {
    fn parse_enum(self, enum_name: &'static str) -> CustomResult<T, errors::ParsingError>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        T::from_str(&self)
            .change_context(errors::ParsingError::EnumParseFailure(enum_name))
            .attach_printable_lazy(|| format!("Invalid enum variant {self:?} for enum {enum_name}"))
    }

    fn parse_struct<'de>(
        &'de self,
        type_name: &'static str,
    ) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>,
    {
        serde_json::from_str::<T>(self)
            .change_context(errors::ParsingError::StructParseFailure(type_name))
            .attach_printable_lazy(|| {
                format!("Unable to parse {type_name} from string {:?}", &self)
            })
    }
}

///
/// Extending functionalities of Wrapper types for idiomatic
///
#[cfg(feature = "async_ext")]
#[cfg_attr(feature = "async_ext", async_trait::async_trait)]
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

#[cfg(feature = "async_ext")]
#[cfg_attr(feature = "async_ext", async_trait::async_trait)]
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

#[cfg(feature = "async_ext")]
#[cfg_attr(feature = "async_ext", async_trait::async_trait)]
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

impl ConfigExt for u32 {
    fn is_empty_after_trim(&self) -> bool {
        false
    }
}

impl ConfigExt for String {
    fn is_empty_after_trim(&self) -> bool {
        self.trim().is_empty()
    }
}

impl<T, U> ConfigExt for Secret<T, U>
where
    T: ConfigExt + Default + PartialEq<T>,
    U: Strategy<T>,
{
    fn is_default(&self) -> bool
    where
        T: Default + PartialEq<T>,
    {
        *self.peek() == T::default()
    }

    fn is_empty_after_trim(&self) -> bool {
        self.peek().is_empty_after_trim()
    }

    fn is_default_or_empty(&self) -> bool
    where
        T: Default + PartialEq<T>,
    {
        self.peek().is_default() || self.peek().is_empty_after_trim()
    }
}

/// Extension trait for deserializing XML strings using `quick-xml` crate
pub trait XmlExt {
    ///
    /// Deserialize an XML string into the specified type `<T>`.
    ///
    fn parse_xml<T>(self) -> Result<T, quick_xml::de::DeError>
    where
        T: serde::de::DeserializeOwned;
}

impl XmlExt for &str {
    fn parse_xml<T>(self) -> Result<T, quick_xml::de::DeError>
    where
        T: serde::de::DeserializeOwned,
    {
        de::from_str(self)
    }
}

/// Extension trait for Option to validate missing fields
pub trait OptionExt<T> {
    /// check if the current option is Some
    fn check_value_present(
        &self,
        field_name: &'static str,
    ) -> CustomResult<(), errors::ValidationError>;

    /// Throw missing required field error when the value is None
    fn get_required_value(
        self,
        field_name: &'static str,
    ) -> CustomResult<T, errors::ValidationError>;

    /// Try parsing the option as Enum
    fn parse_enum<E>(self, enum_name: &'static str) -> CustomResult<E, errors::ParsingError>
    where
        T: AsRef<str>,
        E: std::str::FromStr,
        // Requirement for converting the `Err` variant of `FromStr` to `Report<Err>`
        <E as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static;

    /// Try parsing the option as Type
    fn parse_value<U>(self, type_name: &'static str) -> CustomResult<U, errors::ParsingError>
    where
        T: ValueExt,
        U: serde::de::DeserializeOwned;

    /// update option value
    fn update_value(&mut self, value: Option<T>);
}

impl<T> OptionExt<T> for Option<T>
where
    T: std::fmt::Debug,
{
    #[track_caller]
    fn check_value_present(
        &self,
        field_name: &'static str,
    ) -> CustomResult<(), errors::ValidationError> {
        when(self.is_none(), || {
            Err(errors::ValidationError::MissingRequiredField {
                field_name: field_name.to_string(),
            })
            .attach_printable(format!("Missing required field {field_name} in {self:?}"))
        })
    }

    // This will allow the error message that was generated in this function to point to the call site
    #[track_caller]
    fn get_required_value(
        self,
        field_name: &'static str,
    ) -> CustomResult<T, errors::ValidationError> {
        match self {
            Some(v) => Ok(v),
            None => Err(errors::ValidationError::MissingRequiredField {
                field_name: field_name.to_string(),
            })
            .attach_printable(format!("Missing required field {field_name} in {self:?}")),
        }
    }

    #[track_caller]
    fn parse_enum<E>(self, enum_name: &'static str) -> CustomResult<E, errors::ParsingError>
    where
        T: AsRef<str>,
        E: std::str::FromStr,
        <E as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        let value = self
            .get_required_value(enum_name)
            .change_context(errors::ParsingError::UnknownError)?;

        E::from_str(value.as_ref())
            .change_context(errors::ParsingError::UnknownError)
            .attach_printable_lazy(|| format!("Invalid {{ {enum_name}: {value:?} }} "))
    }

    #[track_caller]
    fn parse_value<U>(self, type_name: &'static str) -> CustomResult<U, errors::ParsingError>
    where
        T: ValueExt,
        U: serde::de::DeserializeOwned,
    {
        let value = self
            .get_required_value(type_name)
            .change_context(errors::ParsingError::UnknownError)?;
        value.parse_value(type_name)
    }

    fn update_value(&mut self, value: Self) {
        if let Some(a) = value {
            *self = Some(a)
        }
    }
}
