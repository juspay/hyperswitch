use api_models::{enums as api_enums, payment_methods};
use common_utils::id_type;
use diesel_models::enums;
pub use diesel_models::payment_method::{
    PaymentMethod, PaymentMethodNew, PaymentMethodUpdate, PaymentMethodUpdateInternal,
    TokenizeCoreWorkflow,
};
use serde;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::types::api::{self, payments};

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

#[derive(Debug, Clone, serde::Serialize, Default, serde::Deserialize)]
pub struct PaymentMethodDataWithId {
    pub payment_method: Option<enums::PaymentMethod>,
    pub payment_method_data: Option<payments::PaymentMethodData>,
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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodListContext {
    pub card_details: Option<api::CardDetailFromLocker>,
    pub hyperswitch_token_data: PaymentTokenData,
    #[cfg(feature = "payouts")]
    pub bank_transfer_details: Option<api::BankPayout>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReferenceRecord {
    pub connector_mandate_id: String,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub original_payment_authorized_amount: Option<i64>,
    pub original_payment_authorized_currency: Option<common_enums::Currency>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReference(pub HashMap<String, PaymentsMandateReferenceRecord>);

impl Deref for PaymentsMandateReference {
    type Target = HashMap<String, PaymentsMandateReferenceRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PaymentsMandateReference {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct UpdateMandate {
    pub connector_mandate_id: Option<String>,
    pub connector_variant: api_enums::Connector,
    pub profile_id: String,
}
// impl PaymentsMandateReference {
//     pub fn collect_connector_mandate_id(&self) -> String {
//     let h = self.0.get()
//     }
// }

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentMethodMandateUpdateTrackingData {
    pub merchant_id: String,
    pub list_mca_ids: HashMap<String, UpdateMandate>,
    pub card_updation_obj: payment_methods::CardDetailUpdate,
    pub customer_id: id_type::CustomerId,
}
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentMethodStatusTrackingData {
    pub payment_method_id: String,
    pub prev_status: enums::PaymentMethodStatus,
    pub curr_status: enums::PaymentMethodStatus,
    pub merchant_id: String,
}
