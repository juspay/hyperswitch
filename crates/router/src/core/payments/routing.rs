mod transformers;

use std::{
    collections::hash_map,
    hash::{Hash, Hasher},
    sync::Arc,
};

use api_models::{
    admin as admin_api,
    enums::{self as api_enums, CountryAlpha2},
    payments::Address,
    routing::ConnectorSelection,
};
use common_utils::static_cache::StaticCache;
use diesel_models::enums as storage_enums;
use error_stack::{IntoReport, ResultExt};
use euclid::{
    backend::{self, inputs as dsl_inputs, EuclidBackend},
    dssa::graph::{self as euclid_graph, Memoization},
    enums as euclid_enums,
    frontend::ast,
};
use kgraph_utils::{
    mca as mca_graph,
    transformers::{IntoContext, IntoDirValue},
};
use masking::PeekInterface;
use rand::{
    distributions::{self, Distribution},
    SeedableRng,
};
use rustc_hash::FxHashMap;

#[cfg(not(feature = "business_profile_routing"))]
use crate::utils::StringExt;
use crate::{
    core::{
        errors as oss_errors, errors, payments as payments_oss, routing::helpers as routing_helpers,
    },
    logger,
    types::{
        api, api::routing as routing_types, domain, storage as oss_storage,
        transformers::ForeignInto,
    },
    utils::{OptionExt, ValueExt},
    AppState,
};

pub(super) enum CachedAlgorithm {
    Single(Box<routing_types::RoutableConnectorChoice>),
    Priority(Vec<routing_types::RoutableConnectorChoice>),
    VolumeSplit(Vec<routing_types::ConnectorVolumeSplit>),
    Advanced(backend::VirInterpreterBackend<ConnectorSelection>),
}

pub struct SessionFlowRoutingInput<'a> {
    pub state: &'a AppState,
    pub country: Option<CountryAlpha2>,
    pub key_store: &'a domain::MerchantKeyStore,
    pub merchant_account: &'a domain::MerchantAccount,
    pub payment_attempt: &'a oss_storage::PaymentAttempt,
    pub payment_intent: &'a oss_storage::PaymentIntent,
    pub chosen: Vec<api::SessionConnectorData>,
}

pub struct SessionRoutingPmTypeInput<'a> {
    state: &'a AppState,
    key_store: &'a domain::MerchantKeyStore,
    merchant_last_modified: i64,
    attempt_id: &'a str,
    routing_algorithm: &'a MerchantAccountRoutingAlgorithm,
    backend_input: dsl_inputs::BackendInput,
    allowed_connectors: FxHashMap<String, api::GetToken>,
    #[cfg(any(
        feature = "business_profile_routing",
        feature = "profile_specific_fallback_routing"
    ))]
    profile_id: Option<String>,
}
static ROUTING_CACHE: StaticCache<CachedAlgorithm> = StaticCache::new();
static KGRAPH_CACHE: StaticCache<euclid_graph::KnowledgeGraph<'_>> = StaticCache::new();

type RoutingResult<O> = oss_errors::CustomResult<O, errors::RoutingError>;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum MerchantAccountRoutingAlgorithm {
    V1(routing_types::RoutingAlgorithmRef),
}

impl Default for MerchantAccountRoutingAlgorithm {
    /// Returns a new instance of the current type with the default value, which is a `RoutingAlgorithmRef` initialized with its default value.
    fn default() -> Self {
        Self::V1(routing_types::RoutingAlgorithmRef::default())
    }
}

/// This function takes a payment data and constructs a DSL input from it, which includes mandate data, payment method input, payment input, and metadata. It then returns a `RoutingResult` containing the constructed `BackendInput`.
pub fn make_dsl_input<F>(
    payment_data: &payments_oss::PaymentData<F>,
) -> RoutingResult<dsl_inputs::BackendInput>
where
    F: Clone,
{
    // implementation details...
}

