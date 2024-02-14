use std::{str::FromStr, vec::IntoIter};

use api_models::payouts::PayoutCreateRequest;
use error_stack::{IntoReport, ResultExt};
use router_env::{
    logger,
    tracing::{self, instrument},
};

use super::{call_connector_payout, PayoutData};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payouts,
    },
    db::StorageInterface,
    routes::{self, app, metrics},
    types::{api, domain, storage},
    utils,
};

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn do_gsm_actions(
    state: &app::AppState,
    mut connectors: IntoIter<api::ConnectorData>,
    original_connector_data: api::ConnectorData,
    mut payout_data: PayoutData,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &PayoutCreateRequest,
) -> RouterResult<PayoutData> {
    let mut retries = None;

    metrics::AUTO_PAYOUT_RETRY_ELIGIBLE_REQUEST_COUNT.add(&metrics::CONTEXT, 1, &[]);

    let mut connector = original_connector_data;

    loop {
        let gsm = get_gsm(state, &connector, &payout_data).await?;

        match get_gsm_decision(gsm) {
            api_models::gsm::GsmDecision::Retry => {
                retries = get_retries(state, retries, &merchant_account.merchant_id).await;

                if retries.is_none() || retries == Some(0) {
                    metrics::AUTO_PAYOUT_RETRY_EXHAUSTED_COUNT.add(&metrics::CONTEXT, 1, &[]);
                    logger::info!("retries exhausted for auto_retry payment");
                    break;
                }

                if connectors.len() == 0 {
                    logger::info!("connectors exhausted for auto_retry payment");
                    metrics::AUTO_PAYOUT_RETRY_EXHAUSTED_COUNT.add(&metrics::CONTEXT, 1, &[]);
                    break;
                }

                connector = super::get_next_connector(&mut connectors)?;

                payout_data = do_retry(
                    &state.clone(),
                    connector.to_owned(),
                    merchant_account,
                    key_store,
                    payout_data,
                    req,
                )
                .await?;

                retries = retries.map(|i| i - 1);
            }
            api_models::gsm::GsmDecision::Requeue => {
                Err(errors::ApiErrorResponse::NotImplemented {
                    message: errors::api_error_response::NotImplementedMessage::Reason(
                        "Requeue not implemented".to_string(),
                    ),
                })
                .into_report()?
            }
            api_models::gsm::GsmDecision::DoDefault => break,
        }
    }
    Ok(payout_data)
}

#[instrument(skip_all)]
pub async fn get_retries(
    state: &app::AppState,
    retries: Option<i32>,
    merchant_id: &str,
) -> Option<i32> {
    match retries {
        Some(retries) => Some(retries),
        None => {
            let key = format!("max_auto_payout_retries_enabled_{merchant_id}");
            let db = &*state.store;
            db.find_config_by_key(key.as_str())
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .and_then(|retries_config| {
                    retries_config
                        .config
                        .parse::<i32>()
                        .into_report()
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Retries config parsing failed")
                })
                .map_err(|err| {
                    logger::error!(retries_error=?err);
                    None::<i32>
                })
                .ok()
        }
    }
}

#[instrument(skip_all)]
pub async fn get_gsm(
    state: &app::AppState,
    original_connector_data: &api::ConnectorData,
    payout_data: &PayoutData,
) -> RouterResult<Option<storage::gsm::GatewayStatusMap>> {
    let error_code = payout_data.payout_attempt.error_code.to_owned();
    let error_message = payout_data.payout_attempt.error_message.to_owned();
    let connector_name = Some(original_connector_data.connector_name.to_string());
    let flow = "payout_flow".to_string();

    Ok(
        payouts::helpers::get_gsm_record(state, error_code, error_message, connector_name, flow)
            .await,
    )
}

