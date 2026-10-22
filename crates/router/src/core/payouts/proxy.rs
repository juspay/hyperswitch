use api_models::enums::VaultConnectors;
use common_enums::{
    ExecutionMode, ExecutionPath, PaymentMethodStatus, PayoutExecutionKind, PayoutStatus,
    PayoutType,
};
use common_utils::id_type;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_methods::{PaymentMethod, PaymentMethodVaultSourceDetails, VaultPaymentMethodData},
    payments::HeaderPayload,
    payouts::proxy::{ExternalVaultPayoutCardData, ExternalVaultPayoutMethodData},
};
use hyperswitch_masking::PeekInterface;

use super::{
    gateway::context::RouterGatewayContext,
    validator::{self, invalid_payout_request, validate_payout_condition},
    PayoutData,
};
use crate::{
    core::{
        configs::dimension_state,
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
        payment_methods::transformers::fetch_payment_method_from_modular_service,
        payments::helpers,
        unified_connector_service, utils as core_utils,
    },
    routes::SessionState,
    types::{
        self,
        api::{self, payouts},
        domain,
    },
    utils::OptionExt,
};

/// Provider-profile vault configuration, separate from payout connector credentials and tokens.
#[derive(Clone, Debug, Default)]
pub enum PayoutExecutionContext {
    #[default]
    Normal,
    ExternalVaultProxy {
        external_vault_mca: helpers::MerchantConnectorAccountType,
    },
}

impl PayoutExecutionContext {
    /// Encoded vault credentials must not be logged or persisted.
    pub(super) fn external_vault_proxy_metadata(
        &self,
        state: &SessionState,
    ) -> RouterResult<Option<String>> {
        match self {
            Self::Normal => Ok(None),
            Self::ExternalVaultProxy { external_vault_mca } => {
                unified_connector_service::build_unified_connector_service_external_vault_proxy_metadata_v1(
                    external_vault_mca.clone(),
                    &state.conf.connectors,
                    &state.conf.proxy,
                )
                .map(Some)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to construct external vault payout proxy metadata")
            }
        }
    }
}

pub(in crate::core) struct ExternalVaultPayout<'a> {
    pub state: &'a SessionState,
    pub platform: &'a domain::Platform,
}

struct ProxyCreateInput {
    request: payouts::PayoutCreateRequest,
    payout_id: id_type::PayoutId,
    profile: domain::Profile,
    customer: domain::Customer,
    payment_method: PaymentMethod,
    execution_context: PayoutExecutionContext,
}

impl PayoutData {
    fn set_proxy_execution(
        &mut self,
        external_vault_pmd: ExternalVaultPayoutMethodData,
        execution_context: PayoutExecutionContext,
    ) {
        self.external_vault_pmd = Some(external_vault_pmd);
        self.execution_context = execution_context;
    }

    fn set_proxy_connector(
        &mut self,
        merchant_connector_account: helpers::MerchantConnectorAccountType,
        connector_data: &api::ConnectorData,
    ) -> api::ConnectorData {
        self.payout_attempt.merchant_connector_id = merchant_connector_account.get_mca_id();
        self.merchant_connector_account = Some(merchant_connector_account);
        let mut selected_connector = connector_data.clone();
        selected_connector.merchant_connector_id =
            self.payout_attempt.merchant_connector_id.clone();
        selected_connector
    }

    fn set_proxy_transfer_method_id(&mut self, transfer_method_id: Option<String>) {
        self.connector_transfer_method_id = transfer_method_id;
    }
}

