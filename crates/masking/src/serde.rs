//!
//! Serde-related.
//!

pub use erased_serde::Serialize as ErasedSerialize;
pub use serde::{de, Deserialize, Serialize, Serializer};
use serde_json::{value::Serializer as JsonValueSerializer, Value};

use crate::{Secret, Strategy, StrongSecret, ZeroizableSecret};

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

impl SerializableSecret for Value {}
impl SerializableSecret for u8 {}
impl SerializableSecret for u16 {}

impl<'de, T, I> Deserialize<'de> for Secret<T, I>
where
    T: Clone + de::DeserializeOwned + Sized,
    I: Strategy<T>,
{
        /// Deserialize the given value using the provided deserializer and return the result as a Result.
    /// 
    /// # Arguments
    /// 
    /// * `deserializer` - The deserializer to use for deserializing the value
    /// 
    /// # Returns
    /// 
    /// A Result containing the deserialized value or an error from the deserializer
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
        /// This method takes a serializer `S` and uses the `pii_serializer` to serialize the current object, returning a Result containing the serialized value (`S::Ok`) or an error (`S::Error`).
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        pii_serializer::pii_serialize(self, serializer)
    }
}

impl<'de, T, I> Deserialize<'de> for StrongSecret<T, I>
where
    T: Clone + de::DeserializeOwned + Sized + ZeroizableSecret,
    I: Strategy<T>,
{
        /// Deserialize an instance of `Self` using the given deserializer.
    /// 
    /// # Arguments
    /// 
    /// * `deserializer` - The deserializer to use for deserialization
    /// 
    /// # Returns
    /// 
    /// A `Result` containing either the deserialized instance of `Self` or an error
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
        S: Serializer,
    {
        pii_serializer::pii_serialize(self, serializer)
    }
}

///
/// Masked serialization.
///
/// the default behaviour for secrets is to serialize in exposed format since the common use cases
/// for storing the secret to database or sending it over the network requires the secret to be exposed
/// This method allows to serialize the secret in masked format if needed for logs or other insecure exposures
pub fn masked_serialize<T: Serialize>(value: &T) -> Result<Value, serde_json::Error> {
    value.serialize(PIISerializer {
        inner: JsonValueSerializer,
    })
}

///
/// Masked serialization.
///
/// Trait object for supporting serialization to Value while accounting for masking
/// The usual Serde Serialize trait cannot be used as trait objects
/// like &dyn Serialize or boxed trait objects like Box<dyn Serialize> because of Rust's "object safety" rules.
/// In particular, the trait contains generic methods which cannot be made into a trait object.
/// In this case we remove the generic for assuming the serialization to be of 2 types only raw json or masked json
pub trait ErasedMaskSerialize: ErasedSerialize {
    /// Masked serialization.
    fn masked_serialize(&self) -> Result<Value, serde_json::Error>;
}

impl<T: Serialize + ErasedSerialize> ErasedMaskSerialize for T {
        /// Serialize the data with masking applied.
    /// 
    /// This method takes the data and applies masking to sensitive information before serializing it.
    /// 
    /// Returns a Result containing the serialized value if successful, or a serde_json::Error if an error occurs during serialization.
    fn masked_serialize(&self) -> Result<Value, serde_json::Error> {
        masked_serialize(self)
    }
}

impl<'a> Serialize for dyn ErasedMaskSerialize + 'a {
        /// This method serializes the current object using the given serializer, and returns the result or an error.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        erased_serde::serialize(self, serializer)
    }
}

impl<'a> Serialize for dyn ErasedMaskSerialize + 'a + Send {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        erased_serde::serialize(self, serializer)
    }
}

use pii_serializer::PIISerializer;

mod pii_serializer {
    use std::fmt::Display;

        /// This method serializes the given value using the specified serializer. If the serializer is of type PIISerializer, it masks the value before serializing. If the serializer is of type FlatMapSerializer over PiiSerializer, it sends an empty map. Otherwise, it serializes the value as is.
    pub(super) fn pii_serialize<
        V: Serialize,
        T: std::fmt::Debug + PeekInterface<V>,
        S: Serializer,
    >(
        value: &T,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        // Mask the value if the serializer is of type PIISerializer
        // or send empty map if the serializer is of type FlatMapSerializer over PiiSerializer
        if std::any::type_name::<S>() == std::any::type_name::<PIISerializer>() {
            format!("{value:?}").serialize(serializer)
        } else if std::any::type_name::<S>()
            == std::any::type_name::<
                serde::__private::ser::FlatMapSerializer<'_, SerializeMap<PIISerializer>>,
            >()
        {
            std::collections::HashMap::<String, String>::from([]).serialize(serializer)
        } else {
            value.peek().serialize(serializer)
        }
    }

