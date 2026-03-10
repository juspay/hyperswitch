use api_models::{
    enums,
    payments::{
        self as payments_api, GetPaymentMethodType, PaymentMethodData,
        SplitPaymentMethodDataRequest,
    },
};
use common_enums::CallConnectorAction;
use common_utils::{
    ext_traits::{OptionExt, ValueExt},
    id_type,
    types::MinorUnit,
};
use error_stack::{report, Report, ResultExt};
use hyperswitch_domain_models::payments::{
    split_payments, HeaderPayload, PaymentConfirmData, PaymentIntent,
};
use masking::ExposeInterface;

use super::errors::StorageErrorExt;
use crate::{
    core::{
        errors::{self, RouterResponse},
        payment_method_balance,
        payments::{
            operations::{self, Operation, PaymentIntentConfirm},
            payments_operation_core,
            transformers::GenerateResponse,
        },
    },
    db::errors::RouterResult,
    routes::{app::ReqState, SessionState},
    types::{api, domain},
};

pub(crate) struct SplitPaymentResponseData {
    pub primary_payment_response_data: PaymentConfirmData<api::Authorize>,
    pub secondary_payment_response_data: Vec<PaymentConfirmData<api::Authorize>>,
}

impl SplitPaymentResponseData {
    fn get_intent_status(&self) -> common_enums::IntentStatus {
        let primary_status = common_enums::IntentStatus::from(
            self.primary_payment_response_data.payment_attempt.status,
        );

        let secondary_status_vec: Vec<common_enums::IntentStatus> = self
            .secondary_payment_response_data
            .iter()
            .map(|elem| common_enums::IntentStatus::from(elem.payment_attempt.status))
            .collect();

        // If all statuses are the same, return that status
        let all_same = secondary_status_vec
            .iter()
            .all(|status| *status == primary_status);

        if all_same {
            primary_status
        } else {
            // Return the last secondary status if array is not empty, otherwise primary status
            secondary_status_vec
                .last()
                .copied()
                .unwrap_or(primary_status)
        }
    }
}

