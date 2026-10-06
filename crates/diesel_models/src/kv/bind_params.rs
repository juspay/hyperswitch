use base64::Engine;
use common_utils::consts::BASE64_ENGINE;
use hyperswitch_masking::{PeekInterface, Secret};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::SecretBinaryData;

pub fn serialize<S: Serializer>(
    binds: &[Option<SecretBinaryData>],
    s: S,
) -> Result<S::Ok, S::Error> {
    let encoded: Vec<Option<String>> = binds
        .iter()
        .map(|b| b.as_ref().map(|bytes| BASE64_ENGINE.encode(bytes.peek())))
        .collect();
    encoded.serialize(s)
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Vec<Option<SecretBinaryData>>, D::Error> {
    let encoded: Vec<Option<String>> = Vec::deserialize(d)?;
    encoded
        .into_iter()
        .map(|b| {
            b.map(|s| {
                BASE64_ENGINE
                    .decode(&s)
                    .map(Secret::new)
                    .map_err(serde::de::Error::custom)
            })
            .transpose()
        })
        .collect()
}
