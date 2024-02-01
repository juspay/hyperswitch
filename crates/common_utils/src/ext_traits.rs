//!
//! This module holds traits for extending functionalities for existing datatypes
//! & inbuilt datatypes.
//!

use error_stack::{IntoReport, ResultExt};
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

impl<'e, P, A> Encode<'e, P> for A
where
    Self: 'e + std::fmt::Debug,
{
        /// This method converts the input into a type P, serializes it into a JSON string, and then applies custom error handling and contextual information before returning the result.
    fn convert_and_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize,
    {
        serde_json::to_string(
            &P::try_from(self).change_context(errors::ParsingError::UnknownError)?,
        )
        .into_report()
        .change_context(errors::ParsingError::EncodeError("string"))
        .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

        /// This method takes a reference to a value and converts it to a URL-encoded string. It uses the `TryFrom` and `Serialize` traits to perform the conversion, and returns a `CustomResult` with the URL-encoded string or a `ParsingError` if an error occurs during the conversion process.
    fn convert_and_url_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        P: TryFrom<&'e Self> + Serialize,
        Result<P, <P as TryFrom<&'e Self>>::Error>: ResultExt,
        <Result<P, <P as TryFrom<&'e Self>>::Error> as ResultExt>::Ok: Serialize,
    {
        serde_urlencoded::to_string(
            &P::try_from(self).change_context(errors::ParsingError::UnknownError)?,
        )
        .into_report()
        .change_context(errors::ParsingError::EncodeError("url-encoded"))
        .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

    // Check without two functions can we combine this
    fn url_encode(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_urlencoded::to_string(self)
            .into_report()
            .change_context(errors::ParsingError::EncodeError("url-encoded"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

        /// Encodes the struct to a JSON string and returns it as a result. If successful, returns the encoded JSON string, otherwise returns a ParsingError.
    fn encode_to_string_of_json(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_string(self)
            .into_report()
            .change_context(errors::ParsingError::EncodeError("json"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

        /// Encodes the current struct into an XML string representation.
    /// Returns a Result containing the XML string if successful, or a ParsingError if encoding fails.
    fn encode_to_string_of_xml(&'e self) -> CustomResult<String, errors::ParsingError>
    where
        Self: Serialize,
    {
        quick_xml::se::to_string(self)
            .into_report()
            .change_context(errors::ParsingError::EncodeError("xml"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a request"))
    }

        /// Encodes the current object into a serde_json::Value and returns a CustomResult containing the encoded value or a ParsingError.
    fn encode_to_value(&'e self) -> CustomResult<serde_json::Value, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_value(self)
            .into_report()
            .change_context(errors::ParsingError::EncodeError("json-value"))
            .attach_printable_lazy(|| format!("Unable to convert {self:?} to a value"))
    }

        /// Encodes the current value into a byte vector using serde_json and returns the result as a CustomResult.
    /// 
    fn encode_to_vec(&'e self) -> CustomResult<Vec<u8>, errors::ParsingError>
    where
        Self: Serialize,
    {
        serde_json::to_vec(self)
            .into_report()
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
        /// Parses a struct of type T from the given bytes using serde_json, and returns a CustomResult containing the parsed struct or a ParsingError.
    fn parse_struct<'de, T>(
        &'de self,
        type_name: &'static str,
    ) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>,
    {
        use bytes::Buf;

        serde_json::from_slice::<T>(self.chunk())
            .into_report()
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
        /// This method takes a reference to a struct, a type name, and attempts to deserialize the struct from a JSON byte slice. 
    /// If successful, it returns a CustomResult containing the deserialized struct. If unsuccessful, it returns a ParsingError 
    /// with a context indicating the failure to parse the specified type from the byte slice, along with a printable message 
    /// explaining the failure.
    fn parse_struct<'de, T>(
        &'de self,
        type_name: &'static str,
    ) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>,
    {
        serde_json::from_slice(self)
            .into_report()
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
        /// Parses the value into the specified type using serde_json, returning a CustomResult
    fn parse_value<T>(self, type_name: &'static str) -> CustomResult<T, errors::ParsingError>
    where
        T: serde::de::DeserializeOwned,
    {
        let debug = format!(
            "Unable to parse {type_name} from serde_json::Value: {:?}",
            &self
        );
        serde_json::from_value::<T>(self)
            .into_report()
            .change_context(errors::ParsingError::StructParseFailure(type_name))
            .attach_printable_lazy(|| debug)
    }
}

impl<MaskingStrategy> ValueExt for Secret<serde_json::Value, MaskingStrategy>
where
    MaskingStrategy: Strategy<serde_json::Value>,
{
        /// Parses the value of type T from the given type_name using serde deserialization.
    fn parse_value<T>(self, type_name: &'static str) -> CustomResult<T, errors::ParsingError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.expose().parse_value(type_name)
    }
}

impl<E: ValueExt + Clone> ValueExt for crypto::Encryptable<E> {
        /// Parses the value of the specified type using serde deserialization and returns a CustomResult
    /// containing the parsed value or a ParsingError if deserialization fails.
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
        /// Parses the input string into an enum variant of type T, and returns a CustomResult containing the parsed value or a ParsingError.
    fn parse_enum(self, enum_name: &'static str) -> CustomResult<T, errors::ParsingError>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        T::from_str(&self)
            .into_report()
            .change_context(errors::ParsingError::EnumParseFailure(enum_name))
            .attach_printable_lazy(|| format!("Invalid enum variant {self:?} for enum {enum_name}"))
    }

        /// Parse a JSON string into a specific struct type using serde deserialization,
    /// and return a CustomResult containing the deserialized struct or a ParsingError.
    fn parse_struct<'de>(
        &'de self,
        type_name: &'static str,
    ) -> CustomResult<T, errors::ParsingError>
    where
        T: Deserialize<'de>,
    {
        serde_json::from_str::<T>(self)
            .into_report()
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
        /// Asynchronously applies the provided function to the result of a successful future, then awaits the resulting future,
    /// returning a new future with the transformed value. If the original future is an error, it simply returns the error.
    ///
    /// # Arguments
    /// * `func` - A function that takes the successful value of the original future and returns a future with the transformed value.
    ///
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

        /// Asynchronously maps the result of a `Future` using the provided function `func`.
    ///
    /// # Arguments
    /// * `func` - The function used to map the result of the `Future`.
    ///
    /// # Returns
    /// The mapped result of the `Future` wrapped in the same type of `Result`.
    ///
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
        /// Asynchronously applies a function to the wrapped value and returns the result as a future.
    ///
    /// # Arguments
    /// * `func` - A function that takes the wrapped value and returns a future of the same type.
    ///
    /// # Returns
    /// The result of applying the function to the wrapped value as a future.
    ///
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

        /// Asynchronously maps the option to another option by applying the provided function.
    /// 
    /// The `async_map` method takes a closure `func` that accepts the value inside the `Option` and returns a future. 
    /// It then awaits the future and returns a new `Option` containing the result. 
    /// If the original option is `Some`, the closure `func` is applied to the value and the result is awaited. 
    /// If the original option is `None`, the `async_map` method returns `None` without applying the closure.
    /// 
    /// # Arguments
    /// 
    /// * `func` - A closure that defines the mapping operation to be applied to the value inside the `Option`.
    /// 
    /// # Returns
    /// 
    /// The method returns a new `Option` containing the result of applying the function to the value inside the original `Option`, or `None` if the original option was `None`.
    /// 
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
        /// Checks if the string, after trimming leading and trailing whitespace, is empty or not.
    fn is_empty_after_trim(&self) -> bool {
        false
    }
}

impl ConfigExt for String {
        /// This method trims the string and checks if the resulting string is empty.
    fn is_empty_after_trim(&self) -> bool {
        self.trim().is_empty()
    }
}

impl<T, U> ConfigExt for Secret<T, U>
where
    T: ConfigExt + Default + PartialEq<T>,
    U: Strategy<T>,
{
        /// Checks if the value returned by the peek method is equal to the default value of the type T.
    fn is_default(&self) -> bool
    where
        T: Default + PartialEq<T>,
    {
        *self.peek() == T::default()
    }

        /// Checks if the string, after trimming any leading or trailing whitespace, is empty.
    fn is_empty_after_trim(&self) -> bool {
        self.peek().is_empty_after_trim()
    }

        /// Checks if the value returned by peek() is either the default value for type T or empty after trimming.
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
        /// This method takes a string of XML data and attempts to parse it into a specified type using the serde deserialization framework. It returns a Result containing the parsed value of the specified type if successful, or a quick_xml::de::DeError if parsing fails.
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
        /// Check if the specified field is present in the current object. If the field is not present, return a `ValidationError` with a message indicating that the field is required.
    fn check_value_present(
        &self,
        field_name: &'static str,
    ) -> CustomResult<(), errors::ValidationError> {
        when(self.is_none(), || {
            Err(errors::ValidationError::MissingRequiredField {
                field_name: field_name.to_string(),
            })
            .into_report()
            .attach_printable(format!("Missing required field {field_name} in {self:?}"))
        })
    }

    // This will allow the error message that was generated in this function to point to the call site
    #[track_caller]
        /// Retrieves the required value from an Option. If the Option contains a value, it returns the value wrapped in a Result::Ok. If the Option is None, it returns a Result::Err containing a ValidationError indicating the missing required field.
    fn get_required_value(
        self,
        field_name: &'static str,
    ) -> CustomResult<T, errors::ValidationError> {
        match self {
            Some(v) => Ok(v),
            None => Err(errors::ValidationError::MissingRequiredField {
                field_name: field_name.to_string(),
            })
            .into_report()
            .attach_printable(format!("Missing required field {field_name} in {self:?}")),
        }
    }

    #[track_caller]
        /// Parses the value of the specified enum from the input string, returning a CustomResult
    /// containing the parsed enum value or a ParsingError if parsing fails.
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
            .into_report()
            .change_context(errors::ParsingError::UnknownError)
            .attach_printable_lazy(|| format!("Invalid {{ {enum_name}: {value:?} }} "))
    }

    #[track_caller]
        /// Parses the value of type U from the given type_name using the ValueExt trait and serde deserialization.
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

        /// Updates the value of the optional object with the provided value, if the value is not None.
    ///
    /// # Arguments
    ///
    /// * `value` - The new value to update the optional object with.
    ///
    fn update_value(&mut self, value: Self) {
        if let Some(a) = value {
            *self = Some(a)
        }
    }
}
