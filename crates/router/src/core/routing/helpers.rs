//! Analysis for usage of all helper functions for use case of routing
//!
//! Functions that are used to perform the retrieval of merchant's
//! routing dict, configs, defaults
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use std::str::FromStr;
use std::{fmt::Debug, sync::Arc};

use api_models::routing as routing_types;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use common_utils::ext_traits::ValueExt;
use common_utils::{ext_traits::Encode, id_type, types::keymanager::KeyManagerState};
use diesel_models::configs;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use diesel_models::dynamic_routing_stats::DynamicRoutingStatsNew;
#[cfg(feature = "v1")]
use diesel_models::routing_algorithm;
use error_stack::ResultExt;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use external_services::grpc_client::dynamic_routing::{
    contract_routing_client::ContractBasedDynamicRouting,
    success_rate_client::SuccessBasedDynamicRouting,
};
#[cfg(feature = "v1")]
use hyperswitch_domain_models::api::ApplicationResponse;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use router_env::logger;
#[cfg(any(feature = "dynamic_routing", feature = "v1"))]
use router_env::{instrument, tracing};
use rustc_hash::FxHashSet;
use storage_impl::redis::cache::{self, Cacheable};

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use crate::db::errors::StorageErrorExt;
#[cfg(feature = "v2")]
use crate::types::domain::MerchantConnectorAccount;
use crate::{
    core::errors::{self, RouterResult},
    db::StorageInterface,
    routes::SessionState,
    types::{domain, storage},
    utils::StringExt,
};
#[cfg(feature = "v1")]
use crate::{core::metrics as core_metrics, types::transformers::ForeignInto};
pub const SUCCESS_BASED_DYNAMIC_ROUTING_ALGORITHM: &str =
    "Success rate based dynamic routing algorithm";
pub const ELIMINATION_BASED_DYNAMIC_ROUTING_ALGORITHM: &str =
    "Elimination based dynamic routing algorithm";
pub const CONTRACT_BASED_DYNAMIC_ROUTING_ALGORITHM: &str =
    "Contract based dynamic routing algorithm";

/// Provides us with all the configured configs of the Merchant in the ascending time configured
/// manner and chooses the first of them
pub async fn get_merchant_default_config(
    db: &dyn StorageInterface,
    // Cannot make this as merchant id domain type because, we are passing profile id also here
    merchant_id: &str,
    transaction_type: &storage::enums::TransactionType,
) -> RouterResult<Vec<routing_types::RoutableConnectorChoice>> {
    let key = get_default_config_key(merchant_id, transaction_type);
    let maybe_config = db.find_config_by_key(&key).await;

    match maybe_config {
        Ok(config) => config
            .config
            .parse_struct("Vec<RoutableConnectors>")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Merchant default config has invalid structure"),

        Err(e) if e.current_context().is_db_not_found() => {
            let new_config_conns = Vec::<routing_types::RoutableConnectorChoice>::new();
            let serialized = new_config_conns
                .encode_to_string_of_json()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Error while creating and serializing new merchant default config",
                )?;

            let new_config = configs::ConfigNew {
                key,
                config: serialized,
            };

            db.insert_config(new_config)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error inserting new default routing config into DB")?;

            Ok(new_config_conns)
        }

        Err(e) => Err(e)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error fetching default config for merchant"),
    }
}

/// Merchant's already created config can be updated and this change will be reflected
/// in DB as well for the particular updated config
pub async fn update_merchant_default_config(
    db: &dyn StorageInterface,
    merchant_id: &str,
    connectors: Vec<routing_types::RoutableConnectorChoice>,
    transaction_type: &storage::enums::TransactionType,
) -> RouterResult<()> {
    let key = get_default_config_key(merchant_id, transaction_type);
    let config_str = connectors
        .encode_to_string_of_json()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to serialize merchant default routing config during update")?;

    let config_update = configs::ConfigUpdate::Update {
        config: Some(config_str),
    };

    db.update_config_by_key(&key, config_update)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating the default routing config in DB")?;

    Ok(())
}

pub async fn update_merchant_routing_dictionary(
    db: &dyn StorageInterface,
    merchant_id: &str,
    dictionary: routing_types::RoutingDictionary,
) -> RouterResult<()> {
    let key = get_routing_dictionary_key(merchant_id);
    let dictionary_str = dictionary
        .encode_to_string_of_json()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to serialize routing dictionary during update")?;

    let config_update = configs::ConfigUpdate::Update {
        config: Some(dictionary_str),
    };

    db.update_config_by_key(&key, config_update)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error saving routing dictionary to DB")?;

    Ok(())
}

