use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct NewPayment {
    pub amount: BigDecimal,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Payment {
    pub id: u64,
    pub amount: BigDecimal,
}

#[derive(Debug)]
pub enum Verify {
    Ok,
    Error { message: String },
}