/// This function has been written to support multiple gift cards + at most one non-gift card
/// payment method.
async fn get_payment_method_amount_split(
    state: &SessionState,
    payment_id: &id_type::GlobalPaymentId,
    request: &payments_api::PaymentsConfirmIntentRequest,
    payment_intent: &PaymentIntent,
) -> RouterResult<split_payments::PaymentMethodAmountSplit> {
    // This function is called inside split payments flow, so its mandatory to have split_payment_method_data
    let split_payment_method_data = request
        .split_payment_method_data
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("split_payment_method_data not found in split payments flow")?;

    // The primary/secondary payment method distinction is decided on the backend. Its irrelevant whether a payment_method
    // is received in the top level `payment_method_data` field or inside `split_payment_method_data`
    // Add the outer payment_method_data to the PMs inside split_payment_method_data to create a combined Vec and then segregate.
    let payment_method_data = SplitPaymentMethodDataRequest {
        payment_method_data: request
            .payment_method_data
            .payment_method_data
            .clone()
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method_data",
            })?,
        payment_method_type: request.payment_method_type,
        payment_method_subtype: request.payment_method_subtype,
    };

    let combined_pm_data: Vec<_> = split_payment_method_data
        .into_iter()
        .chain(std::iter::once(payment_method_data))
        .collect();

    let (gift_card_pm_data, non_gift_card_pm_data): (Vec<_>, Vec<_>) = combined_pm_data
        .into_iter()
        .partition(|pm_data| pm_data.payment_method_type.is_gift_card());

    // Validate at most one non-gift-card payment method
    if non_gift_card_pm_data.len() > 1 {
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "At most one non-gift card payment method is allowed".to_string(),
        })?
    }

    // It is possible that non-gift card payment method is not present, e.g. only 2 gift cards were provided
    let non_gift_card_pm_data = non_gift_card_pm_data.into_iter().next();

    let gift_card_data_vec = gift_card_pm_data
        .iter()
        .map(|elem| {
            if let PaymentMethodData::GiftCard(gift_card_data) = elem.payment_method_data.clone() {
                Ok(gift_card_data.as_ref().clone())
            } else {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Only Gift Card supported for Split Payments".to_string(),
                }))
            }
        })
        .collect::<RouterResult<Vec<_>>>()?;

    let balance_check_data_vec = gift_card_data_vec
        .iter()
        .cloned()
        .map(api_models::payments::BalanceCheckPaymentMethodData::GiftCard)
        .collect::<Vec<_>>();

    let balances = payment_method_balance::fetch_payment_methods_balances_from_redis(
        state,
        payment_id,
        &balance_check_data_vec,
    )
    .await?;

    // TODO: Add surcharge calculation when it is added in v2
    let total_amount = payment_intent.amount_details.calculate_net_amount();

    let mut remaining_to_allocate = total_amount;
    let pm_split_amt_tuple: Vec<(PaymentMethodData, MinorUnit)> = gift_card_data_vec
        .iter()
        .filter_map(|gift_card_card| {
            if remaining_to_allocate == MinorUnit::zero() {
                return None; // Payment already fully covered
            }

            let pm_balance_key = domain::PaymentMethodBalanceKey {
                payment_method_type: common_enums::PaymentMethod::GiftCard,
                payment_method_subtype: gift_card_card.get_payment_method_type(),
                payment_method_key: domain::GiftCardData::from(gift_card_card.clone())
                    .get_payment_method_key()
                    .change_context(errors::ApiErrorResponse::InvalidRequestData {
                        message: "Unable to get unique key for payment method".to_string(),
                    })
                    .ok()?
                    .expose(),
            };

            let pm_balance = balances
                .get(&pm_balance_key)
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Payment Method Balance not present in Redis")
                .ok()?;

            // Use minimum of available balance and remaining amount
            let amount_to_use = pm_balance.balance.min(remaining_to_allocate);
            remaining_to_allocate = remaining_to_allocate - amount_to_use;

            Some(Ok((
                PaymentMethodData::GiftCard(Box::new(gift_card_card.to_owned())),
                amount_to_use,
            )))
        })
        .collect::<RouterResult<Vec<_>>>()?;

    // If the gift card balances are not sufficient for payment, use the non-gift card payment method
    // for the remaining amount
    if remaining_to_allocate > MinorUnit::zero() {
        let non_gift_card_pm_data = non_gift_card_pm_data
            .ok_or(errors::ApiErrorResponse::InvalidRequestData {
                message: "Requires additional payment method data".to_string(),
            })?
            .payment_method_data;

        Ok(split_payments::PaymentMethodAmountSplit {
            balance_pm_split: pm_split_amt_tuple,
            non_balance_pm_split: Some((non_gift_card_pm_data, remaining_to_allocate)),
        })
    } else {
        Ok(split_payments::PaymentMethodAmountSplit {
            balance_pm_split: pm_split_amt_tuple,
            non_balance_pm_split: None,
        })
    }
}

