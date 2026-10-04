//! Isolated S2S payout preparation. Tokens never enter normal payout method data.

use api_models::enums::VaultConnectors;
use common_enums::{PaymentMethodStatus, PayoutType};
use common_utils::id_type;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_methods::{PaymentMethod, PaymentMethodVaultSourceDetails, VaultPaymentMethodData},
    payouts::proxy::{ExternalVaultPayoutCardData, ExternalVaultPayoutMethodData},
};
use hyperswitch_masking::PeekInterface;

use super::validator;
use crate::{
    core::{
        configs::dimension_state,
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
        payment_methods::transformers::fetch_payment_method_from_modular_service,
        payments::helpers::{self as payment_helpers, MerchantConnectorAccountType},
        unified_connector_service, utils as core_utils,
    },
    routes::SessionState,
    types::{api::payouts, domain},
    utils::OptionExt,
};

/// Runtime execution configuration, separate from the payout connector MCA and
/// from token-bearing method data. Resolve the vault MCA through the provider's
/// business profile, as payments do; never accept it from merchant metadata.
#[derive(Clone, Debug, Default)]
pub enum PayoutExecutionContext {
    #[default]
    Normal,
    ExternalVaultProxy {
        external_vault_mca: MerchantConnectorAccountType,
    },
}

impl PayoutExecutionContext {
    /// Shares payments' metadata encoding, vault credentials, endpoint and egress proxy setup.
    /// The encoded header contains credentials and must never be logged or persisted.
    pub fn external_vault_proxy_metadata(
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

/// Internal normalization only; the public create contract stays PayoutCreateRequest.
/// Explicit conflicting flags are errors, not values to silently overwrite.
fn normalize_proxy_create_request(
    mut req: payouts::PayoutCreateRequest,
) -> RouterResult<payouts::PayoutCreateRequest> {
    req.payout_id.as_ref().get_required_value("payout_id")?;
    req.amount.as_ref().get_required_value("amount")?;
    req.currency.as_ref().get_required_value("currency")?;
    let payout_method_id = req
        .payout_method_id
        .as_ref()
        .get_required_value("payout_method_id")?;
    if payout_method_id.trim().is_empty() {
        return Err(invalid_proxy_request("payout_method_id must not be empty"));
    }
    req.get_customer_id().get_required_value("customer_id")?;
    if let (Some(legacy_id), Some(customer_id)) = (
        req.customer_id.as_ref(),
        req.customer
            .as_ref()
            .and_then(|customer| customer.id.as_ref()),
    ) {
        if legacy_id != customer_id {
            return Err(invalid_proxy_request("Conflicting customer identifiers"));
        }
    }

    if req.confirm == Some(false)
        || req.auto_fulfill == Some(false)
        || req.recurring == Some(true)
        || req
            .payout_type
            .is_some_and(|payout_type| payout_type != PayoutType::Card)
    {
        return Err(invalid_proxy_request(
            "External vault proxy payouts require confirm=true, auto_fulfill=true, recurring=false and payout_type=card",
        ));
    }
    if req.payout_method_data.is_some()
        || req.payout_token.is_some()
        || req.client_secret.is_some()
        || req.payout_link == Some(true)
        || req.payout_link_config.is_some()
        || req.session_expiry.is_some()
        || req.source_bank_data.is_some()
    {
        return Err(invalid_proxy_request(
            "External vault proxy payouts do not accept inline method data, payout_token, client_secret, payout-link options or source_bank_data",
        ));
    }
    req.confirm = Some(true);
    req.auto_fulfill = Some(true);
    req.recurring = Some(false);
    req.payout_type = Some(PayoutType::Card);
    Ok(req)
}

fn invalid_proxy_request(message: &str) -> error_stack::Report<errors::ApiErrorResponse> {
    report!(errors::ApiErrorResponse::InvalidRequestData {
        message: message.to_owned(),
    })
}

/// Validates the saved row, not a merchant-provided token or vault account override.
fn validate_proxy_payment_method<'a>(
    payment_method: &'a PaymentMethod,
    platform: &domain::Platform,
    customer_id: &id_type::CustomerId,
) -> RouterResult<&'a id_type::MerchantConnectorAccountId> {
    if payment_method.merchant_id != *platform.get_processor().get_account().get_id()
        || payment_method.customer_id.as_ref() != Some(customer_id)
    {
        return Err(invalid_proxy_request(
            "Payment method does not belong to this merchant and customer",
        ));
    }
    if payment_method.status != PaymentMethodStatus::Active
        || payment_method.payment_method != Some(common_enums::PaymentMethod::Card)
    {
        return Err(invalid_proxy_request(
            "External vault proxy payouts require an active saved card payment method",
        ));
    }
    match &payment_method.vault_source_details {
        PaymentMethodVaultSourceDetails::ExternalVault {
            external_vault_source,
        } => Ok(external_vault_source),
        PaymentMethodVaultSourceDetails::InternalVault => Err(invalid_proxy_request(
            "Payment method is not stored in an external vault",
        )),
    }
}

