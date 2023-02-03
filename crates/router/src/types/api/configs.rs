#[derive(Clone, serde::Serialize, Debug)]
pub struct Config {
    pub key: String,
    pub value: String,
}

#[derive(Clone, serde::Deserialize, Debug)]
pub struct ConfigUpdate {
    #[serde(skip_deserializing)]
    pub key: String,
    pub value: String,
}
