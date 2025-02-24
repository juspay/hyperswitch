use std::marker::PhantomData;

use api_models::enums::{AttemptStatus, FrmSuggestion, IntentStatus};
use async_trait::async_trait;
use error_stack::ResultExt;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{helpers, operations, PaymentData},
        utils::ValidatePlatformMerchant,
    },
    events::audit_events::{AuditEvent, AuditEventType},
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums as storage_enums},
        PaymentAddress,
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "capture")]
pub struct PaymentApprove;

type PaymentApproveOperation<'a, F> =
    BoxedOperation<'a, F, api::PaymentsCaptureRequest, PaymentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentData<F>, api::PaymentsCaptureRequest>
    for PaymentApprove
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        _request: &api::PaymentsCaptureRequest,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<
        operations::GetTrackerResponse<'a, F, api::PaymentsCaptureRequest, PaymentData<F>>,
    > {
        let db = &*state.store;
        let key_manager_state = &state.into();
        let merchant_id = merchant_account.get_id();
        let storage_scheme = merchant_account.storage_scheme;
        let (mut payment_intent, payment_attempt, currency, amount);

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

        helpers::validate_payment_status_against_not_allowed_statuses(
            payment_intent.status,
            &[IntentStatus::Failed, IntentStatus::Succeeded],
            "approve",
        )?;

        let profile_id = payment_intent
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;

        let business_profile = state
            .store
            .find_business_profile_by_profile_id(key_manager_state, key_store, profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;

        let attempt_id = payment_intent.active_attempt.get_id().clone();
        payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
                merchant_id,
                &attempt_id.clone(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

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

        payment_intent.shipping_address_id = shipping_address.clone().map(|i| i.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|i| i.address_id);

        let frm_response = if cfg!(feature = "frm") {
            db.find_fraud_check_by_payment_id(payment_intent.payment_id.clone(), merchant_account.get_id().clone())
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound)
                .attach_printable_lazy(|| {
                    format!("Error while retrieving frm_response, merchant_id: {}, payment_id: {attempt_id}", merchant_account.get_id().get_string_repr())
                })
                .ok()
        } else {
            None
        };

        let payment_data = PaymentData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            currency,
            amount,
            email: None,
            mandate_id: None,
            mandate_connector: None,
            setup_mandate: None,
            customer_acceptance: None,
            token: None,
            token_data: None,
            address: PaymentAddress::new(
                shipping_address.as_ref().map(From::from),
                billing_address.as_ref().map(From::from),
                payment_method_billing.as_ref().map(From::from),
                business_profile.use_billing_as_payment_method_billing,
            ),
            confirm: None,
            payment_method_data: None,
            payment_method_info: None,
            force_sync: None,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
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
            frm_message: frm_response,
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
impl<F: Clone + Sync> UpdateTracker<F, PaymentData<F>, api::PaymentsCaptureRequest>
    for PaymentApprove
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        req_state: ReqState,
        mut payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(PaymentApproveOperation<'b, F>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
        if matches!(frm_suggestion, Some(FrmSuggestion::FrmAuthorizeTransaction)) {
            payment_data.payment_intent.status = IntentStatus::RequiresCapture; // In Approve flow, payment which has payment_capture_method "manual" and attempt status as "Unresolved",
            payment_data.payment_attempt.status = AttemptStatus::Authorized; // We shouldn't call the connector instead we need to update the payment attempt and payment intent.
        }
        let intent_status_update = storage::PaymentIntentUpdate::ApproveUpdate {
            status: payment_data.payment_intent.status,
            merchant_decision: Some(api_models::enums::MerchantDecision::Approved.to_string()),
            updated_by: storage_scheme.to_string(),
        };
        payment_data.payment_intent = state
            .store
            .update_payment_intent(
                &state.into(),
                payment_data.payment_intent,
                intent_status_update,
                key_store,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        state
            .store
            .update_payment_attempt_with_attempt_id(
                payment_data.payment_attempt.clone(),
                storage::PaymentAttemptUpdate::StatusUpdate {
                    status: payment_data.payment_attempt.status,
                    updated_by: storage_scheme.to_string(),
                },
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        req_state
            .event_context
            .event(AuditEvent::new(AuditEventType::PaymentApprove))
            .with(payment_data.to_event())
            .emit();

        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone + Sync> ValidateRequest<F, api::PaymentsCaptureRequest, PaymentData<F>>
    for PaymentApprove
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsCaptureRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(PaymentApproveOperation<'b, F>, operations::ValidateResult)> {
        let request_merchant_id = request.merchant_id.as_ref();
        helpers::validate_merchant_id(merchant_account.get_id(), request_merchant_id)
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "merchant_id".to_string(),
                expected_format: "merchant_id from merchant account".to_string(),
            })?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: merchant_account.get_id().to_owned(),
                payment_id: api::PaymentIdType::PaymentIntentId(request.payment_id.clone()),
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}