    use serde::{Serialize, Serializer};
    use serde_json::{value::Serializer as JsonValueSerializer, Map, Value};

    use crate::PeekInterface;

    pub(super) struct PIISerializer {
        pub inner: JsonValueSerializer,
    }

    impl Clone for PIISerializer {
                /// Returns a deep copy of the current JsonValueSerializer.
        fn clone(&self) -> Self {
            Self {
                inner: JsonValueSerializer,
            }
        }
    }

    impl Serializer for PIISerializer {
        type Ok = Value;
        type Error = serde_json::Error;

        type SerializeSeq = SerializeVec<Self>;
        type SerializeTuple = SerializeVec<Self>;
        type SerializeTupleStruct = SerializeVec<Self>;
        type SerializeTupleVariant = SerializeTupleVariant<Self>;
        type SerializeMap = SerializeMap<Self>;
        type SerializeStruct = SerializeMap<Self>;
        type SerializeStructVariant = SerializeStructVariant<Self>;

        #[inline]
                /// Serializes the given boolean value and returns the result.
        /// 
        /// # Arguments
        /// 
        /// * `value` - The boolean value to be serialized.
        /// 
        /// # Returns
        /// 
        /// Returns a Result containing the serialized boolean value if successful, otherwise returns an error.
        fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_bool(value)
        }

