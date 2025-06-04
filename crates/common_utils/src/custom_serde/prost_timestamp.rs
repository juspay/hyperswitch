use prost_types::Timestamp;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A wrapper around `prost_types::Timestamp` to enable custom Serde implementations.
#[derive(Debug, Clone, PartialEq)]
pub struct SerializableTimestamp(pub Timestamp);

impl From<Timestamp> for SerializableTimestamp {
    fn from(ts: Timestamp) -> Self {
        SerializableTimestamp(ts)
    }
}

impl From<SerializableTimestamp> for Timestamp {
    fn from(sts: SerializableTimestamp) -> Self {
        sts.0
    }
}

// Helper struct for serializing/deserializing the fields of Timestamp
#[derive(Serialize, Deserialize)]
struct TimestampFields {
    seconds: i64,
    nanos: i32,
}

impl Serialize for SerializableTimestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields = TimestampFields {
            seconds: self.0.seconds,
            nanos: self.0.nanos,
        };
        fields.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SerializableTimestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = TimestampFields::deserialize(deserializer)?;
        Ok(SerializableTimestamp(Timestamp {
            seconds: fields.seconds,
            nanos: fields.nanos,
        }))
    }
}

/// Serde module for `Option<SerializableTimestamp>`.
pub mod optional_prost_timestamp {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::SerializableTimestamp;

    pub fn serialize<S>(
        option_timestamp: &Option<SerializableTimestamp>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match option_timestamp {
            Some(timestamp) => timestamp.serialize(serializer), // Directly use SerializableTimestamp's Serialize impl
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SerializableTimestamp>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<SerializableTimestamp>::deserialize(deserializer) // Directly use SerializableTimestamp's Deserialize impl
    }
}
