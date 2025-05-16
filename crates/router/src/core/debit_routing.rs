use std::{collections::HashSet, fmt::Debug};

use api_models::{enums as api_enums, open_router};
use common_enums::enums;
use common_utils::id_type;
use error_stack::ResultExt;
use masking::Secret;

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

        if let Some((call_connector_type, acquirer_country)) = connector
            .clone()
            .zip(business_profile.merchant_business_country)
        {
            debit_routing_output = match call_connector_type {
                ConnectorCallType::PreDetermined(connector_data) => {
                    logger::info!("Performing debit routing for PreDetermined connector");
                    handle_pre_determined_connector(
                        state,
                        &debit_routing_config,
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
                        &debit_routing_config,
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
    if business_profile.is_debit_routing_enabled
        && state.conf.open_router.enabled
        && business_profile.merchant_business_country.is_some()
    {
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
            let payment_intent = payment_data.get_payment_intent();
            let payment_attempt = payment_data.get_payment_attempt();

            request_validation(payment_intent, payment_attempt, debit_routing_config)
        }
        _ => false,
    }
}

pub fn request_validation(
    payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
    payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    debit_routing_config: &settings::DebitRoutingConfig,
) -> bool {
    logger::debug!("Validating request for debit routing");
    let is_currency_supported = payment_intent.currency.map(|currency| {
        debit_routing_config
            .supported_currencies
            .contains(&currency)
    });

    payment_intent.setup_future_usage != Some(enums::FutureUsage::OffSession)
        && payment_intent.amount.is_greater_than(0)
        && is_currency_supported == Some(true)
        && payment_attempt.authentication_type != Some(enums::AuthenticationType::ThreeDs)
        && payment_attempt.payment_method == Some(enums::PaymentMethod::Card)
        && payment_attempt.payment_method_type == Some(enums::PaymentMethodType::Debit)
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
    debit_routing_config: &settings::DebitRoutingConfig,
    debit_routing_supported_connectors: HashSet<api_enums::Connector>,
    connector_data: &api::ConnectorRoutingData,
    payment_data: &mut D,
    acquirer_country: enums::CountryAlpha2,
) -> Option<DebitRoutingResult>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    if debit_routing_supported_connectors.contains(&connector_data.connector_data.connector_name) {
        logger::debug!("Chosen connector is supported for debit routing");

        let debit_routing_output =
            get_debit_routing_output::<F, D>(state, payment_data, acquirer_country).await?;

        logger::debug!(
            "Sorted co-badged networks: {:?}",
            debit_routing_output.co_badged_card_networks
        );

        let valid_connectors = build_connector_routing_data(
            connector_data,
            debit_routing_config,
            &debit_routing_output.co_badged_card_networks,
        );

        if !valid_connectors.is_empty() {
            return Some(DebitRoutingResult {
                debit_routing_connector_call_type: ConnectorCallType::Retryable(valid_connectors),
                debit_routing_output,
            });
        }
    }

    None
}

pub async fn get_debit_routing_output<
    F: Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F>,
>(
    state: &SessionState,
    payment_data: &mut D,
    acquirer_country: enums::CountryAlpha2,
) -> Option<open_router::DebitRoutingOutput> {
    logger::debug!("Fetching sorted card networks");
    let payment_attempt = payment_data.get_payment_attempt();

    let (saved_co_badged_card_data, saved_card_type, card_isin) =
        extract_saved_card_info(payment_data);

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
                merchant_category_code: enums::MerchantCategoryCode::Mcc0001,
                acquirer_country,
                co_badged_card_data,
            };

            routing::perform_open_routing_for_debit_routing(
                state,
                payment_attempt,
                co_badged_card_request,
                card_isin,
            )
            .await
            .map_err(|error| {
                logger::warn!(?error, "Debit routing call to open router failed");
            })
            .ok()
        }
    }
}

fn extract_saved_card_info<F, D>(
    payment_data: &D,
) -> (
    Option<api_models::payment_methods::CoBadgedCardData>,
    Option<String>,
    Option<Secret<String>>,
)
where
    D: OperationSessionGetters<F>,
{
    let payment_method_data_optional = payment_data.get_payment_method_data();
    match payment_data
        .get_payment_method_info()
        .and_then(|info| info.get_payment_methods_data())
    {
        Some(hyperswitch_domain_models::payment_method_data::PaymentMethodsData::Card(card)) => {
            match (&card.co_badged_card_data, &card.card_isin) {
                (Some(co_badged), _) => {
                    logger::debug!("Co-badged card data found in saved payment method");
                    (Some(co_badged.clone()), card.card_type, None)
                }
                (None, Some(card_isin)) => {
                    logger::debug!("No co-badged data; using saved card ISIN");
                    (None, None, Some(Secret::new(card_isin.clone())))
                }
                _ => (None, None, None),
            }
        }
        _ => match payment_method_data_optional {
            Some(hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(card)) => {
                logger::debug!("Using card data from payment request");
                (
                    None,
                    None,
                    Some(Secret::new(card.card_number.get_card_isin())),
                )
            }
            _ => (None, None, None),
        },
    }
}

fn check_connector_support_for_network(
    debit_routing_config: &settings::DebitRoutingConfig,
    connector_name: api_enums::Connector,
    network: &enums::CardNetwork,
) -> Option<enums::CardNetwork> {
    debit_routing_config
        .connector_supported_debit_networks
        .get(&connector_name)
        .and_then(|supported_networks| {
            (supported_networks.contains(network) || network.is_global_network())
                .then(|| network.clone())
        })
}

fn build_connector_routing_data(
    connector_data: &api::ConnectorRoutingData,
    debit_routing_config: &settings::DebitRoutingConfig,
    fee_sorted_debit_networks: &[enums::CardNetwork],
) -> Vec<api::ConnectorRoutingData> {
    fee_sorted_debit_networks
        .iter()
        .filter_map(|network| {
            check_connector_support_for_network(
                debit_routing_config,
                connector_data.connector_data.connector_name,
                network,
            )
            .map(|valid_network| api::ConnectorRoutingData {
                connector_data: connector_data.connector_data.clone(),
                network: Some(valid_network),
            })
        })
        .collect()
}

async fn handle_retryable_connector<F, D>(
    state: &SessionState,
    debit_routing_config: &settings::DebitRoutingConfig,
    debit_routing_supported_connectors: HashSet<api_enums::Connector>,
    connector_data_list: Vec<api::ConnectorRoutingData>,
    payment_data: &mut D,
    acquirer_country: enums::CountryAlpha2,
) -> Option<DebitRoutingResult>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let is_any_debit_routing_connector_supported =
        connector_data_list.iter().any(|connector_data| {
            debit_routing_supported_connectors
                .contains(&connector_data.connector_data.connector_name)
        });

    if is_any_debit_routing_connector_supported {
        let debit_routing_output =
            get_debit_routing_output::<F, D>(state, payment_data, acquirer_country).await?;

        logger::debug!(
            "Sorted co-badged networks: {:?}",
            debit_routing_output.co_badged_card_networks
        );

        let supported_connectors: Vec<_> = connector_data_list
            .iter()
            .flat_map(|connector_data| {
                build_connector_routing_data(
                    connector_data,
                    debit_routing_config,
                    &debit_routing_output.co_badged_card_networks,
                )
            })
            .collect();

        if !supported_connectors.is_empty() {
            return Some(DebitRoutingResult {
                debit_routing_connector_call_type: ConnectorCallType::Retryable(
                    supported_connectors,
                ),
                debit_routing_output,
            });
        }
    }

    None
}
