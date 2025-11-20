use api_models::{enums::FrmSuggestion, payments::PaymentsRetrieveRequest};
use async_trait::async_trait;
use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::PaymentStatusData;
use router_env::{instrument, tracing};

use super::{Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{
            helpers,
            operations::{self, ValidateStatusForOperation},
        },
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
pub struct PaymentGet;

impl ValidateStatusForOperation for PaymentGet {
    /// Validate if the current operation can be performed on the current status of the payment intent
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        match intent_status {
            common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Conflicted
            | common_enums::IntentStatus::Expired => Ok(()),
            // These statuses are not valid for this operation
            common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::RequiresPaymentMethod => {
                Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                    current_flow: format!("{self:?}"),
                    field_name: "status".to_string(),
                    current_value: intent_status.to_string(),
                    states: [
                        common_enums::IntentStatus::RequiresCapture,
                        common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture,
                        common_enums::IntentStatus::RequiresCustomerAction,
                        common_enums::IntentStatus::RequiresMerchantAction,
                        common_enums::IntentStatus::Processing,
                        common_enums::IntentStatus::Succeeded,
                        common_enums::IntentStatus::Failed,
                        common_enums::IntentStatus::PartiallyCapturedAndCapturable,
                        common_enums::IntentStatus::PartiallyCaptured,
                        common_enums::IntentStatus::Cancelled,
                    ]
                    .map(|enum_value| enum_value.to_string())
                    .join(", "),
                })
            }
        }
    }
}

type BoxedConfirmOperation<'b, F> =
    super::BoxedOperation<'b, F, PaymentsRetrieveRequest, PaymentStatusData<F>>;

// TODO: change the macro to include changes for v2
// TODO: PaymentData in the macro should be an input
impl<F: Send + Clone + Sync> Operation<F, PaymentsRetrieveRequest> for &PaymentGet {
    type Data = PaymentStatusData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentsRetrieveRequest, Self::Data> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsRetrieveRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, PaymentsRetrieveRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsRetrieveRequest> + Send + Sync)>
    {
        Ok(*self)
    }
}
#[automatically_derived]
impl<F: Send + Clone + Sync> Operation<F, PaymentsRetrieveRequest> for PaymentGet {
    type Data = PaymentStatusData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentsRetrieveRequest, Self::Data> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsRetrieveRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsRetrieveRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsRetrieveRequest> + Send + Sync)>
    {
        Ok(self)
    }
}

impl<F: Send + Clone + Sync> ValidateRequest<F, PaymentsRetrieveRequest, PaymentStatusData<F>>
    for PaymentGet
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &PaymentsRetrieveRequest,
        platform: &'a domain::Platform,
    ) -> RouterResult<operations::ValidateResult> {
        let validate_result = operations::ValidateResult {
            merchant_id: platform.get_processor().get_account().get_id().to_owned(),
            storage_scheme: platform.get_processor().get_account().storage_scheme,
            requeue: false,
        };

        Ok(validate_result)
    }
}

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentStatusData<F>, PaymentsRetrieveRequest>
    for PaymentGet
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &PaymentsRetrieveRequest,
        platform: &domain::Platform,
        _profile: &domain::Profile,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<operations::GetTrackerResponse<PaymentStatusData<F>>> {
        let db = &*state.store;

        let storage_scheme = platform.get_processor().get_account().storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_id(
                payment_id,
                platform.get_processor().get_key_store(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        self.validate_status_for_operation(payment_intent.status)?;

        let active_attempt_id = payment_intent.active_attempt_id.as_ref().ok_or_else(|| {
            errors::ApiErrorResponse::MissingRequiredField {
                field_name: ("active_attempt_id"),
            }
        })?;

        let mut payment_attempt = db
            .find_payment_attempt_by_id(
                platform.get_processor().get_key_store(),
                active_attempt_id,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not find payment attempt given the attempt id")?;

        payment_attempt.encoded_data = request
            .param
            .as_ref()
            .map(|val| masking::Secret::new(val.clone()));

        let should_sync_with_connector =
            request.force_sync && payment_intent.status.should_force_sync_with_connector();

        // We need the address here to send it in the response
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

        let attempts = match request.expand_attempts {
            true => payment_intent
                .active_attempt_id
                .as_ref()
                .async_map(|active_attempt| async {
                    db.find_payment_attempts_by_payment_intent_id(
                        payment_id,
                        platform.get_processor().get_key_store(),
                        storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Could not find payment attempts for the given the intent id")
                })
                .await
                .transpose()?,
            false => None,
        };

        let merchant_connector_details = request.merchant_connector_details.clone();

        let payment_data = PaymentStatusData {
            flow: std::marker::PhantomData,
            payment_intent,
            payment_attempt,
            payment_address,
            attempts,
            should_sync_with_connector,
            merchant_connector_details,
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, PaymentsRetrieveRequest, PaymentStatusData<F>>
    for PaymentGet
{
    async fn get_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentStatusData<F>,
        merchant_key_store: &domain::MerchantKeyStore,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<(BoxedConfirmOperation<'a, F>, Option<domain::Customer>), errors::StorageError>
    {
        match payment_data.payment_intent.customer_id.clone() {
            Some(id) => {
                let customer = state
                    .store
                    .find_customer_by_global_id(&id, merchant_key_store, storage_scheme)
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
        _payment_data: &mut PaymentStatusData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
        _should_retry_with_pan: bool,
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
        _platform: &domain::Platform,
        _business_profile: &domain::Profile,
        state: &SessionState,
        // TODO: do not take the whole payment data here
        payment_data: &mut PaymentStatusData<F>,
    ) -> CustomResult<ConnectorCallType, errors::ApiErrorResponse> {
        let payment_attempt = &payment_data.payment_attempt;

        if payment_data.should_sync_with_connector {
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

            Ok(ConnectorCallType::PreDetermined(
                api::ConnectorRoutingData::from(connector_data),
            ))
        } else {
            Ok(ConnectorCallType::Skip)
        }
    }

    #[cfg(feature = "v2")]
    async fn get_connector_from_request<'a>(
        &'a self,
        state: &SessionState,
        request: &PaymentsRetrieveRequest,
        payment_data: &mut PaymentStatusData<F>,
    ) -> CustomResult<api::ConnectorData, errors::ApiErrorResponse> {
        use crate::core::payments::OperationSessionSetters;

        let connector_data = helpers::get_connector_data_from_request(
            state,
            request.merchant_connector_details.clone(),
        )
        .await?;

        payment_data
            .set_connector_in_payment_attempt(Some(connector_data.connector_name.to_string()));
        Ok(connector_data)
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentStatusData<F>, PaymentsRetrieveRequest>
    for PaymentGet
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        _state: &'b SessionState,
        _req_state: ReqState,
        payment_data: PaymentStatusData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(BoxedConfirmOperation<'b, F>, PaymentStatusData<F>)>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}
