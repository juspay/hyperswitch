use serde::{
    ser::{Error, Serializer},
    Deserialize, Deserializer,
};

#[derive(Clone, serde::Serialize, Debug, serde::Deserialize)]
pub struct Config {
    pub key: String,
    #[serde(
        deserialize_with = "string_to_vec_deser",
        serialize_with = "vec_to_string_serialize"
    )]
    pub value: Vec<u8>,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct ConfigUpdate {
    #[serde(skip_deserializing)]
    pub key: String,
    #[serde(
        deserialize_with = "string_to_vec_deser",
        serialize_with = "vec_to_string_serialize"
    )]
    pub value: Vec<u8>,
}

fn string_to_vec_deser<'a, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'a>,
{
    let value = <String>::deserialize(deserializer)?;
    Ok(value.into())
}

pub fn vec_to_string_serialize<S>(value: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let value = std::str::from_utf8(value)
        .map_err(|_| S::Error::custom("Unable to serialize config value"))?;
    serializer.serialize_str(value)
}
