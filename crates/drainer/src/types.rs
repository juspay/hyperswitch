use std::collections::HashMap;

use common_utils::errors;
use error_stack::ResultExt;
use serde::{de::value::MapDeserializer, Deserialize, Serialize};

use crate::{
    kv,
    utils::{deserialize_db_op, deserialize_i64},
};

#[derive(Deserialize, Serialize)]
pub struct StreamData {
    pub request_id: String,
    pub global_id: String,
    #[serde(deserialize_with = "deserialize_db_op")]
    pub typed_sql: kv::DBOperation,
    #[serde(deserialize_with = "deserialize_i64")]
    pub pushed_at: i64,
}

impl StreamData {
    pub fn from_hashmap(
        hashmap: HashMap<String, redis_interface::RedisValue>,
    ) -> errors::CustomResult<Self, errors::ParsingError> {
        // Convert RedisValue to String, failing explicitly on non-convertible values
        // rather than silently dropping them. This catches data corruption early.
        let mut string_map = HashMap::with_capacity(hashmap.len());
        for (field_name, field_value) in hashmap {
            let string_value = field_value.as_string().ok_or_else(|| {
                error_stack::report!(errors::ParsingError::UnknownError).attach_printable(format!(
                    "Field '{}' contains non-string Redis value that cannot be deserialized",
                    field_name
                ))
            })?;
            string_map.insert(field_name, string_value);
        }

        let iter = MapDeserializer::<
            '_,
            std::collections::hash_map::IntoIter<String, String>,
            serde_json::error::Error,
        >::new(string_map.into_iter());

        Self::deserialize(iter)
            .change_context(errors::ParsingError::StructParseFailure("StreamData"))
    }
}
