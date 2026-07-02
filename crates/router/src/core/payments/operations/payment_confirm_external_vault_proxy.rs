use std::marker::PhantomData;

use api_models::{enums::FrmSuggestion, payments::PaymentsRequest};
use async_trait::async_trait;
use common_enums;
use error_stack::ResultExt;
use hyperswitch_domain_models::payment_methods::VaultPaymentMethodData;
use hyperswitch_masking::{ExposeInterface, Secret};
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        configs::dimension_state,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods::{transformers as pm_transformers, transformers::PaymentMethodFetchData},
        payments::{
            helpers, operations, CustomerDetails, OperationSessionSetters, PaymentAddress,
            PaymentData,
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

/// Derives the external vault payment method data for the proxy flow from the confirm request
/// and the vault tokens fetched from the modular PM service:
///  - `ProxyCard`: inline vault card data carried directly on the request.
///  - `VaultCardTokenData`: a saved card referenced by `payment_token`; its vault tokens come
///    from `payment_method_wrapper.vault_payment_method_token_data`, combined with the CVC /
///    card holder name supplied on the request.
///
/// Shared by `PaymentExternalVaultProxyConfirm::get_trackers` (two-step confirm) and
/// `PaymentCreate::get_trackers` (single-call create+confirm) so both populate
/// `PaymentData::external_vault_pmd` identically.
pub(crate) fn build_external_vault_payment_method_data(
    request: &PaymentsRequest,
    payment_method_wrapper: Option<
        &hyperswitch_domain_models::payment_methods::PaymentMethodWithRawData,
    >,
) -> RouterResult<
    Option<hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData>,
> {
    let external_vault_pmd = match request
        .payment_method_data
        .as_ref()
        .and_then(|pmd| pmd.payment_method_data.as_ref())
    {
        Some(api_models::payments::PaymentMethodData::ProxyCard(card)) => Some(
            hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::Card(
                Box::new(
                    hyperswitch_domain_models::payment_method_data::ExternalVaultCard::from(
                        (**card).clone(),
                    ),
                ),
            ),
        ),
        Some(api_models::payments::PaymentMethodData::VaultCardTokenData(token_data)) => {
            match payment_method_wrapper
                .and_then(|wrapper| wrapper.vault_payment_method_token_data.as_ref())
            {
                Some(VaultPaymentMethodData::VaultCardData(vault_card)) => {
                    // Card expiry is carried on the vault tokens and the CVC on the request.
                    // Both are required to authorize through the external vault proxy, so error
                    // out explicitly rather than forwarding empty strings that fail downstream
                    // connector validation.
                    let card_exp_month = vault_card
                        .card_exp_month
                        .clone()
                        .get_required_value("card_exp_month")?;
                    let card_exp_year = vault_card
                        .card_exp_year
                        .clone()
                        .get_required_value("card_exp_year")?;
                    let card_cvc = token_data.card_cvc.clone().get_required_value("card_cvc")?;

                    Some(
                        hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::Card(
                            Box::new(hyperswitch_domain_models::payment_method_data::ExternalVaultCard {
                                card_number: vault_card.card_number.clone(),
                                card_exp_month,
                                card_exp_year,
                                card_cvc,
                                bin_number: None,
                                last_four: None,
                                card_issuer: None,
                                card_network: None,
                                card_type: None,
                                card_issuing_country: None,
                                bank_code: None,
                                nick_name: None,
                                card_holder_name: token_data.card_holder_name.clone(),
                                co_badged_card_data: None,
                            }),
                        ),
                    )
                }
                None => None,
            }
        }
        _ => None,
    };
    Ok(external_vault_pmd)
}

impl<F: Send + Clone + Sync> Operation<F, PaymentsRequest> for PaymentExternalVaultProxyConfirm {
    type Data = PaymentData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentsRequest, Self::Data> + Send + Sync)> {
        Ok(self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsRequest> + Send + Sync)> {
        Ok(self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsRequest, Self::Data>> {
        Ok(self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsRequest> + Send + Sync)> {
        Ok(self)
    }
}

impl<F: Send + Clone + Sync> Operation<F, PaymentsRequest> for &PaymentExternalVaultProxyConfirm {
    type Data = PaymentData<F>;
    fn to_validate_request(
        &self,
    ) -> RouterResult<&(dyn ValidateRequest<F, PaymentsRequest, Self::Data> + Send + Sync)> {
        Ok(*self)
    }
    fn to_get_tracker(
        &self,
    ) -> RouterResult<&(dyn GetTracker<F, Self::Data, PaymentsRequest> + Send + Sync)> {
        Ok(*self)
    }
    fn to_domain(&self) -> RouterResult<&dyn Domain<F, PaymentsRequest, Self::Data>> {
        Ok(*self)
    }
    fn to_update_tracker(
        &self,
    ) -> RouterResult<&(dyn UpdateTracker<F, Self::Data, PaymentsRequest> + Send + Sync)> {
        Ok(*self)
    }
}

type ExternalVaultProxyOperation<'b, F> = BoxedOperation<'b, F, PaymentsRequest, PaymentData<F>>;

#[async_trait]
impl<F: Send + Clone + Sync> GetTracker<F, PaymentData<F>, PaymentsRequest>
    for PaymentExternalVaultProxyConfirm
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a SessionState,
        payment_id: &api::PaymentIdType,
        request: &PaymentsRequest,
        platform: &domain::Platform,
        _auth_flow: services::AuthFlow,
        _flow_kind: operations::PaymentFlowKind,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        payment_method_fetch_data: PaymentMethodFetchData,
        _dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
        _payment_pre_fetched_info: Option<operations::PaymentPreFetchedInformation>,
    ) -> RouterResult<operations::GetTrackerResponse<'a, F, PaymentsRequest, PaymentData<F>>> {
        let db = &*state.store;
        let payment_method_wrapper = payment_method_fetch_data.payment_method_with_raw_data;
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

        // The external vault proxy flow is non-PCI. Derive the external vault payment method data:
        //  - `VaultDataCard`: inline vault card data carried directly on the confirm request.
        //  - `VaultCardTokenData`: a saved card referenced by `payment_token`; its vault tokens
        //    were retrieved from the modular PM service in `fetch_payment_method` (available here
        //    as `payment_method_wrapper.vault_payment_method_token_data`). We combine those tokens
        //    with the CVC / card holder name supplied on the request.
        let external_vault_pmd =
            build_external_vault_payment_method_data(request, payment_method_wrapper.as_ref())?;

        // `payment_method` / `payment_method_type` are optional on the confirm request but required
        // for the external vault proxy flow so the routing engine can match connectors. When the
        // request omits them, fall back to the stored payment method fetched from the modular PM
        // service (`payment_method_wrapper`).
        let payment_method = request
            .payment_method
            .or_else(|| {
                payment_method_wrapper
                    .as_ref()
                    .and_then(|wrapper| wrapper.payment_method.payment_method)
            })
            .get_required_value("payment_method")?;
        let payment_method_subtype = request
            .payment_method_type
            .or_else(|| {
                payment_method_wrapper
                    .as_ref()
                    .and_then(|wrapper| wrapper.payment_method.payment_method_type)
            })
            .get_required_value("payment_method_type")?;

        payment_attempt.payment_method = Some(payment_method);
        payment_attempt.payment_method_type = Some(payment_method_subtype);

        let payment_method_info = payment_method_wrapper.map(|w| w.payment_method);

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
}

#[async_trait]
impl<F: Clone + Sync> UpdateTracker<F, PaymentData<F>, PaymentsRequest>
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
        BoxedOperation<'b, F, PaymentsRequest, PaymentData<F>>,
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
                    external_threeds_authentication_type: None,
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
                    customer_acceptance: payment_data.payment_attempt.customer_acceptance.clone(),
                    net_amount:
                        hyperswitch_domain_models::payments::payment_attempt::NetAmount::new(
                            payment_data.payment_attempt.net_amount.get_order_amount(),
                            payment_data.payment_intent.shipping_cost,
                            payment_data
                                .payment_attempt
                                .net_amount
                                .get_order_tax_amount(),
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
                    network_transaction_link_id: None,
                    external_surcharge_details: payment_data
                        .payment_attempt
                        .external_surcharge_details
                        .clone(),
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

impl<F: Send + Clone + Sync> ValidateRequest<F, PaymentsRequest, PaymentData<F>>
    for PaymentExternalVaultProxyConfirm
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &PaymentsRequest,
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
                payment_id,
                storage_scheme: processor.get_account().storage_scheme,
                requeue: false,
            },
        ))
    }
}

