pub use api_models::{
    enums::{Connector, PayoutConnectors},
    payouts as payout_types,
};
pub use common_utils::request::RequestBody;
use data_models::payments::{payment_attempt::PaymentAttempt, payment_intent::PaymentIntent};
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
        payment_token: &mut Option<String>,
    ) -> RouterResult<Option<payments::PaymentMethodData>>;
}

#[async_trait::async_trait]
impl PaymentMethodRetrieve for Oss {
    async fn retrieve_payment_method(
        pm_data: &Option<payments::PaymentMethodData>,
        state: &AppState,
        payment_intent: &PaymentIntent,
        payment_attempt: &PaymentAttempt,
        payment_token: &mut Option<String>,
    ) -> RouterResult<Option<payments::PaymentMethodData>> {
        match pm_data {
            pm_opt @ Some(pm @ api::PaymentMethodData::Card(_)) => {
                if helpers::should_store_payment_method_data_in_vault(
                    &state.conf.temp_locker_disable_config,
                    payment_attempt.connector.clone(),
                    enums::PaymentMethod::Card,
                ) {
                    let parent_payment_method_token = helpers::store_in_vault_and_generate_ppmt(
                        state,
                        pm,
                        payment_intent,
                        payment_attempt,
                        enums::PaymentMethod::Card,
                    )
                    .await?;

                    *payment_token = Some(parent_payment_method_token);
                }
                Ok(pm_opt.to_owned())
            }
            pm @ Some(api::PaymentMethodData::PayLater(_)) => Ok(pm.to_owned()),
            pm @ Some(api::PaymentMethodData::Crypto(_)) => Ok(pm.to_owned()),
            pm @ Some(api::PaymentMethodData::BankDebit(_)) => Ok(pm.to_owned()),
            pm @ Some(api::PaymentMethodData::Upi(_)) => Ok(pm.to_owned()),
            pm @ Some(api::PaymentMethodData::Voucher(_)) => Ok(pm.to_owned()),
            pm @ Some(api::PaymentMethodData::Reward) => Ok(pm.to_owned()),
            pm @ Some(api::PaymentMethodData::CardRedirect(_)) => Ok(pm.to_owned()),
            pm @ Some(api::PaymentMethodData::GiftCard(_)) => Ok(pm.to_owned()),
            pm_opt @ Some(pm @ api::PaymentMethodData::BankTransfer(_)) => {
                if helpers::should_store_payment_method_data_in_vault(
                    &state.conf.temp_locker_disable_config,
                    payment_attempt.connector.clone(),
                    enums::PaymentMethod::BankTransfer,
                ) {
                    let parent_payment_method_token = helpers::store_in_vault_and_generate_ppmt(
                        state,
                        pm,
                        payment_intent,
                        payment_attempt,
                        enums::PaymentMethod::BankTransfer,
                    )
                    .await?;

                    *payment_token = Some(parent_payment_method_token);
                }

                Ok(pm_opt.to_owned())
            }
            pm_opt @ Some(pm @ api::PaymentMethodData::Wallet(_)) => {
                if helpers::should_store_payment_method_data_in_vault(
                    &state.conf.temp_locker_disable_config,
                    payment_attempt.connector.clone(),
                    enums::PaymentMethod::Wallet,
                ) {
                    let parent_payment_method_token = helpers::store_in_vault_and_generate_ppmt(
                        state,
                        pm,
                        payment_intent,
                        payment_attempt,
                        enums::PaymentMethod::Wallet,
                    )
                    .await?;

                    *payment_token = Some(parent_payment_method_token);
                }
                Ok(pm_opt.to_owned())
            }
            pm_opt @ Some(pm @ api::PaymentMethodData::BankRedirect(_)) => {
                if helpers::should_store_payment_method_data_in_vault(
                    &state.conf.temp_locker_disable_config,
                    payment_attempt.connector.clone(),
                    enums::PaymentMethod::BankRedirect,
                ) {
                    let parent_payment_method_token = helpers::store_in_vault_and_generate_ppmt(
                        state,
                        pm,
                        payment_intent,
                        payment_attempt,
                        enums::PaymentMethod::BankRedirect,
                    )
                    .await?;
                    *payment_token = Some(parent_payment_method_token);
                }
                Ok(pm_opt.to_owned())
            }
            _ => Ok(None),
        }
    }
}
