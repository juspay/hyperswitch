use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::ext_traits::AsyncExt;
use diesel_models::connector_response::ConnectorResponse;
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payment_methods::PaymentMethodRetrieve,
        payments::{self, helpers, operations, types::MultipleCaptureData},
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums, payment_attempt::PaymentAttemptExt, ConnectorResponseExt},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, router_derive::PaymentOperation)]
#[operation(ops = "all", flow = "capture")]
pub struct PaymentCapture;

#[async_trait]
impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    GetTracker<F, payments::PaymentData<F>, api::PaymentsCaptureRequest, Ctx> for PaymentCapture
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
        BoxedOperation<'a, F, api::PaymentsCaptureRequest, Ctx>,
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

        payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                payment_intent.payment_id.as_str(),
                merchant_id,
                payment_intent.active_attempt.get_id().as_str(),
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

        helpers::validate_status_with_capture_method(payment_intent.status, capture_method)?;

        helpers::validate_amount_to_capture(
            payment_attempt.amount_capturable,
            request.amount_to_capture,
        )?;

        helpers::validate_capture_method(capture_method)?;

        let (multiple_capture_data, connector_response) = if capture_method
            == enums::CaptureMethod::ManualMultiple
        {
            let amount_to_capture = request
                .amount_to_capture
                .get_required_value("amount_to_capture")?;

            helpers::validate_amount_to_capture(
                payment_attempt.amount_capturable,
                Some(amount_to_capture),
            )?;

            let previous_captures = db
                .find_all_captures_by_merchant_id_payment_id_authorized_attempt_id(
                    &payment_attempt.merchant_id,
                    &payment_attempt.payment_id,
                    &payment_attempt.attempt_id,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

            let capture = db
                .insert_capture(
                    payment_attempt
                        .make_new_capture(amount_to_capture, enums::CaptureStatus::Started)?,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::DuplicatePayment {
                    payment_id: payment_id.clone(),
                })?;
            let new_connector_response = db
                .insert_connector_response(
                    ConnectorResponse::make_new_connector_response(
                        capture.payment_id.clone(),
                        capture.merchant_id.clone(),
                        capture.capture_id.clone(),
                        Some(capture.connector.clone()),
                        storage_scheme.to_string(),
                    ),
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::DuplicatePayment { payment_id })?;
            (
                Some(MultipleCaptureData::new_for_create(
                    previous_captures,
                    capture,
                )),
                new_connector_response,
            )
        } else {
            let connector_response = db
                .find_connector_response_by_payment_id_merchant_id_attempt_id(
                    &payment_attempt.payment_id,
                    &payment_attempt.merchant_id,
                    &payment_attempt.attempt_id,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            (None, connector_response)
        };

        currency = payment_attempt.currency.get_required_value("currency")?;

        amount = payment_attempt.amount.into();

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
                multiple_capture_data,
                redirect_response: None,
                surcharge_details: None,
                frm_message: None,
                payment_link_data: None,
            },
            None,
        ))
    }
}

#[async_trait]
impl<F: Clone, Ctx: PaymentMethodRetrieve>
    UpdateTracker<F, payments::PaymentData<F>, api::PaymentsCaptureRequest, Ctx>
    for PaymentCapture
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &dyn StorageInterface,
        mut payment_data: payments::PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _mechant_key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsCaptureRequest, Ctx>,
        payments::PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        payment_data.payment_attempt = match &payment_data.multiple_capture_data {
            Some(multiple_capture_data) => db
                .update_payment_attempt_with_attempt_id(
                    payment_data.payment_attempt,
                    storage::PaymentAttemptUpdate::MultipleCaptureCountUpdate {
                        multiple_capture_count: multiple_capture_data.get_captures_count()?,
                        updated_by: storage_scheme.to_string(),
                    },
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?,
            None => payment_data.payment_attempt,
        };
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    ValidateRequest<F, api::PaymentsCaptureRequest, Ctx> for PaymentCapture
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsCaptureRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsCaptureRequest, Ctx>,
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
