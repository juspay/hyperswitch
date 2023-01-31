#[derive(Clone, serde::Serialize, Debug)]
pub struct Config {
    pub key: String,
    pub value: String,
}
