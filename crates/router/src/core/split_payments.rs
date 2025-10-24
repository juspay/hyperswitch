use api_models::{
    enums,
    payments::{
        self as payments_api, GetPaymentMethodType, PaymentMethodData,
        SplitPaymentMethodDataRequest,
    },
};
use common_enums::{CallConnectorAction, SplitTxnsEnabled};
use common_utils::{ext_traits::OptionExt, id_type, types::MinorUnit};
use error_stack::{report, Report, ResultExt};
use hyperswitch_domain_models::payments::{HeaderPayload, PaymentConfirmData, PaymentIntent};
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

/// There can be multiple gift-cards, but at most one non-gift card PM
struct PaymentMethodAmountSplit {
    pub balance_pm_split: Vec<(PaymentMethodData, MinorUnit)>,
    pub non_balance_pm_split: Option<(PaymentMethodData, MinorUnit)>,
}

/// This function has been written to support multiple gift cards + at most one non-gift card
/// payment method.
async fn get_payment_method_and_amount_split(
    state: &SessionState,
    payment_id: &id_type::GlobalPaymentId,
    request: &payments_api::PaymentsConfirmIntentRequest,
    payment_intent: &PaymentIntent,
) -> RouterResult<PaymentMethodAmountSplit> {
    let split_payment_methods_data = request.split_payment_method_data.clone().ok_or(
        errors::ApiErrorResponse::MissingRequiredField {
            field_name: "split_payment_method_data",
        },
    )?;

    let outer_payment_method_data = SplitPaymentMethodDataRequest {
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

    let combined_pm_data: Vec<_> = split_payment_methods_data
        .into_iter()
        .chain(std::iter::once(outer_payment_method_data))
        .collect();

    let (non_gift_card_pm_data, gift_card_pm_data): (Vec<_>, Vec<_>) = combined_pm_data
        .into_iter()
        .partition(|pm_data| pm_data.payment_method_type != enums::PaymentMethod::GiftCard);

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

    let total_balances = balances
        .iter()
        .fold(MinorUnit::zero(), |acc, x| acc + x.1.balance);

    let total_amount = payment_intent.amount_details.calculate_net_amount();

    let remaining_amount = (total_amount - total_balances).max(MinorUnit::zero());

    let pm_split_amt_tuple: Vec<(PaymentMethodData, MinorUnit)> = gift_card_data_vec
        .iter()
        .map(|elem| {
            let pm_balance_key = domain::PaymentMethodBalanceKey {
                payment_method_type: common_enums::PaymentMethod::GiftCard,
                payment_method_subtype: elem.get_payment_method_type(),
                payment_method_key: domain::GiftCardData::from(elem.clone())
                    .get_payment_method_key()
                    .change_context(errors::ApiErrorResponse::InvalidRequestData {
                        message: "Unable to get unique key for payment method".to_string(),
                    })?
                    .expose(),
            };

            let pm_balance = balances
                .get(&pm_balance_key)
                .ok_or(errors::ApiErrorResponse::InternalServerError)?;

            Ok((
                PaymentMethodData::GiftCard(Box::new(elem.to_owned())),
                pm_balance.balance,
            ))
        })
        .collect::<RouterResult<Vec<_>>>()?;

    // If the gift card balances are not sufficient for payment, use the non-gift card payment method
    // for the remaining amount
    if remaining_amount > MinorUnit::zero() {
        let non_gift_card_pm_data = non_gift_card_pm_data
            .ok_or(errors::ApiErrorResponse::InvalidRequestData {
                message: "Requires additional payment method data".to_string(),
            })?
            .payment_method_data;

        Ok(PaymentMethodAmountSplit {
            balance_pm_split: pm_split_amt_tuple,
            non_balance_pm_split: Some((non_gift_card_pm_data, remaining_amount)),
        })
    } else {
        Ok(PaymentMethodAmountSplit {
            balance_pm_split: pm_split_amt_tuple,
            non_balance_pm_split: None,
        })
    }
}

pub(crate) async fn payments_execute_split_core(
    state: SessionState,
    req_state: ReqState,
    merchant_context: domain::MerchantContext,
    profile: domain::Profile,
    request: payments_api::PaymentsConfirmIntentRequest,
    header_payload: HeaderPayload,
    payment_id: id_type::GlobalPaymentId,
) -> RouterResponse<payments_api::PaymentsResponse> {
    let db = &*state.store;
    let key_manager_state = &(&state).into();

    let payment_intent = db
        .find_payment_intent_by_id(
            key_manager_state,
            &payment_id,
            merchant_context.get_merchant_key_store(),
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    if payment_intent.split_txns_enabled == SplitTxnsEnabled::Skip {
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "Split Payments not enabled in Payment Intent".to_string(),
        })?
    }

    let cell_id = state.conf.cell_information.id.clone();

    let attempts_group_id = id_type::GlobalAttemptGroupId::generate(&cell_id);

    let payment_intent_update =
        hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::AttemptGroupUpdate {
            updated_by: merchant_context
                .get_merchant_account()
                .storage_scheme
                .to_string(),
            active_attempt_id_type: enums::ActiveAttemptIDType::AttemptsGroupID,
            active_attempts_group_id: attempts_group_id.clone(),
        };

    let payment_intent = db
        .update_payment_intent(
            key_manager_state,
            payment_intent,
            payment_intent_update,
            merchant_context.get_merchant_key_store(),
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to update payment intent")?;

    let pm_amount_split =
        get_payment_method_and_amount_split(&state, &payment_id, &request, &payment_intent).await?;

    let primary_pm_response = {
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
                &merchant_context,
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
            merchant_context.clone(),
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
        if payment_data.payment_intent.status == enums::IntentStatus::Succeeded {
            let payment_intent_update =
            hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::SplitPaymentStatusUpdate {
                status: common_enums::IntentStatus::RequiresPaymentMethod,
                updated_by: merchant_context
                    .get_merchant_account()
                    .storage_scheme
                    .to_string(),
            };

            let updated_payment_intent = db
                .update_payment_intent(
                    key_manager_state,
                    payment_data.payment_intent.clone(),
                    payment_intent_update,
                    merchant_context.get_merchant_key_store(),
                    merchant_context.get_merchant_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to update payment intent")?;
        }

        payment_data
    };

    let mut split_pm_response_data = SplitPaymentResponseData {
        primary_payment_response_data: primary_pm_response,
        secondary_payment_response_data: vec![],
    };

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
                &merchant_context,
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
            connector_http_status_code,
            external_latency,
            connector_response_data,
        ) = Box::pin(payments_operation_core(
            &state,
            req_state.clone(),
            merchant_context.clone(),
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
        if payment_data.payment_intent.status == enums::IntentStatus::Succeeded {
            let payment_intent_update =
            hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::SplitPaymentStatusUpdate {
                status: common_enums::IntentStatus::RequiresPaymentMethod,
                updated_by: merchant_context
                    .get_merchant_account()
                    .storage_scheme
                    .to_string(),
            };

            let updated_payment_intent = db
                .update_payment_intent(
                    key_manager_state,
                    payment_data.payment_intent.clone(),
                    payment_intent_update,
                    merchant_context.get_merchant_key_store(),
                    merchant_context.get_merchant_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to update payment intent")?;
        }

        split_pm_response_data
            .secondary_payment_response_data
            .push(payment_data);
    }

    let payment_intent_update =
        hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::SplitPaymentStatusUpdate {
            status: common_enums::IntentStatus::Succeeded,
            updated_by: merchant_context
                .get_merchant_account()
                .storage_scheme
                .to_string(),
        };

    let updated_payment_intent = db
        .update_payment_intent(
            key_manager_state,
            payment_intent,
            payment_intent_update,
            merchant_context.get_merchant_key_store(),
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to update payment intent")?;

    split_pm_response_data.generate_response(
        &state,
        None,
        None,
        header_payload.x_hs_latency,
        &merchant_context,
        &profile,
        None,
    )
}
