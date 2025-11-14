use std::{collections::HashSet, fmt::Debug};

use api_models::{enums as api_enums, open_router};
use common_enums::enums;
use common_utils::{errors::CustomResult, ext_traits::ValueExt, id_type};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};

use super::{
    payments::{OperationSessionGetters, OperationSessionSetters},
    routing::TransactionData,
};
use crate::{
    core::{
        errors,
        payments::{operations::BoxedOperation, routing},
    },
    logger,
    routes::SessionState,
    settings,
    types::{
        api::{self, ConnectorCallType},
        domain,
    },
    utils::id_type::MerchantConnectorAccountId,
};

pub struct DebitRoutingResult {
    pub debit_routing_connector_call_type: ConnectorCallType,
    pub debit_routing_output: open_router::DebitRoutingOutput,
}

pub async fn perform_debit_routing<F, Req, D>(
    operation: &BoxedOperation<'_, F, Req, D>,
    state: &SessionState,
    business_profile: &domain::Profile,
    payment_data: &mut D,
    connector: Option<ConnectorCallType>,
) -> (
    Option<ConnectorCallType>,
    Option<open_router::DebitRoutingOutput>,
)
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let mut debit_routing_output = None;

    if should_execute_debit_routing(state, business_profile, operation, payment_data).await {
        let debit_routing_config = state.conf.debit_routing_config.clone();
        let debit_routing_supported_connectors = debit_routing_config.supported_connectors.clone();

        // If the business profile does not have a country set, we cannot perform debit routing,
        // because the merchant_business_country will be treated as the acquirer_country,
        // which is used to determine whether a transaction is local or global in the open router.
        // For now, since debit routing is only implemented for USD, we can safely assume the
        // acquirer_country is US if not provided by the merchant.

        let acquirer_country = business_profile
            .merchant_business_country
            .unwrap_or_default();

        if let Some(call_connector_type) = connector.clone() {
            debit_routing_output = match call_connector_type {
                ConnectorCallType::PreDetermined(connector_data) => {
                    logger::info!("Performing debit routing for PreDetermined connector");
                    handle_pre_determined_connector(
                        state,
                        debit_routing_supported_connectors,
                        &connector_data,
                        payment_data,
                        acquirer_country,
                    )
                    .await
                }
                ConnectorCallType::Retryable(connector_data) => {
                    logger::info!("Performing debit routing for Retryable connector");
                    handle_retryable_connector(
                        state,
                        debit_routing_supported_connectors,
                        connector_data,
                        payment_data,
                        acquirer_country,
                    )
                    .await
                }
                ConnectorCallType::SessionMultiple(_) => {
                    logger::info!(
                        "SessionMultiple connector type is not supported for debit routing"
                    );
                    None
                }
                #[cfg(feature = "v2")]
                ConnectorCallType::Skip => {
                    logger::info!("Skip connector type is not supported for debit routing");
                    None
                }
            };
        }
    }

    if let Some(debit_routing_output) = debit_routing_output {
        (
            Some(debit_routing_output.debit_routing_connector_call_type),
            Some(debit_routing_output.debit_routing_output),
        )
    } else {
        // If debit_routing_output is None, return the static routing output (connector)
        logger::info!("Debit routing is not performed, returning static routing output");
        (connector, None)
    }
}

async fn should_execute_debit_routing<F, Req, D>(
    state: &SessionState,
    business_profile: &domain::Profile,
    operation: &BoxedOperation<'_, F, Req, D>,
    payment_data: &D,
) -> bool
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    if business_profile.is_debit_routing_enabled && state.conf.open_router.dynamic_routing_enabled {
        logger::info!("Debit routing is enabled for the profile");

        let debit_routing_config = &state.conf.debit_routing_config;

        if should_perform_debit_routing_for_the_flow(operation, payment_data, debit_routing_config)
        {
            let is_debit_routable_connector_present = check_for_debit_routing_connector_in_profile(
                state,
                business_profile.get_id(),
                payment_data,
            )
            .await;

            if is_debit_routable_connector_present {
                logger::debug!("Debit routable connector is configured for the profile");
                return true;
            }
        }
    }
    false
}

pub fn should_perform_debit_routing_for_the_flow<Op: Debug, F: Clone, D>(
    operation: &Op,
    payment_data: &D,
    debit_routing_config: &settings::DebitRoutingConfig,
) -> bool
where
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    match format!("{operation:?}").as_str() {
        "PaymentConfirm" => {
            logger::info!("Checking if debit routing is required");

            request_validation(payment_data, debit_routing_config)
        }
        _ => false,
    }
}