impl ExternalVaultPayout<'_> {
    /// Proxy payouts force UCS Primary without rollout or Direct fallback.
    pub(super) async fn proxy_gateway_context(
        &self,
        header_payload: HeaderPayload,
        connector_data: &api::ConnectorData,
        payout_data: &mut PayoutData,
    ) -> RouterResult<RouterGatewayContext> {
        let state = self.state;
        let platform = self.platform;
        let merchant_connector_account = match payout_data.merchant_connector_account.clone() {
            Some(mca) => mca,
            None => {
                super::get_mca_from_profile_id(
                    state,
                    platform,
                    &payout_data.profile_id,
                    &connector_data.connector_name.to_string(),
                    payout_data
                        .payout_attempt
                        .merchant_connector_id
                        .as_ref()
                        .or(connector_data.merchant_connector_id.as_ref()),
                )
                .await?
            }
        };
        validate_payout_condition(
            merchant_connector_account.is_disabled(),
            "Payout connector account is disabled",
        )?;
        let payout_mca = match &merchant_connector_account {
            helpers::MerchantConnectorAccountType::DbVal(payout_mca) => Ok(payout_mca),
            helpers::MerchantConnectorAccountType::CacheVal(_) => Err(invalid_payout_request(
                "Proxy payouts require a saved payout connector account",
            )),
        }?;
        validate_payout_condition(
            payout_mca.merchant_id != *platform.get_processor().get_account().get_id()
                || payout_data.payouts.merchant_id != payout_mca.merchant_id
                || payout_mca.profile_id != payout_data.profile_id
                || payout_mca.connector_name != connector_data.connector_name.to_string()
                || payout_mca.connector_type != common_enums::ConnectorType::PayoutProcessor
                || connector_data
                    .merchant_connector_id
                    .as_ref()
                    .is_some_and(|selected_id| selected_id != &payout_mca.get_id()),
            "Payout connector account does not match proxy payout routing",
        )?;
        Ok(RouterGatewayContext {
            creds_identifier: None,
            processor: platform.get_processor().clone(),
            header_payload,
            lineage_ids: external_services::grpc_client::LineageIds::new(
                payout_data.payouts.merchant_id.clone(),
                payout_data.profile_id.clone(),
            ),
            merchant_connector_account,
            execution_path: ExecutionPath::UnifiedConnectorService,
            execution_mode: ExecutionMode::Primary,
            kill_switch_enabled: false,
            kill_switch_threshold: 1,
            connector_decline_threshold: None,
            rollout_scope: None,
            payout_execution_context: payout_data.execution_context.clone(),
        })
    }

    pub(in crate::core) async fn construct_proxy_payout_router_data<F>(
        &self,
        connector_data: &api::ConnectorData,
        payout_data: &mut PayoutData,
    ) -> RouterResult<types::PayoutsRouterData<F>> {
        let state = self.state;
        let platform = self.platform;
        // Validate proxy invariants before sharing the router-data constructor.
        self.validate_proxy_runtime(payout_data)?;
        let context = self
            .proxy_gateway_context(HeaderPayload::default(), connector_data, payout_data)
            .await?;
        let connector_data =
            payout_data.set_proxy_connector(context.merchant_connector_account, connector_data);
        core_utils::construct_payout_router_data_common(
            state,
            &connector_data,
            platform,
            payout_data,
        )
        .await
    }

    /// The transport gate precedes token retrieval and all payout writes.
    pub(super) async fn payouts_proxy_core(
        &self,
        header_payload: HeaderPayload,
        req: payouts::PayoutCreateRequest,
        profile: domain::Profile,
        provider_profile: domain::Profile,
    ) -> RouterResponse<payouts::PayoutCreateResponse> {
        let state = self.state;
        let platform = self.platform;
        let input =
            Box::pin(self.validate_proxy_create_request(req, profile, provider_profile)).await?;
        self.ensure_proxy_transport_available()?;
        let dimensions = dimension_state::Dimensions::new()
            .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id())
            .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id())
            .with_profile_id(input.profile.get_id().clone());
        let external_vault_pmd = self
            .fetch_external_vault_payout_method(input.profile.get_id(), &input.payment_method)
            .await?;
        let mut payout_data = Box::pin(super::payout_create_db_entries(
            state,
            platform,
            &input.request,
            &input.payout_id,
            input.profile,
            None,
            &state.locale,
            Some(&input.customer),
            Some(input.payment_method),
            &dimensions,
            PayoutExecutionKind::ExternalVaultProxy,
        ))
        .await?;
        payout_data.set_proxy_execution(external_vault_pmd, input.execution_context);

        let connector_call_type = super::get_connector_choice(
            state,
            platform.get_processor(),
            &dimensions,
            None,
            input.request.routing,
            &mut payout_data,
            input.request.connector,
        )
        .await?;
        let connector_data = match connector_call_type {
            api::ConnectorCallType::PreDetermined(routing) => Ok(routing.connector_data),
            api::ConnectorCallType::Retryable(routing) => {
                // Proxy retries are unsupported; execute only the first routed connector.
                super::get_next_connector(&mut routing.into_iter())
                    .map(|routing| routing.connector_data)
            }
            _ => Err(invalid_payout_request(
                "Unsupported proxy payout connector routing",
            )),
        }?;
        self.execute_proxy_payout(
            header_payload,
            &connector_data,
            &mut payout_data,
            &dimensions.without_profile_id(),
        )
        .await?;
        super::trigger_webhook_and_handle_response(state, platform, &payout_data).await
    }

    /// Public resume must reload vault tokens; only in-request auto-fulfill is supported.
    async fn execute_proxy_fulfill(
        &self,
        header_payload: HeaderPayload,
        connector_data: &api::ConnectorData,
        payout_data: &mut PayoutData,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> RouterResult<()> {
        let state = self.state;
        let platform = self.platform;
        Box::pin(super::fulfill_payout(
            state,
            platform,
            header_payload,
            connector_data,
            payout_data,
            dimensions,
        ))
        .await
    }

    fn ensure_proxy_transport_available(&self) -> RouterResult<()> {
        let state = self.state;
        match state.grpc_client.unified_connector_service_client.as_ref() {
            None => Err(report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unified Connector Service is unavailable for proxy payouts")),
            Some(_) => Ok(()),
        }
    }

    fn validate_proxy_runtime(&self, payout_data: &PayoutData) -> RouterResult<()> {
        validate_payout_condition(
            payout_data.payout_attempt.execution_kind != PayoutExecutionKind::ExternalVaultProxy
                || !matches!(
                    payout_data.execution_context,
                    PayoutExecutionContext::ExternalVaultProxy { .. }
                )
                || payout_data.external_vault_pmd.is_none()
                || payout_data.payout_method_data.is_some()
                || payout_data.payout_attempt.payout_token.is_some()
                || payout_data.payout_attempt.source_bank_data_token.is_some()
                || payout_data.source_bank_data.is_some()
                || payout_data.payouts.confirm != Some(true)
                || !payout_data.payouts.auto_fulfill
                || payout_data.payouts.recurring
                || payout_data.payouts.payout_type != Some(PayoutType::Card),
            "Invalid external vault payout execution context",
        )?;
        let payment_method = payout_data
            .payment_method
            .as_ref()
            .get_required_value("saved proxy payment method")?;
        let external_vault_mca = match &payout_data.execution_context {
            PayoutExecutionContext::ExternalVaultProxy { external_vault_mca } => {
                Ok(external_vault_mca)
            }
            PayoutExecutionContext::Normal => Err(invalid_payout_request(
                "Missing external vault execution context",
            )),
        }?;
        let saved_source_matches = matches!(
            &payment_method.vault_source_details,
            PaymentMethodVaultSourceDetails::ExternalVault { external_vault_source }
                if external_vault_mca.get_mca_id().as_ref() == Some(external_vault_source)
        );
        validate_payout_condition(
            payment_method.merchant_id != payout_data.payouts.merchant_id
                || payment_method.customer_id != payout_data.payouts.customer_id
                || payout_data.payouts.payout_method_id.as_ref()
                    != Some(&payment_method.payment_method_id)
                || payout_data.payout_attempt.merchant_id != payout_data.payouts.merchant_id
                || payout_data.payout_attempt.customer_id != payout_data.payouts.customer_id
                || payout_data.payout_attempt.payout_id != payout_data.payouts.payout_id
                || payout_data.payout_attempt.profile_id != payout_data.profile_id
                || payout_data.business_profile.get_id() != &payout_data.profile_id
                || payment_method.status != PaymentMethodStatus::Active
                || payment_method.payment_method != Some(common_enums::PaymentMethod::Card)
                || !saved_source_matches,
            "Inconsistent saved proxy payment method context",
        )
    }

    /// Default omitted S2S flags but reject conflicting values.
    fn normalize_proxy_create_request(
        &self,
        mut req: payouts::PayoutCreateRequest,
    ) -> RouterResult<payouts::PayoutCreateRequest> {
        req.payout_id.as_ref().get_required_value("payout_id")?;
        req.amount.as_ref().get_required_value("amount")?;
        req.currency.as_ref().get_required_value("currency")?;
        let payout_method_id = req
            .payout_method_id
            .as_ref()
            .get_required_value("payout_method_id")?;
        validate_payout_condition(
            payout_method_id.trim().is_empty(),
            "payout_method_id must not be empty",
        )?;
        req.get_customer_id().get_required_value("customer_id")?;
        validate_payout_condition(
            req.customer_id
                .as_ref()
                .zip(
                    req.customer
                        .as_ref()
                        .and_then(|customer| customer.id.as_ref()),
                )
                .is_some_and(|(legacy_id, customer_id)| legacy_id != customer_id),
            "Conflicting customer identifiers",
        )?;

        validate_payout_condition(
        req.confirm == Some(false)
        || req.auto_fulfill == Some(false)
        || req.recurring == Some(true)
        || req
            .payout_type
            .is_some_and(|payout_type| payout_type != PayoutType::Card),
        "External vault proxy payouts require confirm=true, auto_fulfill=true, recurring=false and payout_type=card",
    )?;
        validate_payout_condition(
        req.payout_method_data.is_some()
        || req.payout_token.is_some()
        || req.client_secret.is_some()
        || req.payout_link == Some(true)
        || req.payout_link_config.is_some()
        || req.session_expiry.is_some()
        || req.source_bank_data.is_some(),
        "External vault proxy payouts do not accept inline method data, payout_token, client_secret, payout-link options or source_bank_data",
    )?;
        req.confirm = Some(true);
        req.auto_fulfill = Some(true);
        req.recurring = Some(false);
        req.payout_type = Some(PayoutType::Card);
        Ok(req)
    }

    fn validate_proxy_payment_method<'a>(
        &self,
        payment_method: &'a PaymentMethod,
        customer_id: &id_type::CustomerId,
    ) -> RouterResult<&'a id_type::MerchantConnectorAccountId> {
        let platform = self.platform;
        validate_payout_condition(
            payment_method.merchant_id != *platform.get_processor().get_account().get_id()
                || payment_method.customer_id.as_ref() != Some(customer_id),
            "Payment method does not belong to this merchant and customer",
        )?;
        validate_payout_condition(
            payment_method.status != PaymentMethodStatus::Active
                || payment_method.payment_method != Some(common_enums::PaymentMethod::Card),
            "External vault proxy payouts require an active saved card payment method",
        )?;
        match &payment_method.vault_source_details {
            PaymentMethodVaultSourceDetails::ExternalVault {
                external_vault_source,
            } => Ok(external_vault_source),
            PaymentMethodVaultSourceDetails::InternalVault => Err(invalid_payout_request(
                "Payment method is not stored in an external vault",
            )),
        }
    }

    /// Vault configuration belongs to the provider, not the payout processor.
    async fn resolve_external_vault_execution_context(
        &self,
        provider_profile: &domain::Profile,
        saved_vault_source: &id_type::MerchantConnectorAccountId,
    ) -> RouterResult<PayoutExecutionContext> {
        let state = self.state;
        let platform = self.platform;
        let vault_connector_id = provider_profile
            .external_vault_details
            .get_vault_connector_id()
            .get_required_value("provider profile external vault connector id")?;
        validate_payout_condition(
            &vault_connector_id != saved_vault_source,
            "Saved payment method vault does not match the provider profile external vault",
        )?;
        let provider = platform.get_provider();
        let external_vault_mca = state
            .store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                provider.get_account().get_id(),
                &vault_connector_id,
                provider.get_key_store(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: vault_connector_id.get_string_repr().to_owned(),
            })?;
        validate_payout_condition(
        external_vault_mca.merchant_id != *provider.get_account().get_id()
            || external_vault_mca.profile_id != *provider_profile.get_id()
            || external_vault_mca.disabled == Some(true)
            || !matches!(
                external_vault_mca.connector_type,
                common_enums::ConnectorType::VaultProcessor
            ),
        "External vault connector account is disabled or incompatible with the provider profile",
    )?;
        match VaultConnectors::try_from(external_vault_mca.connector_name.clone()) {
            Ok(VaultConnectors::HyperswitchVault) => {
                Ok(PayoutExecutionContext::ExternalVaultProxy {
                    external_vault_mca: helpers::MerchantConnectorAccountType::DbVal(Box::new(
                        external_vault_mca,
                    )),
                })
            }
            _ => Err(invalid_payout_request(
                "Unsupported external vault payout proxy connector",
            )),
        }
    }

    /// Fetch opaque tokens without raw-card retrieval or Account Updater sync.
    async fn fetch_external_vault_payout_method(
        &self,
        profile_id: &id_type::ProfileId,
        saved_payment_method: &PaymentMethod,
    ) -> RouterResult<ExternalVaultPayoutMethodData> {
        let state = self.state;
        let platform = self.platform;
        let customer_id = saved_payment_method
            .customer_id
            .as_ref()
            .get_required_value("customer_id")?;
        let fetched = fetch_payment_method_from_modular_service(
            state,
            platform,
            profile_id,
            &saved_payment_method.payment_method_id,
            None,
            false,
            false,
        )
        .await?;
        // Use saved vault/status: the modular transformer defaults both on its reconstructed row.
        validate_payout_condition(
            fetched.payment_method.payment_method_id != saved_payment_method.payment_method_id
                || fetched.payment_method.merchant_id != saved_payment_method.merchant_id
                || fetched.payment_method.customer_id.as_ref() != Some(customer_id)
                || fetched.payment_method.payment_method != Some(common_enums::PaymentMethod::Card)
                || fetched.raw_payment_method_data.is_some(),
            "PM modular returned incompatible proxy payment method data",
        )?;
        let VaultPaymentMethodData::VaultCardData(card) =
            fetched
                .vault_payment_method_token_data
                .get_required_value("external vault proxy card data")?;
        validate_payout_condition(
            card.card_number.peek().is_empty(),
            "External vault card token must not be empty",
        )?;
        let expiry_month = card
            .card_exp_month
            .get_required_value("external vault card expiry month")?;
        let expiry_year = card
            .card_exp_year
            .get_required_value("external vault card expiry year")?;
        validate_payout_condition(
            expiry_month.peek().is_empty() || expiry_year.peek().is_empty(),
            "External vault card expiry must not be empty",
        )?;
        let trusted_card = saved_payment_method
            .get_payment_methods_data()
            .and_then(|pmd| pmd.get_card_details());
        Ok(ExternalVaultPayoutMethodData::Card(Box::new(
            ExternalVaultPayoutCardData {
                card_number: card.card_number,
                expiry_month,
                expiry_year,
                card_holder_name: trusted_card
                    .as_ref()
                    .and_then(|card| card.card_holder_name.clone()),
                card_network: trusted_card.and_then(|card| card.card_network),
            },
        )))
    }

    async fn validate_proxy_create_request(
        &self,
        req: payouts::PayoutCreateRequest,
        profile: domain::Profile,
        provider_profile: domain::Profile,
    ) -> RouterResult<ProxyCreateInput> {
        let state = self.state;
        let platform = self.platform;
        let req = self.normalize_proxy_create_request(req)?;
        let payout_id = self.validate_proxy_create_identity(&req).await?;
        let customer_id = req.get_customer_id().get_required_value("customer_id")?;
        let processor = platform.get_processor();
        let customer = state
            .store
            .find_customer_optional_by_customer_id_merchant_id(
                customer_id,
                processor.get_account().get_id(),
                processor.get_key_store(),
                processor.get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?
            .ok_or(errors::ApiErrorResponse::CustomerNotFound)?;
        let payment_method_id = req
            .payout_method_id
            .as_ref()
            .get_required_value("payout_method_id")?;
        let payment_method = state
            .store
            .find_payment_method(
                processor.get_key_store(),
                payment_method_id,
                processor.get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
        let saved_source = self.validate_proxy_payment_method(&payment_method, customer_id)?;
        let execution_context = self
            .resolve_external_vault_execution_context(&provider_profile, saved_source)
            .await?;
        let dimensions = dimension_state::Dimensions::new()
            .with_processor_merchant_id(processor.get_processor_merchant_id())
            .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id());
        validate_payout_condition(
            !core_utils::get_feature_config(state, platform, &dimensions)
                .await
                .is_payment_method_modular_allowed,
            "External vault proxy payouts require PM modular service",
        )?;
        self.validate_proxy_fraud_policy(&profile, &dimensions)
            .await?;
        // Validate vault settings without retaining credential metadata.
        execution_context.external_vault_proxy_metadata(state)?;
        Ok(ProxyCreateInput {
            request: req,
            payout_id,
            profile,
            customer,
            payment_method,
            execution_context,
        })
    }

    /// Block PAN-dependent protection configurations until metadata-only support exists.
    async fn validate_proxy_fraud_policy(
        &self,
        profile: &domain::Profile,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> RouterResult<()> {
        let state = self.state;
        let dimensions = dimensions.with_profile_id(profile.get_id().clone());
        let payout_frm = dimensions
            .get_payout_frm_call(
                state.store.as_ref(),
                state.superposition_service.as_ref(),
                None,
            )
            .await;
        let payout_blocklist = dimensions
            .get_payout_blocklist_guard(
                state.store.as_ref(),
                state.superposition_service.as_ref(),
                None,
            )
            .await;
        let profile_card_blocking = profile
            .payment_method_blocking
            .as_ref()
            .is_some_and(|blocking| blocking.card.is_some());
        match payout_frm || payout_blocklist || profile_card_blocking {
            true => Err(report!(errors::ApiErrorResponse::NotImplemented {
                message: errors::NotImplementedMessage::Reason(
                    "external vault proxy payouts with FRM or card blocklist require metadata-only protection support".to_owned(),
                ),
            })),
            false => Ok(()),
        }
    }

    async fn persist_proxy_connector_routing(
        &self,
        connector_data: &api::ConnectorData,
        payout_data: &mut PayoutData,
    ) -> RouterResult<()> {
        let state = self.state;
        let platform = self.platform;
        match super::should_update_payout_attempt_routing(&payout_data.payout_attempt) {
            true => {
                let connector_request_reference_id =
                    core_utils::get_payout_connector_request_reference_id(
                        connector_data,
                        &payout_data.payout_attempt,
                    );
                let connector_name = connector_data.connector_name.to_string();
                payout_data.payout_attempt.connector = Some(connector_name.clone());
                let update = types::storage::PayoutAttemptUpdate::UpdateRouting {
                    connector: connector_name,
                    routing_info: payout_data.payout_attempt.routing_info.clone(),
                    merchant_connector_id: payout_data.payout_attempt.merchant_connector_id.clone(),
                    connector_request_reference_id,
                    active_frm_id: payout_data.payout_attempt.active_frm_id.clone(),
                };
                payout_data.payout_attempt = state
                    .store
                    .update_payout_attempt(
                        &payout_data.payout_attempt,
                        update,
                        &payout_data.payouts,
                        platform.get_processor().get_account().storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error updating routing info in payout_attempt")?;
                Ok(())
            }
            false => Ok(()),
        }
    }

    fn validate_proxy_recipient_account(
        &self,
        connector_data: &api::ConnectorData,
        payout_data: &PayoutData,
    ) -> RouterResult<()> {
        match (
            connector_data
                .connector_name
                .supports_vendor_disburse_account_create_for_payout(),
            payout_data.connector_transfer_method_id.as_ref(),
        ) {
            // Enrollment cannot infer a transfer-method ID from a generic payout-response ID.
            (true, None) => Err(report!(errors::ApiErrorResponse::NotImplemented {
                message: errors::NotImplementedMessage::Reason(
                    "proxy recipient-account enrollment requires an explicit transfer-method response".to_owned(),
                ),
            })),
            _ => Ok(()),
        }
    }

    async fn complete_proxy_fulfill(
        &self,
        header_payload: HeaderPayload,
        connector_data: &api::ConnectorData,
        payout_data: &mut PayoutData,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> RouterResult<()> {
        match !payout_data.should_terminate
            && payout_data.payout_attempt.status == PayoutStatus::RequiresFulfillment
        {
            true => {
                self.execute_proxy_fulfill(header_payload, connector_data, payout_data, dimensions)
                    .await
            }
            false => Ok(()),
        }
    }

    async fn execute_proxy_payout(
        &self,
        header_payload: HeaderPayload,
        connector_data: &api::ConnectorData,
        payout_data: &mut PayoutData,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
    ) -> RouterResult<()> {
        let state = self.state;
        let platform = self.platform;
        let context = self
            .proxy_gateway_context(header_payload.clone(), connector_data, payout_data)
            .await?;
        let selected_connector =
            payout_data.set_proxy_connector(context.merchant_connector_account, connector_data);
        let connector_data = &selected_connector;
        payout_data.set_proxy_transfer_method_id(
            super::helpers::should_create_connector_transfer_method(payout_data, connector_data)?,
        );
        self.validate_proxy_recipient_account(connector_data, payout_data)?;
        self.persist_proxy_connector_routing(connector_data, payout_data)
            .await?;
        Box::pin(super::complete_payout_eligibility(
            state,
            platform,
            header_payload.clone(),
            connector_data,
            payout_data,
        ))
        .await?;
        validate_payout_condition(
            payout_data.payout_attempt.is_eligible == Some(false),
            "Payout method data is ineligible",
        )?;
        Box::pin(super::complete_create_recipient(
            state,
            platform,
            header_payload.clone(),
            connector_data,
            payout_data,
        ))
        .await?;
        Box::pin(super::complete_create_recipient_disburse_account(
            state,
            platform,
            header_payload.clone(),
            connector_data,
            payout_data,
        ))
        .await?;
        Box::pin(super::complete_create_payout(
            state,
            platform,
            header_payload.clone(),
            connector_data,
            payout_data,
        ))
        .await?;
        self.complete_proxy_fulfill(header_payload, connector_data, payout_data, dimensions)
            .await
    }

    async fn validate_proxy_create_identity(
        &self,
        req: &payouts::PayoutCreateRequest,
    ) -> RouterResult<id_type::PayoutId> {
        let state = self.state;
        let platform = self.platform;
        let merchant_id = platform.get_processor().get_account().get_id();
        crate::utils::when(
            req.merchant_id
                .as_ref()
                .is_some_and(|mid| mid != merchant_id),
            || {
                Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: "merchant_id".into(),
                    expected_format: "merchant_id from merchant account".to_owned(),
                })
                .attach_printable("invalid merchant_id in request"))
            },
        )?;

        let payout_id = req
            .payout_id
            .as_ref()
            .get_required_value("payout_id")?
            .clone();
        match validator::validate_uniqueness_of_payout_id_against_merchant_id(
            &*state.store,
            &payout_id,
            merchant_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .attach_printable_lazy(|| {
            format!(
                "Unique violation while checking payout_id: {payout_id:?} against merchant_id: {merchant_id:?}"
            )
        })? {
            Some(_) => Err(report!(errors::ApiErrorResponse::DuplicatePayout { payout_id })),
            None => Ok(payout_id),
        }
    }
}
