use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct NewPayment {
    pub amount: BigDecimal,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Payment {
    pub id: u64,
    pub amount: BigDecimal,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Verify {
    Ok,
    Error { message: String },
}
