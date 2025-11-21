use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::{ext_traits::AsyncExt, id_type::GlobalPaymentId};
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::{
    BoxedOperation, Domain, GetTracker, Operation, OperationSessionSetters, UpdateTracker,
    ValidateRequest, ValidateStatusForOperation,
};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::operations,
    },
    routes::{app::ReqState, SessionState},
    types::{
        self,
        api::{self, ConnectorCallType, PaymentIdTypeExt},
        domain,
        storage::{self, enums},
        PaymentsCancelData,
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentsCancel;

type BoxedCancelOperation<'b, F> = BoxedOperation<
    'b,
    F,
    api::PaymentsCancelRequest,
    hyperswitch_domain_models::payments::PaymentCancelData<F>,
>;

// Manual Operation trait implementation for V2
impl<F: Send + Clone + Sync> Operation<F, api::PaymentsCancelRequest> for &PaymentsCancel {
    type Data = hyperswitch_domain_models::payments::PaymentCancelData<F>;

    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, api::PaymentsCancelRequest, Self::Data> + Send + Sync)>
    {
        Ok(*self)
    }

    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, api::PaymentsCancelRequest> + Send + Sync)>
    {
        Ok(*self)
    }

    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, api::PaymentsCancelRequest, Self::Data>)> {
        Ok(*self)
    }

    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, api::PaymentsCancelRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}

#[automatically_derived]
impl<F: Send + Clone + Sync> Operation<F, api::PaymentsCancelRequest> for PaymentsCancel {
    type Data = hyperswitch_domain_models::payments::PaymentCancelData<F>;

    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, api::PaymentsCancelRequest, Self::Data> + Send + Sync)>
    {
        Ok(self)
    }

    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, api::PaymentsCancelRequest> + Send + Sync)>
    {
        Ok(self)
    }

    fn to_domain(&self) -> RouterResult<&dyn Domain<F, api::PaymentsCancelRequest, Self::Data>> {
        Ok(self)
    }

    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, api::PaymentsCancelRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

#[cfg(feature = "v2")]
impl<F: Send + Clone + Sync>
    ValidateRequest<
        F,
        api::PaymentsCancelRequest,
        hyperswitch_domain_models::payments::PaymentCancelData<F>,
    > for PaymentsCancel
{
    #[instrument(skip_all)]
    fn validate_request(
        &self,
        _request: &api::PaymentsCancelRequest,
        platform: &domain::Platform,
    ) -> RouterResult<operations::ValidateResult> {
        Ok(operations::ValidateResult {
            merchant_id: platform.get_processor().get_account().get_id().to_owned(),
            storage_scheme: platform.get_processor().get_account().storage_scheme,
            requeue: false,
        })
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<F: Send + Clone + Sync>
    GetTracker<
        F,
        hyperswitch_domain_models::payments::PaymentCancelData<F>,
        api::PaymentsCancelRequest,
    > for PaymentsCancel
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &api::PaymentsCancelRequest,
        platform: &domain::Platform,
        profile: &domain::Profile,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<
        operations::GetTrackerResponse<hyperswitch_domain_models::payments::PaymentCancelData<F>>,
    > {
        let db = &*state.store;

        let merchant_id = platform.get_processor().get_account().get_id();
        let storage_scheme = platform.get_processor().get_account().storage_scheme;
        let payment_intent = db
            .find_payment_intent_by_id(
                payment_id,
                platform.get_processor().get_key_store(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable("Failed to find payment intent for cancellation")?;

        self.validate_status_for_operation(payment_intent.status)?;

        let active_attempt_id = payment_intent.active_attempt_id.as_ref().ok_or_else(|| {
            errors::ApiErrorResponse::InvalidRequestData {
                message: "Payment cancellation not possible - no active payment attempt found"
                    .to_string(),
            }
        })?;

        let payment_attempt = db
            .find_payment_attempt_by_id(
                platform.get_processor().get_key_store(),
                active_attempt_id,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable("Failed to find payment attempt for cancellation")?;

        let mut payment_data = hyperswitch_domain_models::payments::PaymentCancelData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
        };

        payment_data.set_cancellation_reason(request.cancellation_reason.clone());

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<F: Clone + Send + Sync>
    UpdateTracker<
        F,
        hyperswitch_domain_models::payments::PaymentCancelData<F>,
        api::PaymentsCancelRequest,
    > for PaymentsCancel
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        req_state: ReqState,
        mut payment_data: hyperswitch_domain_models::payments::PaymentCancelData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        merchant_key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(
        BoxedCancelOperation<'b, F>,
        hyperswitch_domain_models::payments::PaymentCancelData<F>,
    )>
    where
        F: 'b + Send,
    {
        let db = &*state.store;

        let payment_attempt_update = hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate::VoidUpdate {
            status: enums::AttemptStatus::VoidInitiated,
            cancellation_reason: payment_data.payment_attempt.cancellation_reason.clone(),
            updated_by: storage_scheme.to_string(),
        };

        let updated_payment_attempt = db
            .update_payment_attempt(
                merchant_key_store,
                payment_data.payment_attempt.clone(),
                payment_attempt_update,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable("Failed to update payment attempt for cancellation")?;
        payment_data.set_payment_attempt(updated_payment_attempt);

        Ok((Box::new(self), payment_data))
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl<F: Send + Clone + Sync>
    Domain<F, api::PaymentsCancelRequest, hyperswitch_domain_models::payments::PaymentCancelData<F>>
    for PaymentsCancel
{
    async fn get_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut hyperswitch_domain_models::payments::PaymentCancelData<F>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<(BoxedCancelOperation<'a, F>, Option<domain::Customer>), errors::StorageError>
    {
        Ok((Box::new(*self), None))
    }

    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut hyperswitch_domain_models::payments::PaymentCancelData<F>,
        _storage_scheme: enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
        _should_retry_with_pan: bool,
    ) -> RouterResult<(
        BoxedCancelOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(*self), None, None))
    }

    async fn perform_routing<'a>(
        &'a self,
        _platform: &domain::Platform,
        _business_profile: &domain::Profile,
        state: &SessionState,
        payment_data: &mut hyperswitch_domain_models::payments::PaymentCancelData<F>,
    ) -> RouterResult<api::ConnectorCallType> {
        let payment_attempt = &payment_data.payment_attempt;
        let connector = payment_attempt
            .connector
            .as_ref()
            .get_required_value("connector")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Connector not found for payment cancellation")?;

        let merchant_connector_id = payment_attempt
            .merchant_connector_id
            .as_ref()
            .get_required_value("merchant_connector_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Merchant connector ID not found for payment cancellation")?;

        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector,
            api::GetToken::Connector,
            Some(merchant_connector_id.to_owned()),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received")?;

        Ok(api::ConnectorCallType::PreDetermined(connector_data.into()))
    }
}

impl ValidateStatusForOperation for PaymentsCancel {
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        match intent_status {
            common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable
            | common_enums::IntentStatus::RequiresCapture => Ok(()),
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::Conflicted
            | common_enums::IntentStatus::Expired => {
                Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                    current_flow: format!("{self:?}"),
                    field_name: "status".to_string(),
                    current_value: intent_status.to_string(),
                    states: [
                        common_enums::IntentStatus::RequiresCapture,
                        common_enums::IntentStatus::PartiallyCapturedAndCapturable,
                        common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture,
                    ]
                    .map(|enum_value| enum_value.to_string())
                    .join(", "),
                })
            }
        }
    }
}
