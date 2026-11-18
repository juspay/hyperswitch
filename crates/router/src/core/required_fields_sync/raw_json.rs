//! Lossless bounded JSON: reject duplicate keys before projection, retain raw numeric lexemes.
//! serde_json::Value alone could round unknown numbers; SDK outputs discard unknown members.

use std::{collections::BTreeMap, fmt};

use serde::de::{MapAccess, Visitor};
use serde_json::value::RawValue;
use sha2::{Digest as _, Sha256};

use super::{
    types::{failure, InventoryErrorKind as Kind, InventoryResult},
    Digest,
};

const MAX_DEPTH: usize = 32;
const MAX_NODES: usize = 100_000;

#[derive(Clone)]
enum Shape {
    Object(BTreeMap<String, Json>),
    Array(Vec<Json>),
    Scalar,
}

#[derive(Clone)]
pub(crate) struct Json {
    raw: Box<RawValue>,
    shape: Shape,
}

impl fmt::Debug for Json {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RetainedJson")
            .field("bytes", &self.raw.get().len())
            .finish_non_exhaustive()
    }
}

struct Members(BTreeMap<String, Box<RawValue>>);

impl<'de> serde::Deserialize<'de> for Members {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueMembers;
        impl<'de> Visitor<'de> for UniqueMembers {
            type Value = Members;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("object with unique members")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Members, M::Error> {
                let mut members = BTreeMap::new();
                while let Some((key, value)) = map.next_entry::<String, Box<RawValue>>()? {
                    if members.contains_key(&key) {
                        return Err(serde::de::Error::custom("duplicate JSON member"));
                    }
                    members.insert(key, value);
                }
                Ok(Members(members))
            }
        }
        deserializer.deserialize_map(UniqueMembers)
    }
}

impl Json {
    pub(crate) fn parse(bytes: &[u8]) -> InventoryResult<Self> {
        let raw = serde_json::from_slice(bytes).map_err(|_| failure(Kind::MalformedJson))?;
        Self::node(raw, 0, &mut 0)
    }

    fn node(raw: Box<RawValue>, depth: usize, nodes: &mut usize) -> InventoryResult<Self> {
        *nodes += 1;
        if depth > MAX_DEPTH || *nodes > MAX_NODES {
            return Err(failure(Kind::JsonLimit));
        }
        let shape = match raw.get().as_bytes().first() {
            Some(b'{') => {
                let members: Members =
                    serde_json::from_str(raw.get()).map_err(|_| failure(Kind::MalformedJson))?;
                Shape::Object(
                    members
                        .0
                        .into_iter()
                        .map(|(key, value)| Ok((key, Self::node(value, depth + 1, nodes)?)))
                        .collect::<InventoryResult<_>>()?,
                )
            }
            Some(b'[') => {
                let values: Vec<Box<RawValue>> =
                    serde_json::from_str(raw.get()).map_err(|_| failure(Kind::MalformedJson))?;
                Shape::Array(
                    values
                        .into_iter()
                        .map(|value| Self::node(value, depth + 1, nodes))
                        .collect::<InventoryResult<_>>()?,
                )
            }
            Some(_) => Shape::Scalar,
            None => return Err(failure(Kind::MalformedJson)),
        };
        Ok(Self { raw, shape })
    }

    pub(crate) fn raw(&self) -> &str {
        self.raw.get()
    }

    /// Rebuild only the containing object. Every retained child keeps its original JSON bytes.
    pub(crate) fn members(members: &BTreeMap<String, Self>) -> InventoryResult<Self> {
        let mut bytes = Vec::new();
        bytes.push(b'{');
        for (index, (key, value)) in members.iter().enumerate() {
            if index != 0 {
                bytes.push(b',');
            }
            serde_json::to_writer(&mut bytes, key).map_err(|_| failure(Kind::MalformedJson))?;
            bytes.push(b':');
            bytes.extend_from_slice(value.raw().as_bytes());
        }
        bytes.push(b'}');
        Self::parse(&bytes)
    }

    pub(crate) fn replacing(&self, key: &str, value: Self) -> InventoryResult<Self> {
        let mut members = self.object()?.clone();
        members.insert(key.to_owned(), value);
        Self::members(&members)
    }
    pub(crate) fn object(&self) -> InventoryResult<&BTreeMap<String, Self>> {
        match &self.shape {
            Shape::Object(value) => Ok(value),
            _ => Err(failure(Kind::MalformedResponse)),
        }
    }
    pub(crate) fn array(&self) -> InventoryResult<&[Self]> {
        match &self.shape {
            Shape::Array(value) => Ok(value),
            _ => Err(failure(Kind::MalformedResponse)),
        }
    }
    pub(crate) fn get(&self, key: &str) -> InventoryResult<&Self> {
        self.object()?
            .get(key)
            .ok_or_else(|| failure(Kind::MissingField))
    }
    pub(crate) fn optional(&self, key: &str) -> InventoryResult<Option<&Self>> {
        Ok(self.object()?.get(key))
    }
    pub(crate) fn string(&self) -> InventoryResult<String> {
        serde_json::from_str(self.raw()).map_err(|_| failure(Kind::MalformedResponse))
    }
    pub(crate) fn boolean(&self) -> InventoryResult<bool> {
        serde_json::from_str(self.raw()).map_err(|_| failure(Kind::MalformedResponse))
    }
    pub(crate) fn unsigned(&self) -> InventoryResult<u64> {
        self.raw()
            .parse()
            .map_err(|_| failure(Kind::MalformedResponse))
    }
    pub(crate) fn null(&self) -> bool {
        self.raw() == "null"
    }
    pub(crate) fn scalar(&self) -> bool {
        matches!(self.shape, Shape::Scalar)
    }

    pub(crate) fn canonical(&self) -> InventoryResult<Vec<u8>> {
        fn write(value: &Json, output: &mut Vec<u8>) -> InventoryResult<()> {
            match &value.shape {
                Shape::Object(members) => {
                    output.push(b'{');
                    for (index, (key, value)) in members.iter().enumerate() {
                        if index != 0 {
                            output.push(b',');
                        }
                        serde_json::to_writer(&mut *output, key)
                            .map_err(|_| failure(Kind::MalformedJson))?;
                        output.push(b':');
                        write(value, output)?;
                    }
                    output.push(b'}');
                }
                Shape::Array(values) => {
                    output.push(b'[');
                    for (index, value) in values.iter().enumerate() {
                        if index != 0 {
                            output.push(b',');
                        }
                        write(value, output)?;
                    }
                    output.push(b']');
                }
                Shape::Scalar if value.raw().starts_with('"') => {
                    serde_json::to_writer(output, &value.string()?)
                        .map_err(|_| failure(Kind::MalformedJson))?;
                }
                Shape::Scalar => output.extend_from_slice(value.raw().as_bytes()),
            }
            Ok(())
        }
        let mut output = Vec::new();
        write(self, &mut output)?;
        Ok(output)
    }
}

pub(crate) fn fingerprint(domain: &str, bytes: &[u8]) -> InventoryResult<Digest> {
    let mut digest = Sha256::new();
    digest.update(b"required-fields/inventory/v1\0");
    digest.update(domain.as_bytes());
    digest.update(b"\0");
    digest.update(bytes);
    format!("sha256:{:x}", digest.finalize())
        .try_into()
        .map_err(|_| failure(Kind::MalformedJson))
}