fn request_validation<F: Clone, D>(
    payment_data: &D,
    debit_routing_config: &settings::DebitRoutingConfig,
) -> bool
where
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    let payment_intent = payment_data.get_payment_intent();
    let payment_attempt = payment_data.get_payment_attempt();

    let is_currency_supported = is_currency_supported(payment_intent, debit_routing_config);

    let is_valid_payment_method = validate_payment_method_for_debit_routing(payment_data);

    payment_intent.setup_future_usage != Some(enums::FutureUsage::OffSession)
        && payment_intent.amount.is_greater_than(0)
        && is_currency_supported
        && payment_attempt.authentication_type == Some(enums::AuthenticationType::NoThreeDs)
        && is_valid_payment_method
}

fn is_currency_supported(
    payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
    debit_routing_config: &settings::DebitRoutingConfig,
) -> bool {
    payment_intent
        .currency
        .map(|currency| {
            debit_routing_config
                .supported_currencies
                .contains(&currency)
        })
        .unwrap_or(false)
}

fn validate_payment_method_for_debit_routing<F: Clone, D>(payment_data: &D) -> bool
where
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    let payment_attempt = payment_data.get_payment_attempt();
    match payment_attempt.payment_method {
        Some(enums::PaymentMethod::Card) => {
            payment_attempt.payment_method_type == Some(enums::PaymentMethodType::Debit)
        }
        Some(enums::PaymentMethod::Wallet) => {
            payment_attempt.payment_method_type == Some(enums::PaymentMethodType::ApplePay)
                && payment_data
                    .get_payment_method_data()
                    .and_then(|data| data.get_wallet_data())
                    .and_then(|data| data.get_apple_pay_wallet_data())
                    .and_then(|data| data.get_payment_method_type())
                    == Some(enums::PaymentMethodType::Debit)
                && matches!(
                    payment_data.get_payment_method_token().cloned(),
                    Some(
                        hyperswitch_domain_models::router_data::PaymentMethodToken::ApplePayDecrypt(
                            _
                        )
                    )
                )
        }
        _ => false,
    }
}

pub async fn check_for_debit_routing_connector_in_profile<
    F: Clone,
    D: OperationSessionGetters<F>,
>(
    state: &SessionState,
    business_profile_id: &id_type::ProfileId,
    payment_data: &D,
) -> bool {
    logger::debug!("Checking for debit routing connector in profile");
    let debit_routing_supported_connectors =
        state.conf.debit_routing_config.supported_connectors.clone();

    let transaction_data = super::routing::PaymentsDslInput::new(
        payment_data.get_setup_mandate(),
        payment_data.get_payment_attempt(),
        payment_data.get_payment_intent(),
        payment_data.get_payment_method_data(),
        payment_data.get_address(),
        payment_data.get_recurring_details(),
        payment_data.get_currency(),
    );

    let fallback_config_optional = super::routing::helpers::get_merchant_default_config(
        &*state.clone().store,
        business_profile_id.get_string_repr(),
        &enums::TransactionType::from(&TransactionData::Payment(transaction_data)),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .map_err(|error| {
        logger::warn!(?error, "Failed to fetch default connector for a profile");
    })
    .ok();

    let is_debit_routable_connector_present = fallback_config_optional
        .map(|fallback_config| {
            fallback_config.iter().any(|fallback_config_connector| {
                debit_routing_supported_connectors.contains(&api_enums::Connector::from(
                    fallback_config_connector.connector,
                ))
            })
        })
        .unwrap_or(false);

    is_debit_routable_connector_present
}

async fn handle_pre_determined_connector<F, D>(
    state: &SessionState,
    debit_routing_supported_connectors: HashSet<api_enums::Connector>,
    connector_data: &api::ConnectorRoutingData,
    payment_data: &mut D,
    acquirer_country: enums::CountryAlpha2,
) -> Option<DebitRoutingResult>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let db = state.store.as_ref();
    let merchant_id = payment_data.get_payment_attempt().merchant_id.clone();
    let profile_id = payment_data.get_payment_attempt().profile_id.clone();

    if debit_routing_supported_connectors.contains(&connector_data.connector_data.connector_name) {
        logger::debug!("Chosen connector is supported for debit routing");

        let debit_routing_output =
            get_debit_routing_output::<F, D>(state, payment_data, acquirer_country).await?;

        logger::debug!(
            "Sorted co-badged networks info: {:?}",
            debit_routing_output.co_badged_card_networks_info
        );

        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::MerchantAccountNotFound)
            .map_err(|error| {
                logger::error!(
                    "Failed to get merchant key store by merchant_id  {:?}",
                    error
                )
            })
            .ok()?;

        let connector_routing_data = build_connector_routing_data(
            state,
            &profile_id,
            &key_store,
            vec![connector_data.clone()],
            debit_routing_output
                .co_badged_card_networks_info
                .clone()
                .get_card_networks(),
        )
        .await
        .map_err(|error| {
            logger::error!(
                "Failed to build connector routing data for debit routing {:?}",
                error
            )
        })
        .ok()?;

        if !connector_routing_data.is_empty() {
            return Some(DebitRoutingResult {
                debit_routing_connector_call_type: ConnectorCallType::Retryable(
                    connector_routing_data,
                ),
                debit_routing_output,
            });
        }
    }

    None
}

