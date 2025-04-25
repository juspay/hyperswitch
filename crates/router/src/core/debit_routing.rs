use api_models::enums as api_enums;
use common_enums::enums;
use masking::Secret;

use super::{
    payments::OperationSessionGetters, payments::OperationSessionSetters, routing::TransactionData,
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
use common_utils::id_type;
use error_stack::ResultExt;
use std::{collections::HashSet, fmt::Debug};

pub async fn perform_debit_routing<F, Req, D>(
    operation: &BoxedOperation<'_, F, Req, D>,
    state: &SessionState,
    business_profile: &domain::Profile,
    payment_data: &D,
    connector: Option<ConnectorCallType>,
) -> (Option<ConnectorCallType>, bool)
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let mut debit_routing_output = None;
    let mut is_debit_routing_performed = true;

    if business_profile.is_debit_routing_enabled && state.conf.open_router.enabled {
        if let Some(acquirer_country) = business_profile.merchant_business_country {
            logger::info!("Debit routing is enabled for the profile");

            let debit_routing_config = state.conf.debit_routing_config.clone();
            let debit_routing_supported_connectors =
                state.conf.debit_routing_config.supported_connectors.clone();

            if should_perform_debit_routing_for_the_flow(
                operation,
                payment_data,
                &debit_routing_config,
            ) {
                let is_debit_routable_connector_present_in_profile =
                    check_for_debit_routing_connector_in_profile(
                        state,
                        business_profile.get_id(),
                        payment_data,
                    )
                    .await;

                if is_debit_routable_connector_present_in_profile {
                    logger::debug!("Debit routable connector is configured for the profile");

                    if let Some(call_connector_type) = connector.clone() {
                        debit_routing_output = match call_connector_type {
                            ConnectorCallType::PreDetermined(connector_data) => {
                                logger::info!(
                                    "Performing debit routing for PreDetermined connector"
                                );
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
                            ConnectorCallType::SessionMultiple(session_connector_data) => {
                                logger::info!("SessionMultiple connector type is not supported for debit routing");
                                Some(ConnectorCallType::SessionMultiple(session_connector_data))
                            }
                            #[cfg(feature = "v2")]
                            ConnectorCallType::Skip => {
                                logger::info!(
                                    "Skip connector type is not supported for debit routing"
                                );
                                Some(ConnectorCallType::Skip)
                            }
                        };
                    }
                }
            }
        }
    }

    // If debit_routing_output is None, we return the output of static routing
    if debit_routing_output.is_none() {
        debit_routing_output = connector;
        is_debit_routing_performed = false;
        logger::info!("Debit routing is not performed, returning static routing output");
    }

    (debit_routing_output, is_debit_routing_performed)
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
        && payment_intent.amount.get_amount_as_i64() > 0
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
    payment_data: &D,
    acquirer_country: enums::CountryAlpha2,
) -> Option<ConnectorCallType>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    if debit_routing_supported_connectors.contains(&connector_data.connector_data.connector_name) {
        logger::debug!("Chosen connector is supported for debit routing");
        let fee_sorted_debit_networks =
            get_sorted_co_badged_networks_by_fee::<F, D>(state, payment_data, acquirer_country)
                .await?;

        let mut valid_connectors = Vec::new();

        for network in fee_sorted_debit_networks {
            if let Some(local_networks) = check_connector_support_for_network(
                debit_routing_config,
                connector_data.connector_data.connector_name,
                &network,
            ) {
                valid_connectors.push(api::ConnectorRoutingData {
                    connector_data: connector_data.connector_data.clone(),
                    network: Some(local_networks),
                });
            }
        }

        if !valid_connectors.is_empty() {
            return Some(ConnectorCallType::Retryable(valid_connectors));
        }
    }
    None
}

pub async fn get_sorted_co_badged_networks_by_fee<F: Clone, D: OperationSessionGetters<F>>(
    state: &SessionState,
    payment_data: &D,
    acquirer_country: enums::CountryAlpha2,
) -> Option<Vec<enums::CardNetwork>> {
    logger::debug!("Fetching sorted card networks based on their respective network fees");

    let payment_method_data_optional = payment_data.get_payment_method_data();
    let payment_attempt = payment_data.get_payment_attempt();

    let co_badged_card_request = api_models::open_router::CoBadgedCardRequest {
        merchant_category_code: enums::MerchantCategoryCode::Mcc0001,
        acquirer_country,
        // we need to populate this in case of save card flows
        // this should we populated by checking for the payment method info
        // payment method info is some then we will have to send the card isin as null
        co_badged_card_data: None,
    };

    if let Some(hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(card)) =
        payment_method_data_optional
    {
        // perform_open_routing_for_debit_routing
        let debit_routing_output_optional = routing::perform_open_routing_for_debit_routing(
            state,
            payment_attempt,
            co_badged_card_request,
            Some(Secret::new(card.card_number.get_card_isin())),
        )
        .await
        .map_err(|error| {
            logger::warn!(?error, "Failed to calculate total fees per network");
        })
        .ok()
        .map(|data| data.co_badged_card_networks);

        return debit_routing_output_optional;
    }
    None
}

async fn handle_retryable_connector<F, D>(
    state: &SessionState,
    debit_routing_config: &settings::DebitRoutingConfig,
    debit_routing_supported_connectors: HashSet<api_enums::Connector>,
    connector_data_list: Vec<api::ConnectorRoutingData>,
    payment_data: &D,
    acquirer_country: enums::CountryAlpha2,
) -> Option<ConnectorCallType>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let mut supported_connectors = Vec::new();

    let is_any_connector_supported = connector_data_list.iter().any(|connector_data| {
        debit_routing_supported_connectors.contains(&connector_data.connector_data.connector_name)
    });

    if is_any_connector_supported {
        let fee_sorted_debit_networks =
            get_sorted_co_badged_networks_by_fee::<F, D>(state, payment_data, acquirer_country)
                .await?;

        for connector_data in connector_data_list {
            for network in &fee_sorted_debit_networks {
                if let Some(valid_network) = check_connector_support_for_network(
                    debit_routing_config,
                    connector_data.connector_data.connector_name,
                    network,
                ) {
                    supported_connectors.push(api::ConnectorRoutingData {
                        connector_data: connector_data.connector_data.clone(),
                        network: Some(valid_network),
                    });
                }
            }
        }
    }

    if !supported_connectors.is_empty() {
        Some(ConnectorCallType::Retryable(supported_connectors))
    } else {
        None
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
