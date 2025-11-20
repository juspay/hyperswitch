use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::PaymentsContinueRedirectionRequest};
use async_trait::async_trait;
use common_utils::{ext_traits::Encode, fp_utils::when, types::keymanager::ToEncryptable as _};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::payments::PaymentConfirmData;
use masking::PeekInterface as _;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        mandate::helpers as m_helpers,
        payments::{self, helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
    },
    events::audit_events::{AuditEvent, AuditEventType},
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api::{self, CustomerAcceptance, PaymentIdTypeExt},
        domain::{self, types as domain_types},
        storage::{self, enums as storage_enums},
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy)]
pub struct ContinueRedirection;

type ContinueRedirectionOperation<'b, F> =
    BoxedOperation<'b, F, PaymentsContinueRedirectionRequest, PaymentConfirmData<F>>;

impl<F: Send + Clone + Sync> Operation<F, PaymentsContinueRedirectionRequest>
    for &ContinueRedirection
{
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsContinueRedirectionRequest, Self::Data> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<
        &(dyn GetTracker<F, Self::Data, PaymentsContinueRedirectionRequest> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_domain(
        &self,
    ) -> RouterResult<&(dyn Domain<F, PaymentsContinueRedirectionRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn UpdateTracker<F, Self::Data, PaymentsContinueRedirectionRequest> + Send + Sync),
    > {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone + Sync> Operation<F, PaymentsContinueRedirectionRequest>
    for ContinueRedirection
{
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsContinueRedirectionRequest, Self::Data> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<
        &(dyn GetTracker<F, Self::Data, PaymentsContinueRedirectionRequest> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_domain(
        &self,
    ) -> RouterResult<&dyn Domain<F, PaymentsContinueRedirectionRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn UpdateTracker<F, Self::Data, PaymentsContinueRedirectionRequest> + Send + Sync),
    > {
        Ok(self)
    }
}

impl operations::ValidateStatusForOperation for ContinueRedirection {
    /// Validate if the current operation can be performed on the current status of the payment intent
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        match intent_status {
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Processing => Ok(()),
            common_enums::IntentStatus::Conflicted
            | common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
            | common_enums::IntentStatus::Expired => {
                Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                    current_flow: format!("{self:?}"),
                    field_name: "status".to_string(),
                    current_value: intent_status.to_string(),
                    states: ["requires_payment_method", "failed", "processing"].join(", "),
                })
            }
        }
    }
}

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentConfirmData<F>, PaymentsContinueRedirectionRequest>
    for ContinueRedirection
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsContinueRedirectionRequest,
        merchant_context: &domain::MerchantContext,
        _profile: &domain::Profile,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<operations::GetTrackerResponse<PaymentConfirmData<F>>> {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let storage_scheme = merchant_context.get_merchant_account().storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_id(
                key_manager_state,
                payment_id,
                merchant_context.get_merchant_key_store(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // TODO (#7195): Add platform merchant account validation once publishable key auth is solved

        // self.validate_status_for_operation(payment_intent.status)?;

        let cell_id = state.conf.cell_information.id.clone();

        let active_attempt_id = payment_intent.active_attempt_id.as_ref().ok_or_else(|| {
            errors::ApiErrorResponse::MissingRequiredField {
                field_name: ("active_attempt_id"),
            }
        })?;

        let mut payment_attempt = db
            .find_payment_attempt_by_id(
                key_manager_state,
                merchant_context.get_merchant_key_store(),
                active_attempt_id,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not find payment attempt given the attempt id")?;

        let payment_address = hyperswitch_domain_models::payment_address::PaymentAddress::new(
            payment_intent
                .shipping_address
                .clone()
                .map(|address| address.into_inner()),
            payment_intent
                .billing_address
                .clone()
                .map(|address| address.into_inner()),
            payment_attempt
                .payment_method_billing_address
                .clone()
                .map(|address| address.into_inner()),
            Some(true),
        );

        let redirect_response = request
            .feature_metadata
            .as_ref()
            .and_then(|fm| fm.redirect_response.clone());

        let payment_data = PaymentConfirmData {
            flow: std::marker::PhantomData,
            payment_intent,
            payment_attempt,
            payment_method_data: None,
            payment_address,
            mandate_data: None,
            payment_method: None,
            merchant_connector_details: None,
            redirect_response,
            external_vault_pmd: None,
            webhook_url: None,
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, PaymentsContinueRedirectionRequest, PaymentConfirmData<F>>
    for ContinueRedirection
{
    async fn get_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<
        (
            ContinueRedirectionOperation<'a, F>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        match payment_data.payment_intent.customer_id.clone() {
            Some(id) => {
                let customer = state
                    .store
                    .find_customer_by_global_id(
                        &state.into(),
                        &id,
                        merchant_key_store,
                        storage_scheme,
                    )
                    .await?;

                Ok((Box::new(self), Some(customer)))
            }
            None => Ok((Box::new(self), None)),
        }
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        business_profile: &domain::Profile,
        _should_retry_with_pan: bool,
    ) -> RouterResult<(
        ContinueRedirectionOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    #[instrument(skip_all)]
    async fn add_task_to_process_tracker<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_attempt: &storage::PaymentAttempt,
        _requeue: bool,
        _schedule_time: Option<time::PrimitiveDateTime>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    #[cfg(feature = "v2")]
    async fn perform_routing<'a>(
        &'a self,
        merchant_context: &domain::MerchantContext,
        business_profile: &domain::Profile,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
    ) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
        payments::connector_selection(
            state,
            merchant_context,
            business_profile,
            payment_data,
            None,
        )
        .await
    }

    #[cfg(feature = "v2")]
    async fn create_or_fetch_payment_method<'a>(
        &'a self,
        state: &SessionState,
        merchant_context: &domain::MerchantContext,
        business_profile: &domain::Profile,
        payment_data: &mut PaymentConfirmData<F>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        let (payment_method, payment_method_data) = match (
            &payment_data.payment_attempt.payment_token,
            &payment_data.payment_method_data,
            &payment_data.payment_attempt.customer_acceptance,
        ) {
            (Some(ppmt), _, _) => {
                use crate::core::payment_methods;

                let payment_token_data = payment_methods::get_card_data_from_redis(
                    state,
                    ppmt.to_owned(),
                    payment_data.payment_attempt.payment_method_type,
                )
                .await?;

                let pm_data = Box::pin(payment_methods::retrieve_payment_method_with_token(
                    state,
                    merchant_context.get_merchant_key_store(),
                    &payment_token_data,
                    &payment_data.payment_intent,
                    &payment_data.payment_attempt,
                    None,
                    // domain::CardToken::default(),
                    // customer,
                    // storage_scheme,
                    // mandate_id,
                    // payment_data.payment_method_info.clone(),
                    // business_profile,
                    // should_retry_with_pan,
                    // vault_data,
                ))
                .await;

                let payment_method_details = pm_data.attach_printable("in 'make_pm_data'")?;

                // Don't modify payment_method_data in this case, only the payment_method and payment_method_id
                (
                    None::<domain::PaymentMethod>,
                    payment_method_details.payment_method_data,
                )
            }
            _ => (None, None), // Pass payment_data unmodified for any other case
        };

        if let Some(pm_data) = payment_method_data {
            payment_data.update_payment_method_data(pm_data);
        }
        if let Some(pm) = payment_method {
            payment_data.update_payment_method_and_pm_id(pm.get_id().clone(), pm);
        }

        Ok(())
    }
    // #[instrument(skip_all)]
    // async fn guard_payment_against_blocklist<'a>(
    //     &'a self,
    //     _state: &SessionState,
    //     _merchant_context: &domain::MerchantContext,
    //     _payment_data: &mut PaymentData<F>,
    // ) -> CustomResult<bool, errors::ApiErrorResponse> {
    //     Ok(false)
    // }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentConfirmData<F>, PaymentsContinueRedirectionRequest>
    for ContinueRedirection
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        req_state: ReqState,
        mut payment_data: PaymentConfirmData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(ContinueRedirectionOperation<'b, F>, PaymentConfirmData<F>)>
    where
        F: 'b + Send,
    {
        // let payment_intent_update = hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::ContinueRedirectionUpdate {
        //     shipping_address_id: payment_data.payment_intent.shipping_address_id.clone()
        // };

        // let db = &*state.store;
        // let payment_intent = payment_data.payment_intent.clone();

        // let updated_payment_intent = db
        //     .update_payment_intent(
        //         &state.into(),
        //         payment_intent,
        //         payment_intent_update,
        //         key_store,
        //         storage_scheme,
        //     )
        //     .await
        //     .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // req_state
        //     .event_context
        //     .event(AuditEvent::new(AuditEventType::PaymentContinueRedirection))
        //     .with(payment_data.to_event())
        //     .emit();

        // payment_data.payment_intent = updated_payment_intent;
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone + Sync>
    ValidateRequest<F, PaymentsContinueRedirectionRequest, PaymentConfirmData<F>>
    for ContinueRedirection
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &PaymentsContinueRedirectionRequest,
        merchant_context: &'a domain::MerchantContext,
    ) -> RouterResult<operations::ValidateResult> {
        let validate_result = operations::ValidateResult {
            merchant_id: merchant_context.get_merchant_account().get_id().to_owned(),
            storage_scheme: merchant_context.get_merchant_account().storage_scheme,
            requeue: false,
        };

        Ok(validate_result)
    }
}
