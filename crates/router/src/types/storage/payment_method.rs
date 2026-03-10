use api_models::payment_methods;
use diesel_models::enums;
pub use diesel_models::payment_method::{
    PaymentMethod, PaymentMethodNew, PaymentMethodUpdate, PaymentMethodUpdateInternal,
    TokenizeCoreWorkflow,
};

use crate::types::{api, domain};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentTokenKind {
    Temporary,
    Permanent,
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CardTokenData {
    pub payment_method_id: Option<String>,
    pub locker_id: Option<String>,
    pub token: String,
    pub network_token_locker_id: Option<String>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CardTokenData {
    pub payment_method_id: common_utils::id_type::GlobalPaymentMethodId,
    pub locker_id: Option<String>,
    pub token: String,
}

#[derive(Debug, Clone, serde::Serialize, Default, serde::Deserialize)]
pub struct PaymentMethodDataWithId {
    pub payment_method: Option<enums::PaymentMethod>,
    pub payment_method_data: Option<domain::PaymentMethodData>,
    pub payment_method_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenericTokenData {
    pub token: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WalletTokenData {
    pub payment_method_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg(feature = "v1")]
pub enum PaymentTokenData {
    // The variants 'Temporary' and 'Permanent' are added for backwards compatibility
    // with any tokenized data present in Redis at the time of deployment of this change
    Temporary(GenericTokenData),
    TemporaryGeneric(GenericTokenData),
    Permanent(CardTokenData),
    PermanentCard(CardTokenData),
    AuthBankDebit(payment_methods::BankAccountTokenData),
    WalletToken(WalletTokenData),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg(feature = "v2")]
pub enum PaymentTokenData {
    TemporaryGeneric(GenericTokenData),
    PermanentCard(CardTokenData),
    AuthBankDebit(payment_methods::BankAccountTokenData),
}

impl PaymentTokenData {
    #[cfg(feature = "v1")]
    pub fn permanent_card(
        payment_method_id: Option<String>,
        locker_id: Option<String>,
        token: String,
        network_token_locker_id: Option<String>,
    ) -> Self {
        Self::PermanentCard(CardTokenData {
            payment_method_id,
            locker_id,
            token,
            network_token_locker_id,
        })
    }

    #[cfg(feature = "v2")]
    pub fn permanent_card(
        payment_method_id: common_utils::id_type::GlobalPaymentMethodId,
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

    #[cfg(feature = "v1")]
    pub fn wallet_token(payment_method_id: String) -> Self {
        Self::WalletToken(WalletTokenData { payment_method_id })
    }

    #[cfg(feature = "v1")]
    pub fn is_permanent_card(&self) -> bool {
        matches!(self, Self::PermanentCard(_) | Self::Permanent(_))
    }

    #[cfg(feature = "v2")]
    pub fn is_permanent_card(&self) -> bool {
        matches!(self, Self::PermanentCard(_))
    }
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodListContext {
    pub card_details: Option<api::CardDetailFromLocker>,
    pub hyperswitch_token_data: Option<PaymentTokenData>,
    #[cfg(feature = "payouts")]
    pub bank_transfer_details: Option<api::BankPayout>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PaymentMethodListContext {
    Card {
        card_details: api::CardDetailFromLocker,
        // TODO: Why can't these fields be mandatory?
        token_data: Option<PaymentTokenData>,
    },
    Bank {
        token_data: Option<PaymentTokenData>,
    },
    #[cfg(feature = "payouts")]
    BankTransfer {
        bank_transfer_details: api::BankPayout,
        token_data: Option<PaymentTokenData>,
    },
    TemporaryToken {
        token_data: Option<PaymentTokenData>,
    },
}

#[cfg(feature = "v2")]
impl PaymentMethodListContext {
    pub(crate) fn get_token_data(&self) -> Option<PaymentTokenData> {
        match self {
            Self::Card { token_data, .. }
            | Self::Bank { token_data }
            | Self::BankTransfer { token_data, .. }
            | Self::TemporaryToken { token_data } => token_data.clone(),
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentMethodStatusTrackingData {
    pub payment_method_id: String,
    pub prev_status: enums::PaymentMethodStatus,
    pub curr_status: enums::PaymentMethodStatus,
    pub merchant_id: common_utils::id_type::MerchantId,
}
