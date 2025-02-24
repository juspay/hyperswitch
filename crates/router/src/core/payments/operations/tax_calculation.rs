use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use common_utils::{ext_traits::AsyncExt, types::keymanager::KeyManagerState};
use error_stack::ResultExt;
use masking::PeekInterface;
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payment_methods::cards::create_encrypted_data,
        payments::{self, helpers, operations, PaymentData, PaymentMethodChecker},
        utils as core_utils,
    },
    db::errors::ConnectorErrorExt,
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

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "sdk_session_update")]
pub struct PaymentSessionUpdate;

type PaymentSessionUpdateOperation<'b, F> =
    BoxedOperation<'b, F, api::PaymentsDynamicTaxCalculationRequest, PaymentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync>
    GetTracker<F, PaymentData<F>, api::PaymentsDynamicTaxCalculationRequest>
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
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        _platform_merchant_account: Option<&domain::MerchantAccount>,
    ) -> RouterResult<
        operations::GetTrackerResponse<
            'a,
            F,
            api::PaymentsDynamicTaxCalculationRequest,
            PaymentData<F>,
        >,
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

        // TODO (#7195): Add platform merchant account validation once publishable key auth is solved

        helpers::validate_payment_status_against_not_allowed_statuses(
            payment_intent.status,
            &[
                storage_enums::IntentStatus::Failed,
                storage_enums::IntentStatus::Succeeded,
            ],
            "create a session update for",
        )?;

        helpers::authenticate_client_secret(Some(request.client_secret.peek()), &payment_intent)?;

        let mut payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
                merchant_id,
                payment_intent.active_attempt.get_id().as_str(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let currency = payment_intent.currency.get_required_value("currency")?;

        let amount = payment_attempt.get_total_amount().into();

        payment_attempt.payment_method_type = Some(request.payment_method_type);

        let shipping_address = helpers::get_address_by_id(
            state,
            payment_intent.shipping_address_id.clone(),
            key_store,
            &payment_intent.payment_id,
            merchant_id,
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
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;

        let tax_data = payments::TaxData {
            shipping_details: request.shipping.clone().into(),
            payment_method_type: request.payment_method_type,
        };

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
            tax_data: Some(tax_data),
            session_id: request.session_id.clone(),
            service_details: None,
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
impl<F: Clone + Send + Sync> Domain<F, api::PaymentsDynamicTaxCalculationRequest, PaymentData<F>>
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
            PaymentSessionUpdateOperation<'a, F>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        Ok((Box::new(self), None))
    }

    async fn payments_dynamic_tax_calculation<'a>(
        &'a self,
        state: &SessionState,
        payment_data: &mut PaymentData<F>,
        _connector_call_type: &ConnectorCallType,
        business_profile: &domain::Profile,
        key_store: &domain::MerchantKeyStore,
        merchant_account: &domain::MerchantAccount,
    ) -> errors::CustomResult<(), errors::ApiErrorResponse> {
        let is_tax_connector_enabled = business_profile.get_is_tax_connector_enabled();
        let skip_external_tax_calculation = payment_data
            .payment_intent
            .skip_external_tax_calculation
            .unwrap_or(false);
        if is_tax_connector_enabled && !skip_external_tax_calculation {
            let db = state.store.as_ref();
            let key_manager_state: &KeyManagerState = &state.into();

            let merchant_connector_id = business_profile
                .tax_connector_id
                .as_ref()
                .get_required_value("business_profile.tax_connector_id")?;

            #[cfg(feature = "v1")]
            let mca = db
                .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                    key_manager_state,
                    &business_profile.merchant_id,
                    merchant_connector_id,
                    key_store,
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: merchant_connector_id.get_string_repr().to_string(),
                    },
                )?;

            #[cfg(feature = "v2")]
            let mca = db
                .find_merchant_connector_account_by_id(
                    key_manager_state,
                    merchant_connector_id,
                    key_store,
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: merchant_connector_id.get_string_repr().to_string(),
                    },
                )?;

            let connector_data =
                api::TaxCalculateConnectorData::get_connector_by_name(&mca.connector_name)?;

            let router_data = core_utils::construct_payments_dynamic_tax_calculation_router_data(
                state,
                merchant_account,
                key_store,
                payment_data,
                &mca,
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
            .to_payment_failed_response()
            .attach_printable("Tax connector Response Failed")?;

            let tax_response = response.response.map_err(|err| {
                errors::ApiErrorResponse::ExternalConnectorError {
                    code: err.code,
                    message: err.message,
                    connector: connector_data.connector_name.clone().to_string(),
                    status_code: err.status_code,
                    reason: err.reason,
                }
            })?;

            let payment_method_type = payment_data
                .tax_data
                .clone()
                .map(|tax_data| tax_data.payment_method_type)
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("missing tax_data.payment_method_type")?;

            payment_data.payment_intent.tax_details = Some(diesel_models::TaxDetails {
                payment_method_type: Some(diesel_models::PaymentMethodTypeTax {
                    order_tax_amount: tax_response.order_tax_amount,
                    pmt: payment_method_type,
                }),
                default: None,
            });
            Ok(())
        } else {
            Ok(())
        }
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        _state: &'a SessionState,
        _payment_data: &mut PaymentData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _merchant_key_store: &domain::MerchantKeyStore,
        _customer: &Option<domain::Customer>,
        _business_profile: &domain::Profile,
    ) -> RouterResult<(
        PaymentSessionUpdateOperation<'a, F>,
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
impl<F: Clone + Sync> UpdateTracker<F, PaymentData<F>, api::PaymentsDynamicTaxCalculationRequest>
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
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<(PaymentSessionUpdateOperation<'b, F>, PaymentData<F>)>
    where
        F: 'b + Send,
    {
        // For Google Pay and Apple Pay, we don’t need to call the connector again; we can directly confirm the payment after tax_calculation. So, we update the required fields in the database during the update_tracker call.
        if payment_data.should_update_in_update_tracker() {
            let shipping_address = payment_data
                .tax_data
                .clone()
                .map(|tax_data| tax_data.shipping_details);
            let key_manager_state = state.into();

            let shipping_details = shipping_address
                .clone()
                .async_map(|shipping_details| {
                    create_encrypted_data(&key_manager_state, key_store, shipping_details)
                })
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to encrypt shipping details")?;

            let shipping_address = helpers::create_or_update_address_for_payment_by_request(
                state,
                shipping_address.map(From::from).as_ref(),
                payment_data.payment_intent.shipping_address_id.as_deref(),
                &payment_data.payment_intent.merchant_id,
                payment_data.payment_intent.customer_id.as_ref(),
                key_store,
                &payment_data.payment_intent.payment_id,
                storage_scheme,
            )
            .await?;

            let payment_intent_update = hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::SessionResponseUpdate {
            tax_details: payment_data.payment_intent.tax_details.clone().ok_or(errors::ApiErrorResponse::InternalServerError).attach_printable("payment_intent.tax_details not found")?,
            shipping_address_id: shipping_address.map(|address| address.address_id),
            updated_by: payment_data.payment_intent.updated_by.clone(),
            shipping_details,
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
        } else {
            Ok((Box::new(self), payment_data))
        }
    }
}

impl<F: Send + Clone + Sync>
    ValidateRequest<F, api::PaymentsDynamicTaxCalculationRequest, PaymentData<F>>
    for PaymentSessionUpdate
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsDynamicTaxCalculationRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        PaymentSessionUpdateOperation<'b, F>,
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
