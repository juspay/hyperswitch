use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use error_stack::{IntoReport, ResultExt};
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payment_methods::PaymentMethodRetrieve,
        payments::{helpers, operations, PaymentAddress, PaymentData},
        utils as core_utils,
    },
    routes::AppState,
    services,
    types::{
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "capture")]
pub struct PaymentApprove;

#[async_trait]
impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    GetTracker<F, PaymentData<F>, api::PaymentsCaptureRequest, Ctx> for PaymentApprove
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        _request: &api::PaymentsCaptureRequest,
        _mandate_type: Option<api::MandateTransactionType>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        _payment_confirm_source: Option<common_enums::PaymentSource>,
    ) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsCaptureRequest, Ctx>> {
        let db = &*state.store;
        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;
        let (mut payment_intent, payment_attempt, currency, amount);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        helpers::validate_payment_status_against_not_allowed_statuses(
            &payment_intent.status,
            &[
                storage_enums::IntentStatus::Failed,
                storage_enums::IntentStatus::Succeeded,
            ],
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
            .find_business_profile_by_profile_id(profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                id: profile_id.to_string(),
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
            db,
            payment_intent.shipping_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let billing_address = helpers::get_address_by_id(
            db,
            payment_intent.billing_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let payment_method_billing = helpers::get_address_by_id(
            db,
            payment_attempt.payment_method_billing_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await?;

        payment_intent.shipping_address_id = shipping_address.clone().map(|i| i.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|i| i.address_id);

        let frm_response = db
        .find_fraud_check_by_payment_id(payment_intent.payment_id.clone(), merchant_account.merchant_id.clone())
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable_lazy(|| {
            format!("Error while retrieving frm_response, merchant_id: {}, payment_id: {attempt_id}", &merchant_account.merchant_id)
        });

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
            address: PaymentAddress {
                shipping: shipping_address.as_ref().map(|a| a.into()),
                billing: billing_address.as_ref().map(|a| a.into()),
                payment_method_billing: payment_method_billing
                    .as_ref()
                    .map(|address| address.into()),
            },
            confirm: None,
            payment_method_data: None,
            force_sync: None,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
            sessions_token: vec![],
            card_cvc: None,
            creds_identifier: None,
            payment_method_status: None,
            pm_token: None,
            connector_customer_id: None,
            recurring_mandate_payment_data: None,
            ephemeral_key: None,
            multiple_capture_data: None,
            redirect_response: None,
            surcharge_details: None,
            frm_message: frm_response.ok(),
            payment_link_data: None,
            incremental_authorization_details: None,
            authorizations: vec![],
            frm_metadata: None,
        };

        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(self),
            customer_details: None,
            payment_data,
            business_profile,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone, Ctx: PaymentMethodRetrieve>
    UpdateTracker<F, PaymentData<F>, api::PaymentsCaptureRequest, Ctx> for PaymentApprove
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        db: &'b AppState,
        mut payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsCaptureRequest, Ctx>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        let intent_status_update = storage::PaymentIntentUpdate::ApproveUpdate {
            merchant_decision: Some(api_models::enums::MerchantDecision::Approved.to_string()),
            updated_by: storage_scheme.to_string(),
        };
        payment_data.payment_intent = db
            .store
            .update_payment_intent(
                payment_data.payment_intent,
                intent_status_update,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    ValidateRequest<F, api::PaymentsCaptureRequest, Ctx> for PaymentApprove
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
        let request_merchant_id = request.merchant_id.as_deref();
        helpers::validate_merchant_id(&merchant_account.merchant_id, request_merchant_id)
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "merchant_id".to_string(),
                expected_format: "merchant_id from merchant account".to_string(),
            })?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: api::PaymentIdType::PaymentIntentId(
                    core_utils::validate_id(request.payment_id.clone(), "payment_id")
                        .into_report()?,
                ),
                mandate_type: None,
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}