pub async fn get_debit_routing_output<
    F: Clone + Send,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
>(
    state: &SessionState,
    payment_data: &mut D,
    acquirer_country: enums::CountryAlpha2,
) -> Option<open_router::DebitRoutingOutput> {
    logger::debug!("Fetching sorted card networks");

    let card_info = extract_card_info(payment_data);

    let saved_co_badged_card_data = card_info.co_badged_card_data;
    let saved_card_type = card_info.card_type;
    let card_isin = card_info.card_isin;

    match (
        saved_co_badged_card_data
            .clone()
            .zip(saved_card_type.clone()),
        card_isin.clone(),
    ) {
        (None, None) => {
            logger::debug!("Neither co-badged data nor ISIN found; skipping routing");
            None
        }
        _ => {
            let co_badged_card_data = saved_co_badged_card_data
                .zip(saved_card_type)
                .and_then(|(co_badged, card_type)| {
                    open_router::DebitRoutingRequestData::try_from((co_badged, card_type))
                        .map(Some)
                        .map_err(|error| {
                            logger::warn!("Failed to convert co-badged card data: {:?}", error);
                        })
                        .ok()
                })
                .flatten();

            if co_badged_card_data.is_none() && card_isin.is_none() {
                logger::debug!("Neither co-badged data nor ISIN found; skipping routing");
                return None;
            }

            let co_badged_card_request = open_router::CoBadgedCardRequest {
                merchant_category_code: enums::DecisionEngineMerchantCategoryCode::Mcc0001,
                acquirer_country,
                co_badged_card_data,
            };

            routing::perform_open_routing_for_debit_routing(
                state,
                co_badged_card_request,
                card_isin,
                payment_data,
            )
            .await
            .map_err(|error| {
                logger::warn!(?error, "Debit routing call to open router failed");
            })
            .ok()
        }
    }
}

#[derive(Debug, Clone)]
struct ExtractedCardInfo {
    co_badged_card_data: Option<api_models::payment_methods::CoBadgedCardData>,
    card_type: Option<String>,
    card_isin: Option<Secret<String>>,
}

impl ExtractedCardInfo {
    fn new(
        co_badged_card_data: Option<api_models::payment_methods::CoBadgedCardData>,
        card_type: Option<String>,
        card_isin: Option<Secret<String>>,
    ) -> Self {
        Self {
            co_badged_card_data,
            card_type,
            card_isin,
        }
    }

    fn empty() -> Self {
        Self::new(None, None, None)
    }
}

fn extract_card_info<F, D>(payment_data: &D) -> ExtractedCardInfo
where
    D: OperationSessionGetters<F>,
{
    extract_from_saved_payment_method(payment_data)
        .unwrap_or_else(|| extract_from_payment_method_data(payment_data))
}

fn extract_from_saved_payment_method<F, D>(payment_data: &D) -> Option<ExtractedCardInfo>
where
    D: OperationSessionGetters<F>,
{
    let payment_methods_data = payment_data
        .get_payment_method_info()?
        .get_payment_methods_data()?;

    if let hyperswitch_domain_models::payment_method_data::PaymentMethodsData::Card(card) =
        payment_methods_data
    {
        return Some(extract_card_info_from_saved_card(&card));
    }

    None
}

fn extract_card_info_from_saved_card(
    card: &hyperswitch_domain_models::payment_method_data::CardDetailsPaymentMethod,
) -> ExtractedCardInfo {
    match (&card.co_badged_card_data, &card.card_isin) {
        (Some(co_badged), _) => {
            logger::debug!("Co-badged card data found in saved payment method");
            ExtractedCardInfo::new(Some(co_badged.clone()), card.card_type.clone(), None)
        }
        (None, Some(card_isin)) => {
            logger::debug!("No co-badged data; using saved card ISIN");
            ExtractedCardInfo::new(None, None, Some(Secret::new(card_isin.clone())))
        }
        _ => ExtractedCardInfo::empty(),
    }
}

