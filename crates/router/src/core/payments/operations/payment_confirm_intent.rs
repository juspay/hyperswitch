use api_models::{
    admin::ExtendedCardInfoConfig,
    enums::FrmSuggestion,
    payments::{ExtendedCardInfo, GetAddressFromPaymentMethodData, PaymentsConfirmIntentRequest},
};
use async_trait::async_trait;
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::{
    payment_attempt::PaymentAttempt, PaymentConfirmData, PaymentIntent,
};
use router_env::{instrument, tracing};
use tracing_futures::Instrument;

use super::{Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        authentication,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{
            self, helpers,
            operations::{self, ValidateStatusForOperation},
            populate_surcharge_details, CustomerDetails, PaymentAddress, PaymentData,
        },
        utils as core_utils,
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{
        self,
        api::{self, ConnectorCallType, PaymentIdTypeExt},
        domain::{self},
        storage::{self, enums as storage_enums},
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentsIntentConfirm;

impl ValidateStatusForOperation for PaymentsIntentConfirm {
    /// Validate if the current operation can be performed on the current status
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        match intent_status {
            common_enums::IntentStatus::RequiresPaymentMethod => Ok(()),
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => {
                Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                    current_flow: "cofirm_intent".to_string(),
                    field_name: "status".to_string(),
                    current_value: intent_status.to_string(),
                    states: vec!["requires_payment_method".to_string()].join(", "),
                })
            }
        }
    }
}

type BoxedConfirmOperation<'b, F> =
    super::BoxedOperation<'b, F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>;

// TODO: change the macro to include changes for v2
// TODO: PaymentData in the macro should be an input
impl<F: Send + Clone> Operation<F, PaymentsConfirmIntentRequest> for &PaymentsIntentConfirm {
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsConfirmIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_domain(
        &self,
    ) -> RouterResult<&(dyn Domain<F, PaymentsConfirmIntentRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone> Operation<F, PaymentsConfirmIntentRequest> for PaymentsIntentConfirm {
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, PaymentsConfirmIntentRequest, Self::Data> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsConfirmIntentRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsConfirmIntentRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

impl<F: Send + Clone> ValidateRequest<F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>
    for PaymentsIntentConfirm
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &PaymentsConfirmIntentRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(BoxedConfirmOperation<'b, F>, operations::ValidateResult)> {
        let validate_result = operations::ValidateResult {
            merchant_id: merchant_account.get_id().to_owned(),
            storage_scheme: merchant_account.storage_scheme,
            requeue: false,
        };

        Ok((Box::new(self), validate_result))
    }
}

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentConfirmData<F>, PaymentsConfirmIntentRequest>
    for PaymentsIntentConfirm
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsConfirmIntentRequest,
        merchant_account: &domain::MerchantAccount,
        profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<
        operations::GetTrackerResponse<'a, F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>,
    > {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let storage_scheme = merchant_account.storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_id(key_manager_state, payment_id, key_store, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        self.validate_status_for_operation(payment_intent.status)?;

        let cell_id = state.conf.cell_information.id.clone();

        let payment_attempt_domain_model =
            hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt::create_domain_model(
                &payment_intent,
                cell_id,
                storage_scheme,
                request
            )
            .await?;

        let payment_attempt = db
            .insert_payment_attempt(
                key_manager_state,
                key_store,
                payment_attempt_domain_model,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not insert payment attempt")?;

        let payment_method_data = request
            .payment_method_data
            .payment_method_data
            .clone()
            .map(hyperswitch_domain_models::payment_method_data::PaymentMethodData::from);

        let payment_data = PaymentConfirmData {
            flow: std::marker::PhantomData,
            payment_intent,
            payment_attempt,
            payment_method_data,
        };

        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(self),
            payment_data,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send> Domain<F, PaymentsConfirmIntentRequest, PaymentConfirmData<F>>
    for PaymentsIntentConfirm
{
    async fn get_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
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
                        id.get_string_repr(),
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
        state: &'a SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        business_profile: &domain::Profile,
    ) -> RouterResult<(
        BoxedConfirmOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        request: &PaymentsConfirmIntentRequest,
        _payment_intent: &storage::PaymentIntent,
        _key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        todo!()
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, PaymentConfirmData<F>, PaymentsConfirmIntentRequest>
    for PaymentsIntentConfirm
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        req_state: ReqState,
        mut payment_data: PaymentConfirmData<F>,
        customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        frm_suggestion: Option<FrmSuggestion>,
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(BoxedConfirmOperation<'b, F>, PaymentConfirmData<F>)>
    where
        F: 'b + Send,
    {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let intent_status = common_enums::IntentStatus::Processing;
        let attempt_status = common_enums::AttemptStatus::Pending;

        let connector = payment_data
            .payment_attempt
            .connector
            .clone()
            .get_required_value("connector")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Connector is none when constructing response")?;

        let merchant_connector_id = payment_data
            .payment_attempt
            .merchant_connector_id
            .clone()
            .get_required_value("merchant_connector_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Merchant connector id is none when constructing response")?;

        let payment_intent_update =
            hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::ConfirmIntent {
                status: intent_status,
                updated_by: storage_scheme.to_string(),
            };

        let payment_attempt_update = hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate::ConfirmIntent {
            status: attempt_status,
            updated_by: storage_scheme.to_string(),
            connector,
            merchant_connector_id,
        };

        let updated_payment_intent = db
            .update_payment_intent(
                key_manager_state,
                payment_data.payment_intent.clone(),
                payment_intent_update,
                key_store,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment intent")?;

        payment_data.payment_intent = updated_payment_intent;

        let updated_payment_attempt = db
            .update_payment_attempt(
                key_manager_state,
                key_store,
                payment_data.payment_attempt.clone(),
                payment_attempt_update,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment attempt")?;

        payment_data.payment_attempt = updated_payment_attempt;

        Ok((Box::new(self), payment_data))
    }
}
