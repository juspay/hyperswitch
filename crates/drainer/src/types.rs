use std::collections::HashMap;

use common_utils::errors;
use error_stack::{IntoReport, ResultExt};
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
        /// Constructs a StreamData object from a HashMap of key-value pairs representing JSON data.
    pub fn from_hashmap(
        hashmap: HashMap<String, String>,
    ) -> errors::CustomResult<Self, errors::ParsingError> {
        let iter = MapDeserializer::<
            '_,
            std::collections::hash_map::IntoIter<String, String>,
            serde_json::error::Error,
        >::new(hashmap.into_iter());
    
        Self::deserialize(iter)
            .into_report()
            .change_context(errors::ParsingError::StructParseFailure("StreamData"))
    }
}
