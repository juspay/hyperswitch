#[derive(Clone, serde::Deserialize)]
pub struct ConfigKeyCreate {
    pub key: String,
    pub value: String,
}