fn extract_from_payment_method_data<F, D>(payment_data: &D) -> ExtractedCardInfo
where
    D: OperationSessionGetters<F>,
{
    match payment_data.get_payment_method_data() {
        Some(hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(card)) => {
            logger::debug!("Using card data from payment request");
            ExtractedCardInfo::new(
                None,
                None,
                Some(Secret::new(card.card_number.get_extended_card_bin())),
            )
        }
        Some(hyperswitch_domain_models::payment_method_data::PaymentMethodData::Wallet(
            wallet_data,
        )) => extract_from_wallet_data(wallet_data, payment_data),
        _ => ExtractedCardInfo::empty(),
    }
}

fn extract_from_wallet_data<F, D>(
    wallet_data: &hyperswitch_domain_models::payment_method_data::WalletData,
    payment_data: &D,
) -> ExtractedCardInfo
where
    D: OperationSessionGetters<F>,
{
    match wallet_data {
        hyperswitch_domain_models::payment_method_data::WalletData::ApplePay(_) => {
            logger::debug!("Using Apple Pay data from payment request");
            let apple_pay_isin = extract_apple_pay_isin(payment_data);
            ExtractedCardInfo::new(None, None, apple_pay_isin)
        }
        _ => ExtractedCardInfo::empty(),
    }
}

fn extract_apple_pay_isin<F, D>(payment_data: &D) -> Option<Secret<String>>
where
    D: OperationSessionGetters<F>,
{
    payment_data.get_payment_method_token().and_then(|token| {
        if let hyperswitch_domain_models::router_data::PaymentMethodToken::ApplePayDecrypt(
            apple_pay_decrypt_data,
        ) = token
        {
            logger::debug!("Using Apple Pay decrypt data from payment method token");
            Some(Secret::new(
                apple_pay_decrypt_data
                    .application_primary_account_number
                    .peek()
                    .chars()
                    .take(8)
                    .collect::<String>(),
            ))
        } else {
            None
        }
    })
}

async fn handle_retryable_connector<F, D>(
    state: &SessionState,
    debit_routing_supported_connectors: HashSet<api_enums::Connector>,
    connector_data_list: Vec<api::ConnectorRoutingData>,
    payment_data: &mut D,
    acquirer_country: enums::CountryAlpha2,
) -> Option<DebitRoutingResult>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let db = state.store.as_ref();
    let profile_id = payment_data.get_payment_attempt().profile_id.clone();
    let merchant_id = payment_data.get_payment_attempt().merchant_id.clone();
    let is_any_debit_routing_connector_supported =
        connector_data_list.iter().any(|connector_data| {
            debit_routing_supported_connectors
                .contains(&connector_data.connector_data.connector_name)
        });

    if is_any_debit_routing_connector_supported {
        let debit_routing_output =
            get_debit_routing_output::<F, D>(state, payment_data, acquirer_country).await?;
        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::MerchantAccountNotFound)
            .map_err(|error| {
                logger::error!(
                    "Failed to get merchant key store by merchant_id  {:?}",
                    error
                )
            })
            .ok()?;

        let connector_routing_data = build_connector_routing_data(
            state,
            &profile_id,
            &key_store,
            connector_data_list.clone(),
            debit_routing_output
                .co_badged_card_networks_info
                .clone()
                .get_card_networks(),
        )
        .await
        .map_err(|error| {
            logger::error!(
                "Failed to build connector routing data for debit routing {:?}",
                error
            )
        })
        .ok()?;

        if !connector_routing_data.is_empty() {
            return Some(DebitRoutingResult {
                debit_routing_connector_call_type: ConnectorCallType::Retryable(
                    connector_routing_data,
                ),
                debit_routing_output,
            });
        };
    }

    None
}

async fn build_connector_routing_data(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
    key_store: &domain::MerchantKeyStore,
    eligible_connector_data_list: Vec<api::ConnectorRoutingData>,
    fee_sorted_debit_networks: Vec<common_enums::CardNetwork>,
) -> CustomResult<Vec<api::ConnectorRoutingData>, errors::ApiErrorResponse> {
    let debit_routing_config = &state.conf.debit_routing_config;

    let mcas_for_profile = fetch_merchant_connector_accounts(state, profile_id, key_store).await?;

    let mut connector_routing_data = Vec::new();
    let mut has_us_local_network = false;

    for connector_data in eligible_connector_data_list {
        if let Some(routing_data) = process_connector_for_networks(
            &connector_data,
            &mcas_for_profile,
            &fee_sorted_debit_networks,
            debit_routing_config,
            &mut has_us_local_network,
        )? {
            connector_routing_data.extend(routing_data);
        }
    }

    validate_us_local_network_requirement(has_us_local_network)?;
    Ok(connector_routing_data)
}

