use api_models::{
    payment_methods::SurchargeDetailsResponse,
    payments, routing,
    surcharge_decision_configs::{self, SurchargeDecisionConfigs, SurchargeDecisionManagerRecord},
};
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use common_utils::{ext_traits::StringExt, types as common_utils_types};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use common_utils::{
    ext_traits::{OptionExt, StringExt},
    types as common_utils_types,
};
use error_stack::{self, ResultExt};
use euclid::{
    backend,
    backend::{inputs as dsl_inputs, EuclidBackend},
};
use router_env::{instrument, logger, tracing};
use serde::{Deserialize, Serialize};
use storage_impl::redis::cache::{self, SURCHARGE_CACHE};

use crate::{
    core::{
        errors::{self, ConditionalConfigError as ConfigError},
        payments::{
            conditional_configs::ConditionalConfigResult, routing::make_dsl_input_for_surcharge,
            types,
        },
    },
    db::StorageInterface,
    types::{
        storage::{self, payment_attempt::PaymentAttemptExt},
        transformers::ForeignTryFrom,
    },
    SessionState,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VirInterpreterBackendCacheWrapper {
    cached_algorithm: backend::VirInterpreterBackend<SurchargeDecisionConfigs>,
    merchant_surcharge_configs: surcharge_decision_configs::MerchantSurchargeConfigs,
}

impl TryFrom<SurchargeDecisionManagerRecord> for VirInterpreterBackendCacheWrapper {
    type Error = error_stack::Report<ConfigError>;

    fn try_from(value: SurchargeDecisionManagerRecord) -> Result<Self, Self::Error> {
        let cached_algorithm = backend::VirInterpreterBackend::with_program(value.algorithm)
            .change_context(ConfigError::DslBackendInitError)
            .attach_printable("Error initializing DSL interpreter backend")?;
        let merchant_surcharge_configs = value.merchant_surcharge_configs;
        Ok(Self {
            cached_algorithm,
            merchant_surcharge_configs,
        })
    }
}

enum SurchargeSource {
    /// Surcharge will be generated through the surcharge rules
    Generate(VirInterpreterBackendCacheWrapper),
    /// Surcharge is predefined by the merchant through payment create request
    Predetermined(payments::RequestSurchargeDetails),
}

impl SurchargeSource {
    pub fn generate_surcharge_details_and_populate_surcharge_metadata(
        &self,
        backend_input: &backend::BackendInput,
        payment_attempt: &storage::PaymentAttempt,
        surcharge_metadata_and_key: (&mut types::SurchargeMetadata, types::SurchargeKey),
    ) -> ConditionalConfigResult<Option<types::SurchargeDetails>> {
        match self {
            Self::Generate(interpreter) => {
                let surcharge_output = execute_dsl_and_get_conditional_config(
                    backend_input.clone(),
                    &interpreter.cached_algorithm,
                )?;
                Ok(surcharge_output
                    .surcharge_details
                    .map(|surcharge_details| {
                        get_surcharge_details_from_surcharge_output(
                            surcharge_details,
                            payment_attempt,
                        )
                    })
                    .transpose()?
                    .inspect(|surcharge_details| {
                        let (surcharge_metadata, surcharge_key) = surcharge_metadata_and_key;
                        surcharge_metadata
                            .insert_surcharge_details(surcharge_key, surcharge_details.clone());
                    }))
            }
            Self::Predetermined(request_surcharge_details) => Ok(Some(
                types::SurchargeDetails::from((request_surcharge_details, payment_attempt)),
            )),
        }
    }
}

#[cfg(feature = "v2")]
pub async fn perform_surcharge_decision_management_for_payment_method_list(
    _state: &SessionState,
    _algorithm_ref: routing::RoutingAlgorithmRef,
    _payment_attempt: &storage::PaymentAttempt,
    _payment_intent: &storage::PaymentIntent,
    _billing_address: Option<payments::Address>,
    _response_payment_method_types: &mut [api_models::payment_methods::ResponsePaymentMethodsEnabled],
) -> ConditionalConfigResult<(
    types::SurchargeMetadata,
    surcharge_decision_configs::MerchantSurchargeConfigs,
)> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn perform_surcharge_decision_management_for_payment_method_list(
    state: &SessionState,
    algorithm_ref: routing::RoutingAlgorithmRef,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    billing_address: Option<hyperswitch_domain_models::address::Address>,
    response_payment_method_types: &mut [api_models::payment_methods::ResponsePaymentMethodsEnabled],
) -> ConditionalConfigResult<(
    types::SurchargeMetadata,
    surcharge_decision_configs::MerchantSurchargeConfigs,
)> {
    let mut surcharge_metadata = types::SurchargeMetadata::new(payment_attempt.attempt_id.clone());

    let (surcharge_source, merchant_surcharge_configs) = match (
        payment_attempt.get_surcharge_details(),
        algorithm_ref.surcharge_config_algo_id,
    ) {
        (Some(request_surcharge_details), _) => (
            SurchargeSource::Predetermined(request_surcharge_details),
            surcharge_decision_configs::MerchantSurchargeConfigs::default(),
        ),
        (None, Some(algorithm_id)) => {
            let cached_algo = ensure_algorithm_cached(
                &*state.store,
                &payment_attempt.merchant_id,
                algorithm_id.as_str(),
            )
            .await?;

            let merchant_surcharge_config = cached_algo.merchant_surcharge_configs.clone();
            (
                SurchargeSource::Generate(cached_algo),
                merchant_surcharge_config,
            )
        }
        (None, None) => {
            return Ok((
                surcharge_metadata,
                surcharge_decision_configs::MerchantSurchargeConfigs::default(),
            ))
        }
    };
    let surcharge_source_log_message = match &surcharge_source {
        SurchargeSource::Generate(_) => "Surcharge was calculated through surcharge rules",
        SurchargeSource::Predetermined(_) => "Surcharge was sent in payment create request",
    };
    logger::debug!(payment_method_list_surcharge_source = surcharge_source_log_message);

    let mut backend_input =
        make_dsl_input_for_surcharge(payment_attempt, payment_intent, billing_address)
            .change_context(ConfigError::InputConstructionError)?;

    for payment_methods_enabled in response_payment_method_types.iter_mut() {
        for payment_method_type_response in
            &mut payment_methods_enabled.payment_method_types.iter_mut()
        {
            let payment_method_type = payment_method_type_response.payment_method_type;
            backend_input.payment_method.payment_method_type = Some(payment_method_type);
            backend_input.payment_method.payment_method =
                Some(payment_methods_enabled.payment_method);

            if let Some(card_network_list) = &mut payment_method_type_response.card_networks {
                for card_network_type in card_network_list.iter_mut() {
                    backend_input.payment_method.card_network =
                        Some(card_network_type.card_network.clone());
                    let surcharge_details = surcharge_source
                        .generate_surcharge_details_and_populate_surcharge_metadata(
                            &backend_input,
                            payment_attempt,
                            (
                                &mut surcharge_metadata,
                                types::SurchargeKey::PaymentMethodData(
                                    payment_methods_enabled.payment_method,
                                    payment_method_type_response.payment_method_type,
                                    Some(card_network_type.card_network.clone()),
                                ),
                            ),
                        )?;
                    card_network_type.surcharge_details = surcharge_details
                        .map(|surcharge_details| {
                            SurchargeDetailsResponse::foreign_try_from((
                                &surcharge_details,
                                payment_attempt,
                            ))
                            .change_context(ConfigError::DslExecutionError)
                            .attach_printable("Error while constructing Surcharge response type")
                        })
                        .transpose()?;
                }
            } else {
                let surcharge_details = surcharge_source
                    .generate_surcharge_details_and_populate_surcharge_metadata(
                        &backend_input,
                        payment_attempt,
                        (
                            &mut surcharge_metadata,
                            types::SurchargeKey::PaymentMethodData(
                                payment_methods_enabled.payment_method,
                                payment_method_type_response.payment_method_type,
                                None,
                            ),
                        ),
                    )?;
                payment_method_type_response.surcharge_details = surcharge_details
                    .map(|surcharge_details| {
                        SurchargeDetailsResponse::foreign_try_from((
                            &surcharge_details,
                            payment_attempt,
                        ))
                        .change_context(ConfigError::DslExecutionError)
                        .attach_printable("Error while constructing Surcharge response type")
                    })
                    .transpose()?;
            }
        }
    }
    Ok((surcharge_metadata, merchant_surcharge_configs))
}

#[cfg(feature = "v1")]
pub async fn perform_surcharge_decision_management_for_session_flow(
    state: &SessionState,
    algorithm_ref: routing::RoutingAlgorithmRef,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    billing_address: Option<hyperswitch_domain_models::address::Address>,
    payment_method_type_list: &Vec<common_enums::PaymentMethodType>,
) -> ConditionalConfigResult<types::SurchargeMetadata> {
    let mut surcharge_metadata = types::SurchargeMetadata::new(payment_attempt.attempt_id.clone());
    let surcharge_source = match (
        payment_attempt.get_surcharge_details(),
        algorithm_ref.surcharge_config_algo_id,
    ) {
        (Some(request_surcharge_details), _) => {
            SurchargeSource::Predetermined(request_surcharge_details)
        }
        (None, Some(algorithm_id)) => {
            let cached_algo = ensure_algorithm_cached(
                &*state.store,
                &payment_attempt.merchant_id,
                algorithm_id.as_str(),
            )
            .await?;

            SurchargeSource::Generate(cached_algo)
        }
        (None, None) => return Ok(surcharge_metadata),
    };
    let mut backend_input =
        make_dsl_input_for_surcharge(payment_attempt, payment_intent, billing_address)
            .change_context(ConfigError::InputConstructionError)?;
    for payment_method_type in payment_method_type_list {
        backend_input.payment_method.payment_method_type = Some(*payment_method_type);
        // in case of session flow, payment_method will always be wallet
        backend_input.payment_method.payment_method = Some(payment_method_type.to_owned().into());
        surcharge_source.generate_surcharge_details_and_populate_surcharge_metadata(
            &backend_input,
            payment_attempt,
            (
                &mut surcharge_metadata,
                types::SurchargeKey::PaymentMethodData(
                    payment_method_type.to_owned().into(),
                    *payment_method_type,
                    None,
                ),
            ),
        )?;
    }
    Ok(surcharge_metadata)
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub async fn perform_surcharge_decision_management_for_saved_cards(
    state: &SessionState,
    algorithm_ref: routing::RoutingAlgorithmRef,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    customer_payment_method_list: &mut [api_models::payment_methods::CustomerPaymentMethod],
) -> ConditionalConfigResult<types::SurchargeMetadata> {
    let mut surcharge_metadata = types::SurchargeMetadata::new(payment_attempt.attempt_id.clone());
    let surcharge_source = match (
        payment_attempt.get_surcharge_details(),
        algorithm_ref.surcharge_config_algo_id,
    ) {
        (Some(request_surcharge_details), _) => {
            SurchargeSource::Predetermined(request_surcharge_details)
        }
        (None, Some(algorithm_id)) => {
            let cached_algo = ensure_algorithm_cached(
                &*state.store,
                &payment_attempt.merchant_id,
                algorithm_id.as_str(),
            )
            .await?;

            SurchargeSource::Generate(cached_algo)
        }
        (None, None) => return Ok(surcharge_metadata),
    };
    let surcharge_source_log_message = match &surcharge_source {
        SurchargeSource::Generate(_) => "Surcharge was calculated through surcharge rules",
        SurchargeSource::Predetermined(_) => "Surcharge was sent in payment create request",
    };
    logger::debug!(customer_saved_card_list_surcharge_source = surcharge_source_log_message);
    let mut backend_input = make_dsl_input_for_surcharge(payment_attempt, payment_intent, None)
        .change_context(ConfigError::InputConstructionError)?;

    for customer_payment_method in customer_payment_method_list.iter_mut() {
        let payment_token = customer_payment_method.payment_token.clone();

        backend_input.payment_method.payment_method = Some(customer_payment_method.payment_method);
        backend_input.payment_method.payment_method_type =
            customer_payment_method.payment_method_type;

        let card_network = customer_payment_method
            .card
            .as_ref()
            .and_then(|card| card.scheme.as_ref())
            .map(|scheme| {
                scheme
                    .clone()
                    .parse_enum("CardNetwork")
                    .change_context(ConfigError::DslExecutionError)
            })
            .transpose()?;

        backend_input.payment_method.card_network = card_network;

        let surcharge_details = surcharge_source
            .generate_surcharge_details_and_populate_surcharge_metadata(
                &backend_input,
                payment_attempt,
                (
                    &mut surcharge_metadata,
                    types::SurchargeKey::Token(payment_token),
                ),
            )?;
        customer_payment_method.surcharge_details = surcharge_details
            .map(|surcharge_details| {
                SurchargeDetailsResponse::foreign_try_from((&surcharge_details, payment_attempt))
                    .change_context(ConfigError::DslParsingError)
            })
            .transpose()?;
    }
    Ok(surcharge_metadata)
}

// TODO: uncomment and resolve compiler error when required
// #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
// pub async fn perform_surcharge_decision_management_for_saved_cards(
//     state: &SessionState,
//     algorithm_ref: routing::RoutingAlgorithmRef,
//     payment_attempt: &storage::PaymentAttempt,
//     payment_intent: &storage::PaymentIntent,
//     customer_payment_method_list: &mut [api_models::payment_methods::CustomerPaymentMethod],
// ) -> ConditionalConfigResult<types::SurchargeMetadata> {
//     // let mut surcharge_metadata = types::SurchargeMetadata::new(payment_attempt.id.clone());
//     let mut surcharge_metadata = todo!();

//     let surcharge_source = match (
//         payment_attempt.get_surcharge_details(),
//         algorithm_ref.surcharge_config_algo_id,
//     ) {
//         (Some(request_surcharge_details), _) => {
//             SurchargeSource::Predetermined(request_surcharge_details)
//         }
//         (None, Some(algorithm_id)) => {
//             let cached_algo = ensure_algorithm_cached(
//                 &*state.store,
//                 &payment_attempt.merchant_id,
//                 algorithm_id.as_str(),
//             )
//             .await?;

//             SurchargeSource::Generate(cached_algo)
//         }
//         (None, None) => return Ok(surcharge_metadata),
//     };
//     let surcharge_source_log_message = match &surcharge_source {
//         SurchargeSource::Generate(_) => "Surcharge was calculated through surcharge rules",
//         SurchargeSource::Predetermined(_) => "Surcharge was sent in payment create request",
//     };
//     logger::debug!(customer_saved_card_list_surcharge_source = surcharge_source_log_message);
//     let mut backend_input = make_dsl_input_for_surcharge(payment_attempt, payment_intent, None)
//         .change_context(ConfigError::InputConstructionError)?;

//     for customer_payment_method in customer_payment_method_list.iter_mut() {
//         let payment_token = customer_payment_method
//             .payment_token
//             .clone()
//             .get_required_value("payment_token")
//             .change_context(ConfigError::InputConstructionError)?;

//         backend_input.payment_method.payment_method =
//             Some(customer_payment_method.payment_method_type);
//         backend_input.payment_method.payment_method_type =
//             customer_payment_method.payment_method_subtype;

//         let card_network = match customer_payment_method.payment_method_data.as_ref() {
//             Some(api_models::payment_methods::PaymentMethodListData::Card(card)) => {
//                 card.card_network.clone()
//             }
//             _ => None,
//         };
//         backend_input.payment_method.card_network = card_network;

//         let surcharge_details = surcharge_source
//             .generate_surcharge_details_and_populate_surcharge_metadata(
//                 &backend_input,
//                 payment_attempt,
//                 (
//                     &mut surcharge_metadata,
//                     types::SurchargeKey::Token(payment_token),
//                 ),
//             )?;
//         customer_payment_method.surcharge_details = surcharge_details
//             .map(|surcharge_details| {
//                 SurchargeDetailsResponse::foreign_try_from((&surcharge_details, payment_attempt))
//                     .change_context(ConfigError::DslParsingError)
//             })
//             .transpose()?;
//     }
//     Ok(surcharge_metadata)
// }

#[cfg(feature = "v2")]
fn get_surcharge_details_from_surcharge_output(
    _surcharge_details: surcharge_decision_configs::SurchargeDetailsOutput,
    _payment_attempt: &storage::PaymentAttempt,
) -> ConditionalConfigResult<types::SurchargeDetails> {
    todo!()
}

#[cfg(feature = "v1")]
fn get_surcharge_details_from_surcharge_output(
    surcharge_details: surcharge_decision_configs::SurchargeDetailsOutput,
    payment_attempt: &storage::PaymentAttempt,
) -> ConditionalConfigResult<types::SurchargeDetails> {
    let surcharge_amount = match surcharge_details.surcharge.clone() {
        surcharge_decision_configs::SurchargeOutput::Fixed { amount } => amount,
        surcharge_decision_configs::SurchargeOutput::Rate(percentage) => percentage
            .apply_and_ceil_result(payment_attempt.net_amount.get_total_amount())
            .change_context(ConfigError::DslExecutionError)
            .attach_printable("Failed to Calculate surcharge amount by applying percentage")?,
    };
    let tax_on_surcharge_amount = surcharge_details
        .tax_on_surcharge
        .clone()
        .map(|tax_on_surcharge| {
            tax_on_surcharge
                .apply_and_ceil_result(surcharge_amount)
                .change_context(ConfigError::DslExecutionError)
                .attach_printable("Failed to Calculate tax amount")
        })
        .transpose()?
        .unwrap_or_default();
    Ok(types::SurchargeDetails {
        original_amount: payment_attempt.net_amount.get_order_amount(),
        surcharge: match surcharge_details.surcharge {
            surcharge_decision_configs::SurchargeOutput::Fixed { amount } => {
                common_utils_types::Surcharge::Fixed(amount)
            }
            surcharge_decision_configs::SurchargeOutput::Rate(percentage) => {
                common_utils_types::Surcharge::Rate(percentage)
            }
        },
        tax_on_surcharge: surcharge_details.tax_on_surcharge,
        surcharge_amount,
        tax_on_surcharge_amount,
    })
}

#[instrument(skip_all)]
pub async fn ensure_algorithm_cached(
    store: &dyn StorageInterface,
    merchant_id: &common_utils::id_type::MerchantId,
    algorithm_id: &str,
) -> ConditionalConfigResult<VirInterpreterBackendCacheWrapper> {
    let key = merchant_id.get_surcharge_dsk_key();

    let value_to_cache = || async {
        let config: diesel_models::Config = store.find_config_by_key(algorithm_id).await?;
        let record: SurchargeDecisionManagerRecord = config
            .config
            .parse_struct("Program")
            .change_context(errors::StorageError::DeserializationFailed)
            .attach_printable("Error parsing routing algorithm from configs")?;
        VirInterpreterBackendCacheWrapper::try_from(record)
            .change_context(errors::StorageError::ValueNotFound("Program".to_string()))
            .attach_printable("Error initializing DSL interpreter backend")
    };
    let interpreter = cache::get_or_populate_in_memory(
        store.get_cache_store().as_ref(),
        &key,
        value_to_cache,
        &SURCHARGE_CACHE,
    )
    .await
    .change_context(ConfigError::CacheMiss)
    .attach_printable("Unable to retrieve cached routing algorithm even after refresh")?;
    Ok(interpreter)
}

pub fn execute_dsl_and_get_conditional_config(
    backend_input: dsl_inputs::BackendInput,
    interpreter: &backend::VirInterpreterBackend<SurchargeDecisionConfigs>,
) -> ConditionalConfigResult<SurchargeDecisionConfigs> {
    let routing_output = interpreter
        .execute(backend_input)
        .map(|out| out.connector_selection)
        .change_context(ConfigError::DslExecutionError)?;
    Ok(routing_output)
}
