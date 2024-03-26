#[derive(Clone, serde::Serialize, Debug, serde::Deserialize)]
pub struct Config {
    pub key: String,
    pub value: String,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct ConfigUpdate {
    #[serde(skip_deserializing)]
    pub key: String,
    pub value: String,
}