/// This will help make one of all configured algorithms to be in active state for a particular
/// merchant
#[cfg(feature = "v1")]
pub async fn update_merchant_active_algorithm_ref(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    config_key: cache::CacheKind<'_>,
    algorithm_id: routing_types::RoutingAlgorithmRef,
) -> RouterResult<()> {
    let ref_value = algorithm_id
        .encode_to_value()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed converting routing algorithm ref to json value")?;

    let merchant_account_update = storage::MerchantAccountUpdate::Update {
        merchant_name: None,
        merchant_details: None,
        return_url: None,
        webhook_details: None,
        sub_merchants_enabled: None,
        parent_merchant_id: None,
        enable_payment_response_hash: None,
        payment_response_hash_key: None,
        redirect_to_merchant_with_http_post: None,
        publishable_key: None,
        locker_id: None,
        metadata: None,
        routing_algorithm: Some(ref_value),
        primary_business_details: None,
        intent_fulfillment_time: None,
        frm_routing_algorithm: None,
        payout_routing_algorithm: None,
        default_profile: None,
        payment_link_config: None,
        pm_collect_link_config: None,
    };

    let db = &*state.store;
    db.update_specific_fields_in_merchant(
        &state.into(),
        &key_store.merchant_id,
        merchant_account_update,
        key_store,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update routing algorithm ref in merchant account")?;

    cache::redact_from_redis_and_publish(db.get_cache_store().as_ref(), [config_key])
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to invalidate the config cache")?;

    Ok(())
}

#[cfg(feature = "v1")]
pub async fn update_profile_active_algorithm_ref(
    db: &dyn StorageInterface,
    key_manager_state: &KeyManagerState,
    merchant_key_store: &domain::MerchantKeyStore,
    current_business_profile: domain::Profile,
    algorithm_id: routing_types::RoutingAlgorithmRef,
    transaction_type: &storage::enums::TransactionType,
) -> RouterResult<()> {
    let ref_val = algorithm_id
        .encode_to_value()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert routing ref to value")?;

    let merchant_id = current_business_profile.merchant_id.clone();

    let profile_id = current_business_profile.get_id().to_owned();

    let routing_cache_key = cache::CacheKind::Routing(
        format!(
            "routing_config_{}_{}",
            merchant_id.get_string_repr(),
            profile_id.get_string_repr(),
        )
        .into(),
    );

    let (routing_algorithm, payout_routing_algorithm) = match transaction_type {
        storage::enums::TransactionType::Payment => (Some(ref_val), None),
        #[cfg(feature = "payouts")]
        storage::enums::TransactionType::Payout => (None, Some(ref_val)),
    };

    let business_profile_update = domain::ProfileUpdate::RoutingAlgorithmUpdate {
        routing_algorithm,
        payout_routing_algorithm,
    };

    db.update_profile_by_profile_id(
        key_manager_state,
        merchant_key_store,
        current_business_profile,
        business_profile_update,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update routing algorithm ref in business profile")?;

    cache::redact_from_redis_and_publish(db.get_cache_store().as_ref(), [routing_cache_key])
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to invalidate routing cache")?;
    Ok(())
}

#[cfg(feature = "v1")]
pub async fn update_business_profile_active_dynamic_algorithm_ref(
    db: &dyn StorageInterface,
    key_manager_state: &KeyManagerState,
    merchant_key_store: &domain::MerchantKeyStore,
    current_business_profile: domain::Profile,
    dynamic_routing_algorithm_ref: routing_types::DynamicRoutingAlgorithmRef,
) -> RouterResult<()> {
    let ref_val = dynamic_routing_algorithm_ref
        .encode_to_value()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert dynamic routing ref to value")?;
    let business_profile_update = domain::ProfileUpdate::DynamicRoutingAlgorithmUpdate {
        dynamic_routing_algorithm: Some(ref_val),
    };
    db.update_profile_by_profile_id(
        key_manager_state,
        merchant_key_store,
        current_business_profile,
        business_profile_update,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update dynamic routing algorithm ref in business profile")?;
    Ok(())
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub struct RoutingAlgorithmHelpers<'h> {
    pub name_mca_id_set: ConnectNameAndMCAIdForProfile<'h>,
    pub name_set: ConnectNameForProfile<'h>,
    pub routing_algorithm: &'h routing_types::RoutingAlgorithm,
}

#[derive(Clone, Debug)]
pub struct ConnectNameAndMCAIdForProfile<'a>(
    pub  FxHashSet<(
        &'a common_enums::connector_enums::Connector,
        id_type::MerchantConnectorAccountId,
    )>,
);
#[derive(Clone, Debug)]
pub struct ConnectNameForProfile<'a>(pub FxHashSet<&'a common_enums::connector_enums::Connector>);

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub struct MerchantConnectorAccounts(pub Vec<MerchantConnectorAccount>);

#[cfg(feature = "v2")]
impl MerchantConnectorAccounts {
    pub async fn get_all_mcas(
        merchant_id: &id_type::MerchantId,
        key_store: &domain::MerchantKeyStore,
        state: &SessionState,
    ) -> RouterResult<Self> {
        let db = &*state.store;
        let key_manager_state = &state.into();
        Ok(Self(
            db.find_merchant_connector_account_by_merchant_id_and_disabled_list(
                key_manager_state,
                merchant_id,
                true,
                key_store,
            )
            .await
            .change_context(
                errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                    id: merchant_id.get_string_repr().to_owned(),
                },
            )?,
        ))
    }

    fn filter_and_map<'a, T>(
        &'a self,
        filter: impl Fn(&'a MerchantConnectorAccount) -> bool,
        func: impl Fn(&'a MerchantConnectorAccount) -> T,
    ) -> FxHashSet<T>
    where
        T: std::hash::Hash + Eq,
    {
        self.0
            .iter()
            .filter(|mca| filter(mca))
            .map(func)
            .collect::<FxHashSet<_>>()
    }

    pub fn filter_by_profile<'a, T>(
        &'a self,
        profile_id: &'a id_type::ProfileId,
        func: impl Fn(&'a MerchantConnectorAccount) -> T,
    ) -> FxHashSet<T>
    where
        T: std::hash::Hash + Eq,
    {
        self.filter_and_map(|mca| mca.profile_id == *profile_id, func)
    }
}

#[cfg(feature = "v2")]
impl RoutingAlgorithmHelpers<'_> {
    fn connector_choice(
        &self,
        choice: &routing_types::RoutableConnectorChoice,
    ) -> RouterResult<()> {
        if let Some(ref mca_id) = choice.merchant_connector_id {
            let connector_choice = common_enums::connector_enums::Connector::from(choice.connector);
            error_stack::ensure!(
                self.name_mca_id_set.0.contains(&(&connector_choice, mca_id.clone())),
                errors::ApiErrorResponse::InvalidRequestData {
                    message: format!(
                        "connector with name '{}' and merchant connector account id '{:?}' not found for the given profile",
                        connector_choice,
                        mca_id,
                    )
                }
            );
        } else {
            let connector_choice = common_enums::connector_enums::Connector::from(choice.connector);
            error_stack::ensure!(
                self.name_set.0.contains(&connector_choice),
                errors::ApiErrorResponse::InvalidRequestData {
                    message: format!(
                        "connector with name '{}' not found for the given profile",
                        connector_choice,
                    )
                }
            );
        };
        Ok(())
    }

    pub fn validate_connectors_in_routing_config(&self) -> RouterResult<()> {
        match self.routing_algorithm {
            routing_types::RoutingAlgorithm::Single(choice) => {
                self.connector_choice(choice)?;
            }

            routing_types::RoutingAlgorithm::Priority(list) => {
                for choice in list {
                    self.connector_choice(choice)?;
                }
            }

            routing_types::RoutingAlgorithm::VolumeSplit(splits) => {
                for split in splits {
                    self.connector_choice(&split.connector)?;
                }
            }

            routing_types::RoutingAlgorithm::Advanced(program) => {
                let check_connector_selection =
                    |selection: &routing_types::ConnectorSelection| -> RouterResult<()> {
                        match selection {
                            routing_types::ConnectorSelection::VolumeSplit(splits) => {
                                for split in splits {
                                    self.connector_choice(&split.connector)?;
                                }
                            }

                            routing_types::ConnectorSelection::Priority(list) => {
                                for choice in list {
                                    self.connector_choice(choice)?;
                                }
                            }
                        }

                        Ok(())
                    };

                check_connector_selection(&program.default_selection)?;

                for rule in &program.rules {
                    check_connector_selection(&rule.connector_selection)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "v1")]
pub async fn validate_connectors_in_routing_config(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    merchant_id: &id_type::MerchantId,
    profile_id: &id_type::ProfileId,
    routing_algorithm: &routing_types::RoutingAlgorithm,
) -> RouterResult<()> {
    let all_mcas = &*state
        .store
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            &state.into(),
            merchant_id,
            true,
            key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_id.get_string_repr().to_owned(),
        })?;
    let name_mca_id_set = all_mcas
        .iter()
        .filter(|mca| mca.profile_id == *profile_id)
        .map(|mca| (&mca.connector_name, mca.get_id()))
        .collect::<FxHashSet<_>>();

    let name_set = all_mcas
        .iter()
        .filter(|mca| mca.profile_id == *profile_id)
        .map(|mca| &mca.connector_name)
        .collect::<FxHashSet<_>>();

    let connector_choice = |choice: &routing_types::RoutableConnectorChoice| {
        if let Some(ref mca_id) = choice.merchant_connector_id {
            error_stack::ensure!(
                name_mca_id_set.contains(&(&choice.connector.to_string(), mca_id.clone())),
                errors::ApiErrorResponse::InvalidRequestData {
                    message: format!(
                        "connector with name '{}' and merchant connector account id '{:?}' not found for the given profile",
                        choice.connector,
                        mca_id,
                    )
                }
            );
        } else {
            error_stack::ensure!(
                name_set.contains(&choice.connector.to_string()),
                errors::ApiErrorResponse::InvalidRequestData {
                    message: format!(
                        "connector with name '{}' not found for the given profile",
                        choice.connector,
                    )
                }
            );
        }

        Ok(())
    };

    match routing_algorithm {
        routing_types::RoutingAlgorithm::Single(choice) => {
            connector_choice(choice)?;
        }

        routing_types::RoutingAlgorithm::Priority(list) => {
            for choice in list {
                connector_choice(choice)?;
            }
        }

        routing_types::RoutingAlgorithm::VolumeSplit(splits) => {
            for split in splits {
                connector_choice(&split.connector)?;
            }
        }

        routing_types::RoutingAlgorithm::Advanced(program) => {
            let check_connector_selection =
                |selection: &routing_types::ConnectorSelection| -> RouterResult<()> {
                    match selection {
                        routing_types::ConnectorSelection::VolumeSplit(splits) => {
                            for split in splits {
                                connector_choice(&split.connector)?;
                            }
                        }

                        routing_types::ConnectorSelection::Priority(list) => {
                            for choice in list {
                                connector_choice(choice)?;
                            }
                        }
                    }

                    Ok(())
                };

            check_connector_selection(&program.default_selection)?;

            for rule in &program.rules {
                check_connector_selection(&rule.connector_selection)?;
            }
        }
    }

    Ok(())
}

/// Provides the identifier for the specific merchant's routing_dictionary_key
#[inline(always)]
pub fn get_routing_dictionary_key(merchant_id: &str) -> String {
    format!("routing_dict_{merchant_id}")
}

/// Provides the identifier for the specific merchant's default_config
#[inline(always)]
pub fn get_default_config_key(
    merchant_id: &str,
    transaction_type: &storage::enums::TransactionType,
) -> String {
    match transaction_type {
        storage::enums::TransactionType::Payment => format!("routing_default_{merchant_id}"),
        #[cfg(feature = "payouts")]
        storage::enums::TransactionType::Payout => format!("routing_default_po_{merchant_id}"),
    }
}

#[async_trait::async_trait]
pub trait DynamicRoutingCache {
    async fn get_cached_dynamic_routing_config_for_profile(
        state: &SessionState,
        key: &str,
    ) -> Option<Arc<Self>>;

    async fn refresh_dynamic_routing_cache<T, F, Fut>(
        state: &SessionState,
        key: &str,
        func: F,
    ) -> RouterResult<T>
    where
        F: FnOnce() -> Fut + Send,
        T: Cacheable + serde::Serialize + serde::de::DeserializeOwned + Debug + Clone,
        Fut: futures::Future<Output = errors::CustomResult<T, errors::StorageError>> + Send;
}

#[async_trait::async_trait]
impl DynamicRoutingCache for routing_types::SuccessBasedRoutingConfig {
    async fn get_cached_dynamic_routing_config_for_profile(
        state: &SessionState,
        key: &str,
    ) -> Option<Arc<Self>> {
        cache::SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE
            .get_val::<Arc<Self>>(cache::CacheKey {
                key: key.to_string(),
                prefix: state.tenant.redis_key_prefix.clone(),
            })
            .await
    }

    async fn refresh_dynamic_routing_cache<T, F, Fut>(
        state: &SessionState,
        key: &str,
        func: F,
    ) -> RouterResult<T>
    where
        F: FnOnce() -> Fut + Send,
        T: Cacheable + serde::Serialize + serde::de::DeserializeOwned + Debug + Clone,
        Fut: futures::Future<Output = errors::CustomResult<T, errors::StorageError>> + Send,
    {
        cache::get_or_populate_in_memory(
            state.store.get_cache_store().as_ref(),
            key,
            func,
            &cache::SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to populate SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE")
    }
}

#[async_trait::async_trait]
impl DynamicRoutingCache for routing_types::ContractBasedRoutingConfig {
    async fn get_cached_dynamic_routing_config_for_profile(
        state: &SessionState,
        key: &str,
    ) -> Option<Arc<Self>> {
        cache::CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE
            .get_val::<Arc<Self>>(cache::CacheKey {
                key: key.to_string(),
                prefix: state.tenant.redis_key_prefix.clone(),
            })
            .await
    }

    async fn refresh_dynamic_routing_cache<T, F, Fut>(
        state: &SessionState,
        key: &str,
        func: F,
    ) -> RouterResult<T>
    where
        F: FnOnce() -> Fut + Send,
        T: Cacheable + serde::Serialize + serde::de::DeserializeOwned + Debug + Clone,
        Fut: futures::Future<Output = errors::CustomResult<T, errors::StorageError>> + Send,
    {
        cache::get_or_populate_in_memory(
            state.store.get_cache_store().as_ref(),
            key,
            func,
            &cache::CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to populate CONTRACT_BASED_DYNAMIC_ALGORITHM_CACHE")
    }
}

/// Cfetch dynamic routing configs
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn fetch_dynamic_routing_configs<T>(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
    routing_id: id_type::RoutingId,
) -> RouterResult<T>
where
    T: serde::de::DeserializeOwned
        + Clone
        + DynamicRoutingCache
        + Cacheable
        + serde::Serialize
        + Debug,
{
    let key = format!(
        "{}_{}",
        profile_id.get_string_repr(),
        routing_id.get_string_repr()
    );

    if let Some(config) =
        T::get_cached_dynamic_routing_config_for_profile(state, key.as_str()).await
    {
        Ok(config.as_ref().clone())
    } else {
        let func = || async {
            let routing_algorithm = state
                .store
                .find_routing_algorithm_by_profile_id_algorithm_id(profile_id, &routing_id)
                .await
                .change_context(errors::StorageError::ValueNotFound(
                    "RoutingAlgorithm".to_string(),
                ))
                .attach_printable("unable to retrieve routing_algorithm for profile from db")?;

            let dynamic_routing_config = routing_algorithm
                .algorithm_data
                .parse_value::<T>("dynamic_routing_config")
                .change_context(errors::StorageError::DeserializationFailed)
                .attach_printable("unable to parse dynamic_routing_config")?;

            Ok(dynamic_routing_config)
        };

        let dynamic_routing_config =
            T::refresh_dynamic_routing_cache(state, key.as_str(), func).await?;

        Ok(dynamic_routing_config)
    }
}

/// metrics for success based dynamic routing
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn push_metrics_with_update_window_for_success_based_routing(
    state: &SessionState,
    payment_attempt: &storage::PaymentAttempt,
    routable_connectors: Vec<routing_types::RoutableConnectorChoice>,
    profile_id: &id_type::ProfileId,
    dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef,
    dynamic_routing_config_params_interpolator: DynamicRoutingConfigParamsInterpolator,
) -> RouterResult<()> {
    let success_based_algo_ref = dynamic_routing_algo_ref
        .success_based_algorithm
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("success_based_algorithm not found in dynamic_routing_algorithm from business_profile table")?;

    if success_based_algo_ref.enabled_feature != routing_types::DynamicRoutingFeatures::None {
        let client = state
            .grpc_client
            .dynamic_routing
            .success_rate_client
            .as_ref()
            .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
                message: "success_rate gRPC client not found".to_string(),
            })?;

        let payment_connector = &payment_attempt.connector.clone().ok_or(
            errors::ApiErrorResponse::GenericNotFoundError {
                message: "unable to derive payment connector from payment attempt".to_string(),
            },
        )?;

        let success_based_routing_configs =
            fetch_dynamic_routing_configs::<routing_types::SuccessBasedRoutingConfig>(
                state,
                profile_id,
                success_based_algo_ref
                    .algorithm_id_with_timestamp
                    .algorithm_id
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "success_based_routing_algorithm_id not found in business_profile",
                    )?,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to retrieve success_rate based dynamic routing configs")?;

        let success_based_routing_config_params = dynamic_routing_config_params_interpolator
            .get_string_val(
                success_based_routing_configs
                    .params
                    .as_ref()
                    .ok_or(errors::RoutingError::SuccessBasedRoutingParamsNotFoundError)
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
            );

        let success_based_connectors = client
            .calculate_entity_and_global_success_rate(
                profile_id.get_string_repr().into(),
                success_based_routing_configs.clone(),
                success_based_routing_config_params.clone(),
                routable_connectors.clone(),
                state.get_grpc_headers(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to calculate/fetch success rate from dynamic routing service",
            )?;

        let payment_status_attribute =
            get_desired_payment_status_for_dynamic_routing_metrics(payment_attempt.status);

        let first_merchant_success_based_connector = &success_based_connectors
            .entity_scores_with_labels
            .first()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to fetch the first connector from list of connectors obtained from dynamic routing service",
            )?;

        let (first_merchant_success_based_connector_label, _) = first_merchant_success_based_connector.label
            .split_once(':')
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!(
                "unable to split connector_name and mca_id from the first connector {:?} obtained from dynamic routing service",
                first_merchant_success_based_connector.label
            ))?;

        let first_global_success_based_connector = &success_based_connectors
            .global_scores_with_labels
            .first()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to fetch the first global connector from list of connectors obtained from dynamic routing service",
            )?;

        let outcome = get_dynamic_routing_based_metrics_outcome_for_payment(
            payment_status_attribute,
            payment_connector.to_string(),
            first_merchant_success_based_connector_label.to_string(),
        );

        let dynamic_routing_stats = DynamicRoutingStatsNew {
            payment_id: payment_attempt.payment_id.to_owned(),
            attempt_id: payment_attempt.attempt_id.clone(),
            merchant_id: payment_attempt.merchant_id.to_owned(),
            profile_id: payment_attempt.profile_id.to_owned(),
            amount: payment_attempt.get_total_amount(),
            success_based_routing_connector: first_merchant_success_based_connector_label
                .to_string(),
            payment_connector: payment_connector.to_string(),
            payment_method_type: payment_attempt.payment_method_type,
            currency: payment_attempt.currency,
            payment_method: payment_attempt.payment_method,
            capture_method: payment_attempt.capture_method,
            authentication_type: payment_attempt.authentication_type,
            payment_status: payment_attempt.status,
            conclusive_classification: outcome,
            created_at: common_utils::date_time::now(),
            global_success_based_connector: Some(
                first_global_success_based_connector.label.to_string(),
            ),
        };

        core_metrics::DYNAMIC_SUCCESS_BASED_ROUTING.add(
            1,
            router_env::metric_attributes!(
                (
                    "tenant",
                    state.tenant.tenant_id.get_string_repr().to_owned(),
                ),
                (
                    "merchant_profile_id",
                    format!(
                        "{}:{}",
                        payment_attempt.merchant_id.get_string_repr(),
                        payment_attempt.profile_id.get_string_repr()
                    ),
                ),
                (
                    "merchant_specific_success_based_routing_connector",
                    first_merchant_success_based_connector_label.to_string(),
                ),
                (
                    "merchant_specific_success_based_routing_connector_score",
                    first_merchant_success_based_connector.score.to_string(),
                ),
                (
                    "global_success_based_routing_connector",
                    first_global_success_based_connector.label.to_string(),
                ),
                (
                    "global_success_based_routing_connector_score",
                    first_global_success_based_connector.score.to_string(),
                ),
                ("payment_connector", payment_connector.to_string()),
                (
                    "currency",
                    payment_attempt
                        .currency
                        .map_or_else(|| "None".to_string(), |currency| currency.to_string()),
                ),
                (
                    "payment_method",
                    payment_attempt.payment_method.map_or_else(
                        || "None".to_string(),
                        |payment_method| payment_method.to_string(),
                    ),
                ),
                (
                    "payment_method_type",
                    payment_attempt.payment_method_type.map_or_else(
                        || "None".to_string(),
                        |payment_method_type| payment_method_type.to_string(),
                    ),
                ),
                (
                    "capture_method",
                    payment_attempt.capture_method.map_or_else(
                        || "None".to_string(),
                        |capture_method| capture_method.to_string(),
                    ),
                ),
                (
                    "authentication_type",
                    payment_attempt.authentication_type.map_or_else(
                        || "None".to_string(),
                        |authentication_type| authentication_type.to_string(),
                    ),
                ),
                ("payment_status", payment_attempt.status.to_string()),
                ("conclusive_classification", outcome.to_string()),
            ),
        );
        logger::debug!("successfully pushed success_based_routing metrics");

        state
            .store
            .insert_dynamic_routing_stat_entry(dynamic_routing_stats)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to push dynamic routing stats to db")?;

        client
            .update_success_rate(
                profile_id.get_string_repr().into(),
                success_based_routing_configs,
                success_based_routing_config_params,
                vec![routing_types::RoutableConnectorChoiceWithStatus::new(
                    routing_types::RoutableConnectorChoice {
                        choice_kind: api_models::routing::RoutableChoiceKind::FullStruct,
                        connector: common_enums::RoutableConnectors::from_str(
                            payment_connector.as_str(),
                        )
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("unable to infer routable_connector from connector")?,
                        merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
                    },
                    payment_status_attribute == common_enums::AttemptStatus::Charged,
                )],
                state.get_grpc_headers(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to update success based routing window in dynamic routing service",
            )?;
        Ok(())
    } else {
        Ok(())
    }
}

