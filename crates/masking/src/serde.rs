//!
//! Serde-related.
//!

pub use serde::{de, ser, Deserialize, Serialize, Serializer};
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

use pii_serializer::PIISerializer;

mod pii_serializer {
    use std::fmt::Display;

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
        fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_bool(value)
        }

        #[inline]
        fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(value.into())
        }

        #[inline]
        fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(value.into())
        }

        #[inline]
        fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
            self.serialize_i64(value.into())
        }

        fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_i64(value)
        }

        fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_i128(value)
        }

        #[inline]
        fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
            self.serialize_u64(value.into())
        }

        #[inline]
        fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
            self.serialize_u64(value.into())
        }

        #[inline]
        fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
            self.serialize_u64(value.into())
        }

        #[inline]
        fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Number(value.into()))
        }

        fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_u128(value)
        }

        #[inline]
        fn serialize_f32(self, float: f32) -> Result<Self::Ok, Self::Error> {
            Ok(Value::from(float))
        }

        #[inline]
        fn serialize_f64(self, float: f64) -> Result<Self::Ok, Self::Error> {
            Ok(Value::from(float))
        }

        #[inline]
        fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
            let mut s = String::new();
            s.push(value);
            Ok(Value::String(s))
        }

        #[inline]
        fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
            Ok(Value::String(value.to_owned()))
        }

        fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
            let vec = value.iter().map(|&b| Value::Number(b.into())).collect();
            Ok(Value::Array(vec))
        }

        #[inline]
        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Null)
        }

        #[inline]
        fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
            self.serialize_unit()
        }

        #[inline]
        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            self.serialize_str(variant)
        }

        #[inline]
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
        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            self.serialize_unit()
        }

        #[inline]
        fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: ?Sized + Serialize,
        {
            value.serialize(self)
        }

        fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Ok(SerializeVec {
                vec: Vec::with_capacity(len.unwrap_or(0)),
                ser: self,
            })
        }

        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            self.serialize_seq(Some(len))
        }

        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            self.serialize_seq(Some(len))
        }

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

        fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Ok(SerializeMap {
                inner: self.clone().inner.serialize_map(len)?,
                ser: self,
            })
        }

        fn serialize_struct(
            self,
            _name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            self.serialize_map(Some(len))
        }

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

        fn serialize_element<V>(&mut self, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            self.vec.push(value.serialize(self.ser.clone())?);
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Array(self.vec))
        }
    }

    impl<T: Serializer<Ok = Value> + Clone> serde::ser::SerializeTuple for SerializeVec<T> {
        type Ok = Value;
        type Error = T::Error;

        fn serialize_element<V>(&mut self, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            serde::ser::SerializeSeq::serialize_element(self, value)
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            serde::ser::SerializeSeq::end(self)
        }
    }

    impl<T: Serializer<Ok = Value> + Clone> serde::ser::SerializeTupleStruct for SerializeVec<T> {
        type Ok = Value;
        type Error = T::Error;

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

        fn serialize_field<V>(&mut self, key: &'static str, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            self.map
                .insert(String::from(key), value.serialize(self.ser.clone())?);
            Ok(())
        }

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

        fn serialize_field<V>(&mut self, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            self.vec.push(value.serialize(self.ser.clone())?);
            Ok(())
        }

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

        fn serialize_key<V>(&mut self, key: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            self.inner.serialize_key(key)?;
            Ok(())
        }

        fn serialize_value<V>(&mut self, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            let value = value.serialize(self.ser.clone())?;
            self.inner.serialize_value(&value)?;
            Ok(())
        }

        fn end(self) -> Result<Value, Self::Error> {
            self.inner.end()
        }
    }

    impl<T: Serializer<Ok = Value, Error = serde_json::Error> + Clone> serde::ser::SerializeStruct
        for SerializeMap<T>
    {
        type Ok = Value;
        type Error = T::Error;

        fn serialize_field<V>(&mut self, key: &'static str, value: &V) -> Result<(), Self::Error>
        where
            V: ?Sized + Serialize,
        {
            serde::ser::SerializeMap::serialize_entry(self, key, value)
        }

        fn end(self) -> Result<Value, Self::Error> {
            serde::ser::SerializeMap::end(self)
        }
    }
}