/// Asynchronously performs static routing for a given merchant using the specified routing algorithm reference and payment data, and returns a vector of routable connector choices as a result. If the algorithm reference contains an algorithm ID, the method ensures that the cached algorithm is retrieved and used for routing. If the algorithm reference does not contain an algorithm ID, the method attempts to retrieve the merchant's default configuration and returns it as a fallback. The method handles different types of cached algorithms (single, priority, volume split, and advanced) and executes the appropriate routing logic based on the cached algorithm type and the input payment data.
pub async fn perform_static_routing_v1<F: Clone>(
    state: &AppState,
    merchant_id: &str,
    algorithm_ref: routing_types::RoutingAlgorithmRef,
    payment_data: &mut payments_oss::PaymentData<F>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    // method implementation
}

/// Ensure that the routing algorithm result is cached and up to date, and refresh the cache if necessary.
async fn ensure_algorithm_cached_v1(
    state: &AppState,
    merchant_id: &str,
    timestamp: i64,
    algorithm_id: &str,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<String> {
    // ... (method implementation)
}

/// Perform straight-through routing based on the specified algorithm and payment data.
pub fn perform_straight_through_routing<F: Clone>(
    algorithm: &routing_types::StraightThroughAlgorithm,
    payment_data: &payments_oss::PaymentData<F>,
) -> RoutingResult<(Vec<routing_types::RoutableConnectorChoice>, bool)> {
    Ok(match algorithm {
        routing_types::StraightThroughAlgorithm::Single(conn) => (
            vec![(**conn).clone()],
            payment_data.creds_identifier.is_none(),
        ),

        routing_types::StraightThroughAlgorithm::Priority(conns) => (conns.clone(), true),

        routing_types::StraightThroughAlgorithm::VolumeSplit(splits) => (
            perform_volume_split(splits.to_vec(), None)
                .change_context(errors::RoutingError::ConnectorSelectionFailed)
                .attach_printable(
                    "Volume Split connector selection error in straight through routing",
                )?,
            true,
        ),
    })
}

/// Executes a DSL input using a VirInterpreterBackend and returns a vector of RoutableConnectorChoice.
///
/// # Arguments
///
/// * `backend_input` - A dsl_inputs::BackendInput containing the input for the DSL execution.
/// * `interpreter` - A reference to a backend::VirInterpreterBackend<ConnectorSelection> used to execute the DSL.
///
/// # Returns
///
/// Returns a Result containing a vector of routing_types::RoutableConnectorChoice or an error of type errors::RoutingError.
fn execute_dsl_and_get_connector_v1(
    backend_input: dsl_inputs::BackendInput,
    interpreter: &backend::VirInterpreterBackend<ConnectorSelection>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let routing_output: routing_types::RoutingAlgorithm = interpreter
        .execute(backend_input)
        .map(|out| out.connector_selection.foreign_into())
        .into_report()
        .change_context(errors::RoutingError::DslExecutionError)?;

    Ok(match routing_output {
        routing_types::RoutingAlgorithm::Priority(plist) => plist,

        routing_types::RoutingAlgorithm::VolumeSplit(splits) => perform_volume_split(splits, None)
            .change_context(errors::RoutingError::DslFinalConnectorSelectionFailed)?,

        _ => Err(errors::RoutingError::DslIncorrectSelectionAlgorithm)
            .into_report()
            .attach_printable("Unsupported algorithm received as a result of static routing")?,
    })
}

/// Asynchronously refreshes the routing cache with the specified key, algorithm, and timestamp.
pub async fn refresh_routing_cache_v1(
    state: &AppState,
    key: String,
    algorithm_id: &str,
    timestamp: i64,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<()> {
    // Method implementation...
}

/// Perform a volume split on a list of ConnectorVolumeSplit objects based on their split weights.
/// If a random number generator seed is provided, use it to perform the split deterministically.
/// If no seed is provided, use the thread-local random number generator to perform the split.
/// Returns a list of RoutableConnectorChoice objects based on the split results.
pub fn perform_volume_split(
    mut splits: Vec<routing_types::ConnectorVolumeSplit>,
    rng_seed: Option<&str>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let weights: Vec<u8> = splits.iter().map(|sp| sp.split).collect();
    let weighted_index = distributions::WeightedIndex::new(weights)
        .into_report()
        .change_context(errors::RoutingError::VolumeSplitFailed)
        .attach_printable("Error creating weighted distribution for volume split")?;

    let idx = if let Some(seed) = rng_seed {
        let mut hasher = hash_map::DefaultHasher::new();
        seed.hash(&mut hasher);
        let hash = hasher.finish();

        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(hash);
        weighted_index.sample(&mut rng)
    } else {
        let mut rng = rand::thread_rng();
        weighted_index.sample(&mut rng)
    };

    splits
        .get(idx)
        .ok_or(errors::RoutingError::VolumeSplitFailed)
        .into_report()
        .attach_printable("Volume split index lookup failed")?;

    // Panic Safety: We have performed a `get(idx)` operation just above which will
    // ensure that the index is always present, else throw an error.
    let removed = splits.remove(idx);
    splits.insert(0, removed);

    Ok(splits.into_iter().map(|sp| sp.connector).collect())
}

/// Asynchronously retrieves the merchant's knowledge graph from the cache if present and not expired, otherwise refreshes the cache with the latest data and returns the updated knowledge graph. If the business profile routing feature is enabled, the method uses the provided profile ID to create a unique cache key for the knowledge graph. If the feature is not enabled, the cache key is created using the merchant ID only.
pub async fn get_merchant_kgraph<'a>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    merchant_last_modified: i64,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Arc<euclid_graph::KnowledgeGraph<'a>>> {
    // method implementation
}

/// Asynchronously refreshes the kgraph cache for a merchant using the provided parameters. It retrieves the merchant connector accounts from the store based on the merchant ID and disabled list, filters out certain connector types, and optionally filters the accounts based on the business profile ID. Then it converts the filtered accounts into admin API merchant connector responses, constructs a kgraph from the API merchant connector responses, and saves the kgraph to the cache using the specified key and timestamp. If the operation is successful, it returns Ok(()), otherwise it returns a RoutingResult containing an error.
pub async fn refresh_kgraph_cache(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    timestamp: i64,
    key: String,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<()> {
    // method implementation
}

/// Performs K-Graph filtering based on the provided parameters and returns a vector of RoutableConnectorChoice.
///
/// # Arguments
///
/// * `state` - The state of the application
/// * `key_store` - The key store of the merchant
/// * `merchant_last_modified` - The last modified timestamp of the merchant
/// * `chosen` - A vector of RoutableConnectorChoice representing the chosen connectors
/// * `backend_input` - The backend input for the K-Graph filtering
/// * `eligible_connectors` - Optional vector of eligible connectors
/// * `profile_id` - Optional profile ID if the 'business_profile_routing' feature is enabled
///
/// # Returns
///
/// A `RoutingResult` containing the filtered vector of RoutableConnectorChoice
///
/// # Examples
///
///

/// Performs eligibility analysis for routing a payment through various connector choices.
///
/// # Arguments
/// * `state` - The application state
/// * `key_store` - The merchant key store
/// * `merchant_last_modified` - The last modified timestamp of the merchant
/// * `chosen` - The chosen connector choices
/// * `payment_data` - The payment data
/// * `eligible_connectors` - Optionally, the eligible connectors
/// * `profile_id` - Optionally, the profile ID for business profile routing
///
/// # Returns
/// The result of the routing analysis, containing the eligible connector choices.
pub async fn perform_eligibility_analysis<F: Clone>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    merchant_last_modified: i64,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    payment_data: &payments_oss::PaymentData<F>,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let backend_input = make_dsl_input(payment_data)?;

    perform_kgraph_filtering(
        state,
        key_store,
        merchant_last_modified,
        chosen,
        backend_input,
        eligible_connectors,
        #[cfg(feature = "business_profile_routing")]
        profile_id,
    )
    .await
}

/// Performs fallback routing for a payment transaction, based on the provided parameters and configurations.
pub async fn perform_fallback_routing<F: Clone>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    merchant_last_modified: i64,
    payment_data: &payments_oss::PaymentData<F>,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let fallback_config = routing_helpers::get_merchant_default_config(
        &*state.store,
        #[cfg(not(feature = "profile_specific_fallback_routing"))]
        &key_store.merchant_id,
        #[cfg(feature = "profile_specific_fallback_routing")]
        {
            payment_data
                .payment_intent
                .profile_id
                .as_ref()
                .get_required_value("profile_id")
                .change_context(errors::RoutingError::ProfileIdMissing)?
        },
    )
    .await
    .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;
    let backend_input = make_dsl_input(payment_data)?;

    perform_kgraph_filtering(
        state,
        key_store,
        merchant_last_modified,
        fallback_config,
        backend_input,
        eligible_connectors,
        #[cfg(feature = "business_profile_routing")]
        profile_id,
    )
    .await
}

/// Performs eligibility analysis for routing with a fallback option, based on the given parameters.
pub async fn perform_eligibility_analysis_with_fallback<F: Clone>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    merchant_last_modified: i64,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    payment_data: &payments_oss::PaymentData<F>,
    eligible_connectors: Option<Vec<api_enums::RoutableConnectors>>,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let mut final_selection = perform_eligibility_analysis(
        state,
        key_store,
        merchant_last_modified,
        chosen,
        payment_data,
        eligible_connectors.as_ref(),
        #[cfg(feature = "business_profile_routing")]
        profile_id.clone(),
    )
    .await?;

    let fallback_selection = perform_fallback_routing(
        state,
        key_store,
        merchant_last_modified,
        payment_data,
        eligible_connectors.as_ref(),
        #[cfg(feature = "business_profile_routing")]
        profile_id,
    )
    .await;

    final_selection.append(
        &mut fallback_selection
            .unwrap_or_default()
            .iter()
            .filter(|&routable_connector_choice| {
                !final_selection.contains(routable_connector_choice)
            })
            .cloned()
            .collect::<Vec<_>>(),
    );

    let final_selected_connectors = final_selection
        .iter()
        .map(|item| item.connector)
        .collect::<Vec<_>>();
    logger::debug!(final_selected_connectors_for_routing=?final_selected_connectors, "List of final selected connectors for routing");

    Ok(final_selection)
}

/// Performs the session flow routing based on the given session input, returning the routing result which consists of a map of payment method types to session routing choices.
pub async fn perform_session_flow_routing(
    session_input: SessionFlowRoutingInput<'_>,
) -> RoutingResult<FxHashMap<api_enums::PaymentMethodType, routing_types::SessionRoutingChoice>> {

    // method implementation...
}

/// Performs session routing for a specific payment method type based on the input parameters.
/// It retrieves routing algorithm, executes volume split, performs K-Graph filtering, and selects the final connector
/// based on the allowed connectors provided.
async fn perform_session_routing_for_pm_type(
    session_pm_input: SessionRoutingPmTypeInput<'_>,
) -> RoutingResult<Option<(api::ConnectorData, Option<String>)>> {
    // method implementation
}

/// Creates a DSL input for surcharge based on the provided payment attempt, payment intent, and billing address.
pub fn make_dsl_input_for_surcharge(
    payment_attempt: &oss_storage::PaymentAttempt,
    payment_intent: &oss_storage::PaymentIntent,
    billing_address: Option<Address>,
) -> RoutingResult<dsl_inputs::BackendInput> {
    // method implementation
}
