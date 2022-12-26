use std::marker::PhantomData;

use async_trait::async_trait;
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{self, helpers, operations},
    },
    db::StorageInterface,
    routes::AppState,
    types::{
        api::{self, PaymentIdTypeExt},
        storage::{self, enums},
        transformers::ForeignInto,
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, router_derive::PaymentOperation)]
#[operation(ops = "all", flow = "capture")]
pub struct PaymentCapture;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, payments::PaymentData<F>, api::PaymentsCaptureRequest>
    for PaymentCapture
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        merchant_id: &str,
        request: &api::PaymentsCaptureRequest,
        _mandate_type: Option<api::MandateTxnType>,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsCaptureRequest>,
        payments::PaymentData<F>,
        Option<payments::CustomerDetails>,
    )> {
        let db = &*state.store;
        let (payment_intent, mut payment_attempt, currency, amount);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        helpers::validate_status(payment_intent.status)?;

        helpers::validate_amount_to_capture(payment_intent.amount, request.amount_to_capture)?;

        payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id(
                &payment_id,
                merchant_id,
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        payment_attempt
            .amount_to_capture
            .update_value(request.amount_to_capture);

        let capture_method = payment_attempt
            .capture_method
            .get_required_value("capture_method")?;

        helpers::validate_capture_method(capture_method)?;

        currency = payment_attempt.currency.get_required_value("currency")?;

        amount = payment_attempt.amount.into();

        let connector_response = db
            .find_connector_response_by_payment_id_merchant_id_attempt_id(
                &payment_attempt.payment_id,
                &payment_attempt.merchant_id,
                &payment_attempt.attempt_id,
                storage_scheme,
            )
            .await
            .map_err(|error| {
                error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            })?;

        let shipping_address = helpers::get_address_for_payment_request(
            db,
            None,
            payment_intent.shipping_address_id.as_deref(),
            merchant_id,
            &payment_intent.customer_id,
        )
        .await?;

        let billing_address = helpers::get_address_for_payment_request(
            db,
            None,
            payment_intent.billing_address_id.as_deref(),
            merchant_id,
            &payment_intent.customer_id,
        )
        .await?;

        // TODO: get payment method data for response
        Ok((
            Box::new(self),
            payments::PaymentData {
                flow: PhantomData,
                payment_intent,
                payment_attempt,
                currency,
                force_sync: None,
                amount,
                email: None,
                mandate_id: None,
                setup_mandate: None,
                token: None,
                address: payments::PaymentAddress {
                    shipping: shipping_address.as_ref().map(|a| a.foreign_into()),
                    billing: billing_address.as_ref().map(|a| a.foreign_into()),
                },
                confirm: None,
                payment_method_data: None,
                refunds: vec![],
                connector_response,
                sessions_token: vec![],
                card_cvc: None,
            },
            None,
        ))
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, payments::PaymentData<F>, api::PaymentsCaptureRequest>
    for PaymentCapture
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        _db: &dyn StorageInterface,
        _payment_id: &api::PaymentIdType,
        payment_data: payments::PaymentData<F>,
        _customer: Option<storage::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsCaptureRequest>,
        payments::PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone> ValidateRequest<F, api::PaymentsCaptureRequest> for PaymentCapture {
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsCaptureRequest,
        merchant_account: &'a storage::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsCaptureRequest>,
        operations::ValidateResult<'a>,
    )> {
        let payment_id = request
            .payment_id
            .as_ref()
            .get_required_value("payment_id")?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: api::PaymentIdType::PaymentIntentId(payment_id.to_owned()),
                mandate_type: None,
                storage_scheme: merchant_account.storage_scheme,
            },
        ))
    }
}
