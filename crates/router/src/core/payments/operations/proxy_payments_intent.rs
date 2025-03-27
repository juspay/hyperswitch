use api_models::payments::ProxyPaymentsRequest;
use async_trait::async_trait;
use common_enums::enums;
use common_utils::types::keymanager::ToEncryptable;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData, payments::PaymentConfirmData,
};
use masking::PeekInterface;
use router_env::{instrument, tracing};

use super::{Domain, GetTracker, Operation, PostUpdateTracker, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{
            operations::{self, ValidateStatusForOperation},
            OperationSessionGetters,
        },
    },
    routes::{app::ReqState, SessionState},
    types::{
        self,
        api::{self, ConnectorCallType},
        domain::{self, types as domain_types},
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentProxyIntent;

impl ValidateStatusForOperation for PaymentProxyIntent {
    /// Validate if the current operation can be performed on the current status of the payment intent
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        match intent_status {
            //Failed state is included here so that in PCR, retries can be done for failed payments, otherwise for a failed attempt it was asking for new payment_intent
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::Failed => Ok(()),
            //Failed state is included here so that in PCR, retries can be done for failed payments, otherwise for a failed attempt it was asking for new payment_intent
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::Failed => Ok(()),
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => {
                Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                    current_flow: format!("{self:?}"),
                    field_name: "status".to_string(),
                    current_value: intent_status.to_string(),
                    states: ["requires_payment_method".to_string()].join(", "),
                })
            }
        }
    }
}
type BoxedConfirmOperation<'b, F> =
    super::BoxedOperation<'b, F, ProxyPaymentsRequest, PaymentConfirmData<F>>;

impl<F: Send + Clone + Sync> Operation<F, ProxyPaymentsRequest> for &PaymentProxyIntent {
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, ProxyPaymentsRequest, Self::Data> + Send + Sync)>
    {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, ProxyPaymentsRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F, ProxyPaymentsRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, ProxyPaymentsRequest> + Send + Sync)> {
        Ok(*self)
    }
}

#[automatically_derived]
impl<F: Send + Clone + Sync> Operation<F, ProxyPaymentsRequest> for PaymentProxyIntent {
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, ProxyPaymentsRequest, Self::Data> + Send + Sync)>
    {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, ProxyPaymentsRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, ProxyPaymentsRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, ProxyPaymentsRequest> + Send + Sync)> {
        Ok(self)
    }
}

