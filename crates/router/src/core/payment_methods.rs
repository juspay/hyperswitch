pub mod cards;
pub mod surcharge_decision_configs;
pub mod transformers;
pub mod utils;
pub mod vault;
pub use api_models::enums::Connector;
use api_models::payments::CardToken;
#[cfg(feature = "payouts")]
pub use api_models::{enums::PayoutConnectors, payouts as payout_types};
use common_utils::id_type;
use diesel_models::enums;
use hyperswitch_domain_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::RouterResult, payment_methods::transformers as pm_transformers, payments::helpers,
        pm_auth as core_pm_auth,
    },
    routes::SessionState,
    types::{
        api::{self, payments},
        domain, storage,
    },
};

#[instrument(skip_all)]
pub async fn retrieve_payment_method(
    pm_data: &Option<payments::PaymentMethodData>,
    state: &SessionState,
    payment_intent: &PaymentIntent,
    payment_attempt: &PaymentAttempt,
    merchant_key_store: &domain::MerchantKeyStore,
) -> RouterResult<(Option<payments::PaymentMethodData>, Option<String>)> {
    match pm_data {
        pm_opt @ Some(pm @ api::PaymentMethodData::Card(_)) => {
            let payment_token = helpers::store_payment_method_data_in_vault(
                state,
                payment_attempt,
                payment_intent,
                enums::PaymentMethod::Card,
                pm,
                merchant_key_store,
            )
            .await?;

            Ok((pm_opt.to_owned(), payment_token))
        }
        pm @ Some(api::PaymentMethodData::PayLater(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(api::PaymentMethodData::Crypto(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(api::PaymentMethodData::BankDebit(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(api::PaymentMethodData::Upi(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(api::PaymentMethodData::Voucher(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(api::PaymentMethodData::Reward) => Ok((pm.to_owned(), None)),
        pm @ Some(api::PaymentMethodData::CardRedirect(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(api::PaymentMethodData::GiftCard(_)) => Ok((pm.to_owned(), None)),
        pm_opt @ Some(pm @ api::PaymentMethodData::BankTransfer(_)) => {
            let payment_token = helpers::store_payment_method_data_in_vault(
                state,
                payment_attempt,
                payment_intent,
                enums::PaymentMethod::BankTransfer,
                pm,
                merchant_key_store,
            )
            .await?;

            Ok((pm_opt.to_owned(), payment_token))
        }
        pm_opt @ Some(pm @ api::PaymentMethodData::Wallet(_)) => {
            let payment_token = helpers::store_payment_method_data_in_vault(
                state,
                payment_attempt,
                payment_intent,
                enums::PaymentMethod::Wallet,
                pm,
                merchant_key_store,
            )
            .await?;

            Ok((pm_opt.to_owned(), payment_token))
        }
        pm_opt @ Some(pm @ api::PaymentMethodData::BankRedirect(_)) => {
            let payment_token = helpers::store_payment_method_data_in_vault(
                state,
                payment_attempt,
                payment_intent,
                enums::PaymentMethod::BankRedirect,
                pm,
                merchant_key_store,
            )
            .await?;

            Ok((pm_opt.to_owned(), payment_token))
        }
        _ => Ok((None, None)),
    }
}

#[instrument(skip_all)]
pub async fn retrieve_payment_method_with_token(
    state: &SessionState,
    merchant_key_store: &domain::MerchantKeyStore,
    token_data: &storage::PaymentTokenData,
    payment_intent: &PaymentIntent,
    card_token_data: Option<&CardToken>,
    customer: &Option<domain::Customer>,
    storage_scheme: common_enums::enums::MerchantStorageScheme,
) -> RouterResult<storage::PaymentMethodDataWithId> {
    let token = match token_data {
        storage::PaymentTokenData::TemporaryGeneric(generic_token) => {
            helpers::retrieve_payment_method_with_temporary_token(
                state,
                &generic_token.token,
                payment_intent,
                merchant_key_store,
                card_token_data,
            )
            .await?
            .map(
                |(payment_method_data, payment_method)| storage::PaymentMethodDataWithId {
                    payment_method_data: Some(payment_method_data),
                    payment_method: Some(payment_method),
                    payment_method_id: None,
                },
            )
            .unwrap_or_default()
        }

        storage::PaymentTokenData::Temporary(generic_token) => {
            helpers::retrieve_payment_method_with_temporary_token(
                state,
                &generic_token.token,
                payment_intent,
                merchant_key_store,
                card_token_data,
            )
            .await?
            .map(
                |(payment_method_data, payment_method)| storage::PaymentMethodDataWithId {
                    payment_method_data: Some(payment_method_data),
                    payment_method: Some(payment_method),
                    payment_method_id: None,
                },
            )
            .unwrap_or_default()
        }

        storage::PaymentTokenData::Permanent(card_token) => {
            helpers::retrieve_card_with_permanent_token(
                state,
                card_token.locker_id.as_ref().unwrap_or(&card_token.token),
                card_token
                    .payment_method_id
                    .as_ref()
                    .unwrap_or(&card_token.token),
                payment_intent,
                card_token_data,
                merchant_key_store,
                storage_scheme,
            )
            .await
            .map(|card| Some((card, enums::PaymentMethod::Card)))?
            .map(
                |(payment_method_data, payment_method)| storage::PaymentMethodDataWithId {
                    payment_method_data: Some(payment_method_data),
                    payment_method: Some(payment_method),
                    payment_method_id: Some(
                        card_token
                            .payment_method_id
                            .as_ref()
                            .unwrap_or(&card_token.token)
                            .to_string(),
                    ),
                },
            )
            .unwrap_or_default()
        }

        storage::PaymentTokenData::PermanentCard(card_token) => {
            helpers::retrieve_card_with_permanent_token(
                state,
                card_token.locker_id.as_ref().unwrap_or(&card_token.token),
                card_token
                    .payment_method_id
                    .as_ref()
                    .unwrap_or(&card_token.token),
                payment_intent,
                card_token_data,
                merchant_key_store,
                storage_scheme,
            )
            .await
            .map(|card| Some((card, enums::PaymentMethod::Card)))?
            .map(
                |(payment_method_data, payment_method)| storage::PaymentMethodDataWithId {
                    payment_method_data: Some(payment_method_data),
                    payment_method: Some(payment_method),
                    payment_method_id: Some(
                        card_token
                            .payment_method_id
                            .as_ref()
                            .unwrap_or(&card_token.token)
                            .to_string(),
                    ),
                },
            )
            .unwrap_or_default()
        }

        storage::PaymentTokenData::AuthBankDebit(auth_token) => {
            core_pm_auth::retrieve_payment_method_from_auth_service(
                state,
                merchant_key_store,
                auth_token,
                payment_intent,
                customer,
            )
            .await?
            .map(
                |(payment_method_data, payment_method)| storage::PaymentMethodDataWithId {
                    payment_method_data: Some(payment_method_data),
                    payment_method: Some(payment_method),
                    payment_method_id: None,
                },
            )
            .unwrap_or_default()
        }

        storage::PaymentTokenData::WalletToken(_) => storage::PaymentMethodDataWithId {
            payment_method: None,
            payment_method_data: None,
            payment_method_id: None,
        },
    };
    Ok(token)
}

pub struct PaymentMethodAddServer;
pub struct PaymentMethodAddClient;

#[async_trait::async_trait]
pub trait PaymentMethodAdd<T: Send + Sync> {
    async fn perform_preprocessing(
        &self,
        state: &SessionState,
        req: &api::PaymentMethodCreate,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        data: T,
    ) -> RouterResult<T>;
    async fn vault_payment_method(
        &self,
        state: &SessionState,
        req: &api::PaymentMethodCreate,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        data: T,
    ) -> RouterResult<T>;
    async fn handle_duplication(
        &self,
        state: &SessionState,
        req: &api::PaymentMethodCreate,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        data: T,
    ) -> RouterResult<T>;
}

// pub trait PaymentMethodAddData {
//     fn get_customer(&self) -> Arc<domain::Customer>;
//     fn get_payment_method(&self) -> Arc<diesel_models::PaymentMethod>;
//     fn get_response(&self) -> Option<api::PaymentMethodResponse>;
//     fn get_duplication_check(&self) -> Option<pm_transformers::DataDuplicationCheck>;
// }

pub struct PaymentMethodVaultingData {
    pub pm_id: Option<String>,
    pub payment_method: Option<diesel_models::PaymentMethod>,
    pub customer: Option<domain::Customer>,
    pub response: Option<api::PaymentMethodResponse>,
    pub duplication_check: Option<pm_transformers::DataDuplicationCheck>,
}
// impl PaymentMethodAddData for PaymentMethodPreprocessingData {}
// impl PaymentMethodAddData for PaymentMethodVaultingData {
//     fn get_customer(&self) -> Arc<domain::Customer> {
//         self.customer.clone()
//     }

//     fn get_response(&self) -> Option<api::PaymentMethodResponse> {
//         None
//     }

//     fn get_duplication_check(&self) -> Option<pm_transformers::DataDuplicationCheck> {
//         None
//     }

//     fn get_payment_method(&self) -> Arc<diesel_models::PaymentMethod> {
//         self.payment_method.clone()
//     }
// }
