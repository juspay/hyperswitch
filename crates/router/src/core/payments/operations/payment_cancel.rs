use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::ext_traits::AsyncExt;
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
#[operation(ops = "all", flow = "cancel")]
pub struct PaymentCancel;

#[async_trait]
impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    GetTracker<F, PaymentData<F>, api::PaymentsCancelRequest, Ctx> for PaymentCancel
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsCancelRequest,
        _mandate_type: Option<api::MandateTransactionType>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsCancelRequest, Ctx>,
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
                enums::IntentStatus::Cancelled,
                enums::IntentStatus::Processing,
                enums::IntentStatus::RequiresMerchantAction,
            ],
            "cancel",
        )?;

        let mut payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                payment_intent.payment_id.as_str(),
                merchant_id,
                payment_intent.active_attempt.get_id().as_str(),
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

        payment_attempt.cancellation_reason = request.cancellation_reason.clone();

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
                creds_identifier,
                pm_token: None,
                connector_customer_id: None,
                recurring_mandate_payment_data: None,
                ephemeral_key: None,
                multiple_capture_data: None,
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
    UpdateTracker<F, PaymentData<F>, api::PaymentsCancelRequest, Ctx> for PaymentCancel
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
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsCancelRequest, Ctx>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        let cancellation_reason = payment_data.payment_attempt.cancellation_reason.clone();
        let (intent_status_update, attempt_status_update) =
            if payment_data.payment_intent.status != enums::IntentStatus::RequiresCapture {
                let payment_intent_update = storage::PaymentIntentUpdate::PGStatusUpdate {
                    status: enums::IntentStatus::Cancelled,
                    updated_by: storage_scheme.to_string(),
                };
                (Some(payment_intent_update), enums::AttemptStatus::Voided)
            } else {
                (None, enums::AttemptStatus::VoidInitiated)
            };

        if let Some(payment_intent_update) = intent_status_update {
            payment_data.payment_intent = db
                .update_payment_intent(
                    payment_data.payment_intent,
                    payment_intent_update,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        }

        db.update_payment_attempt_with_attempt_id(
            payment_data.payment_attempt.clone(),
            storage::PaymentAttemptUpdate::VoidUpdate {
                status: attempt_status_update,
                cancellation_reason,
                updated_by: storage_scheme.to_string(),
            },
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    ValidateRequest<F, api::PaymentsCancelRequest, Ctx> for PaymentCancel
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsCancelRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsCancelRequest, Ctx>,
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
