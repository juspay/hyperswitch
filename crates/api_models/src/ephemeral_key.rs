use serde;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Eq, PartialEq)]
pub struct EphemeralKeyCreateResponse {
    pub created_at: i64,
    pub expires: i64,
    pub secret: String,
}
