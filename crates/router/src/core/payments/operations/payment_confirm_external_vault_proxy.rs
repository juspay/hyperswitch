use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::ExternalVaultProxyConfirmRequest};
use async_trait::async_trait;
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
#[cfg(feature = "pm_modular")]
use crate::core::payments::operations::PaymentMethodWithRawData;
use crate::{
    core::{
        configs::dimension_state,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments::{
            helpers, operations,
            CustomerDetails, PaymentAddress, PaymentData,
        },
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt,
};

#[derive(Debug, Clone, Copy)]
pub struct PaymentExternalVaultProxyConfirm;

impl<F: Send + Clone + Sync> Operation<F, ExternalVaultProxyConfirmRequest>
    for PaymentExternalVaultProxyConfirm
{
    type Data = PaymentData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, ExternalVaultProxyConfirmRequest, Self::Data> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<
        &(dyn GetTracker<F, Self::Data, ExternalVaultProxyConfirmRequest> + Send + Sync),
    > {
        Ok(self)
    }
    fn to_domain(
        &self,
    ) -> RouterResult<&dyn Domain<F, ExternalVaultProxyConfirmRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn UpdateTracker<F, Self::Data, ExternalVaultProxyConfirmRequest> + Send + Sync),
    > {
        Ok(self)
    }
}

