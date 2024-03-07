use std::collections::HashMap;

use api_models::payment_methods;
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
pub struct CardTokenData {
    pub payment_method_id: Option<String>,
    pub locker_id: Option<String>,
    pub token: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenericTokenData {
    pub token: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WalletTokenData {
    pub payment_method_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PaymentTokenData {
    // The variants 'Temporary' and 'Permanent' are added for backwards compatibility
    // with any tokenized data present in Redis at the time of deployment of this change
    Temporary(GenericTokenData),
    TemporaryGeneric(GenericTokenData),
    Permanent(CardTokenData),
    PermanentCard(CardTokenData),
    AuthBankDebit(payment_methods::BankAccountConnectorDetails),
    WalletToken(WalletTokenData),
}

impl PaymentTokenData {
    pub fn permanent_card(
        payment_method_id: Option<String>,
        locker_id: Option<String>,
        token: String,
    ) -> Self {
        Self::PermanentCard(CardTokenData {
            payment_method_id,
            locker_id,
            token,
        })
    }

    pub fn temporary_generic(token: String) -> Self {
        Self::TemporaryGeneric(GenericTokenData { token })
    }

    pub fn wallet_token(payment_method_id: String) -> Self {
        Self::WalletToken(WalletTokenData { payment_method_id })
    }

    pub fn get_token(&self) -> &String {
        match &self {
            Self::Temporary(data) | Self::TemporaryGeneric(data) => &data.token,
            Self::Permanent(data) | Self::PermanentCard(data) => &data.token,
            Self::WalletToken(data) => &data.payment_method_id,
            Self::AuthBankDebit(auth_details) => match auth_details.access_token {
                payment_methods::BankAccountAccessCreds::AccessToken(ref token) => token,
            },
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReference(pub HashMap<String, Option<String>>);