impl<F: Send + Clone + Sync> ValidateRequest<F, ProxyPaymentsRequest, PaymentConfirmData<F>>
    for PaymentProxyIntent
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &ProxyPaymentsRequest,
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
impl<F: Send + Clone + Sync> GetTracker<F, PaymentConfirmData<F>, ProxyPaymentsRequest>
    for PaymentProxyIntent
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &ProxyPaymentsRequest,
        merchant_account: &domain::MerchantAccount,
        _profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        _platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<operations::GetTrackerResponse<PaymentConfirmData<F>>> {
        let db = &*state.store;
        let key_manager_state = &state.into();

        let storage_scheme = merchant_account.storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_id(key_manager_state, payment_id, key_store, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        self.validate_status_for_operation(payment_intent.status)?;

        let cell_id = state.conf.cell_information.id.clone();

        let batch_encrypted_data = domain_types::crypto_operation(
            key_manager_state,
            common_utils::type_name!(hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt),
            domain_types::CryptoOperation::BatchEncrypt(
                hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt::to_encryptable(
                    hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt {
                        payment_method_billing_address: None,
                    },
                ),
            ),
            common_utils::types::keymanager::Identifier::Merchant(merchant_account.get_id().to_owned()),
            key_store.key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while encrypting payment intent details".to_string())?;

        let encrypted_data =
             hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt::from_encryptable(batch_encrypted_data)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while encrypting payment intent details")?;

        let payment_attempt_domain_model: hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt =
            hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt::proxy_create_domain_model(
                &payment_intent,
                cell_id,
                storage_scheme,
                request,
                encrypted_data
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
        let processor_payment_token = request.recurring_details.processor_payment_token.clone();

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
        let mandate_data_input = api_models::payments::MandateIds {
            mandate_id: None,
            mandate_reference_id: Some(
                api_models::payments::MandateReferenceId::ConnectorMandateId(
                    api_models::payments::ConnectorMandateReferenceId::new(
                        Some(processor_payment_token),
                        None,
                        None,
                        None,
                        None,
                    ),
                ),
            ),
        };
        let payment_data = PaymentConfirmData {
            flow: std::marker::PhantomData,
            payment_intent,
            payment_attempt,
            payment_method_data: Some(PaymentMethodData::MandatePayment),
            payment_address,
            mandate_data: Some(mandate_data_input),
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, ProxyPaymentsRequest, PaymentConfirmData<F>>
    for PaymentProxyIntent
{
    async fn get_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut PaymentConfirmData<F>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> CustomResult<(BoxedConfirmOperation<'a, F>, Option<domain::Customer>), errors::StorageError>
    {
        Ok((Box::new(self), None))
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut PaymentConfirmData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
        _should_retry_with_pan: bool,
    ) -> RouterResult<(
        BoxedConfirmOperation<'a, F>,
        Option<PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn perform_routing<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        _business_profile: &domain::Profile,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        _mechant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<ConnectorCallType, errors::ApiErrorResponse> {
        let connector_name = payment_data.get_payment_attempt_connector();
        if let Some(connector_name) = connector_name {
            let merchant_connector_id = payment_data.get_merchant_connector_id_in_attempt();
            let connector_data = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                connector_name,
                api::GetToken::Connector,
                merchant_connector_id,
            )?;

            Ok(ConnectorCallType::PreDetermined(connector_data))
        } else {
            Err(error_stack::Report::new(
                errors::ApiErrorResponse::InternalServerError,
            ))
        }
    }
}
#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentConfirmData<F>, ProxyPaymentsRequest>
    for PaymentProxyIntent
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        _req_state: ReqState,
        mut payment_data: PaymentConfirmData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<api_models::enums::FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
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
                active_attempt_id: Some(payment_data.payment_attempt.id.clone()),
            };

        let authentication_type = payment_data
            .payment_intent
            .authentication_type
            .unwrap_or_default();

        let payment_attempt_update = hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate::ConfirmIntent {
            status: attempt_status,
            updated_by: storage_scheme.to_string(),
            connector,
            merchant_connector_id,
            authentication_type,
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

#[cfg(feature = "v2")]
#[async_trait]
impl<F: Clone> PostUpdateTracker<F, PaymentConfirmData<F>, types::PaymentsAuthorizeData>
    for PaymentProxyIntent
{
    async fn update_tracker<'b>(
        &'b self,
        state: &'b SessionState,
        mut payment_data: PaymentConfirmData<F>,
        response: types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        key_store: &domain::MerchantKeyStore,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<PaymentConfirmData<F>>
    where
        F: 'b + Send + Sync,
        types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>:
            hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<
                F,
                types::PaymentsAuthorizeData,
                PaymentConfirmData<F>,
            >,
    {
        use hyperswitch_domain_models::router_data::TrackerPostUpdateObjects;

        let db = &*state.store;
        let key_manager_state = &state.into();

        let response_router_data = response;

        let payment_intent_update =
            response_router_data.get_payment_intent_update(&payment_data, storage_scheme);
        let payment_attempt_update =
            response_router_data.get_payment_attempt_update(&payment_data, storage_scheme);

        let updated_payment_intent = db
            .update_payment_intent(
                key_manager_state,
                payment_data.payment_intent,
                payment_intent_update,
                key_store,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment intent")?;

        let updated_payment_attempt = db
            .update_payment_attempt(
                key_manager_state,
                key_store,
                payment_data.payment_attempt,
                payment_attempt_update,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment attempt")?;

        payment_data.payment_intent = updated_payment_intent;
        payment_data.payment_attempt = updated_payment_attempt;

        Ok(payment_data)
    }
}
