use std::sync::Arc;

use api_models::{
    payment_methods::SurchargeDetailsResponse,
    payments, routing,
    surcharge_decision_configs::{self, SurchargeDecisionConfigs, SurchargeDecisionManagerRecord},
};
use common_utils::{ext_traits::StringExt, static_cache::StaticCache, types as common_utils_types};
use error_stack::{self, IntoReport, ResultExt};
use euclid::{
    backend,
    backend::{inputs as dsl_inputs, EuclidBackend},
};
use router_env::{instrument, tracing};

use crate::{
    core::payments::{types, PaymentData},
    db::StorageInterface,
    types::{
        storage::{self as oss_storage, payment_attempt::PaymentAttemptExt},
        transformers::ForeignTryFrom,
    },
};
static CONF_CACHE: StaticCache<VirInterpreterBackendCacheWrapper> = StaticCache::new();
use crate::{
    core::{
        errors::ConditionalConfigError as ConfigError,
        payments::{
            conditional_configs::ConditionalConfigResult, routing::make_dsl_input_for_surcharge,
        },
    },
    AppState,
};

struct VirInterpreterBackendCacheWrapper {
    cached_alogorith: backend::VirInterpreterBackend<SurchargeDecisionConfigs>,
    merchant_surcharge_configs: surcharge_decision_configs::MerchantSurchargeConfigs,
}

impl TryFrom<SurchargeDecisionManagerRecord> for VirInterpreterBackendCacheWrapper {
    type Error = error_stack::Report<ConfigError>;

    fn try_from(value: SurchargeDecisionManagerRecord) -> Result<Self, Self::Error> {
        let cached_alogorith = backend::VirInterpreterBackend::with_program(value.algorithm)
            .into_report()
            .change_context(ConfigError::DslBackendInitError)
            .attach_printable("Error initializing DSL interpreter backend")?;
        let merchant_surcharge_configs = value.merchant_surcharge_configs;
        Ok(Self {
            cached_alogorith,
            merchant_surcharge_configs,
        })
    }
}

enum SurchargeSource {
    /// Surcharge will be generated through the surcharge rules
    Generate(Arc<VirInterpreterBackendCacheWrapper>),
    /// Surcharge is predefined by the merchant through payment create request
    Predetermined(payments::RequestSurchargeDetails),
}

impl SurchargeSource {
    pub fn generate_surcharge_details_and_populate_surcharge_metadata(
        &self,
        backend_input: &backend::BackendInput,
        payment_attempt: &oss_storage::PaymentAttempt,
        surcharge_metadata_and_key: (&mut types::SurchargeMetadata, types::SurchargeKey),
    ) -> ConditionalConfigResult<Option<types::SurchargeDetails>> {
        match self {
            Self::Generate(interpreter) => {
                let surcharge_output = execute_dsl_and_get_conditional_config(
                    backend_input.clone(),
                    &interpreter.cached_alogorith,
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
                    .map(|surcharge_details| {
                        let (surcharge_metadata, surcharge_key) = surcharge_metadata_and_key;
                        surcharge_metadata
                            .insert_surcharge_details(surcharge_key, surcharge_details.clone());
                        surcharge_details
                    }))
            }
            Self::Predetermined(request_surcharge_details) => Ok(Some(
                types::SurchargeDetails::from((request_surcharge_details, payment_attempt)),
            )),
        }
    }
}

pub async fn perform_surcharge_decision_management_for_payment_method_list(
    state: &AppState,
    algorithm_ref: routing::RoutingAlgorithmRef,
    payment_attempt: &oss_storage::PaymentAttempt,
    payment_intent: &oss_storage::PaymentIntent,
    billing_address: Option<payments::Address>,
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
            let key = ensure_algorithm_cached(
                &*state.store,
                &payment_attempt.merchant_id,
                algorithm_ref.timestamp,
                algorithm_id.as_str(),
            )
            .await?;
            let cached_algo = CONF_CACHE
                .retrieve(&key)
                .into_report()
                .change_context(ConfigError::CacheMiss)
                .attach_printable(
                    "Unable to retrieve cached routing algorithm even after refresh",
                )?;
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
                            .into_report()
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
                        .into_report()
                        .change_context(ConfigError::DslExecutionError)
                        .attach_printable("Error while constructing Surcharge response type")
                    })
                    .transpose()?;
            }
        }
    }
    Ok((surcharge_metadata, merchant_surcharge_configs))
}

