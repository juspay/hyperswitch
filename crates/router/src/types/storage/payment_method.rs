pub use diesel_models::payment_method::{
    PaymentMethod, PaymentMethodNew, PaymentMethodUpdate, PaymentMethodUpdateInternal,
    TokenizeCoreWorkflow,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypsTokenKind {
    Temporary,
    Permanent,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HyperswitchTokenData {
    pub token: String,
    pub kind: HypsTokenKind,
}

impl HyperswitchTokenData {
    pub fn temporary(token: String) -> Self {
        Self {
            token,
            kind: HypsTokenKind::Temporary,
        }
    }

    pub fn permanent(token: String) -> Self {
        Self {
            token,
            kind: HypsTokenKind::Permanent,
        }
    }
}