#[async_trait]
impl<F: Clone + Send + Sync> Domain<F, PaymentsRequest, PaymentData<F>>
    for PaymentExternalVaultProxyConfirm
{
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn fetch_payment_method(
        &self,
        state: &SessionState,
        req: &PaymentsRequest,
        platform: &domain::Platform,
        feature_config: &core_utils::FeatureConfig,
    ) -> RouterResult<PaymentMethodFetchData> {
        // Only the saved-card token flow (`VaultCardTokenData`) needs an up-front fetch. The inline
        // `VaultDataCard` flow carries the card on the request and is parsed in `get_trackers`.
        let card_token = req
            .payment_method_data
            .as_ref()
            .and_then(|pmd| pmd.payment_method_data.as_ref())
            .and_then(|pmd| match pmd {
                api_models::payments::PaymentMethodData::VaultCardTokenData(token_data) => {
                    Some(domain::CardToken {
                        card_cvc: token_data.card_cvc.clone(),
                        card_holder_name: token_data.card_holder_name.clone(),
                        card_cvc_token: None,
                    })
                }
                _ => None,
            });

        // A fetch is only needed for the saved-card token flow, and only when the org is eligible
        // for the modular PM service. Every other case has nothing to fetch up front.
        let fetch_data = match card_token {
            Some(card_token) if feature_config.is_payment_method_modular_allowed => {
                // 1. Resolve the payment_token to the actual stored payment_method_id via Redis.
                //    This uses the same key construction as the regular Confirm flow's
                //    fetch_payment_method (`pm_token_{token}_{payment_method}_hyperswitch`).
                let payment_token = req
                    .payment_token
                    .clone()
                    .get_required_value("payment_token")
                    .attach_printable("payment_token is required for the vault card token flow")?;

                let token_data =
                    helpers::retrieve_payment_token_data(state, payment_token, req.payment_method)
                        .await?;

                let payment_method_id = match token_data {
                    storage::PaymentTokenData::Permanent(card_token_data)
                    | storage::PaymentTokenData::PermanentCard(card_token_data) => {
                        card_token_data.payment_method_id
                    }
                    _ => None,
                }
                .get_required_value("payment_method_id")
                .attach_printable("could not resolve a payment_method_id from the payment_token")?;

                // 2. Retrieve the external vault tokens for that payment method from the modular PM
                //    service.
                let profile_id = platform
                    .get_processor()
                    .get_account()
                    .get_default_profile()
                    .clone()
                    .get_required_value("profile_id")
                    .attach_printable(
                        "profile_id is required to fetch external vault tokens from the modular service",
                    )?;

                let payment_method_with_raw_data =
                    pm_transformers::fetch_payment_method_from_modular_service(
                        state,
                        platform,
                        &profile_id,
                        &payment_method_id,
                        Some(card_token),
                        // External vault proxy cards are not in the internal vault; requesting raw
                        // detail fails. The external vault token reference is returned without it.
                        false,
                    )
                    .await
                    .attach_printable(
                        "Failed to fetch external vault token data from the modular PM service",
                    )?;

                PaymentMethodFetchData::from_modular(payment_method_with_raw_data)
            }
            Some(_) => {
                router_env::logger::info!(
                    "Organization is not eligible for PM Modular Service; skipping external vault token fetch."
                );
                PaymentMethodFetchData::default()
            }
            // Not a token flow — nothing to fetch up front.
            None => PaymentMethodFetchData::default(),
        };

        Ok(fetch_data)
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn create_payment_method(
        &self,
        state: &SessionState,
        _req: &PaymentsRequest,
        platform: &domain::Platform,
        payment_data: &mut PaymentData<F>,
        customer: Option<&domain::Customer>,
        business_profile: &domain::Profile,
        _feature_config: &core_utils::FeatureConfig,
    ) -> RouterResult<()> {
        // Only create if customer has given acceptance
        if payment_data.customer_acceptance.is_none() {
            router_env::logger::info!("Skipping PM creation: customer_acceptance is None");
            return Ok(());
        }
        // A repeat payment that reuses a saved card (the `VaultCardTokenData` / `payment_token`
        // flow) already had its existing payment method resolved into `payment_method_info` during
        // `get_trackers`. Creating again here would insert a duplicate `payment_methods` record for
        // the same card on every off-session repeat, so reuse the existing one — just record its id
        // on the attempt. The PM was fetched from the modular service (already `version: V2`), so the
        // post-payment update still takes the modular acknowledgement path, not the legacy save path.
        if let Some(existing_pm_id) = payment_data
            .payment_method_info
            .as_ref()
            .map(|existing_pm| existing_pm.get_id().clone())
        {
            router_env::logger::info!(
                "Reusing existing payment method resolved from payment_token; skipping duplicate PM creation"
            );
            payment_data.set_payment_method_id_in_attempt(Some(existing_pm_id));
            return Ok(());
        }
        // Only create for ExternalVaultCard (proxy card flow)
        let mut vault_card = match payment_data.external_vault_pmd.clone() {
            Some(
                hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::Card(
                    card,
                ),
            ) => *card,
            Some(other) => {
                router_env::logger::info!(?other, "Skipping PM creation: external_vault_pmd is not a Card variant");
                return Ok(());
            }
            None => {
                router_env::logger::info!("Skipping PM creation: external_vault_pmd is None");
                return Ok(());
            }
        };
        let global_customer_id = customer
            .ok_or(errors::ApiErrorResponse::CustomerNotFound)?
            .get_global_id()
            .cloned()
            .get_required_value("id")?;
        let payment_method = payment_data
            .payment_attempt
            .payment_method
            .unwrap_or(common_enums::PaymentMethod::Card);
        let payment_method_type = payment_data.payment_attempt.payment_method_type;

        // When the external vault is the hyperswitch vault, the `card_number` carried on the request
        // is a *temporary* vault token. Exchange it for the permanent payment method id (served by
        // the external SaaS PM service) and persist that as the token in the PM entry. The external
        // service is a separate deployment, so authenticate with the external vault connector
        // account's credentials (api-key + profile id) fetched from the profile's vault MCA. For any
        // other vault, the request token is stored as-is.
        if business_profile
            .external_vault_details
            .is_hyperswitch_vault()
        {
            let vault_connector_id = business_profile
                .external_vault_details
                .get_vault_connector_id()
                .get_required_value("external vault connector id")?;

            let vault_mca = state
                .store
                .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                    platform.get_processor().get_account().get_id(),
                    &vault_connector_id,
                    platform.get_processor().get_key_store(),
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: vault_connector_id.get_string_repr().to_string(),
                    },
                )?;

            let (api_key, vault_profile_id) = match vault_mca
                .get_connector_account_details()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse external vault connector auth")?
            {
                hyperswitch_domain_models::router_data::ConnectorAuthType::SignatureKey {
                    api_key,
                    api_secret,
                    ..
                } => Ok((api_key, api_secret)),
                _ => Err(error_stack::report!(
                    errors::ApiErrorResponse::InternalServerError
                )
                .attach_printable(
                    "Unexpected auth type for hyperswitch external vault connector; expected SignatureKey",
                )),
            }?;

            let temporary_token = vault_card.card_number.clone().expose();
            let permanent_pm_id = pm_transformers::get_permanent_pm_id_from_temporary_token(
                state,
                api_key,
                vault_profile_id,
                temporary_token,
            )
            .await?;
            vault_card.card_number = Secret::new(permanent_pm_id);
        }

        match pm_transformers::create_proxy_card_payment_method_in_modular_service(
            state,
            platform.get_provider().get_account().get_id(),
            platform.get_processor().get_account().get_id(),
            business_profile.get_id(),
            payment_method,
            payment_method_type,
            vault_card,
            payment_data.address.get_payment_method_billing().cloned(),
            global_customer_id,
        )
        .await
        {
            Ok(mut pm_info) => {
                router_env::logger::info!(
                    "Proxy card payment method created in modular service successfully"
                );
                // The payment method is created in the modular (V2) payment method service.
                // Mark it as such so the post-payment acknowledgement path
                // (`handle_pm_and_mandate_post_update` -> `should_use_modular_pm_path`) is taken
                // and the modular PM status is acknowledged (new -> active). Without this the PM
                // defaults to `version: V1`, the gate falls back to the legacy `save_pm_and_mandate`
                // path, and the acknowledgement call is never made.
                pm_info.version = common_enums::ApiVersion::V2;
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
        _request: &PaymentsRequest,
        _payment_intent: &storage::PaymentIntent,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        helpers::get_connector_default(state, None).await
    }
}
