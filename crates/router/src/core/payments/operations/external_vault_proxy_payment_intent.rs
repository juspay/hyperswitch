use api_models::payments::ExternalVaultProxyPaymentsRequest;
use async_trait::async_trait;
use common_enums::enums;
use common_utils::{
    crypto::Encryptable,
    ext_traits::{AsyncExt, ValueExt},
    types::keymanager::ToEncryptable,
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData, payments::PaymentConfirmData,
};
use hyperswitch_interfaces::api::ConnectorSpecifications;
use masking::PeekInterface;
use router_env::{instrument, tracing};

use super::{Domain, GetTracker, Operation, PostUpdateTracker, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods::{self, PaymentMethodExt},
        payments::{
            self,
            operations::{self, ValidateStatusForOperation},
            OperationSessionGetters, OperationSessionSetters,
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
pub struct ExternalVaultProxyPaymentIntent;

impl ValidateStatusForOperation for ExternalVaultProxyPaymentIntent {
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

type BoxedConfirmOperation<'b, F> =
    super::BoxedOperation<'b, F, ExternalVaultProxyPaymentsRequest, PaymentConfirmData<F>>;

impl<F: Send + Clone + Sync> Operation<F, ExternalVaultProxyPaymentsRequest>
    for &ExternalVaultProxyPaymentIntent
{
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, ExternalVaultProxyPaymentsRequest, Self::Data> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<
        &(dyn GetTracker<F, Self::Data, ExternalVaultProxyPaymentsRequest> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_domain(
        &self,
    ) -> RouterResult<&(dyn Domain<F, ExternalVaultProxyPaymentsRequest, Self::Data>)> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn UpdateTracker<F, Self::Data, ExternalVaultProxyPaymentsRequest> + Send + Sync),
    > {
        Ok(*self)
    }
}

#[automatically_derived]
impl<F: Send + Clone + Sync> Operation<F, ExternalVaultProxyPaymentsRequest>
    for ExternalVaultProxyPaymentIntent
{
    type Data = PaymentConfirmData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, ExternalVaultProxyPaymentsRequest, Self::Data> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<
        &(dyn GetTracker<F, Self::Data, ExternalVaultProxyPaymentsRequest> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_domain(
        &self,
    ) -> RouterResult<&dyn Domain<F, ExternalVaultProxyPaymentsRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn UpdateTracker<F, Self::Data, ExternalVaultProxyPaymentsRequest> + Send + Sync),
    > {
        Ok(self)
    }
}

impl<F: Send + Clone + Sync>
    ValidateRequest<F, ExternalVaultProxyPaymentsRequest, PaymentConfirmData<F>>
    for ExternalVaultProxyPaymentIntent
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        _request: &ExternalVaultProxyPaymentsRequest,
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
impl<F: Send + Clone + Sync> GetTracker<F, PaymentConfirmData<F>, ExternalVaultProxyPaymentsRequest>
    for ExternalVaultProxyPaymentIntent
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        request: &ExternalVaultProxyPaymentsRequest,
        platform: &domain::Platform,
        _profile: &domain::Profile,
        header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<operations::GetTrackerResponse<PaymentConfirmData<F>>> {
        let db = &*state.store;
        let key_manager_state = &state.into();

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
            common_utils::types::keymanager::Identifier::Merchant(platform.get_processor().get_account().get_id().to_owned()),
            platform.get_processor().get_key_store().key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while encrypting payment intent details".to_string())?;

        let encrypted_data =
             hyperswitch_domain_models::payments::payment_attempt::FromRequestEncryptablePaymentAttempt::from_encryptable(batch_encrypted_data)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while encrypting payment intent details")?;

        let payment_attempt = match payment_intent.active_attempt_id.clone() {
            Some(ref active_attempt_id) => db
                .find_payment_attempt_by_id(
                    platform.get_processor().get_key_store(),
                    active_attempt_id,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound)
                .attach_printable("Could not find payment attempt")?,

            None => {
                // TODO: Implement external vault specific payment attempt creation logic
                let payment_attempt_domain_model: hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt =
                hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt::external_vault_proxy_create_domain_model(
                    &payment_intent,
                    cell_id,
                    storage_scheme,
                    request,
                    encrypted_data
                )
                .await?;
                db.insert_payment_attempt(
                    platform.get_processor().get_key_store(),
                    payment_attempt_domain_model,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Could not insert payment attempt")?
            }
        };

        // TODO: Extract external vault specific token/credentials from request
        let processor_payment_token = None; // request.external_vault_details.processor_payment_token.clone();

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

        // TODO: Implement external vault specific mandate data handling
        let mandate_data_input = api_models::payments::MandateIds {
            mandate_id: None,
            mandate_reference_id: processor_payment_token.map(|token| {
                api_models::payments::MandateReferenceId::ConnectorMandateId(
                    api_models::payments::ConnectorMandateReferenceId::new(
                        Some(token),
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                )
            }),
        };
        let payment_method_data = request.payment_method_data.payment_method_data.clone().map(
            hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::from,
        );

        let payment_data = PaymentConfirmData {
            flow: std::marker::PhantomData,
            payment_intent,
            payment_attempt,
            payment_method_data: None, // TODO: Review for external vault
            payment_address,
            mandate_data: Some(mandate_data_input),
            payment_method: None,
            merchant_connector_details: None,
            external_vault_pmd: payment_method_data,
            webhook_url: None,
        };

        let get_trackers_response = operations::GetTrackerResponse { payment_data };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, ExternalVaultProxyPaymentsRequest, PaymentConfirmData<F>>
    for ExternalVaultProxyPaymentIntent
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
        // TODO: Implement external vault specific payment method data creation
        Ok((Box::new(self), None, None))
    }

    async fn create_or_fetch_payment_method<'a>(
        &'a self,
        state: &SessionState,
        platform: &domain::Platform,
        business_profile: &domain::Profile,
        payment_data: &mut PaymentConfirmData<F>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        match (
            payment_data.payment_intent.customer_id.clone(),
            payment_data.external_vault_pmd.clone(),
            payment_data.payment_attempt.customer_acceptance.clone(),
            payment_data.payment_attempt.payment_token.clone(),
        ) {
            (Some(customer_id), Some(hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::Card(card_details)), Some(_), None) => {

                let payment_method_data =
                    api::PaymentMethodCreateData::ProxyCard(api::ProxyCardDetails::from(*card_details));
                let billing = payment_data
                    .payment_address
                    .get_payment_method_billing()
                    .cloned()
                    .map(From::from);

                let req = api::PaymentMethodCreate {
                    payment_method_type: payment_data.payment_attempt.payment_method_type,
                    payment_method_subtype: payment_data.payment_attempt.payment_method_subtype,
                    metadata: None,
                    customer_id,
                    payment_method_data,
                    billing,
                    psp_tokenization: None,
                    network_tokenization: None,
                };

                let (_pm_response, payment_method) = Box::pin(payment_methods::create_payment_method_core(
                    state,
                    &state.get_req_state(),
                    req,
                    platform,
                    business_profile,
                ))
                .await?;

                payment_data.payment_method = Some(payment_method);
            }
            (_, Some(hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::VaultToken(vault_token)), None, Some(payment_token)) => {
                payment_data.external_vault_pmd = Some(payment_methods::get_external_vault_token(
                    state,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                    payment_token.clone(),
                    vault_token.clone(),
                    &payment_data.payment_attempt.payment_method_type
                )
                .await?);
            }
            _ => {
                router_env::logger::debug!(
                    "No payment method to create or fetch for external vault proxy payment intent"
                );
            }
        }

        Ok(())
    }

    #[cfg(feature = "v2")]
    async fn update_payment_method<'a>(
        &'a self,
        state: &SessionState,
        platform: &domain::Platform,
        payment_data: &mut PaymentConfirmData<F>,
    ) {
        if let (true, Some(payment_method)) = (
            payment_data.payment_attempt.customer_acceptance.is_some(),
            payment_data.payment_method.as_ref(),
        ) {
            payment_methods::update_payment_method_status_internal(
                state,
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().storage_scheme,
                common_enums::PaymentMethodStatus::Active,
                payment_method.get_id(),
            )
            .await
            .map_err(|err| router_env::logger::error!(err=?err));
        };
    }

    #[instrument(skip_all)]
    async fn populate_payment_data<'a>(
        &'a self,
        _state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
        _platform: &domain::Platform,
        _business_profile: &domain::Profile,
        connector_data: &api::ConnectorData,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        let connector_request_reference_id = connector_data
            .connector
            .generate_connector_request_reference_id(
                &payment_data.payment_intent,
                &payment_data.payment_attempt,
            );
        payment_data.set_connector_request_reference_id(Some(connector_request_reference_id));
        Ok(())
    }

    async fn perform_routing<'a>(
        &'a self,
        platform: &domain::Platform,
        business_profile: &domain::Profile,
        state: &SessionState,
        payment_data: &mut PaymentConfirmData<F>,
    ) -> CustomResult<ConnectorCallType, errors::ApiErrorResponse> {
        payments::connector_selection(state, platform, business_profile, payment_data, None).await
    }
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentConfirmData<F>, ExternalVaultProxyPaymentsRequest>
    for ExternalVaultProxyPaymentIntent
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

        let intent_status = common_enums::IntentStatus::Processing;
        let attempt_status = common_enums::AttemptStatus::Pending;

        let connector = payment_data
            .payment_attempt
            .connector
            .clone()
            .get_required_value("connector")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Connector is none when constructing response")?;

        let merchant_connector_id = Some(
            payment_data
                .payment_attempt
                .merchant_connector_id
                .clone()
                .get_required_value("merchant_connector_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Merchant connector id is none when constructing response")?,
        );

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

        let connector_request_reference_id = payment_data
            .payment_attempt
            .connector_request_reference_id
            .clone();

        let connector_response_reference_id = payment_data
            .payment_attempt
            .connector_response_reference_id
            .clone();

        let payment_attempt_update = hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate::ConfirmIntent {
            status: attempt_status,
            updated_by: storage_scheme.to_string(),
            connector,
            merchant_connector_id,
            authentication_type,
            connector_request_reference_id,
            connector_response_reference_id,
        };

        let updated_payment_intent = db
            .update_payment_intent(
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
    for ExternalVaultProxyPaymentIntent
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

        let response_router_data = response;

        let payment_intent_update =
            response_router_data.get_payment_intent_update(&payment_data, storage_scheme);
        let payment_attempt_update =
            response_router_data.get_payment_attempt_update(&payment_data, storage_scheme);

        let updated_payment_intent = db
            .update_payment_intent(
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

        // TODO: Add external vault specific post-update logic

        Ok(payment_data)
    }
}
