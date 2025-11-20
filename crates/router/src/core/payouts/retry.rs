use std::vec::IntoIter;

use common_enums::PayoutRetryType;
use error_stack::ResultExt;
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
pub async fn do_gsm_multiple_connector_actions(
    state: &app::SessionState,
    mut connectors_routing_data: IntoIter<api::ConnectorRoutingData>,
    original_connector_data: api::ConnectorData,
    payout_data: &mut PayoutData,
    platform: &domain::Platform,
) -> RouterResult<()> {
    let mut retries = None;

    metrics::AUTO_PAYOUT_RETRY_ELIGIBLE_REQUEST_COUNT.add(1, &[]);

    let mut connector = original_connector_data;

    loop {
        let gsm = get_gsm(state, &connector, payout_data).await?;

        match get_gsm_decision(gsm) {
            common_enums::GsmDecision::Retry => {
                retries = get_retries(
                    state,
                    retries,
                    platform.get_processor().get_account().get_id(),
                    PayoutRetryType::MultiConnector,
                )
                .await;

                if retries.is_none() || retries == Some(0) {
                    metrics::AUTO_PAYOUT_RETRY_EXHAUSTED_COUNT.add(1, &[]);
                    logger::info!("retries exhausted for auto_retry payout");
                    break;
                }

                if connectors_routing_data.len() == 0 {
                    logger::info!("connectors exhausted for auto_retry payout");
                    metrics::AUTO_PAYOUT_RETRY_EXHAUSTED_COUNT.add(1, &[]);
                    break;
                }

                connector = super::get_next_connector(&mut connectors_routing_data)?.connector_data;

                Box::pin(do_retry(
                    &state.clone(),
                    connector.to_owned(),
                    platform,
                    payout_data,
                ))
                .await?;

                retries = retries.map(|i| i - 1);
            }
            common_enums::GsmDecision::DoDefault => break,
        }
    }
    Ok(())
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn do_gsm_single_connector_actions(
    state: &app::SessionState,
    original_connector_data: api::ConnectorData,
    payout_data: &mut PayoutData,
    platform: &domain::Platform,
) -> RouterResult<()> {
    let mut retries = None;

    metrics::AUTO_PAYOUT_RETRY_ELIGIBLE_REQUEST_COUNT.add(1, &[]);

    let mut previous_gsm = None; // to compare previous status

    loop {
        let gsm = get_gsm(state, &original_connector_data, payout_data).await?;

        // if the error config is same as previous, we break out of the loop
        if gsm == previous_gsm {
            break;
        }
        previous_gsm.clone_from(&gsm);

        match get_gsm_decision(gsm) {
            common_enums::GsmDecision::Retry => {
                retries = get_retries(
                    state,
                    retries,
                    platform.get_processor().get_account().get_id(),
                    PayoutRetryType::SingleConnector,
                )
                .await;

                if retries.is_none() || retries == Some(0) {
                    metrics::AUTO_PAYOUT_RETRY_EXHAUSTED_COUNT.add(1, &[]);
                    logger::info!("retries exhausted for auto_retry payment");
                    break;
                }

                Box::pin(do_retry(
                    &state.clone(),
                    original_connector_data.to_owned(),
                    platform,
                    payout_data,
                ))
                .await?;

                retries = retries.map(|i| i - 1);
            }
            common_enums::GsmDecision::DoDefault => break,
        }
    }
    Ok(())
}

#[instrument(skip_all)]
pub async fn get_retries(
    state: &app::SessionState,
    retries: Option<i32>,
    merchant_id: &common_utils::id_type::MerchantId,
    retry_type: PayoutRetryType,
) -> Option<i32> {
    match retries {
        Some(retries) => Some(retries),
        None => {
            let key = merchant_id.get_max_auto_single_connector_payout_retries_enabled(retry_type);
            let db = &*state.store;
            db.find_config_by_key(key.as_str())
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .and_then(|retries_config| {
                    retries_config
                        .config
                        .parse::<i32>()
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
    state: &app::SessionState,
    original_connector_data: &api::ConnectorData,
    payout_data: &PayoutData,
) -> RouterResult<Option<hyperswitch_domain_models::gsm::GatewayStatusMap>> {
    let error_code = payout_data.payout_attempt.error_code.to_owned();
    let error_message = payout_data.payout_attempt.error_message.to_owned();
    let connector_name = Some(original_connector_data.connector_name.to_string());

    Ok(payouts::helpers::get_gsm_record(
        state,
        error_code,
        error_message,
        connector_name,
        common_utils::consts::PAYOUT_FLOW_STR,
    )
    .await)
}

#[instrument(skip_all)]
pub fn get_gsm_decision(
    option_gsm: Option<hyperswitch_domain_models::gsm::GatewayStatusMap>,
) -> common_enums::GsmDecision {
    let option_gsm_decision = option_gsm.map(|gsm| gsm.feature_data.get_decision());

    if option_gsm_decision.is_some() {
        metrics::AUTO_PAYOUT_RETRY_GSM_MATCH_COUNT.add(1, &[]);
    }
    option_gsm_decision.unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn do_retry(
    state: &routes::SessionState,
    connector: api::ConnectorData,
    platform: &domain::Platform,
    payout_data: &mut PayoutData,
) -> RouterResult<()> {
    metrics::AUTO_RETRY_PAYOUT_COUNT.add(1, &[]);

    modify_trackers(state, &connector, platform, payout_data).await?;

    Box::pin(call_connector_payout(
        state,
        platform,
        &connector,
        payout_data,
    ))
    .await
}

#[instrument(skip_all)]
pub async fn modify_trackers(
    state: &routes::SessionState,
    connector: &api::ConnectorData,
    platform: &domain::Platform,
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
    payout_data.payouts = db
        .update_payout(
            &payout_data.payouts,
            updated_payouts,
            &payout_data.payout_attempt,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating payouts")?;

    let payout_attempt_id = utils::get_payout_attempt_id(
        payout_id.get_string_repr(),
        payout_data.payouts.attempt_count,
    );

    let payout_attempt_req = storage::PayoutAttemptNew {
        payout_attempt_id: payout_attempt_id.to_string(),
        payout_id: payout_id.to_owned(),
        merchant_order_reference_id: payout_data
            .payout_attempt
            .merchant_order_reference_id
            .clone(),
        customer_id: payout_data.payout_attempt.customer_id.to_owned(),
        connector: Some(connector.connector_name.to_string()),
        merchant_id: payout_data.payout_attempt.merchant_id.to_owned(),
        address_id: payout_data.payout_attempt.address_id.to_owned(),
        business_country: payout_data.payout_attempt.business_country.to_owned(),
        business_label: payout_data.payout_attempt.business_label.to_owned(),
        payout_token: payout_data.payout_attempt.payout_token.to_owned(),
        profile_id: payout_data.payout_attempt.profile_id.to_owned(),
        connector_payout_id: None,
        status: common_enums::PayoutStatus::default(),
        is_eligible: None,
        error_message: None,
        error_code: None,
        created_at: common_utils::date_time::now(),
        last_modified_at: common_utils::date_time::now(),
        merchant_connector_id: None,
        routing_info: None,
        unified_code: None,
        unified_message: None,
        additional_payout_method_data: payout_data
            .payout_attempt
            .additional_payout_method_data
            .to_owned(),
        payout_connector_metadata: None,
    };
    payout_data.payout_attempt = db
        .insert_payout_attempt(
            payout_attempt_req,
            &payouts,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicatePayout { payout_id })
        .attach_printable("Error inserting payouts in db")?;

    payout_data.merchant_connector_account = None;

    Ok(())
}

pub async fn config_should_call_gsm_payout(
    db: &dyn StorageInterface,
    merchant_id: &common_utils::id_type::MerchantId,
    retry_type: PayoutRetryType,
) -> bool {
    let key = merchant_id.get_should_call_gsm_payout_key(retry_type);
    let config = db
        .find_config_by_key_unwrap_or(key.as_str(), Some("false".to_string()))
        .await;
    match config {
        Ok(conf) => conf.config == "true",
        Err(error) => {
            logger::error!(?error);
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
            | common_enums::PayoutStatus::RequiresConfirmation
            | common_enums::PayoutStatus::Cancelled
            | common_enums::PayoutStatus::Pending
            | common_enums::PayoutStatus::Initiated
            | common_enums::PayoutStatus::Reversed
            | common_enums::PayoutStatus::Expired
            | common_enums::PayoutStatus::Ineligible
            | common_enums::PayoutStatus::RequiresCreation
            | common_enums::PayoutStatus::RequiresPayoutMethodData
            | common_enums::PayoutStatus::RequiresVendorAccountCreation
            | common_enums::PayoutStatus::RequiresFulfillment => false,
            common_enums::PayoutStatus::Failed => true,
        }
    }
}
