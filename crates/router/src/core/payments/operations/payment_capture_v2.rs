use api_models::{enums::FrmSuggestion, payments::PaymentsCaptureRequest};
use async_trait::async_trait;
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::PaymentCaptureData;
use router_env::{instrument, tracing};

use super::{Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{
            helpers,
            operations::{self, ValidateStatusForOperation},
        },
        utils::ValidatePlatformMerchant,
    },
    routes::{app::ReqState, SessionState},
    types::{
        api::{self, ConnectorCallType},
        domain::{self},
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentsCapture;

impl ValidateStatusForOperation for PaymentsCapture {
    /// Validate if the current operation can be performed on the current status of the payment intent
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        match intent_status {
            common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => Ok(()),
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::RequiresConfirmation => {
                Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                    current_flow: format!("{self:?}"),
                    field_name: "status".to_string(),
                    current_value: intent_status.to_string(),
                    states: [
                        common_enums::IntentStatus::RequiresCapture,
                        common_enums::IntentStatus::PartiallyCapturedAndCapturable,
                    ]
                    .map(|enum_value| enum_value.to_string())
                    .join(", "),
                })
            }
        }
    }
}

type BoxedConfirmOperation<'b, F> =
    super::BoxedOperation<'b, F, PaymentsCaptureRequest, PaymentCaptureData<F>>;

// TODO: change the macro to include changes for v2
// TODO: PaymentData in the macro should be an input
impl<F: Send + Clone> Operation<F, PaymentsCaptureRequest> for &PaymentsCapture {
    type Data = PaymentCaptureData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentsCaptureRequest, Self::Data> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsCaptureRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, PaymentsCaptureRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsCaptureRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> Operation<F, PaymentsCaptureRequest> for PaymentsCapture {
    type Data = PaymentCaptureData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentsCaptureRequest, Self::Data> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsCaptureRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsCaptureRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsCaptureRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

impl<F: Send + Clone> ValidateRequest<F, PaymentsCaptureRequest, PaymentCaptureData<F>>
    for PaymentsCapture
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &PaymentsCaptureRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<operations::ValidateResult> {
        let validate_result = operations::ValidateResult {
            merchant_id: merchant_account.get_id().to_owned(),
            storage_scheme: merchant_account.storage_scheme,
            requeue: false,
        };

        Ok(validate_result)
    }
}

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentCaptureData<F>, PaymentsCaptureRequest>
    for PaymentsCapture
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsCaptureRequest,
        merchant_account: &domain::MerchantAccount,
        _profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<operations::GetTrackerResponse<PaymentCaptureData<F>>> {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let storage_scheme = merchant_account.storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_id(key_manager_state, payment_id, key_store, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent
            .validate_platform_merchant(platform_merchant_account.map(|ma| ma.get_id()))?;

        self.validate_status_for_operation(payment_intent.status)?;

        let active_attempt_id = payment_intent
            .active_attempt_id
            .as_ref()
            .get_required_value("active_attempt_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Active attempt id is none when capturing the payment")?;

        let mut payment_attempt = db
            .find_payment_attempt_by_id(
                key_manager_state,
                key_store,
                active_attempt_id,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not find payment attempt given the attempt id")?;

        if let Some(amount_to_capture) = request.amount_to_capture {
            payment_attempt
                .amount_details
                .validate_amount_to_capture(amount_to_capture)
                .change_context(errors::ApiErrorResponse::PreconditionFailed {
                    message: format!(
                        "`amount_to_capture` is greater than the net amount {}",
                        payment_attempt.amount_details.get_net_amount()
                    ),
                })?;

            payment_attempt
                .amount_details
                .set_amount_to_capture(amount_to_capture);
        }

        let payment_data = PaymentCaptureData {
            flow: std::marker::PhantomData,
            payment_intent,
            payment_attempt,
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send> Domain<F, PaymentsCaptureRequest, PaymentCaptureData<F>> for PaymentsCapture {
    async fn get_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentCaptureData<F>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<(BoxedConfirmOperation<'a, F>, Option<domain::Customer>), errors::StorageError>
    {
        match payment_data.payment_intent.customer_id.clone() {
            Some(id) => {
                let customer = state
                    .store
                    .find_customer_by_global_id(
                        &state.into(),
                        &id,
                        &payment_data.payment_intent.merchant_id,
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
        _state: &'a SessionState,
        _payment_data: &mut PaymentCaptureData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        BoxedConfirmOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    #[instrument(skip_all)]
    async fn perform_routing<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        _business_profile: &domain::Profile,
        state: &SessionState,
        // TODO: do not take the whole payment data here
        payment_data: &mut PaymentCaptureData<F>,
        _mechant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<ConnectorCallType, errors::ApiErrorResponse> {
        let payment_attempt = &payment_data.payment_attempt;
        let connector = payment_attempt
            .connector
            .as_ref()
            .get_required_value("connector")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Connector is none when constructing response")?;

        let merchant_connector_id = payment_attempt
            .merchant_connector_id
            .as_ref()
            .get_required_value("merchant_connector_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Merchant connector id is none when constructing response")?;

        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector,
            api::GetToken::Connector,
            Some(merchant_connector_id.to_owned()),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received")?;

        Ok(ConnectorCallType::PreDetermined(connector_data))
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, PaymentCaptureData<F>, PaymentsCaptureRequest> for PaymentsCapture {
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        _req_state: ReqState,
        mut payment_data: PaymentCaptureData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(BoxedConfirmOperation<'b, F>, PaymentCaptureData<F>)>
    where
        F: 'b + Send,
    {
        let payment_attempt_update = hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate::PreCaptureUpdate { amount_to_capture: payment_data.payment_attempt.amount_details.get_amount_to_capture(), updated_by: storage_scheme.to_string() };

        let payment_attempt = state
            .store
            .update_payment_attempt(
                &state.into(),
                key_store,
                payment_data.payment_attempt.clone(),
                payment_attempt_update,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not update payment attempt")?;

        payment_data.payment_attempt = payment_attempt;
        Ok((Box::new(self), payment_data))
    }
}
