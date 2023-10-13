pub mod cards;
pub mod transformers;
pub mod vault;

pub use api_models::{
    enums::{Connector, PayoutConnectors},
    payouts as payout_types,
};
pub use common_utils::request::RequestBody;
use data_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
use diesel_models::enums;

use crate::{
    core::{errors::RouterResult, payments::helpers},
    routes::AppState,
    types::api::{self, payments},
};

pub struct Oss;

#[async_trait::async_trait]
pub trait PaymentMethodRetrieve {
    async fn retrieve_payment_method(
        pm_data: &Option<payments::PaymentMethodData>,
        state: &AppState,
        payment_intent: &PaymentIntent,
        payment_attempt: &PaymentAttempt,
    ) -> RouterResult<(Option<payments::PaymentMethodData>, Option<String>)>;
}

#[async_trait::async_trait]
impl PaymentMethodRetrieve for Oss {
    async fn retrieve_payment_method(
        pm_data: &Option<payments::PaymentMethodData>,
        state: &AppState,
        payment_intent: &PaymentIntent,
        payment_attempt: &PaymentAttempt,
    ) -> RouterResult<(Option<payments::PaymentMethodData>, Option<String>)> {
        match pm_data {
            pm_opt @ Some(pm @ api::PaymentMethodData::Card(_)) => {
                let payment_token = helpers::store_payment_method_data_in_vault(
                    state,
                    payment_attempt,
                    payment_intent,
                    enums::PaymentMethod::Card,
                    pm,
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
                )
                .await?;

                Ok((pm_opt.to_owned(), payment_token))
            }
            _ => Ok((None, None)),
        }
    }
}