impl<F: Send + Clone + Sync> Operation<F, ExternalVaultProxyConfirmRequest>
    for &PaymentExternalVaultProxyConfirm
{
    type Data = PaymentData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<
        &(dyn ValidateRequest<F, ExternalVaultProxyConfirmRequest, Self::Data> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<
        &(dyn GetTracker<F, Self::Data, ExternalVaultProxyConfirmRequest> + Send + Sync),
    > {
        Ok(*self)
    }
    fn to_domain(
        &self,
    ) -> RouterResult<&dyn Domain<F, ExternalVaultProxyConfirmRequest, Self::Data>> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<
        &(dyn UpdateTracker<F, Self::Data, ExternalVaultProxyConfirmRequest> + Send + Sync),
    > {
        Ok(*self)
    }
}

type ExternalVaultProxyOperation<'b, F> =
    BoxedOperation<'b, F, ExternalVaultProxyConfirmRequest, PaymentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync>
    GetTracker<F, PaymentData<F>, ExternalVaultProxyConfirmRequest>
    for PaymentExternalVaultProxyConfirm
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &ExternalVaultProxyConfirmRequest,
        platform: &domain::Platform,
        _auth_flow: services::AuthFlow,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        #[cfg(feature = "pm_modular")] _payment_method_wrapper: Option<PaymentMethodWithRawData>,
        _dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> RouterResult<
        operations::GetTrackerResponse<'a, F, ExternalVaultProxyConfirmRequest, PaymentData<F>>,
    > {
        let db = &*state.store;
        let processor_merchant_id = platform.get_processor().get_account().get_id();
        let storage_scheme = platform.get_processor().get_account().storage_scheme;

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let payment_intent = db
            .find_payment_intent_by_payment_id_processor_merchant_id(
                &payment_id,
                processor_merchant_id,
                platform.get_processor().get_key_store(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        helpers::validate_payment_status_against_not_allowed_statuses(
            payment_intent.status,
            &[
                storage_enums::IntentStatus::Cancelled,
                storage_enums::IntentStatus::Succeeded,
                storage_enums::IntentStatus::Processing,
                storage_enums::IntentStatus::RequiresCapture,
                storage_enums::IntentStatus::RequiresMerchantAction,
                storage_enums::IntentStatus::RequiresCustomerAction,
            ],
            "external_vault_proxy_confirm",
        )?;

        let payment_attempt = db
            .find_payment_attempt_by_payment_id_processor_merchant_id_attempt_id(
                &payment_intent.payment_id,
                processor_merchant_id,
                payment_intent.active_attempt.get_id().as_str(),
                storage_scheme,
                platform.get_processor().get_key_store(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let currency = payment_attempt.currency.get_required_value("currency")?;
        let amount = payment_attempt.get_total_amount().into();

        let shipping_address = helpers::get_address_by_id(
            state,
            payment_intent.shipping_address_id.clone(),
            platform.get_processor().get_key_store(),
            &payment_intent.payment_id,
            processor_merchant_id,
            storage_scheme,
        )
        .await?;

        let billing_address = helpers::get_address_by_id(
            state,
            payment_intent.billing_address_id.clone(),
            platform.get_processor().get_key_store(),
            &payment_intent.payment_id,
            processor_merchant_id,
            storage_scheme,
        )
        .await?;

        let payment_method_billing = helpers::get_address_by_id(
            state,
            payment_attempt.payment_method_billing_address_id.clone(),
            platform.get_processor().get_key_store(),
            &payment_intent.payment_id,
            processor_merchant_id,
            storage_scheme,
        )
        .await?;

        let profile_id = payment_intent
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;

        let business_profile = db
            .find_business_profile_by_profile_id(
                platform.get_processor().get_key_store(),
                profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;

        let customer_details = CustomerDetails {
            customer_id: payment_intent.customer_id.clone(),
            ..CustomerDetails::default()
        };

        // Parse the external vault payment method data from the request
        let external_vault_pmd = request
            .payment_method_data
            .payment_method_data
            .clone()
            .map(
                hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::from,
            );

        let payment_data = PaymentData {
            flow: PhantomData,
            payment_intent,
            currency,
            amount,
            mandate_id: None,
            mandate_connector: None,
            setup_mandate: None,
            customer_acceptance: request.customer_acceptance.clone(),
            token: request.payment_token.clone(),
            address: PaymentAddress::new(
                shipping_address.as_ref().map(From::from),
                billing_address.as_ref().map(From::from),
                payment_method_billing.as_ref().map(From::from),
                business_profile.use_billing_as_payment_method_billing,
            ),
            token_data: None,
            confirm: Some(true),
            payment_attempt,
            payment_method_data: None,
            payment_method_token: None,
            payment_method_info: None,
            force_sync: None,
            all_keys_required: None,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
            sessions_token: vec![],
            card_cvc: None,
            creds_identifier: None,
            pm_token: None,
            connector_customer_id: None,
            recurring_mandate_payment_data: None,
            multiple_capture_data: None,
            redirect_response: None,
            surcharge_details: None,
            frm_message: None,
            payment_link_data: None,
            incremental_authorization_details: None,
            authorizations: vec![],
            authentication: None,
            recurring_details: None,
            poll_config: None,
            tax_data: None,
            session_id: None,
            service_details: None,
            card_testing_guard_data: None,
            vault_operation: None,
            threeds_method_comp_ind: None,
            whole_connector_response: None,
            is_manual_retry_enabled: business_profile.is_manual_retry_enabled,
            is_l2_l3_enabled: business_profile.is_l2_l3_enabled,
            external_authentication_data: None,
            external_vault_pmd,
            client_session_id: None,
        };

        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(*self),
            customer_details: Some(customer_details),
            payment_data,
            business_profile,
            mandate_type: None,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Sync>
    UpdateTracker<F, PaymentData<F>, ExternalVaultProxyConfirmRequest>
    for PaymentExternalVaultProxyConfirm
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        _req_state: ReqState,
        processor: &domain::Processor,
        mut payment_data: PaymentData<F>,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        _dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> RouterResult<(
        BoxedOperation<'b, F, ExternalVaultProxyConfirmRequest, PaymentData<F>>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        let storage_scheme = processor.get_account().storage_scheme;
        let key_store = processor.get_key_store();

        let connector = payment_data.payment_attempt.connector.clone();
        let merchant_connector_id = payment_data.payment_attempt.merchant_connector_id.clone();

        let payment_method = payment_data.payment_attempt.payment_method;
        let authentication_type = payment_data.payment_attempt.authentication_type;
        let connector_request_reference_id = payment_data
            .payment_attempt
            .connector_request_reference_id
            .clone();

        let updated_payment_attempt = state
            .store
            .update_payment_attempt_with_attempt_id(
                payment_data.payment_attempt.clone(),
                storage::PaymentAttemptUpdate::ConfirmUpdate {
                    currency: payment_data.currency,
                    status: storage_enums::AttemptStatus::Pending,
                    payment_method,
                    authentication_type,
                    capture_method: payment_data.payment_attempt.capture_method,
                    browser_info: payment_data.payment_attempt.browser_info.clone(),
                    connector,
                    payment_token: payment_data.token.clone(),
                    payment_method_data: None,
                    payment_method_type: payment_data.payment_attempt.payment_method_type,
                    payment_experience: payment_data.payment_attempt.payment_experience,
                    business_sub_label: payment_data.payment_attempt.business_sub_label.clone(),
                    straight_through_algorithm: payment_data
                        .payment_attempt
                        .straight_through_algorithm
                        .clone(),
                    error_code: None,
                    error_message: None,
                    updated_by: storage_scheme.to_string(),
                    merchant_connector_id,
                    external_three_ds_authentication_attempted: None,
                    authentication_connector: None,
                    authentication_id: None,
                    payment_method_billing_address_id: payment_data
                        .payment_attempt
                        .payment_method_billing_address_id
                        .clone(),
                    fingerprint_id: payment_data.payment_attempt.fingerprint_id.clone(),
                    payment_method_id: payment_data.payment_attempt.payment_method_id.clone(),
                    client_source: None,
                    client_version: None,
                    customer_acceptance: payment_data
                        .payment_attempt
                        .customer_acceptance
                        .clone(),
                    net_amount:
                        hyperswitch_domain_models::payments::payment_attempt::NetAmount::new(
                            payment_data.payment_attempt.net_amount.get_order_amount(),
                            payment_data.payment_intent.shipping_cost,
                            payment_data.payment_attempt.net_amount.get_order_tax_amount(),
                            None,
                            None,
                            payment_data
                                .payment_attempt
                                .net_amount
                                .get_installment_interest(),
                        ),
                    connector_mandate_detail: payment_data
                        .payment_attempt
                        .connector_mandate_detail
                        .clone(),
                    card_discovery: None,
                    routing_approach: payment_data.payment_attempt.routing_approach.clone(),
                    connector_request_reference_id,
                    network_transaction_id: payment_data
                        .payment_attempt
                        .network_transaction_id
                        .clone(),
                    is_stored_credential: payment_data.payment_attempt.is_stored_credential,
                    request_extended_authorization: payment_data
                        .payment_attempt
                        .request_extended_authorization,
                    tokenization: payment_data.payment_attempt.get_tokenization_strategy(),
                    installment_data: None,
                },
                storage_scheme,
                key_store,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_data.payment_attempt = updated_payment_attempt;

        Ok((Box::new(*self), payment_data))
    }
}

impl<F: Send + Clone + Sync>
    ValidateRequest<F, ExternalVaultProxyConfirmRequest, PaymentData<F>>
    for PaymentExternalVaultProxyConfirm
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &ExternalVaultProxyConfirmRequest,
        processor: &'a domain::Processor,
    ) -> RouterResult<(
        ExternalVaultProxyOperation<'b, F>,
        operations::ValidateResult,
    )> {
        let payment_id = request
            .payment_id
            .clone()
            .ok_or(errors::ApiErrorResponse::PaymentNotFound)?;

        Ok((
            Box::new(*self),
            operations::ValidateResult {
                merchant_id: processor.get_account().get_id().to_owned(),
                payment_id: api::PaymentIdType::PaymentIntentId(payment_id),
                storage_scheme: processor.get_account().storage_scheme,
                requeue: false,
            },
        ))
    }
}

#[async_trait]
impl<F: Clone + Send + Sync>
    Domain<F, ExternalVaultProxyConfirmRequest, PaymentData<F>>
    for PaymentExternalVaultProxyConfirm
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        provider: &domain::Provider,
        initiator: Option<&domain::Initiator>,
        _dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
        _mandate_type: Option<api::MandateTransactionType>,
    ) -> CustomResult<
        (ExternalVaultProxyOperation<'a, F>, Option<domain::Customer>),
        errors::StorageError,
    > {
        helpers::create_customer_if_not_exist(
            state,
            Box::new(*self),
            payment_data,
            request,
            provider,
            initiator,
            _dimensions,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        payment_data: &mut PaymentData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _platform: &domain::Platform,
        _business_profile: &domain::Profile,
        _should_retry_with_pan: bool,
    ) -> RouterResult<(
        ExternalVaultProxyOperation<'a, F>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        // The payment method data comes from external_vault_pmd, not pm_data
        // Return None here; the transformer will extract from external_vault_pmd
        Ok((Box::new(*self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _processor: &domain::Processor,
        state: &SessionState,
        _request: &ExternalVaultProxyConfirmRequest,
        _payment_intent: &storage::PaymentIntent,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }
}
