use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::PaymentsRejectRequest};
use async_trait::async_trait;
use error_stack::ResultExt;
use router_derive;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payment_methods::PaymentMethodRetrieve,
        payments::{helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, router_derive::PaymentOperation)]
#[operation(ops = "all", flow = "reject")]
pub struct PaymentReject;

#[async_trait]
impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    GetTracker<F, PaymentData<F>, PaymentsRejectRequest, Ctx> for PaymentReject
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        _request: &PaymentsRejectRequest,
        _mandate_type: Option<api::MandateTransactionType>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
    ) -> RouterResult<(
        BoxedOperation<'a, F, PaymentsRejectRequest, Ctx>,
        PaymentData<F>,
        Option<CustomerDetails>,
    )> {
        let db = &*state.store;
        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;
        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        helpers::validate_payment_status_against_not_allowed_statuses(
            &payment_intent.status,
            &[
                enums::IntentStatus::Failed,
                enums::IntentStatus::Succeeded,
                enums::IntentStatus::Processing,
            ],
            "reject",
        )?;

        let attempt_id = payment_intent.active_attempt.get_id().clone();
        let payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                payment_intent.payment_id.as_str(),
                merchant_id,
                attempt_id.clone().as_str(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let shipping_address = helpers::create_or_find_address_for_payment_by_request(
            db,
            None,
            payment_intent.shipping_address_id.as_deref(),
            merchant_id,
            payment_intent.customer_id.as_ref(),
            key_store,
            &payment_intent.payment_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let billing_address = helpers::create_or_find_address_for_payment_by_request(
            db,
            None,
            payment_intent.billing_address_id.as_deref(),
            merchant_id,
            payment_intent.customer_id.as_ref(),
            key_store,
            &payment_intent.payment_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let connector_response = db
            .find_connector_response_by_payment_id_merchant_id_attempt_id(
                &payment_attempt.payment_id,
                &payment_attempt.merchant_id,
                &payment_attempt.attempt_id,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        let currency = payment_attempt.currency.get_required_value("currency")?;
        let amount = payment_attempt.amount.into();

        let frm_response = db
        .find_fraud_check_by_payment_id(payment_intent.payment_id.clone(), merchant_account.merchant_id.clone())
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable_lazy(|| {
            format!("Error while retrieving frm_response, merchant_id: {}, payment_id: {attempt_id}", &merchant_account.merchant_id)
        });

        Ok((
            Box::new(self),
            PaymentData {
                flow: PhantomData,
                payment_intent,
                payment_attempt,
                currency,
                amount,
                email: None,
                mandate_id: None,
                mandate_connector: None,
                setup_mandate: None,
                token: None,
                address: PaymentAddress {
                    shipping: shipping_address.as_ref().map(|a| a.into()),
                    billing: billing_address.as_ref().map(|a| a.into()),
                },
                confirm: None,
                payment_method_data: None,
                force_sync: None,
                refunds: vec![],
                disputes: vec![],
                attempts: None,
                connector_response,
                sessions_token: vec![],
                card_cvc: None,
                creds_identifier: None,
                pm_token: None,
                connector_customer_id: None,
                recurring_mandate_payment_data: None,
                ephemeral_key: None,
                multiple_capture_data: None,
                redirect_response: None,
                surcharge_details: None,
                frm_message: frm_response.ok(),
                payment_link_data: None,
            },
            None,
        ))
    }
}

#[async_trait]
impl<F: Clone, Ctx: PaymentMethodRetrieve>
    UpdateTracker<F, PaymentData<F>, PaymentsRejectRequest, Ctx> for PaymentReject
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        mut payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _mechant_key_store: &domain::MerchantKeyStore,
        _should_decline_transaction: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, PaymentsRejectRequest, Ctx>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        let intent_status_update = storage::PaymentIntentUpdate::RejectUpdate {
            status: enums::IntentStatus::Failed,
            merchant_decision: Some(enums::MerchantDecision::Rejected.to_string()),
            updated_by: storage_scheme.to_string(),
        };
        let (error_code, error_message) =
            payment_data
                .frm_message
                .clone()
                .map_or((None, None), |fraud_check| {
                    (
                        Some(Some(fraud_check.frm_status.to_string())),
                        Some(fraud_check.frm_reason.map(|reason| reason.to_string())),
                    )
                });
        let attempt_status_update = storage::PaymentAttemptUpdate::RejectUpdate {
            status: enums::AttemptStatus::Failure,
            error_code,
            error_message,
            updated_by: storage_scheme.to_string(),
        };

        payment_data.payment_intent = db
            .update_payment_intent(
                payment_data.payment_intent,
                intent_status_update,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_data.payment_attempt = db
            .update_payment_attempt_with_attempt_id(
                payment_data.payment_attempt.clone(),
                attempt_status_update,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone, Ctx: PaymentMethodRetrieve> ValidateRequest<F, PaymentsRejectRequest, Ctx>
    for PaymentReject
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &PaymentsRejectRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, PaymentsRejectRequest, Ctx>,
        operations::ValidateResult<'a>,
    )> {
        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: api::PaymentIdType::PaymentIntentId(request.payment_id.to_owned()),
                mandate_type: None,
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}
