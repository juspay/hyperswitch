use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::ExternalVaultProxyConfirmRequest};
use async_trait::async_trait;
use common_enums;
use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::core::payments::operations::PaymentMethodWithRawData;
use crate::{
    core::{
        configs::dimension_state,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods::{self, transformers as pm_transformers},
        payments::{
            helpers, operations,
            CustomerDetails, OperationSessionSetters, PaymentAddress, PaymentData,
        },
        utils as core_utils,
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
        payment_method_wrapper: Option<PaymentMethodWithRawData>,
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

        let mut payment_attempt = db
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

        // If payment_method_wrapper was already fetched (from fetch_payment_method), use it.
        // Otherwise, if payment_token is present and no wrapper yet, fetch from modular service now.
        let payment_method_wrapper = if payment_method_wrapper.is_none() {
            if let Some(payment_token) = &request.payment_token {
                router_env::logger::info!(
                    "Fetching payment method from modular service for external vault proxy (token: {})",
                    payment_token
                );
                match pm_transformers::fetch_payment_method_from_modular_service(
                    state,
                    platform,
                    profile_id,
                    payment_token,
                    None,
                )
                .await
                {
                    Ok(pm_info) => {
                        router_env::logger::info!("Payment method fetched from modular service for external vault proxy");
                        Some(pm_info)
                    }
                    Err(err) => {
                        router_env::logger::error!(
                            error=?err,
                            "Failed to fetch payment method from modular service for external vault proxy"
                        );
                        None
                    }
                }
            } else {
                None
            }
        } else {
            payment_method_wrapper
        };

        // Derive external_vault_pmd:
        // 1. If we have a ProxyCard from the modular service retrieve, use it as ExternalVaultCard.
        // 2. Otherwise, parse from the request's payment_method_data (VaultDataCard flow).
        let external_vault_pmd = if let Some(ref wrapper) = payment_method_wrapper {
            // Check if raw data is a ProxyCard (vault token reference from modular service)
            let from_proxy_card = wrapper.raw_payment_method_data.as_ref().and_then(|raw| {
                match raw {
                    hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardWithOptionalCVC(card) => {
                        // Build ExternalVaultCard from the retrieved card data
                        Some(hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::Card(
                            Box::new(hyperswitch_domain_models::payment_method_data::ExternalVaultCard {
                                card_number: hyperswitch_masking::Secret::new(card.card_number.get_card_no()),
                                card_exp_month: card.card_exp_month.clone(),
                                card_exp_year: card.card_exp_year.clone(),
                                card_cvc: card.card_cvc.clone().unwrap_or_default(),
                                bin_number: None,
                                last_four: None,
                                card_issuer: card.card_issuer.clone(),
                                card_network: card.card_network.clone(),
                                card_type: card.card_type.clone(),
                                card_issuing_country: card.card_issuing_country.clone(),
                                bank_code: card.bank_code.clone(),
                                nick_name: card.nick_name.clone(),
                                card_holder_name: card.card_holder_name.clone(),
                                co_badged_card_data: card.co_badged_card_data.clone(),
                            }),
                        ))
                    }
                    _ => None,
                }
            });
            from_proxy_card.or_else(|| {
                request
                    .payment_method_data
                    .payment_method_data
                    .clone()
                    .map(hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::from)
            })
        } else {
            // No wrapper: parse from request
            request
                .payment_method_data
                .payment_method_data
                .clone()
                .map(hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::from)
        };

        // Set payment_method and payment_method_type on the attempt so the routing
        // engine can match connectors (they may not be set on the attempt yet when
        // payment was created without a payment method).
        payment_attempt.payment_method = Some(request.payment_method_type);
        payment_attempt.payment_method_type = Some(request.payment_method_subtype);

        let payment_method_info = payment_method_wrapper.map(|w| w.payment_method.0);

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
            payment_method_info,
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
            vault_session_details: None,
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

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &ExternalVaultProxyConfirmRequest,
        platform: &domain::Platform,
        _auth_flow: services::AuthFlow,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        _payment_method_wrapper: Option<PaymentMethodWithRawData>,
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

        let mut payment_attempt = db
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

        // Set payment_method and payment_method_type on the attempt so the routing
        // engine can match connectors (they may not be set on the attempt yet when
        // payment was created without a payment method).
        payment_attempt.payment_method = Some(request.payment_method_type);
        payment_attempt.payment_method_type = Some(request.payment_method_subtype);

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
            vault_session_details: None,
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
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn fetch_payment_method(
        &self,
        state: &SessionState,
        req: &ExternalVaultProxyConfirmRequest,
        platform: &domain::Platform,
        feature_config: &core_utils::FeatureConfig,
    ) -> RouterResult<Option<PaymentMethodWithRawData>> {
        if !feature_config.is_payment_method_modular_allowed {
            router_env::logger::info!("Organization is not eligible for PM Modular Service, skipping fetch payment method for external vault proxy.");
            return Ok(None);
        }
        let Some(payment_token) = &req.payment_token else {
            return Ok(None);
        };
        // Try to get profile_id from the platform (best-effort; may not be available yet)
        let profile_id = platform
            .get_processor()
            .get_account()
            .get_default_profile()
            .clone();
        let Some(profile_id) = profile_id else {
            // profile_id will be resolved in get_trackers; fetch will happen there instead
            router_env::logger::info!("profile_id not available before get_trackers; PM fetch deferred to get_trackers for external vault proxy.");
            return Ok(None);
        };
        router_env::logger::info!(
            "Fetching payment method from modular service (token: {}) for external vault proxy.",
            payment_token
        );
        match pm_transformers::fetch_payment_method_from_modular_service(
            state,
            platform,
            &profile_id,
            payment_token,
            None,
        )
        .await
        {
            Ok(pm_info) => Ok(Some(pm_info)),
            Err(err) => {
                router_env::logger::error!(error=?err, "Failed to fetch PM from modular service for external vault proxy (pre get_trackers); will retry in get_trackers.");
                Ok(None)
            }
        }
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn create_payment_method(
        &self,
        state: &SessionState,
        _req: &ExternalVaultProxyConfirmRequest,
        platform: &domain::Platform,
        payment_data: &mut PaymentData<F>,
        business_profile: &domain::Profile,
        feature_config: &core_utils::FeatureConfig,
    ) -> RouterResult<()> {
        if !feature_config.is_payment_method_modular_allowed {
            return Ok(());
        }
        // Only create if customer has given acceptance
        if payment_data.customer_acceptance.is_none() {
            return Ok(());
        }
        // Only create for ExternalVaultCard (proxy card flow)
        let vault_card = match payment_data.external_vault_pmd.clone() {
            Some(
                hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::Card(
                    card,
                ),
            ) => *card,
            _ => return Ok(()),
        };
        let customer_id = payment_data
            .payment_intent
            .customer_id
            .clone()
            .get_required_value("customer_id")?;
        let payment_method = payment_data
            .payment_attempt
            .payment_method
            .unwrap_or(common_enums::PaymentMethod::Card);
        let payment_method_type = payment_data.payment_attempt.payment_method_type;

        match pm_transformers::create_proxy_card_payment_method_in_modular_service(
            state,
            platform.get_provider().get_account().get_id(),
            platform.get_processor().get_account().get_id(),
            business_profile.get_id(),
            payment_method,
            payment_method_type,
            vault_card,
            payment_data.address.get_payment_method_billing().cloned(),
            customer_id,
        )
        .await
        {
            Ok(pm_info) => {
                router_env::logger::info!("Proxy card payment method created in modular service successfully");
                payment_data.set_payment_method_id_in_attempt(Some(pm_info.get_id().clone()));
                payment_data.set_payment_method_info(Some(pm_info));
            }
            Err(err) => {
                router_env::logger::error!(error=?err, "Failed to create proxy card PM in modular service for external vault proxy");
            }
        }
        Ok(())
    }

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
        _payment_data: &mut PaymentData<F>,
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