pub(crate) async fn split_payments_execute_core(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    request: payments_api::PaymentsConfirmIntentRequest,
    header_payload: HeaderPayload,
    payment_id: id_type::GlobalPaymentId,
) -> RouterResponse<payments_api::PaymentsResponse> {
    let db = &*state.store;

    let payment_intent = db
        .find_payment_intent_by_id(
            &payment_id,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    if !payment_intent.supports_split_payments() {
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "Split Payments not enabled in Payment Intent".to_string(),
        })?
    }

    let cell_id = state.conf.cell_information.id.clone();

    let attempts_group_id = id_type::GlobalAttemptGroupId::generate(&cell_id);

    // Change the active_attempt_id_type of PaymentIntent to `GroupID`. This indicates that the customer
    // has attempted a split payment for this intent
    let payment_intent_update =
        hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::AttemptGroupUpdate {
            updated_by: platform
                .get_processor()
                .get_account()
                .storage_scheme
                .to_string(),
            active_attempt_id_type: enums::ActiveAttemptIDType::GroupID,
            active_attempts_group_id: attempts_group_id.clone(),
        };

    let payment_intent = db
        .update_payment_intent(
            payment_intent,
            payment_intent_update,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to update payment intent")?;

    let pm_amount_split =
        get_payment_method_amount_split(&state, &payment_id, &request, &payment_intent).await?;

    let (
        primary_pm_response,
        connector_http_status_code,
        external_latency,
        connector_response_data,
    ) = {
        // If a non-balance Payment Method is present, we will execute that first, otherwise we will execute
        // a balance Payment Method
        let (payment_method_data, amount) = pm_amount_split
            .non_balance_pm_split
            .clone()
            .or_else(|| pm_amount_split.balance_pm_split.first().cloned())
            .get_required_value("payment method amount split")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("At least one payment method is required")?;

        let operation = PaymentIntentConfirm;

        let get_tracker_response: operations::GetTrackerResponse<
            PaymentConfirmData<api::Authorize>,
        > = operation
            .to_get_tracker()?
            .get_trackers_for_split_payments(
                &state,
                &payment_id,
                &request,
                &platform,
                &profile,
                &header_payload,
                (payment_method_data, amount),
                &attempts_group_id,
            )
            .await?;

        let (
            payment_data,
            _req,
            _customer,
            connector_http_status_code,
            external_latency,
            connector_response_data,
        ) = Box::pin(payments_operation_core(
            &state,
            req_state.clone(),
            platform.clone(),
            &profile,
            operation,
            request.clone(),
            get_tracker_response,
            CallConnectorAction::Trigger,
            header_payload.clone(),
        ))
        .await?;

        // payments_operation_core marks the intent as succeeded when the attempt is succesful
        // However, for split case, we can't mark the intent as succesful until all the attempts
        // have succeeded, so reverting the state of Payment Intent
        if payment_data.payment_intent.is_succeeded() {
            let payment_intent_update =
            hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::SplitPaymentStatusUpdate {
                status: common_enums::IntentStatus::RequiresPaymentMethod,
                updated_by: platform
                    .get_processor()
                    .get_account()
                    .storage_scheme
                    .to_string(),
            };

            let updated_payment_intent = db
                .update_payment_intent(
                    payment_data.payment_intent.clone(),
                    payment_intent_update,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to update payment intent")?;
        }

        (
            payment_data,
            connector_http_status_code,
            external_latency,
            connector_response_data,
        )
    };

    let mut split_pm_response_data = SplitPaymentResponseData {
        primary_payment_response_data: primary_pm_response,
        secondary_payment_response_data: vec![],
    };

    // We have executed the primary payment method, now get a vector of the secondary payment methods
    // If we had a non-balance payment method, it was executed first, so the remaining ones are the balance PMs
    // otherwise the first balance payment method was executed, so remove it from the remaining PMs
    let remaining_pm_amount_split = if pm_amount_split.non_balance_pm_split.is_some() {
        pm_amount_split.balance_pm_split
    } else {
        pm_amount_split
            .balance_pm_split
            .iter()
            .skip(1)
            .cloned()
            .collect()
    };

    for (payment_method_data, amount) in remaining_pm_amount_split {
        let operation = PaymentIntentConfirm;

        let get_tracker_response: operations::GetTrackerResponse<
            PaymentConfirmData<api::Authorize>,
        > = operation
            .to_get_tracker()?
            .get_trackers_for_split_payments(
                &state,
                &payment_id,
                &request,
                &platform,
                &profile,
                &header_payload,
                (payment_method_data.to_owned(), amount.to_owned()),
                &attempts_group_id,
            )
            .await?;

        let (
            payment_data,
            _req,
            _customer,
            _connector_http_status_code,
            _external_latency,
            _connector_response_data,
        ) = Box::pin(payments_operation_core(
            &state,
            req_state.clone(),
            platform.clone(),
            &profile,
            operation,
            request.clone(),
            get_tracker_response,
            CallConnectorAction::Trigger,
            header_payload.clone(),
        ))
        .await?;

        // payments_operation_core marks the intent as succeeded when the attempt is succesful
        // However, for split case, we can't mark the intent as succesful until all the attempts
        // have succeeded, so reverting the state of Payment Intent
        if payment_data.payment_intent.is_succeeded() {
            let payment_intent_update =
            hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::SplitPaymentStatusUpdate {
                status: common_enums::IntentStatus::RequiresPaymentMethod,
                updated_by: platform
                    .get_processor()
                    .get_account()
                    .storage_scheme
                    .to_string(),
            };

            let _updated_payment_intent = db
                .update_payment_intent(
                    payment_data.payment_intent.clone(),
                    payment_intent_update,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to update payment intent")?;
        } else {
            split_pm_response_data
                .secondary_payment_response_data
                .push(payment_data);

            // Exit the loop if a payment failed
            break;
        }

        split_pm_response_data
            .secondary_payment_response_data
            .push(payment_data);
    }

    let payment_intent_update =
        hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::SplitPaymentStatusUpdate {
            status: split_pm_response_data.get_intent_status(),
            updated_by: platform
                .get_processor()
                .get_account()
                .storage_scheme
                .to_string(),
        };

    let _updated_payment_intent = db
        .update_payment_intent(
            payment_intent,
            payment_intent_update,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to update payment intent")?;

    split_pm_response_data.generate_response(
        &state,
        connector_http_status_code,
        external_latency,
        header_payload.x_hs_latency,
        &platform,
        &profile,
        Some(connector_response_data),
    )
}

/// Construct the domain model from the ConfirmIntentRequest and PaymentIntent
#[cfg(feature = "v2")]
pub async fn create_domain_model_for_split_payment(
    payment_intent: &PaymentIntent,
    cell_id: id_type::CellId,
    storage_scheme: enums::MerchantStorageScheme,
    request: &api_models::payments::PaymentsConfirmIntentRequest,
    encrypted_data: hyperswitch_domain_models::payments::payment_attempt::DecryptedPaymentAttempt,
    split_amount: MinorUnit,
    attempts_group_id: &id_type::GlobalAttemptGroupId,
) -> common_utils::errors::CustomResult<domain::PaymentAttempt, errors::ApiErrorResponse> {
    let id = id_type::GlobalAttemptId::generate(&cell_id);
    let intent_amount_details = payment_intent.amount_details.clone();

    let attempt_amount_details =
        intent_amount_details.create_split_attempt_amount_details(request, split_amount);

    let now = common_utils::date_time::now();

    let payment_method_billing_address = encrypted_data
        .payment_method_billing_address
        .as_ref()
        .map(|data| {
            data.clone()
                .deserialize_inner_value(|value| value.parse_value("Address"))
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to decode billing address")?;

    let connector_token = Some(diesel_models::ConnectorTokenDetails {
        connector_mandate_id: None,
        connector_token_request_reference_id: Some(common_utils::generate_id_with_len(
            hyperswitch_domain_models::consts::CONNECTOR_MANDATE_REQUEST_REFERENCE_ID_LENGTH,
        )),
    });

    let authentication_type = payment_intent.authentication_type.unwrap_or_default();

    Ok(domain::PaymentAttempt {
        payment_id: payment_intent.id.clone(),
        merchant_id: payment_intent.merchant_id.clone(),
        attempts_group_id: Some(attempts_group_id.to_owned()),
        amount_details: attempt_amount_details,
        status: common_enums::AttemptStatus::Started,
        // This will be decided by the routing algorithm and updated in update trackers
        // right before calling the connector
        connector: None,
        authentication_type,
        created_at: now,
        modified_at: now,
        last_synced: None,
        cancellation_reason: None,
        browser_info: request.browser_info.clone(),
        payment_token: request.payment_token.clone(),
        connector_metadata: None,
        payment_experience: None,
        payment_method_data: None,
        routing_result: None,
        preprocessing_step_id: None,
        multiple_capture_count: None,
        connector_response_reference_id: None,
        updated_by: storage_scheme.to_string(),
        redirection_data: None,
        encoded_data: None,
        merchant_connector_id: None,
        external_three_ds_authentication_attempted: None,
        authentication_connector: None,
        authentication_id: None,
        fingerprint_id: None,
        charges: None,
        client_source: None,
        client_version: None,
        customer_acceptance: request
            .customer_acceptance
            .clone()
            .map(masking::Secret::new),
        profile_id: payment_intent.profile_id.clone(),
        organization_id: payment_intent.organization_id.clone(),
        payment_method_type: request.payment_method_type,
        payment_method_id: request.payment_method_id.clone(),
        connector_payment_id: None,
        payment_method_subtype: request.payment_method_subtype,
        authentication_applied: None,
        external_reference_id: None,
        payment_method_billing_address,
        error: None,
        connector_token_details: connector_token,
        id,
        card_discovery: None,
        feature_metadata: None,
        processor_merchant_id: payment_intent.merchant_id.clone(),
        created_by: None,
        connector_request_reference_id: None,
        network_transaction_id: None,
        authorized_amount: None,
    })
}
