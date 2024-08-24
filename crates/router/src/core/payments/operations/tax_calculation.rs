use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::types::keymanager::KeyManagerState;
use error_stack::ResultExt;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

// use crate::core::payments::Operation;
use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::{self, helpers, operations, PaymentData},
        utils as core_utils,
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{
        self,
        api::{self, ConnectorCallType, PaymentIdTypeExt},
        domain,
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt,
};
// use api_models::payments::PaymentsDynamicTaxCalculationRequest;
// use crate::types::api::PaymentsDynamicTaxCalculationRequest;

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "tax_calculation")]
pub struct PaymentSessionUpdate;

#[async_trait]
impl<F: Send + Clone> GetTracker<F, PaymentData<F>, api::PaymentsDynamicTaxCalculationRequest>
    for PaymentSessionUpdate
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsDynamicTaxCalculationRequest,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
        _header_payload: &api::HeaderPayload,
    ) -> RouterResult<
        operations::GetTrackerResponse<'a, F, api::PaymentsDynamicTaxCalculationRequest>,
    > {
        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        let db = &*state.store;
        let key_manager_state: &KeyManagerState = &state.into();
        let merchant_id = merchant_account.get_id();
        let storage_scheme = merchant_account.storage_scheme;

        let payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(
                &state.into(),
                &payment_id,
                merchant_id,
                key_store,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        helpers::validate_payment_status_against_not_allowed_statuses(
            &payment_intent.status,
            &[
                storage_enums::IntentStatus::Failed,
                storage_enums::IntentStatus::Succeeded,
            ],
            "create a session update for",
        )?;

        helpers::authenticate_client_secret(Some(&request.client_secret), &payment_intent)?;

        let payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                payment_intent.payment_id.as_str(),
                merchant_id,
                payment_intent.active_attempt.get_id().as_str(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let currency = payment_intent.currency.get_required_value("currency")?;

        let amount = payment_attempt.get_total_amount().into();

        let shipping_address = helpers::create_or_update_address_for_payment_by_request(
            state,
            Some(&request.shipping),
            payment_intent.shipping_address_id.as_deref(),
            merchant_id,
            payment_intent.customer_id.as_ref(),
            key_store,
            &payment_intent.payment_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let profile_id = payment_intent
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;

        let business_profile = db
            .find_business_profile_by_profile_id(key_manager_state, key_store, profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                id: profile_id.to_string(),
            })?;

        let payment_data = PaymentData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            currency,
            amount,
            email: None,
            mandate_id: None,
            mandate_connector: None,
            customer_acceptance: None,
            token: None,
            token_data: None,
            setup_mandate: None,
            address: payments::PaymentAddress::new(
                shipping_address.as_ref().map(From::from),
                None,
                None,
                business_profile.use_billing_as_payment_method_billing,
            ),
            confirm: None,
            payment_method_data: None,
            payment_method_info: None,
            force_sync: None,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
            sessions_token: vec![],
            card_cvc: None,
            creds_identifier: None,
            pm_token: None,
            connector_customer_id: None,
            recurring_mandate_payment_data: None,
            ephemeral_key: None,
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
        };
        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(self),
            customer_details: None,
            payment_data,
            business_profile,
            mandate_type: None,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send> Domain<F, api::PaymentsDynamicTaxCalculationRequest>
    for PaymentSessionUpdate
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        _state: &SessionState,
        _payment_data: &mut PaymentData<F>,
        _request: Option<payments::CustomerDetails>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> errors::CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsDynamicTaxCalculationRequest>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((Box::new(self), None))
    }

    async fn payments_dynamic_tax_calculation<'a>(
        &'a self,
        state: &SessionState,
        // key_manager_state: &common_utils::types::keymanager::KeyManagerState,
        payment_data: &mut PaymentData<F>,
        _should_continue_confirm_transaction: &mut bool,
        _connector_call_type: &ConnectorCallType,
        // _merchant_account: &storage::BusinessProfile,
        key_store: &domain::MerchantKeyStore,
        // storage_scheme: storage_enums::MerchantStorageScheme,
        merchant_account: &domain::MerchantAccount,
    ) -> errors::CustomResult<(), errors::ApiErrorResponse> {
        // let db = state.store.as_ref();
        let payment_intent = payment_data.payment_intent.clone();

        // let attempt_id = payment_intent.active_attempt.get_id().clone();

        let payment_attempt = payment_data.payment_attempt.clone();

        // Derive this connector from business profile
        let connector_data = api::TaxCalculateConnectorData::get_connector_by_name(
            payment_attempt.connector.as_ref().unwrap(),
        )?;

        let router_data = core_utils::construct_payments_dynamic_tax_calculation_router_data(
            state,
            &payment_intent,
            &payment_attempt,
            merchant_account,
            key_store,
            // &customer,
        )
        .await?;
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::CalculateTax,
            types::PaymentsTaxCalculationData,
            types::TaxCalculationResponseData,
        > = connector_data.connector.get_connector_integration();

        let response = services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let tax_response =
            response
                .response
                .map_err(|err| errors::ApiErrorResponse::ExternalConnectorError {
                    code: err.code,
                    message: err.message,
                    connector: connector_data.connector_name.clone().to_string(),
                    status_code: err.status_code,
                    reason: err.reason,
                })?;

        // Update payment_data.tax_details with new amount which was returned by the connector
        // When constructing the router data, add this to the net amount
        //payment_data

        // match tax_response {
        // hyperswitch_domain_models::router_response_types::PaymentsResponseData::TaxCalculationResponse { order_tax_amount, .. } => {
        //     // Update payment_data.payment_intent.tax_details.order_tax_amount with the order_tax_amount from the TaxCalculationResponse

        payment_data
            .payment_intent
            .tax_details
            .clone()
            .map(|tax_details| {
                tax_details.pmt.map(|mut pmt| {
                    pmt.order_tax_amount = tax_response.order_tax_amount;
                });
            });
        // }
        // _ => {
        //     Err(errors::ApiErrorResponse::InternalServerError)?
        // }
        // }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut PaymentData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: Option<&domain::BusinessProfile>,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsDynamicTaxCalculationRequest>,
        Option<domain::PaymentMethodData>,
        Option<String>,
    )> {
        Ok((Box::new(self), None, None))
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &SessionState,
        _request: &api::PaymentsDynamicTaxCalculationRequest,
        _payment_intent: &storage::PaymentIntent,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> errors::CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }

    #[instrument(skip_all)]
    async fn guard_payment_against_blocklist<'a>(
        &'a self,
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        _payment_data: &mut PaymentData<F>,
    ) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
        Ok(false)
    }
}

