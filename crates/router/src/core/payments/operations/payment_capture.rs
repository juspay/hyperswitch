use std::marker::PhantomData;

use async_trait::async_trait;
use common_utils::{errors::CustomResult, ext_traits::AsyncExt};
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
    services,
    types::{
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums},
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
        request: &api::PaymentsCaptureRequest,
        _mandate_type: Option<api::MandateTransactionType>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsCaptureRequest>,
        payments::PaymentData<F>,
        Option<payments::CustomerDetails>,
    )> {
        let db = &*state.store;
        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;
        let (payment_intent, mut payment_attempt, currency, amount);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        helpers::validate_status(payment_intent.status)?;

        helpers::validate_amount_to_capture(
            payment_intent.amount - payment_intent.amount_captured.unwrap_or(0),
            request.amount_to_capture,
        )?;

        payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                payment_intent.payment_id.as_str(),
                merchant_id,
                payment_intent.active_attempt_id.as_str(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

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
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let shipping_address = helpers::get_address_for_payment_request(
            db,
            None,
            payment_intent.shipping_address_id.as_deref(),
            merchant_id,
            payment_intent.customer_id.as_ref(),
            key_store,
        )
        .await?;

        let billing_address = helpers::get_address_for_payment_request(
            db,
            None,
            payment_intent.billing_address_id.as_deref(),
            merchant_id,
            payment_intent.customer_id.as_ref(),
            key_store,
        )
        .await?;

        let creds_identifier = request
            .merchant_connector_details
            .as_ref()
            .map(|mcd| mcd.creds_identifier.to_owned());
        request
            .merchant_connector_details
            .to_owned()
            .async_map(|mcd| async {
                helpers::insert_merchant_connector_creds_to_config(
                    db,
                    merchant_account.merchant_id.as_str(),
                    mcd,
                )
                .await
            })
            .await
            .transpose()?;

        let capture = match payment_attempt.capture_method {
            Some(enums::CaptureMethod::ManualMultiple) => {
                let new_capture = Self::create_capture(state, &payment_attempt, storage_scheme)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to create capture in DB")?;
                Some(new_capture)
            }
            _ => None,
        };

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
                mandate_connector: None,
                setup_mandate: None,
                token: None,
                address: payments::PaymentAddress {
                    shipping: shipping_address.as_ref().map(|a| a.into()),
                    billing: billing_address.as_ref().map(|a| a.into()),
                },
                confirm: None,
                payment_method_data: None,
                refunds: vec![],
                disputes: vec![],
                attempts: None,
                connector_response,
                sessions_token: vec![],
                card_cvc: None,
                creds_identifier,
                pm_token: None,
                connector_customer_id: None,
                recurring_mandate_payment_data: None,
                ephemeral_key: None,
                redirect_response: None,
                capture,
                frm_message: None,
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
        payment_data: payments::PaymentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _mechant_key_store: &domain::MerchantKeyStore,
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
        merchant_account: &'a domain::MerchantAccount,
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
                requeue: false,
            },
        ))
    }
}

impl PaymentCapture {
    async fn create_capture(
        state: &AppState,
        authorized_payment_attempt: &storage::PaymentAttempt,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage::Capture, errors::StorageError> {
        state
            .store
            .insert_capture(
                Self::make_capture(authorized_payment_attempt),
                storage_scheme,
            )
            .await
    }
    fn make_capture(authorized_attempt: &storage::PaymentAttempt) -> storage::CaptureNew {
        let capture_sequence = authorized_attempt.multiple_capture_count.unwrap_or(0) + 1;
        storage::CaptureNew {
            payment_id: authorized_attempt.payment_id.clone(),
            merchant_id: authorized_attempt.merchant_id.clone(),
            capture_id: format!("{}_{}", authorized_attempt.attempt_id, capture_sequence),
            status: enums::CaptureStatus::Started,
            amount: authorized_attempt
                .amount_to_capture
                .unwrap_or(authorized_attempt.amount),
            currency: authorized_attempt.currency,
            connector: authorized_attempt.connector.clone(),
            error_message: None,
            tax_amount: None,
            created_at: Some(common_utils::date_time::now()),
            modified_at: Some(common_utils::date_time::now()),
            error_code: None,
            error_reason: None,
            authorized_attempt_id: authorized_attempt.attempt_id.clone(),
            capture_sequence,
            connector_transaction_id: None,
        }
    }
}