/// Resolve provider configuration as payments do. The processor's payout connector MCA is
/// deliberately resolved separately by payout routing, never from this vault account.
pub async fn resolve_external_vault_execution_context(
    state: &SessionState,
    platform: &domain::Platform,
    payout_profile: &domain::Profile,
    saved_vault_source: &id_type::MerchantConnectorAccountId,
) -> RouterResult<PayoutExecutionContext> {
    let provider_profile =
        payment_helpers::resolve_provider_profile(state, platform, payout_profile).await?;
    let vault_connector_id = provider_profile
        .external_vault_details
        .get_vault_connector_id()
        .get_required_value("provider profile external vault connector id")?;
    if &vault_connector_id != saved_vault_source {
        return Err(invalid_proxy_request(
            "Saved payment method vault does not match the provider profile external vault",
        ));
    }
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
    if external_vault_mca.merchant_id != *provider.get_account().get_id()
        || external_vault_mca.profile_id != *provider_profile.get_id()
        || external_vault_mca.disabled == Some(true)
        || !matches!(
            external_vault_mca.connector_type,
            common_enums::ConnectorType::VaultProcessor
        )
    {
        return Err(invalid_proxy_request(
            "External vault connector account is disabled or incompatible with the provider profile",
        ));
    }
    match VaultConnectors::try_from(external_vault_mca.connector_name.clone()) {
        Ok(VaultConnectors::HyperswitchVault | VaultConnectors::Vgs) => (),
        _ => {
            return Err(invalid_proxy_request(
                "Unsupported external vault payout proxy connector",
            ))
        }
    }
    Ok(PayoutExecutionContext::ExternalVaultProxy {
        external_vault_mca: MerchantConnectorAccountType::DbVal(Box::new(external_vault_mca)),
    })
}

/// Token-only PM modular fetch. No raw-detail retry, internal locker, CVC or PAN conversion.
/// This is consumed by proxy execution once the PR3 UCS payout contract is available.
pub async fn fetch_external_vault_payout_method(
    state: &SessionState,
    platform: &domain::Platform,
    profile_id: &id_type::ProfileId,
    saved_payment_method: &PaymentMethod,
) -> RouterResult<ExternalVaultPayoutMethodData> {
    let customer_id = saved_payment_method
        .customer_id
        .as_ref()
        .get_required_value("customer_id")?;
    validate_proxy_payment_method(saved_payment_method, platform, customer_id)?;
    let fetched = fetch_payment_method_from_modular_service(
        state,
        platform,
        profile_id,
        &saved_payment_method.payment_method_id,
        None,
        false, // Request proxy tokens, never a decrypted PAN.
        false, // Payouts do not trigger payment Account Updater sync.
    )
    .await?;
    // The shared transformer hardcodes InternalVault and Active on its reconstructed PM.
    // Keep vault affinity/status authoritative on the saved row; compare only response identity
    // and method type here, and require the separate VaultCardData token channel below.
    if fetched.payment_method.payment_method_id != saved_payment_method.payment_method_id
        || fetched.payment_method.merchant_id != saved_payment_method.merchant_id
        || fetched.payment_method.customer_id.as_ref() != Some(customer_id)
        || fetched.payment_method.payment_method != Some(common_enums::PaymentMethod::Card)
        || fetched.raw_payment_method_data.is_some()
    {
        return Err(invalid_proxy_request(
            "PM modular returned incompatible proxy payment method data",
        ));
    }
    let VaultPaymentMethodData::VaultCardData(card) = fetched
        .vault_payment_method_token_data
        .get_required_value("external vault proxy card data")?;
    if card.card_number.peek().is_empty() {
        return Err(invalid_proxy_request(
            "External vault card token must not be empty",
        ));
    }
    let expiry_month = card
        .card_exp_month
        .get_required_value("external vault card expiry month")?;
    let expiry_year = card
        .card_exp_year
        .get_required_value("external vault card expiry year")?;
    if expiry_month.peek().is_empty() || expiry_year.peek().is_empty() {
        return Err(invalid_proxy_request(
            "External vault card expiry must not be empty",
        ));
    }
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

/// Dispatch target for POST /payouts/create. Until the UCS CardProxyPayout contract is published,
/// perform only read-only preflight and stop before token fetching or persistent side effects.
/// Do not remove this boundary until token-aware constructors and all card-consuming gateways
/// are wired; normal CardPayout is not a valid representation of an opaque vault token.
pub async fn payouts_proxy_core(
    state: &SessionState,
    platform: &domain::Platform,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let req = normalize_proxy_create_request(req)?;
    validator::validate_create_request_identity(state, platform, &req).await?;
    let customer_id = req.get_customer_id().get_required_value("customer_id")?;
    let processor = platform.get_processor();
    state
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
    let saved_source = validate_proxy_payment_method(&payment_method, platform, customer_id)?;
    let profile = core_utils::get_profile_from_business_details(
        req.business_country,
        req.business_label.as_ref(),
        processor,
        req.profile_id.as_ref(),
        &*state.store,
    )
    .await?;
    let execution_context =
        resolve_external_vault_execution_context(state, platform, &profile, saved_source).await?;
    let dimensions = dimension_state::Dimensions::new()
        .with_processor_merchant_id(processor.get_processor_merchant_id())
        .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id());
    if !core_utils::get_feature_config(state, platform, &dimensions)
        .await
        .is_payment_method_modular_allowed
    {
        return Err(invalid_proxy_request(
            "External vault proxy payouts require PM modular service",
        ));
    }
    // Validate vault configuration without exposing the encoded credential-bearing header.
    execution_context.external_vault_proxy_metadata(state)?;
    Err(report!(errors::ApiErrorResponse::NotImplemented {
        message: errors::NotImplementedMessage::Reason(
            "external vault proxy payouts require the UCS CardProxyPayout contract".to_owned(),
        ),
    }))
}