#[async_trait]
impl<F: Clone> UpdateTracker<F, PaymentData<F>, api::PaymentsDynamicTaxCalculationRequest>
    for PaymentSessionUpdate
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        state: &'b SessionState,
        _req_state: ReqState,
        mut payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsDynamicTaxCalculationRequest>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        //update shipping and tax_details
        let payment_intent_update = hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::CompleteAuthorizeUpdate {
            shipping_address_id: payment_data.payment_intent.shipping_address_id.clone()
        };

        let db = &*state.store;
        let payment_intent = payment_data.payment_intent.clone();

        let updated_payment_intent = db
            .update_payment_intent(
                &state.into(),
                payment_intent,
                payment_intent_update,
                key_store,
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_data.payment_intent = updated_payment_intent;
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone> ValidateRequest<F, api::PaymentsDynamicTaxCalculationRequest>
    for PaymentSessionUpdate
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsDynamicTaxCalculationRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsDynamicTaxCalculationRequest>,
        operations::ValidateResult,
    )> {
        //paymentid is already generated and should be sent in the request
        let given_payment_id = request.payment_id.clone();

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: merchant_account.get_id().to_owned(),
                payment_id: api::PaymentIdType::PaymentIntentId(given_payment_id),
                storage_scheme: merchant_account.storage_scheme,
                requeue: false,
            },
        ))
    }
}