/// Fetches merchant connector accounts for the given profile
async fn fetch_merchant_connector_accounts(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::ApiErrorResponse> {
    state
        .store
        .list_enabled_connector_accounts_by_profile_id(
            profile_id,
            key_store,
            common_enums::ConnectorType::PaymentProcessor,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch merchant connector accounts")
}

/// Processes a single connector to find matching networks
fn process_connector_for_networks(
    connector_data: &api::ConnectorRoutingData,
    mcas_for_profile: &[domain::MerchantConnectorAccount],
    fee_sorted_debit_networks: &[common_enums::CardNetwork],
    debit_routing_config: &settings::DebitRoutingConfig,
    has_us_local_network: &mut bool,
) -> CustomResult<Option<Vec<api::ConnectorRoutingData>>, errors::ApiErrorResponse> {
    let Some(merchant_connector_id) = &connector_data.connector_data.merchant_connector_id else {
        logger::warn!("Skipping connector with missing merchant_connector_id");
        return Ok(None);
    };

    let Some(account) = find_merchant_connector_account(mcas_for_profile, merchant_connector_id)
    else {
        logger::warn!(
            "No MCA found for merchant_connector_id: {:?}",
            merchant_connector_id
        );
        return Ok(None);
    };

    let merchant_debit_networks = extract_debit_networks(&account)?;
    let matching_networks = find_matching_networks(
        &merchant_debit_networks,
        fee_sorted_debit_networks,
        connector_data,
        debit_routing_config,
        has_us_local_network,
    );

    Ok(Some(matching_networks))
}

/// Finds a merchant connector account by ID
fn find_merchant_connector_account(
    mcas: &[domain::MerchantConnectorAccount],
    merchant_connector_id: &MerchantConnectorAccountId,
) -> Option<domain::MerchantConnectorAccount> {
    mcas.iter()
        .find(|mca| mca.merchant_connector_id == *merchant_connector_id)
        .cloned()
}

/// Finds networks that match between merchant and fee-sorted networks
fn find_matching_networks(
    merchant_debit_networks: &HashSet<common_enums::CardNetwork>,
    fee_sorted_debit_networks: &[common_enums::CardNetwork],
    connector_routing_data: &api::ConnectorRoutingData,
    debit_routing_config: &settings::DebitRoutingConfig,
    has_us_local_network: &mut bool,
) -> Vec<api::ConnectorRoutingData> {
    let is_routing_enabled = debit_routing_config
        .supported_connectors
        .contains(&connector_routing_data.connector_data.connector_name.clone());

    fee_sorted_debit_networks
        .iter()
        .filter(|network| merchant_debit_networks.contains(network))
        .filter(|network| is_routing_enabled || network.is_signature_network())
        .map(|network| {
            if network.is_us_local_network() {
                *has_us_local_network = true;
            }

            api::ConnectorRoutingData {
                connector_data: connector_routing_data.connector_data.clone(),
                network: Some(network.clone()),
                action_type: connector_routing_data.action_type.clone(),
            }
        })
        .collect()
}

/// Validates that at least one US local network is present
fn validate_us_local_network_requirement(
    has_us_local_network: bool,
) -> CustomResult<(), errors::ApiErrorResponse> {
    if !has_us_local_network {
        return Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("At least one US local network is required in routing");
    }
    Ok(())
}

fn extract_debit_networks(
    account: &domain::MerchantConnectorAccount,
) -> CustomResult<HashSet<common_enums::CardNetwork>, errors::ApiErrorResponse> {
    let mut networks = HashSet::new();

    if let Some(values) = &account.payment_methods_enabled {
        for val in values {
            let payment_methods_enabled: api_models::admin::PaymentMethodsEnabled =
                val.to_owned().parse_value("PaymentMethodsEnabled")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to parse enabled payment methods for a merchant connector account in debit routing flow")?;

            if let Some(types) = payment_methods_enabled.payment_method_types {
                for method_type in types {
                    if method_type.payment_method_type
                        == api_models::enums::PaymentMethodType::Debit
                    {
                        if let Some(card_networks) = method_type.card_networks {
                            networks.extend(card_networks);
                        }
                    }
                }
            }
        }
    }

    Ok(networks)
}