#[instrument(skip_all)]
pub fn get_gsm_decision(
    option_gsm: Option<storage::gsm::GatewayStatusMap>,
) -> api_models::gsm::GsmDecision {
    let option_gsm_decision = option_gsm
            .and_then(|gsm| {
                api_models::gsm::GsmDecision::from_str(gsm.decision.as_str())
                    .into_report()
                    .map_err(|err| {
                        let api_error = err.change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("gsm decision parsing failed");
                        logger::warn!(get_gsm_decision_parse_error=?api_error, "error fetching gsm decision");
                        api_error
                    })
                    .ok()
            });

    if option_gsm_decision.is_some() {
        metrics::AUTO_PAYOUT_RETRY_GSM_MATCH_COUNT.add(&metrics::CONTEXT, 1, &[]);
    }
    option_gsm_decision.unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn do_retry(
    state: &routes::AppState,
    connector: api::ConnectorData,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    mut payout_data: PayoutData,
    req: &PayoutCreateRequest,
) -> RouterResult<PayoutData> {
    metrics::AUTO_RETRY_PAYOUT_COUNT.add(&metrics::CONTEXT, 1, &[]);

    modify_trackers(state, &connector, merchant_account, &mut payout_data).await?;

    call_connector_payout(
        state,
        merchant_account,
        key_store,
        req,
        &connector,
        &mut payout_data,
    )
    .await
}

#[instrument(skip_all)]
pub async fn modify_trackers(
    state: &routes::AppState,
    connector: &api::ConnectorData,
    merchant_account: &domain::MerchantAccount,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    let new_attempt_count = payout_data.payouts.attempt_count + 1;

    let db = &*state.store;

    // update payout table's attempt count
    let payouts = payout_data.payouts.to_owned();
    let updated_payouts = storage::PayoutsUpdate::AttemptCountUpdate {
        attempt_count: new_attempt_count,
    };

    let payout_id = payouts.payout_id.clone();
    let merchant_id = &merchant_account.merchant_id;
    payout_data.payouts = db
        .update_payout_by_merchant_id_payout_id(merchant_id, &payout_id.to_owned(), updated_payouts)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating payouts")?;

    let payout_attempt_id =
        utils::get_payment_attempt_id(payout_id.to_owned(), payout_data.payouts.attempt_count);

    let new_payout_attempt_req = storage::PayoutAttemptNew::default()
        .set_payout_attempt_id(payout_attempt_id.to_string())
        .set_payout_id(payout_id.to_owned())
        .set_customer_id(payout_data.payout_attempt.customer_id.to_owned())
        .set_connector(Some(connector.connector_name.to_string()))
        .set_merchant_id(payout_data.payout_attempt.merchant_id.to_owned())
        .set_address_id(payout_data.payout_attempt.address_id.to_owned())
        .set_business_country(payout_data.payout_attempt.business_country.to_owned())
        .set_business_label(payout_data.payout_attempt.business_label.to_owned())
        .set_payout_token(payout_data.payout_attempt.payout_token.to_owned())
        .set_created_at(Some(common_utils::date_time::now()))
        .set_last_modified_at(Some(common_utils::date_time::now()))
        .set_profile_id(Some(payout_data.payout_attempt.profile_id.to_string()))
        .to_owned();
    payout_data.payout_attempt = db
        .insert_payout_attempt(new_payout_attempt_req)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayout { payout_id })
        .attach_printable("Error inserting payouts in db")?;

    payout_data.merchant_connector_account = None;

    Ok(())
}

pub async fn config_should_call_gsm_payout(
    db: &dyn StorageInterface,
    merchant_id: &String,
) -> bool {
    let config = db
        .find_config_by_key_unwrap_or(
            format!("should_call_gsm_payout_{}", merchant_id).as_str(),
            Some("false".to_string()),
        )
        .await;
    match config {
        Ok(conf) => conf.config == "true",
        Err(err) => {
            logger::error!("{err}");
            false
        }
    }
}

pub trait GsmValidation {
    // TODO : move this function to appropriate place later.
    fn should_call_gsm(&self) -> bool;
}

impl GsmValidation for PayoutData {
    #[inline(always)]
    fn should_call_gsm(&self) -> bool {
        match self.payout_attempt.status {
            common_enums::PayoutStatus::Success
            | common_enums::PayoutStatus::Cancelled
            | common_enums::PayoutStatus::Pending
            | common_enums::PayoutStatus::Ineligible
            | common_enums::PayoutStatus::RequiresCreation
            | common_enums::PayoutStatus::RequiresPayoutMethodData
            | common_enums::PayoutStatus::RequiresFulfillment => false,
            common_enums::PayoutStatus::Failed => true,
        }
    }
}
