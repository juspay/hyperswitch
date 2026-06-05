use diesel::pg::PgTypeMetadata;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize<S: Serializer>(metadata: &[PgTypeMetadata], s: S) -> Result<S::Ok, S::Error> {
    let pairs: Vec<(u32, u32)> = metadata
        .iter()
        .map(|m| (m.oid().unwrap_or(0), m.array_oid().unwrap_or(0)))
        .collect();
    pairs.serialize(s)
}

pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<PgTypeMetadata>, D::Error> {
    let pairs: Vec<(u32, u32)> = Vec::deserialize(d)?;
    Ok(pairs
        .into_iter()
        .map(|(oid, array_oid)| PgTypeMetadata::from_result(Ok((oid, array_oid))))
        .collect())
}