        #[inline]
                /// Serialize the given i8 value and return a Result containing either the serialized value
        /// or an error.
        fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(value.into())
        }

        #[inline]
                /// Serialize the given i16 value and return the result.
        fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(value.into())
        }

        #[inline]
                /// Serialize the given i32 value and return the result as a Result.
        fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(value.into())
        }

                /// Serialize the given i64 value and return the result.
        fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_i64(value)
        }

                /// Serializes an i128 value and returns the result.
        fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_i128(value)
        }

        #[inline]
                /// Serializes an unsigned 8-bit integer value into a specific format and returns the result.
        fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
            self.serialize_u64(value.into())
        }

        #[inline]
                /// Serializes a u16 value and returns the result.
        ///
        /// This method takes a u16 value as input and attempts to serialize it. 
        /// If successful, it returns the result as Ok. If an error occurs during serialization, 
        /// it returns the error as Err.
        ///
        /// # Arguments
        ///
        /// * `value` - The u16 value to be serialized
        ///
        /// # Returns
        ///
        /// Returns a Result containing the serialized value if successful, or an error if unsuccessful.
        fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
            self.serialize_u64(value.into())
        }

        #[inline]
                /// Serializes a 32-bit unsigned integer value and returns the result.
        fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
            self.serialize_u64(value.into())
        }

        #[inline]
                /// Serializes a u64 value into a JSON number and returns the result.
        fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Number(value.into()))
        }

                /// Serializes a 128-bit unsigned integer value.
        ///
        /// This method takes a 128-bit unsigned integer value and serializes it using the inner serializer. It returns a Result containing the serialized value if successful, or an error if the serialization process fails.
        ///
        /// # Arguments
        ///
        /// * `value` - The 128-bit unsigned integer value to be serialized.
        ///
        /// # Returns
        ///
        /// A Result containing the serialized value if successful, or an error if the serialization process fails.
        ///
        fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_u128(value)
        }

        #[inline]
                /// Serializes a 32-bit floating point number into a Value object and returns a Result
        ///
        /// # Arguments
        ///
        /// * `float` - The 32-bit floating point number to be serialized
        ///
        /// # Returns
        ///
        /// * `Result<Self::Ok, Self::Error>` - A Result containing the serialized Value object or an error
        ///
        fn serialize_f32(self, float: f32) -> Result<Self::Ok, Self::Error> {
            Ok(Value::from(float))
        }

        #[inline]
                /// Serializes a f64 value into a JSON Value.
        ///
        /// # Arguments
        /// 
        /// * `float` - The f64 value to be serialized.
        /// 
        /// # Returns
        /// 
        /// Returns a Result containing the serialized JSON Value if successful, or an error if the serialization fails.
        fn serialize_f64(self, float: f64) -> Result<Self::Ok, Self::Error> {
            Ok(Value::from(float))
        }

        #[inline]
                /// Serialize a single char value into a JSON string representation.
        ///
        /// # Arguments
        ///
        /// * `value` - The char value to be serialized.
        ///
        /// # Returns
        ///
        /// A Result containing either the serialized JSON string representation of the char value
        /// or an error if the serialization process fails.
        ///
        fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
            let mut s = String::new();
            s.push(value);
            Ok(Value::String(s))
        }

        #[inline]
                /// Serializes a string value and returns the result.
        /// 
        /// # Arguments
        /// 
        /// * `value` - A reference to the string value to be serialized.
        /// 
        /// # Returns
        /// 
        /// A `Result` containing either the serialized string value or an error.
        fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
            Ok(Value::String(value.to_owned()))
        }

                /// Serialize the given byte array into a JSON array of numbers.
        /// 
        /// # Arguments
        /// * `value` - A reference to the byte array to be serialized
        /// 
        /// # Returns
        /// A Result containing the serialized JSON array if successful, or an error if the serialization fails
        fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
            let vec = value.iter().map(|&b| Value::Number(b.into())).collect();
            Ok(Value::Array(vec))
        }

        #[inline]
        /// Serializes a unit (null) value and returns the result.
        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Null)
        }

        #[inline]
                /// Serializes a unit struct with the given name.
        /// 
        /// # Arguments
        /// 
        /// * `self` - The serializer.
        /// * `_name` - The name of the unit struct.
        /// 
        /// # Returns
        /// 
        /// Returns a Result with the serialized unit struct on success, or an error on failure.
        fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
            self.serialize_unit()
        }

        #[inline]
        /// Serializes a unit variant of an enum by serializing the variant name as a string.
        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            self.serialize_str(variant)
        }

        #[inline]
                /// Serializes a newtype struct with the given name and value.
        /// 
        /// # Arguments
        /// 
        /// * `self` - The serializer instance.
        /// * `_name` - The name of the newtype struct.
        /// * `value` - The value of the newtype struct to be serialized.
        /// 
        /// # Returns
        /// 
        /// A Result containing the serialized value if successful, or an error if serialization fails.
        /// 
        /// # Constraints
        /// 
        /// The type `T` must implement the `Serialize` trait.
        /// 
        fn serialize_newtype_struct<T>(
            self,
            _name: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: ?Sized + Serialize,
        {
            value.serialize(self)
        }

                /// Serializes a newtype variant with the given name, variant index, variant, and value.
        fn serialize_newtype_variant<T>(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: ?Sized + Serialize,
        {
            let mut values = Map::new();
            values.insert(String::from(variant), value.serialize(self)?);
            Ok(Value::Object(values))
        }

        #[inline]
                /// Serializes a `None` value by calling the `serialize_unit` method on the current serializer.
        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            self.serialize_unit()
        }

        #[inline]
                /// Serialize the specified value using the given serializer.
        fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: ?Sized + Serialize,
        {
            value.serialize(self)
        }

                /// Returns a Result containing a SerializeSeq implementation. 
        /// 
        /// This method creates a new SerializeVec with a vector of the specified length, or 0 if None,
        /// and the current serializer, and returns a Result containing the SerializeSeq implementation.
        fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Ok(SerializeVec {
                vec: Vec::with_capacity(len.unwrap_or(0)),
                ser: self,
            })
        }

                /// Serializes a tuple of the specified length.
        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            self.serialize_seq(Some(len))
        }

                /// Serializes a tuple struct with the given name and length.
        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            self.serialize_seq(Some(len))
        }

                /// Serialize a tuple variant of a Rust enum by creating a new SerializeTupleVariant instance with the given variant name, length, and serializer.
        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Ok(SerializeTupleVariant {
                name: String::from(variant),
                vec: Vec::with_capacity(len),
                ser: self,
            })
        }

                /// Serializes a map with the given length, returning a Result containing the SerializeMap or an error.
        fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Ok(SerializeMap {
                inner: self.clone().inner.serialize_map(len)?,
                ser: self,
            })
        }

                /// Serializes a struct with the given name and length, returning a Result with the serialized struct or an error.
        fn serialize_struct(
            self,
            _name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            self.serialize_map(Some(len))
        }

                /// Serialize a struct variant with the given name and variant index, and return a result containing the serialized struct variant or an error.
        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Ok(SerializeStructVariant {
                name: String::from(variant),
                map: Map::new(),
                ser: self,
            })
        }

                /// Collects the string representation of the provided value and returns a `Result` containing either the collected string if successful, or an error if the collection fails.
        fn collect_str<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: ?Sized + Display,
        {
            self.inner.collect_str(value)
        }
    }

    pub(super) struct SerializeVec<T: Serializer> {
        vec: Vec<Value>,
        ser: T,
    }

    impl<T: Serializer<Ok = Value> + Clone> serde::ser::SerializeSeq for SerializeVec<T> {
        type Ok = Value;
        type Error = T::Error;

                /// Serializes the provided value and appends the result to the internal vector.
        fn serialize_element<V>(&mut self, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            self.vec.push(value.serialize(self.ser.clone())?);
            Ok(())
        }

                /// Returns a Result containing the array of values if the operation is successful, or an error if the operation fails.
        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Array(self.vec))
        }
    }

    impl<T: Serializer<Ok = Value> + Clone> serde::ser::SerializeTuple for SerializeVec<T> {
        type Ok = Value;
        type Error = T::Error;

                /// Serialize the given value and add it as an element to the sequence being serialized.
        fn serialize_element<V>(&mut self, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            serde::ser::SerializeSeq::serialize_element(self, value)
        }

                /// Ends the serialization of a sequence and returns the result as a `Result` containing either the serialized value or an error.
        fn end(self) -> Result<Self::Ok, Self::Error> {
            serde::ser::SerializeSeq::end(self)
        }
    }

    impl<T: Serializer<Ok = Value> + Clone> serde::ser::SerializeTupleStruct for SerializeVec<T> {
        type Ok = Value;
        type Error = T::Error;

                /// Serialize the given value and add it as an element to the current sequence being serialized.
        fn serialize_field<V>(&mut self, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            serde::ser::SerializeSeq::serialize_element(self, value)
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            serde::ser::SerializeSeq::end(self)
        }
    }

    pub(super) struct SerializeStructVariant<T: Serializer> {
        name: String,
        map: Map<String, Value>,
        ser: T,
    }

    impl<T: Serializer<Ok = Value> + Clone> serde::ser::SerializeStructVariant
        for SerializeStructVariant<T>
    {
        type Ok = Value;
        type Error = T::Error;

                /// This method serializes a given value and inserts it into the map with the provided key. 
        /// 
        /// # Arguments
        /// 
        /// * `key` - A reference to a string representing the key for the value in the map
        /// * `value` - A reference to the value to be serialized and inserted into the map
        /// 
        /// # Returns
        /// 
        /// This method returns a `Result` with the outcome of the serialization and insertion operation. 
        /// 
        fn serialize_field<V>(&mut self, key: &'static str, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            self.map
                .insert(String::from(key), value.serialize(self.ser.clone())?);
            Ok(())
        }

                /// Inserts the current object into a new Map, with the given name and map of values, and returns it as a Result.
        fn end(self) -> Result<Self::Ok, Self::Error> {
            let mut object = Map::new();

            object.insert(self.name, Value::Object(self.map));

            Ok(Value::Object(object))
        }
    }

    pub(super) struct SerializeTupleVariant<T: Serializer> {
        name: String,
        vec: Vec<Value>,
        ser: T,
    }

    impl<T: Serializer<Ok = Value> + Clone> serde::ser::SerializeTupleVariant
        for SerializeTupleVariant<T>
    {
        type Ok = Value;
        type Error = T::Error;

                /// Serialize the given value and push the serialized result into the internal vector.
        fn serialize_field<V>(&mut self, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            self.vec.push(value.serialize(self.ser.clone())?);
            Ok(())
        }

                /// Inserts the current object's name and vector of values into a new Map object, and returns a Result containing the updated Value or an error if encountered.
        fn end(self) -> Result<Value, Self::Error> {
            let mut object = Map::new();

            object.insert(self.name, Value::Array(self.vec));

            Ok(Value::Object(object))
        }
    }

    pub(super) struct SerializeMap<T: Serializer> {
        inner: <serde_json::value::Serializer as Serializer>::SerializeMap,
        ser: T,
    }

    impl<T: Serializer<Ok = Value, Error = serde_json::Error> + Clone> serde::ser::SerializeMap
        for SerializeMap<T>
    {
        type Ok = Value;
        type Error = T::Error;

                /// Serializes the given key and writes it to the underlying data stream.
        ///
        /// # Arguments
        ///
        /// * `key` - A reference to the key that needs to be serialized
        ///
        /// # Returns
        ///
        /// Returns a Result indicating success or an error if the serialization fails.
        fn serialize_key<V>(&mut self, key: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            self.inner.serialize_key(key)?;
            Ok(())
        }

                /// Serializes the given value and writes it to the inner serializer.
        ///
        /// # Arguments
        /// * `value` - The value to be serialized
        ///
        /// # Returns
        /// * `Result<(), Self::Error>` - A result indicating success or an error of type Self::Error
        ///
        fn serialize_value<V>(&mut self, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            let value = value.serialize(self.ser.clone())?;
            self.inner.serialize_value(&value)?;
            Ok(())
        }

                /// Calls the `end` method on the inner value and returns the result.
        fn end(self) -> Result<Value, Self::Error> {
            self.inner.end()
        }
    }

    impl<T: Serializer<Ok = Value, Error = serde_json::Error> + Clone> serde::ser::SerializeStruct
        for SerializeMap<T>
    {
        type Ok = Value;
        type Error = T::Error;

                /// Serializes a key-value pair into the current map being serialized.
        fn serialize_field<V>(&mut self, key: &'static str, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            serde::ser::SerializeMap::serialize_entry(self, key, value)
        }

                /// Completes the serialization of a map and returns the resulting Value, or an error if the serialization failed.
        fn end(self) -> Result<Value, Self::Error> {
            serde::ser::SerializeMap::end(self)
        }
    }
}