/// metrics for contract based dynamic routing
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn push_metrics_with_update_window_for_contract_based_routing(
    state: &SessionState,
    payment_attempt: &storage::PaymentAttempt,
    routable_connectors: Vec<routing_types::RoutableConnectorChoice>,
    profile_id: &id_type::ProfileId,
    dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef,
    _dynamic_routing_config_params_interpolator: DynamicRoutingConfigParamsInterpolator,
) -> RouterResult<()> {
    let contract_routing_algo_ref = dynamic_routing_algo_ref
        .contract_based_routing
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("contract_routing_algorithm not found in dynamic_routing_algorithm from business_profile table")?;

    if contract_routing_algo_ref.enabled_feature != routing_types::DynamicRoutingFeatures::None {
        let client = state
            .grpc_client
            .dynamic_routing
            .contract_based_client
            .clone()
            .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
                message: "contract_routing gRPC client not found".to_string(),
            })?;

        let payment_connector = &payment_attempt.connector.clone().ok_or(
            errors::ApiErrorResponse::GenericNotFoundError {
                message: "unable to derive payment connector from payment attempt".to_string(),
            },
        )?;

        let contract_based_routing_config =
            fetch_dynamic_routing_configs::<routing_types::ContractBasedRoutingConfig>(
                state,
                profile_id,
                contract_routing_algo_ref
                    .algorithm_id_with_timestamp
                    .algorithm_id
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "contract_based_routing_algorithm_id not found in business_profile",
                    )?,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to retrieve contract based dynamic routing configs")?;

        let mut existing_label_info = None;

        contract_based_routing_config
            .label_info
            .as_ref()
            .map(|label_info_vec| {
                for label_info in label_info_vec {
                    if Some(&label_info.mca_id) == payment_attempt.merchant_connector_id.as_ref() {
                        existing_label_info = Some(label_info.clone());
                    }
                }
            });

        let final_label_info = existing_label_info
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to get LabelInformation from ContractBasedRoutingConfig")?;

        logger::debug!(
            "contract based routing: matched LabelInformation - {:?}",
            final_label_info
        );

        let request_label_info = routing_types::LabelInformation {
            label: format!(
                "{}:{}",
                final_label_info.label.clone(),
                final_label_info.mca_id.get_string_repr()
            ),
            target_count: final_label_info.target_count,
            target_time: final_label_info.target_time,
            mca_id: final_label_info.mca_id.to_owned(),
        };

        let payment_status_attribute =
            get_desired_payment_status_for_dynamic_routing_metrics(payment_attempt.status);

        if payment_status_attribute == common_enums::AttemptStatus::Charged {
            client
                .update_contracts(
                    profile_id.get_string_repr().into(),
                    vec![request_label_info],
                    "".to_string(),
                    vec![],
                    state.get_grpc_headers(),
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "unable to update contract based routing window in dynamic routing service",
                )?;
        }

        let contract_scores = client
            .calculate_contract_score(
                profile_id.get_string_repr().into(),
                contract_based_routing_config.clone(),
                "".to_string(),
                routable_connectors.clone(),
                state.get_grpc_headers(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to calculate/fetch contract scores from dynamic routing service",
            )?;

        let first_contract_based_connector = &contract_scores
            .labels_with_score
            .first()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to fetch the first connector from list of connectors obtained from dynamic routing service",
            )?;

        let (first_contract_based_connector, connector_score, current_payment_cnt) = (first_contract_based_connector.label
            .split_once(':')
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!(
                "unable to split connector_name and mca_id from the first connector {:?} obtained from dynamic routing service",
                first_contract_based_connector
            ))?
            .0, first_contract_based_connector.score, first_contract_based_connector.current_count );

        core_metrics::DYNAMIC_CONTRACT_BASED_ROUTING.add(
            1,
            router_env::metric_attributes!(
                (
                    "tenant",
                    state.tenant.tenant_id.get_string_repr().to_owned(),
                ),
                (
                    "merchant_profile_id",
                    format!(
                        "{}:{}",
                        payment_attempt.merchant_id.get_string_repr(),
                        payment_attempt.profile_id.get_string_repr()
                    ),
                ),
                (
                    "contract_based_routing_connector",
                    first_contract_based_connector.to_string(),
                ),
                (
                    "contract_based_routing_connector_score",
                    connector_score.to_string(),
                ),
                (
                    "current_payment_count_contract_based_routing_connector",
                    current_payment_cnt.to_string(),
                ),
                ("payment_connector", payment_connector.to_string()),
                (
                    "currency",
                    payment_attempt
                        .currency
                        .map_or_else(|| "None".to_string(), |currency| currency.to_string()),
                ),
                (
                    "payment_method",
                    payment_attempt.payment_method.map_or_else(
                        || "None".to_string(),
                        |payment_method| payment_method.to_string(),
                    ),
                ),
                (
                    "payment_method_type",
                    payment_attempt.payment_method_type.map_or_else(
                        || "None".to_string(),
                        |payment_method_type| payment_method_type.to_string(),
                    ),
                ),
                (
                    "capture_method",
                    payment_attempt.capture_method.map_or_else(
                        || "None".to_string(),
                        |capture_method| capture_method.to_string(),
                    ),
                ),
                (
                    "authentication_type",
                    payment_attempt.authentication_type.map_or_else(
                        || "None".to_string(),
                        |authentication_type| authentication_type.to_string(),
                    ),
                ),
                ("payment_status", payment_attempt.status.to_string()),
            ),
        );
        logger::debug!("successfully pushed contract_based_routing metrics");

        Ok(())
    } else {
        Ok(())
    }
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
fn get_desired_payment_status_for_dynamic_routing_metrics(
    attempt_status: common_enums::AttemptStatus,
) -> common_enums::AttemptStatus {
    match attempt_status {
        common_enums::AttemptStatus::Charged
        | common_enums::AttemptStatus::Authorized
        | common_enums::AttemptStatus::PartialCharged
        | common_enums::AttemptStatus::PartialChargedAndChargeable => {
            common_enums::AttemptStatus::Charged
        }
        common_enums::AttemptStatus::Failure
        | common_enums::AttemptStatus::AuthorizationFailed
        | common_enums::AttemptStatus::AuthenticationFailed
        | common_enums::AttemptStatus::CaptureFailed
        | common_enums::AttemptStatus::RouterDeclined => common_enums::AttemptStatus::Failure,
        common_enums::AttemptStatus::Started
        | common_enums::AttemptStatus::AuthenticationPending
        | common_enums::AttemptStatus::AuthenticationSuccessful
        | common_enums::AttemptStatus::Authorizing
        | common_enums::AttemptStatus::CodInitiated
        | common_enums::AttemptStatus::Voided
        | common_enums::AttemptStatus::VoidInitiated
        | common_enums::AttemptStatus::CaptureInitiated
        | common_enums::AttemptStatus::VoidFailed
        | common_enums::AttemptStatus::AutoRefunded
        | common_enums::AttemptStatus::Unresolved
        | common_enums::AttemptStatus::Pending
        | common_enums::AttemptStatus::PaymentMethodAwaited
        | common_enums::AttemptStatus::ConfirmationAwaited
        | common_enums::AttemptStatus::DeviceDataCollectionPending => {
            common_enums::AttemptStatus::Pending
        }
    }
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
fn get_dynamic_routing_based_metrics_outcome_for_payment(
    payment_status_attribute: common_enums::AttemptStatus,
    payment_connector: String,
    first_success_based_connector: String,
) -> common_enums::SuccessBasedRoutingConclusiveState {
    match payment_status_attribute {
        common_enums::AttemptStatus::Charged
            if *first_success_based_connector == *payment_connector =>
        {
            common_enums::SuccessBasedRoutingConclusiveState::TruePositive
        }
        common_enums::AttemptStatus::Failure
            if *first_success_based_connector == *payment_connector =>
        {
            common_enums::SuccessBasedRoutingConclusiveState::FalsePositive
        }
        common_enums::AttemptStatus::Failure
            if *first_success_based_connector != *payment_connector =>
        {
            common_enums::SuccessBasedRoutingConclusiveState::TrueNegative
        }
        common_enums::AttemptStatus::Charged
            if *first_success_based_connector != *payment_connector =>
        {
            common_enums::SuccessBasedRoutingConclusiveState::FalseNegative
        }
        _ => common_enums::SuccessBasedRoutingConclusiveState::NonDeterministic,
    }
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn disable_dynamic_routing_algorithm(
    state: &SessionState,
    key_store: domain::MerchantKeyStore,
    business_profile: domain::Profile,
    dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef,
    dynamic_routing_type: routing_types::DynamicRoutingType,
) -> RouterResult<ApplicationResponse<routing_types::RoutingDictionaryRecord>> {
    let db = state.store.as_ref();
    let key_manager_state = &state.into();
    let profile_id = business_profile.get_id().clone();
    let (algorithm_id, dynamic_routing_algorithm, cache_entries_to_redact) =
        match dynamic_routing_type {
            routing_types::DynamicRoutingType::SuccessRateBasedRouting => {
                let Some(algorithm_ref) = dynamic_routing_algo_ref.success_based_algorithm else {
                    Err(errors::ApiErrorResponse::PreconditionFailed {
                        message: "Success rate based routing is already disabled".to_string(),
                    })?
                };
                let Some(algorithm_id) = algorithm_ref.algorithm_id_with_timestamp.algorithm_id
                else {
                    Err(errors::ApiErrorResponse::PreconditionFailed {
                        message: "Algorithm is already inactive".to_string(),
                    })?
                };

                let cache_key = format!(
                    "{}_{}",
                    business_profile.get_id().get_string_repr(),
                    algorithm_id.get_string_repr()
                );
                let cache_entries_to_redact =
                    vec![cache::CacheKind::SuccessBasedDynamicRoutingCache(
                        cache_key.into(),
                    )];
                (
                    algorithm_id,
                    routing_types::DynamicRoutingAlgorithmRef {
                        success_based_algorithm: Some(routing_types::SuccessBasedAlgorithm {
                            algorithm_id_with_timestamp:
                                routing_types::DynamicAlgorithmWithTimestamp::new(None),
                            enabled_feature: routing_types::DynamicRoutingFeatures::None,
                        }),
                        elimination_routing_algorithm: dynamic_routing_algo_ref
                            .elimination_routing_algorithm,
                        contract_based_routing: dynamic_routing_algo_ref.contract_based_routing,
                        dynamic_routing_volume_split: dynamic_routing_algo_ref
                            .dynamic_routing_volume_split,
                    },
                    cache_entries_to_redact,
                )
            }
            routing_types::DynamicRoutingType::EliminationRouting => {
                let Some(algorithm_ref) = dynamic_routing_algo_ref.elimination_routing_algorithm
                else {
                    Err(errors::ApiErrorResponse::PreconditionFailed {
                        message: "Elimination routing is already disabled".to_string(),
                    })?
                };
                let Some(algorithm_id) = algorithm_ref.algorithm_id_with_timestamp.algorithm_id
                else {
                    Err(errors::ApiErrorResponse::PreconditionFailed {
                        message: "Algorithm is already inactive".to_string(),
                    })?
                };
                let cache_key = format!(
                    "{}_{}",
                    business_profile.get_id().get_string_repr(),
                    algorithm_id.get_string_repr()
                );
                let cache_entries_to_redact =
                    vec![cache::CacheKind::EliminationBasedDynamicRoutingCache(
                        cache_key.into(),
                    )];
                (
                    algorithm_id,
                    routing_types::DynamicRoutingAlgorithmRef {
                        success_based_algorithm: dynamic_routing_algo_ref.success_based_algorithm,
                        dynamic_routing_volume_split: dynamic_routing_algo_ref
                            .dynamic_routing_volume_split,
                        elimination_routing_algorithm: Some(
                            routing_types::EliminationRoutingAlgorithm {
                                algorithm_id_with_timestamp:
                                    routing_types::DynamicAlgorithmWithTimestamp::new(None),
                                enabled_feature: routing_types::DynamicRoutingFeatures::None,
                            },
                        ),
                        contract_based_routing: dynamic_routing_algo_ref.contract_based_routing,
                    },
                    cache_entries_to_redact,
                )
            }
            routing_types::DynamicRoutingType::ContractBasedRouting => {
                let Some(algorithm_ref) = dynamic_routing_algo_ref.contract_based_routing else {
                    Err(errors::ApiErrorResponse::PreconditionFailed {
                        message: "Contract routing is already disabled".to_string(),
                    })?
                };
                let Some(algorithm_id) = algorithm_ref.algorithm_id_with_timestamp.algorithm_id
                else {
                    Err(errors::ApiErrorResponse::PreconditionFailed {
                        message: "Algorithm is already inactive".to_string(),
                    })?
                };
                let cache_key = format!(
                    "{}_{}",
                    business_profile.get_id().get_string_repr(),
                    algorithm_id.get_string_repr()
                );
                let cache_entries_to_redact =
                    vec![cache::CacheKind::ContractBasedDynamicRoutingCache(
                        cache_key.into(),
                    )];
                (
                    algorithm_id,
                    routing_types::DynamicRoutingAlgorithmRef {
                        success_based_algorithm: dynamic_routing_algo_ref.success_based_algorithm,
                        elimination_routing_algorithm: dynamic_routing_algo_ref
                            .elimination_routing_algorithm,
                        dynamic_routing_volume_split: dynamic_routing_algo_ref
                            .dynamic_routing_volume_split,
                        contract_based_routing: Some(routing_types::ContractRoutingAlgorithm {
                            algorithm_id_with_timestamp:
                                routing_types::DynamicAlgorithmWithTimestamp::new(None),
                            enabled_feature: routing_types::DynamicRoutingFeatures::None,
                        }),
                    },
                    cache_entries_to_redact,
                )
            }
        };

    // redact cache for dynamic routing config
    let _ = cache::redact_from_redis_and_publish(
        state.store.get_cache_store().as_ref(),
        cache_entries_to_redact,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable(
        "unable to publish into the redact channel for evicting the dynamic routing config cache",
    )?;

    let record = db
        .find_routing_algorithm_by_profile_id_algorithm_id(business_profile.get_id(), &algorithm_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
    let response = record.foreign_into();
    update_business_profile_active_dynamic_algorithm_ref(
        db,
        key_manager_state,
        &key_store,
        business_profile,
        dynamic_routing_algorithm,
    )
    .await?;

    core_metrics::ROUTING_UNLINK_CONFIG_SUCCESS_RESPONSE.add(
        1,
        router_env::metric_attributes!(("profile_id", profile_id)),
    );

    Ok(ApplicationResponse::Json(response))
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn enable_dynamic_routing_algorithm(
    state: &SessionState,
    key_store: domain::MerchantKeyStore,
    business_profile: domain::Profile,
    feature_to_enable: routing_types::DynamicRoutingFeatures,
    dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef,
    dynamic_routing_type: routing_types::DynamicRoutingType,
) -> RouterResult<ApplicationResponse<routing_types::RoutingDictionaryRecord>> {
    let mut dynamic_routing = dynamic_routing_algo_ref.clone();
    match dynamic_routing_type {
        routing_types::DynamicRoutingType::SuccessRateBasedRouting => {
            dynamic_routing
                .disable_algorithm_id(routing_types::DynamicRoutingType::ContractBasedRouting);

            enable_specific_routing_algorithm(
                state,
                key_store,
                business_profile,
                feature_to_enable,
                dynamic_routing.clone(),
                dynamic_routing_type,
                dynamic_routing.success_based_algorithm,
            )
            .await
        }
        routing_types::DynamicRoutingType::EliminationRouting => {
            enable_specific_routing_algorithm(
                state,
                key_store,
                business_profile,
                feature_to_enable,
                dynamic_routing.clone(),
                dynamic_routing_type,
                dynamic_routing.elimination_routing_algorithm,
            )
            .await
        }
        routing_types::DynamicRoutingType::ContractBasedRouting => {
            Err((errors::ApiErrorResponse::InvalidRequestData {
                message: "Contract routing cannot be set as default".to_string(),
            })
            .into())
        }
    }
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn enable_specific_routing_algorithm<A>(
    state: &SessionState,
    key_store: domain::MerchantKeyStore,
    business_profile: domain::Profile,
    feature_to_enable: routing_types::DynamicRoutingFeatures,
    mut dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef,
    dynamic_routing_type: routing_types::DynamicRoutingType,
    algo_type: Option<A>,
) -> RouterResult<ApplicationResponse<routing_types::RoutingDictionaryRecord>>
where
    A: routing_types::DynamicRoutingAlgoAccessor + Clone + Debug,
{
    // Algorithm wasn't created yet
    let Some(mut algo_type) = algo_type else {
        return default_specific_dynamic_routing_setup(
            state,
            key_store,
            business_profile,
            feature_to_enable,
            dynamic_routing_algo_ref,
            dynamic_routing_type,
        )
        .await;
    };

    // Algorithm was in disabled state
    let Some(algo_type_algorithm_id) = algo_type
        .clone()
        .get_algorithm_id_with_timestamp()
        .algorithm_id
    else {
        return default_specific_dynamic_routing_setup(
            state,
            key_store,
            business_profile,
            feature_to_enable,
            dynamic_routing_algo_ref,
            dynamic_routing_type,
        )
        .await;
    };
    let db = state.store.as_ref();
    let profile_id = business_profile.get_id().clone();
    let algo_type_enabled_features = algo_type.get_enabled_features();
    if *algo_type_enabled_features == feature_to_enable {
        // algorithm already has the required feature
        return Err(errors::ApiErrorResponse::PreconditionFailed {
            message: format!("{} is already enabled", dynamic_routing_type),
        }
        .into());
    };
    *algo_type_enabled_features = feature_to_enable;
    dynamic_routing_algo_ref.update_enabled_features(dynamic_routing_type, feature_to_enable);
    update_business_profile_active_dynamic_algorithm_ref(
        db,
        &state.into(),
        &key_store,
        business_profile,
        dynamic_routing_algo_ref.clone(),
    )
    .await?;

    let routing_algorithm = db
        .find_routing_algorithm_by_profile_id_algorithm_id(&profile_id, &algo_type_algorithm_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
    let updated_routing_record = routing_algorithm.foreign_into();
    core_metrics::ROUTING_CREATE_SUCCESS_RESPONSE.add(
        1,
        router_env::metric_attributes!(("profile_id", profile_id.clone())),
    );
    Ok(ApplicationResponse::Json(updated_routing_record))
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn default_specific_dynamic_routing_setup(
    state: &SessionState,
    key_store: domain::MerchantKeyStore,
    business_profile: domain::Profile,
    feature_to_enable: routing_types::DynamicRoutingFeatures,
    mut dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef,
    dynamic_routing_type: routing_types::DynamicRoutingType,
) -> RouterResult<ApplicationResponse<routing_types::RoutingDictionaryRecord>> {
    let db = state.store.as_ref();
    let key_manager_state = &state.into();
    let profile_id = business_profile.get_id().clone();
    let merchant_id = business_profile.merchant_id.clone();
    let algorithm_id = common_utils::generate_routing_id_of_default_length();
    let timestamp = common_utils::date_time::now();
    let algo = match dynamic_routing_type {
        routing_types::DynamicRoutingType::SuccessRateBasedRouting => {
            let default_success_based_routing_config =
                routing_types::SuccessBasedRoutingConfig::default();
            routing_algorithm::RoutingAlgorithm {
                algorithm_id: algorithm_id.clone(),
                profile_id: profile_id.clone(),
                merchant_id,
                name: SUCCESS_BASED_DYNAMIC_ROUTING_ALGORITHM.to_string(),
                description: None,
                kind: diesel_models::enums::RoutingAlgorithmKind::Dynamic,
                algorithm_data: serde_json::json!(default_success_based_routing_config),
                created_at: timestamp,
                modified_at: timestamp,
                algorithm_for: common_enums::TransactionType::Payment,
            }
        }
        routing_types::DynamicRoutingType::EliminationRouting => {
            let default_elimination_routing_config =
                routing_types::EliminationRoutingConfig::default();
            routing_algorithm::RoutingAlgorithm {
                algorithm_id: algorithm_id.clone(),
                profile_id: profile_id.clone(),
                merchant_id,
                name: ELIMINATION_BASED_DYNAMIC_ROUTING_ALGORITHM.to_string(),
                description: None,
                kind: diesel_models::enums::RoutingAlgorithmKind::Dynamic,
                algorithm_data: serde_json::json!(default_elimination_routing_config),
                created_at: timestamp,
                modified_at: timestamp,
                algorithm_for: common_enums::TransactionType::Payment,
            }
        }

        routing_types::DynamicRoutingType::ContractBasedRouting => {
            return Err((errors::ApiErrorResponse::InvalidRequestData {
                message: "Contract routing cannot be set as default".to_string(),
            })
            .into())
        }
    };

    let record = db
        .insert_routing_algorithm(algo)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to insert record in routing algorithm table")?;

    dynamic_routing_algo_ref.update_algorithm_id(
        algorithm_id,
        feature_to_enable,
        dynamic_routing_type,
    );
    update_business_profile_active_dynamic_algorithm_ref(
        db,
        key_manager_state,
        &key_store,
        business_profile,
        dynamic_routing_algo_ref,
    )
    .await?;

    let new_record = record.foreign_into();

    core_metrics::ROUTING_CREATE_SUCCESS_RESPONSE.add(
        1,
        router_env::metric_attributes!(("profile_id", profile_id.clone())),
    );
    Ok(ApplicationResponse::Json(new_record))
}

#[derive(Debug, Clone)]
pub struct DynamicRoutingConfigParamsInterpolator {
    pub payment_method: Option<common_enums::PaymentMethod>,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub authentication_type: Option<common_enums::AuthenticationType>,
    pub currency: Option<common_enums::Currency>,
    pub country: Option<common_enums::CountryAlpha2>,
    pub card_network: Option<String>,
    pub card_bin: Option<String>,
}

impl DynamicRoutingConfigParamsInterpolator {
    pub fn new(
        payment_method: Option<common_enums::PaymentMethod>,
        payment_method_type: Option<common_enums::PaymentMethodType>,
        authentication_type: Option<common_enums::AuthenticationType>,
        currency: Option<common_enums::Currency>,
        country: Option<common_enums::CountryAlpha2>,
        card_network: Option<String>,
        card_bin: Option<String>,
    ) -> Self {
        Self {
            payment_method,
            payment_method_type,
            authentication_type,
            currency,
            country,
            card_network,
            card_bin,
        }
    }

    pub fn get_string_val(
        &self,
        params: &Vec<routing_types::DynamicRoutingConfigParams>,
    ) -> String {
        let mut parts: Vec<String> = Vec::new();
        for param in params {
            let val = match param {
                routing_types::DynamicRoutingConfigParams::PaymentMethod => self
                    .payment_method
                    .as_ref()
                    .map_or(String::new(), |pm| pm.to_string()),
                routing_types::DynamicRoutingConfigParams::PaymentMethodType => self
                    .payment_method_type
                    .as_ref()
                    .map_or(String::new(), |pmt| pmt.to_string()),
                routing_types::DynamicRoutingConfigParams::AuthenticationType => self
                    .authentication_type
                    .as_ref()
                    .map_or(String::new(), |at| at.to_string()),
                routing_types::DynamicRoutingConfigParams::Currency => self
                    .currency
                    .as_ref()
                    .map_or(String::new(), |cur| cur.to_string()),
                routing_types::DynamicRoutingConfigParams::Country => self
                    .country
                    .as_ref()
                    .map_or(String::new(), |cn| cn.to_string()),
                routing_types::DynamicRoutingConfigParams::CardNetwork => {
                    self.card_network.clone().unwrap_or_default()
                }
                routing_types::DynamicRoutingConfigParams::CardBin => {
                    self.card_bin.clone().unwrap_or_default()
                }
            };
            if !val.is_empty() {
                parts.push(val);
            }
        }
        parts.join(":")
    }
}