pub async fn perform_surcharge_decision_management_for_session_flow<O>(
    state: &AppState,
    algorithm_ref: routing::RoutingAlgorithmRef,
    payment_data: &mut PaymentData<O>,
    payment_method_type_list: &Vec<common_enums::PaymentMethodType>,
) -> ConditionalConfigResult<types::SurchargeMetadata>
where
    O: Send + Clone,
{
    let mut surcharge_metadata =
        types::SurchargeMetadata::new(payment_data.payment_attempt.attempt_id.clone());
    let surcharge_source = match (
        payment_data.payment_attempt.get_surcharge_details(),
        algorithm_ref.surcharge_config_algo_id,
    ) {
        (Some(request_surcharge_details), _) => {
            SurchargeSource::Predetermined(request_surcharge_details)
        }
        (None, Some(algorithm_id)) => {
            let key = ensure_algorithm_cached(
                &*state.store,
                &payment_data.payment_attempt.merchant_id,
                algorithm_ref.timestamp,
                algorithm_id.as_str(),
            )
            .await?;
            let cached_algo = CONF_CACHE
                .retrieve(&key)
                .into_report()
                .change_context(ConfigError::CacheMiss)
                .attach_printable(
                    "Unable to retrieve cached routing algorithm even after refresh",
                )?;
            SurchargeSource::Generate(cached_algo)
        }
        (None, None) => return Ok(surcharge_metadata),
    };
    let mut backend_input = make_dsl_input_for_surcharge(
        &payment_data.payment_attempt,
        &payment_data.payment_intent,
        payment_data.address.billing.clone(),
    )
    .change_context(ConfigError::InputConstructionError)?;
    for payment_method_type in payment_method_type_list {
        backend_input.payment_method.payment_method_type = Some(*payment_method_type);
        // in case of session flow, payment_method will always be wallet
        backend_input.payment_method.payment_method = Some(payment_method_type.to_owned().into());
        surcharge_source.generate_surcharge_details_and_populate_surcharge_metadata(
            &backend_input,
            &payment_data.payment_attempt,
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
pub async fn perform_surcharge_decision_management_for_saved_cards(
    state: &AppState,
    algorithm_ref: routing::RoutingAlgorithmRef,
    payment_attempt: &oss_storage::PaymentAttempt,
    payment_intent: &oss_storage::PaymentIntent,
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
            let key = ensure_algorithm_cached(
                &*state.store,
                &payment_attempt.merchant_id,
                algorithm_ref.timestamp,
                algorithm_id.as_str(),
            )
            .await?;
            let cached_algo = CONF_CACHE
                .retrieve(&key)
                .into_report()
                .change_context(ConfigError::CacheMiss)
                .attach_printable(
                    "Unable to retrieve cached routing algorithm even after refresh",
                )?;
            SurchargeSource::Generate(cached_algo)
        }
        (None, None) => return Ok(surcharge_metadata),
    };
    let mut backend_input = make_dsl_input_for_surcharge(payment_attempt, payment_intent, None)
        .change_context(ConfigError::InputConstructionError)?;

    for customer_payment_method in customer_payment_method_list.iter_mut() {
        backend_input.payment_method.payment_method = Some(customer_payment_method.payment_method);
        backend_input.payment_method.payment_method_type =
            customer_payment_method.payment_method_type;
        backend_input.payment_method.card_network = customer_payment_method
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
        let surcharge_details = surcharge_source
            .generate_surcharge_details_and_populate_surcharge_metadata(
                &backend_input,
                payment_attempt,
                (
                    &mut surcharge_metadata,
                    types::SurchargeKey::Token(customer_payment_method.payment_token.clone()),
                ),
            )?;
        customer_payment_method.surcharge_details = surcharge_details
            .map(|surcharge_details| {
                SurchargeDetailsResponse::foreign_try_from((&surcharge_details, payment_attempt))
                    .into_report()
                    .change_context(ConfigError::DslParsingError)
            })
            .transpose()?;
    }
    Ok(surcharge_metadata)
}

fn get_surcharge_details_from_surcharge_output(
    surcharge_details: surcharge_decision_configs::SurchargeDetailsOutput,
    payment_attempt: &oss_storage::PaymentAttempt,
) -> ConditionalConfigResult<types::SurchargeDetails> {
    let surcharge_amount = match surcharge_details.surcharge.clone() {
        surcharge_decision_configs::SurchargeOutput::Fixed { amount } => amount,
        surcharge_decision_configs::SurchargeOutput::Rate(percentage) => percentage
            .apply_and_ceil_result(payment_attempt.amount.get_original_amount())
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
        .unwrap_or(0);
    let original_amount = payment_attempt.amount.get_original_amount();
    Ok(types::SurchargeDetails {
        original_amount,
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
        final_amount: original_amount + surcharge_amount + tax_on_surcharge_amount,
    })
}

#[instrument(skip_all)]
pub async fn ensure_algorithm_cached(
    store: &dyn StorageInterface,
    merchant_id: &str,
    timestamp: i64,
    algorithm_id: &str,
) -> ConditionalConfigResult<String> {
    let key = format!("surcharge_dsl_{merchant_id}");
    let present = CONF_CACHE
        .present(&key)
        .into_report()
        .change_context(ConfigError::DslCachePoisoned)
        .attach_printable("Error checking presence of DSL")?;
    let expired = CONF_CACHE
        .expired(&key, timestamp)
        .into_report()
        .change_context(ConfigError::DslCachePoisoned)
        .attach_printable("Error checking presence of DSL")?;

    if !present || expired {
        refresh_surcharge_algorithm_cache(store, key.clone(), algorithm_id, timestamp).await?
    }
    Ok(key)
}

#[instrument(skip_all)]
pub async fn refresh_surcharge_algorithm_cache(
    store: &dyn StorageInterface,
    key: String,
    algorithm_id: &str,
    timestamp: i64,
) -> ConditionalConfigResult<()> {
    let config = store
        .find_config_by_key(algorithm_id)
        .await
        .change_context(ConfigError::DslMissingInDb)
        .attach_printable("Error parsing DSL from config")?;
    let record: SurchargeDecisionManagerRecord = config
        .config
        .parse_struct("Program")
        .change_context(ConfigError::DslParsingError)
        .attach_printable("Error parsing routing algorithm from configs")?;
    let value_to_cache = VirInterpreterBackendCacheWrapper::try_from(record)?;
    CONF_CACHE
        .save(key, value_to_cache, timestamp)
        .into_report()
        .change_context(ConfigError::DslCachePoisoned)
        .attach_printable("Error saving DSL to cache")?;
    Ok(())
}

pub fn execute_dsl_and_get_conditional_config(
    backend_input: dsl_inputs::BackendInput,
    interpreter: &backend::VirInterpreterBackend<SurchargeDecisionConfigs>,
) -> ConditionalConfigResult<SurchargeDecisionConfigs> {
    let routing_output = interpreter
        .execute(backend_input)
        .map(|out| out.connector_selection)
        .into_report()
        .change_context(ConfigError::DslExecutionError)?;
    Ok(routing_output)
}
