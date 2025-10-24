use api_models::{
    enums,
    payments::{
        self as payments_api, GetPaymentMethodType, PaymentMethodData,
        SplitPaymentMethodDataRequest,
    },
};
use common_enums::CallConnectorAction;
use common_utils::{id_type, types::MinorUnit};
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

async fn get_payment_method_and_amount_split(
    state: &SessionState,
    payment_id: &id_type::GlobalPaymentId,
    request: &payments_api::PaymentsConfirmIntentRequest,
    payment_intent: &PaymentIntent,
) -> RouterResult<Vec<(PaymentMethodData, MinorUnit)>> {
    let split_payment_methods_data = request.split_payment_method_data.clone().ok_or(
        errors::ApiErrorResponse::MissingRequiredField {
            field_name: "split_payment_method_data",
        },
    )?;

    let parent_pm_data = request
        .payment_method_data
        .payment_method_data
        .clone()
        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "split_payment_method_data",
        })?;
    let outer_payment_method_data = SplitPaymentMethodDataRequest {
        payment_method_data: parent_pm_data,
        payment_method_type: request.payment_method_type,
        payment_method_subtype: request.payment_method_subtype,
    };

    let mut combined_pm_data = split_payment_methods_data;
    combined_pm_data.push(outer_payment_method_data);

    // Validate at most one non-gift-card payment method
    let non_gift_card_count = combined_pm_data
        .iter()
        .filter(|pm_data| pm_data.payment_method_type != enums::PaymentMethod::GiftCard)
        .count();

    if non_gift_card_count > 1 {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "At most one non-gift card payment method is allowed".to_string(),
        }
        .into());
    }

    let non_gift_card_pm = combined_pm_data
        .iter()
        .position(|pm_data| pm_data.payment_method_type != enums::PaymentMethod::GiftCard);

    let non_gift_card_pm_data = if let Some(index) = non_gift_card_pm {
        Some(combined_pm_data.remove(index))
    } else {
        None
    };

    let gift_card_data_vec = combined_pm_data
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
        .map(|elem| api_models::payments::BalanceCheckPaymentMethodData::GiftCard(elem))
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

    let remaining_amount =
        (payment_intent.amount_details.order_amount - total_balances).max(MinorUnit::zero());

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

    if remaining_amount > MinorUnit::zero() {
        let mut pm_split_amt_tuple = pm_split_amt_tuple;
        let non_gift_card_pm_data = non_gift_card_pm_data
            .ok_or(errors::ApiErrorResponse::InvalidRequestData {
                message: "Requires additional payment method data".to_string(),
            })?
            .payment_method_data;
        pm_split_amt_tuple.insert(0, (non_gift_card_pm_data, remaining_amount));

        Ok(pm_split_amt_tuple)
    } else {
        Ok(pm_split_amt_tuple)
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

    let cell_id = state.conf.cell_information.id.clone();

    let payment_intent_update =
        hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::SplitUpdate {
            updated_by: merchant_context
                .get_merchant_account()
                .storage_scheme
                .to_string(),
            active_attempt_id_type: enums::ActiveAttemptIDType::AttemptID,
            active_attempts_group_id: id_type::GlobalAttemptGroupId::generate(&cell_id),
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

    let mut payment_response_data = None;
    for (payment_method_data, amount) in pm_amount_split {
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
            )
            .await?;

        let (
            payment_data,
            _req,
            _customer,
            connector_http_status_code,
            external_latency,
            connector_response_data,
        ) = payments_operation_core(
            &state,
            req_state.clone(),
            merchant_context.clone(),
            &profile,
            operation.clone(),
            request.clone(),
            get_tracker_response,
            CallConnectorAction::Trigger,
            header_payload.clone(),
        )
        .await?;

        let payment_intent_update =
            hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::VoidUpdate {
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

        payment_response_data = Some(payment_data);
    }

    let payment_intent_update =
        hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::VoidUpdate {
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

    payment_response_data.unwrap().generate_response(
        &state,
        None,
        None,
        header_payload.x_hs_latency,
        &merchant_context,
        &profile,
        None,
    )
}
