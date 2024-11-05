use masking::{PeekInterface, Secret, Strategy};
use serde::{Deserialize, Serialize, Serializer};

#[derive(Clone, PartialEq, Debug, Deserialize)]
/// Represents a hashed string using blake3's hashing strategy.
pub struct HashedString<T: Strategy<String>>(Secret<String, T>);

impl<T: Strategy<String>> Serialize for HashedString<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hashed_value = blake3::hash(self.0.peek().as_bytes()).to_hex();
        hashed_value.serialize(serializer)
    }
}

impl<T: Strategy<String>> From<Secret<String, T>> for HashedString<T> {
    fn from(value: Secret<String, T>) -> Self {
        Self(value)
    }
}
