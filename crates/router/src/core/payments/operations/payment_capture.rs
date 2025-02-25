use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{self, helpers, operations, types::MultipleCaptureData},
        utils::ValidatePlatformMerchant,
    },
    events::audit_events::{AuditEvent, AuditEventType},
    routes::{app::ReqState, SessionState},
    services,
    types::{
        self as core_types,
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums, payment_attempt::PaymentAttemptExt},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, router_derive::PaymentOperation)]
#[operation(operations = "all", flow = "capture")]
pub struct PaymentCapture;

type PaymentCaptureOperation<'b, F> =
    BoxedOperation<'b, F, api::PaymentsCaptureRequest, payments::PaymentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, payments::PaymentData<F>, api::PaymentsCaptureRequest>
    for PaymentCapture
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsCaptureRequest,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<
        operations::GetTrackerResponse<
            'a,
            F,
            api::PaymentsCaptureRequest,
            payments::PaymentData<F>,
        >,
    > {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let merchant_id = merchant_account.get_id();
        let storage_scheme = merchant_account.storage_scheme;
        let (payment_intent, mut payment_attempt, currency, amount);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(
                key_manager_state,
                &payment_id,
                merchant_id,
                key_store,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent
            .validate_platform_merchant(platform_merchant_account.map(|ma| ma.get_id()))?;

        payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
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
            payment_attempt.amount_capturable.get_amount_as_i64(),
            request
                .amount_to_capture
                .map(|capture_amount| capture_amount.get_amount_as_i64()),
        )?;

        helpers::validate_capture_method(capture_method)?;

        let multiple_capture_data = if capture_method == enums::CaptureMethod::ManualMultiple {
            let amount_to_capture = request
                .amount_to_capture
                .get_required_value("amount_to_capture")?;

            helpers::validate_amount_to_capture(
                payment_attempt.amount_capturable.get_amount_as_i64(),
                Some(amount_to_capture.get_amount_as_i64()),
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

            Some(MultipleCaptureData::new_for_create(
                previous_captures,
                capture,
            ))
        } else {
            None
        };

        currency = payment_attempt.currency.get_required_value("currency")?;

        amount = payment_attempt.get_total_amount().into();

        let shipping_address = helpers::get_address_by_id(
            state,
            payment_intent.shipping_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let billing_address = helpers::get_address_by_id(
            state,
            payment_intent.billing_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let payment_method_billing = helpers::get_address_by_id(
            state,
            payment_attempt.payment_method_billing_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
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
                    merchant_account.get_id(),
                    mcd,
                )
                .await
            })
            .await
            .transpose()?;

        let profile_id = payment_intent
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;

        let business_profile = db
            .find_business_profile_by_profile_id(key_manager_state, key_store, profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;

        let payment_data = payments::PaymentData {
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
            customer_acceptance: None,
            token: None,
            token_data: None,
            address: core_types::PaymentAddress::new(
                shipping_address.as_ref().map(From::from),
                billing_address.as_ref().map(From::from),
                payment_method_billing.as_ref().map(From::from),
                business_profile.use_billing_as_payment_method_billing,
            ),
            confirm: None,
            payment_method_data: None,
            payment_method_info: None,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
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
            incremental_authorization_details: None,
            authorizations: vec![],
            authentication: None,
            recurring_details: None,
            poll_config: None,
            tax_data: None,
            session_id: None,
            service_details: None,
        };

        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(self),
            customer_details: None,
            payment_data,
            business_profile,
            mandate_type: None,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, payments::PaymentData<F>, api::PaymentsCaptureRequest>
    for PaymentCapture
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &'b SessionState,
        req_state: ReqState,
        mut payment_data: payments::PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _mechant_key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(PaymentCaptureOperation<'b, F>, payments::PaymentData<F>)>
    where
        F: 'b + Send,
    {
        payment_data.payment_attempt = if payment_data.multiple_capture_data.is_some()
            || payment_data.payment_attempt.amount_to_capture.is_some()
        {
            let multiple_capture_count = payment_data
                .multiple_capture_data
                .as_ref()
                .map(|multiple_capture_data| multiple_capture_data.get_captures_count())
                .transpose()?;
            let amount_to_capture = payment_data.payment_attempt.amount_to_capture;
            db.store
                .update_payment_attempt_with_attempt_id(
                    payment_data.payment_attempt,
                    storage::PaymentAttemptUpdate::CaptureUpdate {
                        amount_to_capture,
                        multiple_capture_count,
                        updated_by: storage_scheme.to_string(),
                    },
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?
        } else {
            payment_data.payment_attempt
        };
        let capture_amount = payment_data.payment_attempt.amount_to_capture;
        let multiple_capture_count = payment_data.payment_attempt.multiple_capture_count;
        req_state
            .event_context
            .event(AuditEvent::new(AuditEventType::PaymentCapture {
                capture_amount,
                multiple_capture_count,
            }))
            .with(payment_data.to_event())
            .emit();
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone + Sync>
    ValidateRequest<F, api::PaymentsCaptureRequest, payments::PaymentData<F>> for PaymentCapture
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsCaptureRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(PaymentCaptureOperation<'b, F>, operations::ValidateResult)> {
        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: merchant_account.get_id().to_owned(),
                payment_id: api::PaymentIdType::PaymentIntentId(request.payment_id.to_owned()),
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}
