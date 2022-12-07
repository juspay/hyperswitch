pub struct EphemeralKeyNew {
    pub id: String,
    pub merchant_id: String,
    pub customer_id: String,
    pub secret: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EphemeralKey {
    pub id: String,
    pub merchant_id: String,
    pub customer_id: String,
    pub created_at: time::PrimitiveDateTime,
    pub expires: time::PrimitiveDateTime,
    pub secret: String,
}
