pub struct EphemeralKeyNew {
    pub id: String,
    pub merchant_id: String,
    pub customer_id: String,
    pub secret: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EphemeralKey {
    pub id: String,
    pub merchant_id: String,
    pub customer_id: String,
    pub created_at: i64,
    pub expires: i64,
    pub secret: String,
}
