pub mod cards;
pub mod surcharge_decision_configs;
pub mod transformers;
pub mod vault;

use api_models::payments::CardToken;
pub use api_models::{
    enums::{Connector, PayoutConnectors},
    payouts as payout_types,
};
pub use common_utils::request::RequestBody;
use data_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
use diesel_models::enums;

use crate::{
    core::{
        errors::RouterResult,
        payments::helpers,
        pm_auth::{self as core_pm_auth},
    },
    routes::AppState,
    types::{
        api::{self, payments},
        domain, storage,
    },
};

pub struct Oss;

#[async_trait::async_trait]
pub trait PaymentMethodRetrieve {
    async fn retrieve_payment_method(
        pm_data: &Option<payments::PaymentMethodData>,
        state: &AppState,
        payment_intent: &PaymentIntent,
        payment_attempt: &PaymentAttempt,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<(Option<payments::PaymentMethodData>, Option<String>)>;

    async fn retrieve_payment_method_with_token(
        state: &AppState,
        key_store: &domain::MerchantKeyStore,
        token: &storage::PaymentTokenData,
        payment_intent: &PaymentIntent,
        card_token_data: Option<&CardToken>,
        customer: &Option<domain::Customer>,
    ) -> RouterResult<Option<(payments::PaymentMethodData, enums::PaymentMethod)>>;
}

#[async_trait::async_trait]
impl PaymentMethodRetrieve for Oss {
    async fn retrieve_payment_method(
        pm_data: &Option<payments::PaymentMethodData>,
        state: &AppState,
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

    async fn retrieve_payment_method_with_token(
        state: &AppState,
        merchant_key_store: &domain::MerchantKeyStore,
        token_data: &storage::PaymentTokenData,
        payment_intent: &PaymentIntent,
        card_token_data: Option<&CardToken>,
        customer: &Option<domain::Customer>,
    ) -> RouterResult<Option<(payments::PaymentMethodData, enums::PaymentMethod)>> {
        match token_data {
            storage::PaymentTokenData::TemporaryGeneric(generic_token) => {
                helpers::retrieve_payment_method_with_temporary_token(
                    state,
                    &generic_token.token,
                    payment_intent,
                    merchant_key_store,
                    card_token_data,
                )
                .await
            }

            storage::PaymentTokenData::Temporary(generic_token) => {
                helpers::retrieve_payment_method_with_temporary_token(
                    state,
                    &generic_token.token,
                    payment_intent,
                    merchant_key_store,
                    card_token_data,
                )
                .await
            }

            storage::PaymentTokenData::Permanent(card_token) => {
                helpers::retrieve_card_with_permanent_token(
                    state,
                    card_token.locker_id.as_ref().unwrap_or(&card_token.token),
                    payment_intent,
                    card_token_data,
                )
                .await
                .map(|card| Some((card, enums::PaymentMethod::Card)))
            }

            storage::PaymentTokenData::PermanentCard(card_token) => {
                helpers::retrieve_card_with_permanent_token(
                    state,
                    card_token.locker_id.as_ref().unwrap_or(&card_token.token),
                    payment_intent,
                    card_token_data,
                )
                .await
                .map(|card| Some((card, enums::PaymentMethod::Card)))
            }

            storage::PaymentTokenData::AuthBankDebit(auth_token) => {
                core_pm_auth::retrieve_payment_method_from_auth_service(
                    state,
                    merchant_key_store,
                    auth_token,
                    payment_intent,
                    customer,
                )
                .await
            }
        }
    }
}
