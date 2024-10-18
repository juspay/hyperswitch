use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

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

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CardTokenData {
    pub payment_method_id: Option<String>,
    pub locker_id: Option<String>,
    pub token: String,
    pub network_token_locker_id: Option<String>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CardTokenData {
    pub payment_method_id: Option<common_utils::id_type::GlobalPaymentMethodId>,
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

impl PaymentTokenData {
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
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

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    pub fn permanent_card(
        payment_method_id: Option<common_utils::id_type::GlobalPaymentMethodId>,
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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodListContext {
    pub card_details: Option<api::CardDetailFromLocker>,
    pub hyperswitch_token_data: Option<PaymentTokenData>,
    #[cfg(feature = "payouts")]
    pub bank_transfer_details: Option<api::BankPayout>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReferenceRecord {
    pub connector_mandate_id: String,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub original_payment_authorized_amount: Option<i64>,
    pub original_payment_authorized_currency: Option<common_enums::Currency>,
    pub mandate_metadata: Option<serde_json::Value>,
    pub connector_mandate_status: Option<common_enums::ConnectorMandateStatus>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReference(
    pub HashMap<common_utils::id_type::MerchantConnectorAccountId, PaymentsMandateReferenceRecord>,
);

impl Deref for PaymentsMandateReference {
    type Target =
        HashMap<common_utils::id_type::MerchantConnectorAccountId, PaymentsMandateReferenceRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PaymentsMandateReference {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentMethodStatusTrackingData {
    pub payment_method_id: String,
    pub prev_status: enums::PaymentMethodStatus,
    pub curr_status: enums::PaymentMethodStatus,
    pub merchant_id: common_utils::id_type::MerchantId,
}
