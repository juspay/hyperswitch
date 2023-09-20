pub use diesel_models::payment_method::{
    PaymentMethod, PaymentMethodNew, PaymentMethodUpdate, PaymentMethodUpdateInternal,
    TokenizeCoreWorkflow,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentTokenKind {
    Temporary,
    Permanent,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentTokenData {
    pub token: String,
    pub kind: PaymentTokenKind,
}

impl PaymentTokenData {
    pub fn temporary(token: String) -> Self {
        Self {
            token,
            kind: PaymentTokenKind::Temporary,
        }
    }

    pub fn permanent(token: String) -> Self {
        Self {
            token,
            kind: PaymentTokenKind::Permanent,
        }
    }
}
