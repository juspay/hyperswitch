//! Analysis for usage of all helper functions for use case of routing
//!
//! Functions that are used to perform the retrieval of merchant's
//! routing dict, configs, defaults
use std::fmt::Debug;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use std::str::FromStr;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use std::sync::Arc;

#[cfg(feature = "v1")]
use api_models::open_router;
use api_models::routing as routing_types;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use common_utils::ext_traits::ValueExt;
use common_utils::{ext_traits::Encode, id_type};
use diesel_models::configs;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use diesel_models::dynamic_routing_stats::{DynamicRoutingStatsNew, DynamicRoutingStatsUpdate};
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use diesel_models::routing_algorithm;
use error_stack::ResultExt;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use external_services::grpc_client::dynamic_routing::{
    contract_routing_client::ContractBasedDynamicRouting,
    elimination_based_client::EliminationBasedRouting,
    success_rate_client::SuccessBasedDynamicRouting,
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use hyperswitch_domain_models::api::ApplicationResponse;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use hyperswitch_interfaces::events::routing_api_logs as routing_events;
#[cfg(feature = "v1")]
use router_env::logger;
#[cfg(feature = "v1")]
use router_env::{instrument, tracing};
use rustc_hash::FxHashSet;
use storage_impl::redis::cache;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use storage_impl::redis::cache::Cacheable;

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use crate::db::errors::StorageErrorExt;
#[cfg(feature = "v2")]
use crate::types::domain::MerchantConnectorAccount;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use crate::types::transformers::ForeignFrom;
use crate::{
    core::errors::{self, RouterResult},
    db::StorageInterface,
    routes::SessionState,
    types::{domain, storage},
    utils::StringExt,
};
#[cfg(feature = "v1")]
use crate::{
    core::payments::{
        routing::utils::{self as routing_utils, DecisionEngineApiHandler},
        OperationSessionGetters, OperationSessionSetters,
    },
    services,
};
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use crate::{
    core::{metrics as core_metrics, routing},
    routes::app::SessionStateInfo,
    types::transformers::ForeignInto,
};
pub const SUCCESS_BASED_DYNAMIC_ROUTING_ALGORITHM: &str =
    "Success rate based dynamic routing algorithm";
pub const ELIMINATION_BASED_DYNAMIC_ROUTING_ALGORITHM: &str =
    "Elimination based dynamic routing algorithm";
pub const CONTRACT_BASED_DYNAMIC_ROUTING_ALGORITHM: &str =
    "Contract based dynamic routing algorithm";

pub const DECISION_ENGINE_RULE_CREATE_ENDPOINT: &str = "rule/create";
pub const DECISION_ENGINE_RULE_UPDATE_ENDPOINT: &str = "rule/update";
pub const DECISION_ENGINE_RULE_GET_ENDPOINT: &str = "rule/get";
pub const DECISION_ENGINE_RULE_DELETE_ENDPOINT: &str = "rule/delete";
pub const DECISION_ENGINE_MERCHANT_BASE_ENDPOINT: &str = "merchant-account";
pub const DECISION_ENGINE_MERCHANT_CREATE_ENDPOINT: &str = "merchant-account/create";

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

    let (routing_algorithm, payout_routing_algorithm, three_ds_decision_rule_algorithm) =
        match transaction_type {
            storage::enums::TransactionType::Payment => (Some(ref_val), None, None),
            #[cfg(feature = "payouts")]
            storage::enums::TransactionType::Payout => (None, Some(ref_val), None),
            storage::enums::TransactionType::ThreeDsAuthentication => (None, None, Some(ref_val)),
        };

    let business_profile_update = domain::ProfileUpdate::RoutingAlgorithmUpdate {
        routing_algorithm,
        payout_routing_algorithm,
        three_ds_decision_rule_algorithm,
    };

    db.update_profile_by_profile_id(
        merchant_key_store,
        current_business_profile,
        business_profile_update,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update routing algorithm ref in business profile")?;

    // Invalidate the routing cache for Payments and Payouts transaction types
    if !transaction_type.is_three_ds_authentication() {
        let routing_cache_key = cache::CacheKind::Routing(
            format!(
                "routing_config_{}_{}",
                merchant_id.get_string_repr(),
                profile_id.get_string_repr(),
            )
            .into(),
        );

        cache::redact_from_redis_and_publish(db.get_cache_store().as_ref(), [routing_cache_key])
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to invalidate routing cache")?;
    }
    Ok(())
}

#[cfg(feature = "v1")]
pub async fn update_business_profile_active_dynamic_algorithm_ref(
    db: &dyn StorageInterface,
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
    pub routing_algorithm: &'h routing_types::StaticRoutingAlgorithm,
}

#[cfg(feature = "v1")]
pub enum RoutingDecisionData {
    DebitRouting(DebitRoutingDecisionData),
}
#[cfg(feature = "v1")]
pub struct DebitRoutingDecisionData {
    pub card_network: common_enums::enums::CardNetwork,
    pub debit_routing_result: Option<open_router::DebitRoutingOutput>,
}
#[cfg(feature = "v1")]
impl RoutingDecisionData {
    pub fn apply_routing_decision<F, D>(&self, payment_data: &mut D)
    where
        F: Send + Clone,
        D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    {
        match self {
            Self::DebitRouting(data) => data.apply_debit_routing_decision(payment_data),
        }
    }

    pub fn get_debit_routing_decision_data(
        network: common_enums::enums::CardNetwork,
        debit_routing_result: Option<open_router::DebitRoutingOutput>,
    ) -> Self {
        Self::DebitRouting(DebitRoutingDecisionData {
            card_network: network,
            debit_routing_result,
        })
    }
}
#[cfg(feature = "v1")]
impl DebitRoutingDecisionData {
    pub fn apply_debit_routing_decision<F, D>(&self, payment_data: &mut D)
    where
        F: Send + Clone,
        D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    {
        payment_data.set_card_network(self.card_network.clone());
        self.debit_routing_result
            .as_ref()
            .map(|data| payment_data.set_co_badged_card_data(data));
    }
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
                        "connector with name '{connector_choice}' and merchant connector account id '{mca_id:?}' not found for the given profile",
                    )
                }
            );
        } else {
            let connector_choice = common_enums::connector_enums::Connector::from(choice.connector);
            error_stack::ensure!(
                self.name_set.0.contains(&connector_choice),
                errors::ApiErrorResponse::InvalidRequestData {
                    message: format!(
                        "connector with name '{connector_choice}' not found for the given profile",
                    )
                }
            );
        };
        Ok(())
    }

    pub fn validate_connectors_in_routing_config(&self) -> RouterResult<()> {
        match self.routing_algorithm {
            routing_types::StaticRoutingAlgorithm::Single(choice) => {
                self.connector_choice(choice)?;
            }

            routing_types::StaticRoutingAlgorithm::Priority(list) => {
                for choice in list {
                    self.connector_choice(choice)?;
                }
            }

            routing_types::StaticRoutingAlgorithm::VolumeSplit(splits) => {
                for split in splits {
                    self.connector_choice(&split.connector)?;
                }
            }

            routing_types::StaticRoutingAlgorithm::Advanced(program) => {
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

            routing_types::StaticRoutingAlgorithm::ThreeDsDecisionRule(_) => {
                return Err(errors::ApiErrorResponse::InternalServerError).attach_printable(
                    "Invalid routing algorithm three_ds decision rule received",
                )?;
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
    routing_algorithm: &routing_types::StaticRoutingAlgorithm,
) -> RouterResult<()> {
    let all_mcas = state
        .store
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
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
        routing_types::StaticRoutingAlgorithm::Single(choice) => {
            connector_choice(choice)?;
        }

        routing_types::StaticRoutingAlgorithm::Priority(list) => {
            for choice in list {
                connector_choice(choice)?;
            }
        }

        routing_types::StaticRoutingAlgorithm::VolumeSplit(splits) => {
            for split in splits {
                connector_choice(&split.connector)?;
            }
        }

        routing_types::StaticRoutingAlgorithm::Advanced(program) => {
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

        routing_types::StaticRoutingAlgorithm::ThreeDsDecisionRule(_) => {
            Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Invalid routing algorithm three_ds decision rule received")?
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
        storage::enums::TransactionType::ThreeDsAuthentication => {
            format!("three_ds_authentication_{merchant_id}")
        }
    }
}

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
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

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
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

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
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

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
#[async_trait::async_trait]
impl DynamicRoutingCache for routing_types::EliminationRoutingConfig {
    async fn get_cached_dynamic_routing_config_for_profile(
        state: &SessionState,
        key: &str,
    ) -> Option<Arc<Self>> {
        cache::ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE
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
            &cache::ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to populate ELIMINATION_BASED_DYNAMIC_ALGORITHM_CACHE")
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

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn update_gateway_score_helper_with_open_router(
    state: &SessionState,
    payment_attempt: &storage::PaymentAttempt,
    profile_id: &id_type::ProfileId,
    dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef,
) -> RouterResult<()> {
    let is_success_rate_routing_enabled =
        dynamic_routing_algo_ref.is_success_rate_routing_enabled();
    let is_elimination_enabled = dynamic_routing_algo_ref.is_elimination_enabled();

    if is_success_rate_routing_enabled || is_elimination_enabled {
        let payment_connector = &payment_attempt.connector.clone().ok_or(
            errors::ApiErrorResponse::GenericNotFoundError {
                message: "unable to derive payment connector from payment attempt".to_string(),
            },
        )?;

        let routable_connector = routing_types::RoutableConnectorChoice {
            choice_kind: api_models::routing::RoutableChoiceKind::FullStruct,
            connector: common_enums::RoutableConnectors::from_str(payment_connector.as_str())
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("unable to infer routable_connector from connector")?,
            merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
        };

        logger::debug!(
            "performing update-gateway-score for gateway with id {} in open_router for profile: {}",
            routable_connector,
            profile_id.get_string_repr()
        );
        routing::payments_routing::update_gateway_score_with_open_router(
            state,
            routable_connector.clone(),
            profile_id,
            &payment_attempt.merchant_id,
            &payment_attempt.payment_id,
            payment_attempt.status,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("failed to update gateway score in open_router service")?;
    }

    Ok(())
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
    if let Some(success_based_algo_ref) = dynamic_routing_algo_ref.success_based_algorithm {
        if success_based_algo_ref.enabled_feature != routing_types::DynamicRoutingFeatures::None {
            let client = &state
                .grpc_client
                .dynamic_routing
                .as_ref()
                .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "dynamic routing gRPC client not found".to_string(),
                })?
                .success_rate_client;

            let payment_connector = &payment_attempt.connector.clone().ok_or(
                errors::ApiErrorResponse::GenericNotFoundError {
                    message: "unable to derive payment connector from payment attempt".to_string(),
                },
            )?;

            let routable_connector = routing_types::RoutableConnectorChoice {
                choice_kind: api_models::routing::RoutableChoiceKind::FullStruct,
                connector: common_enums::RoutableConnectors::from_str(payment_connector.as_str())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("unable to infer routable_connector from connector")?,
                merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
            };

            let payment_status_attribute =
                get_desired_payment_status_for_dynamic_routing_metrics(payment_attempt.status);

            let success_based_routing_configs = fetch_dynamic_routing_configs::<
                routing_types::SuccessBasedRoutingConfig,
            >(
                state,
                profile_id,
                success_based_algo_ref
                    .algorithm_id_with_timestamp
                    .algorithm_id
                    .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
                        message: "success_rate algorithm_id not found".to_string(),
                    })
                    .attach_printable(
                        "success_based_routing_algorithm_id not found in business_profile",
                    )?,
            )
            .await
            .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                message: "success_rate based dynamic routing configs not found".to_string(),
            })
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

            let duplicate_stats = state
                .store
                .find_dynamic_routing_stats_optional_by_attempt_id_merchant_id(
                    payment_attempt.attempt_id.clone(),
                    &payment_attempt.merchant_id.to_owned(),
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to fetch dynamic_routing_stats entry")?;

            if duplicate_stats.is_some() {
                let dynamic_routing_update = DynamicRoutingStatsUpdate {
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
                    global_success_based_connector: Some(
                        first_global_success_based_connector.label.to_string(),
                    ),
                };

                state
                    .store
                    .update_dynamic_routing_stats(
                        payment_attempt.attempt_id.clone(),
                        &payment_attempt.merchant_id.to_owned(),
                        dynamic_routing_update,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to update dynamic routing stats to db")?;
            } else {
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

                state
                    .store
                    .insert_dynamic_routing_stat_entry(dynamic_routing_stats)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to push dynamic routing stats to db")?;
            };

            let label_with_status = routing_utils::UpdateLabelWithStatusEventRequest {
                label: routable_connector.clone().to_string(),
                status: payment_status_attribute == common_enums::AttemptStatus::Charged,
            };
            let event_request = routing_utils::UpdateSuccessRateWindowEventRequest {
                id: payment_attempt.profile_id.get_string_repr().to_string(),
                params: success_based_routing_config_params.clone(),
                labels_with_status: vec![label_with_status.clone()],
                global_labels_with_status: vec![label_with_status],
                config: success_based_routing_configs
                    .config
                    .as_ref()
                    .map(routing_utils::UpdateSuccessRateWindowConfig::from),
            };

            let routing_events_wrapper = routing_utils::RoutingEventsWrapper::new(
                state.tenant.tenant_id.clone(),
                state.request_id.clone(),
                payment_attempt.payment_id.get_string_repr().to_string(),
                profile_id.to_owned(),
                payment_attempt.merchant_id.to_owned(),
                "IntelligentRouter: UpdateSuccessRateWindow".to_string(),
                Some(event_request.clone()),
                true,
                false,
            );

            let closure = || async {
                let update_response_result = client
                    .update_success_rate(
                        profile_id.get_string_repr().into(),
                        success_based_routing_configs,
                        success_based_routing_config_params,
                        vec![routing_types::RoutableConnectorChoiceWithStatus::new(
                            routable_connector.clone(),
                            payment_status_attribute == common_enums::AttemptStatus::Charged,
                        )],
                        state.get_grpc_headers(),
                    )
                    .await
                    .change_context(errors::RoutingError::SuccessRateCalculationError)
                    .attach_printable(
                        "unable to update success based routing window in dynamic routing service",
                    );

                match update_response_result {
                    Ok(update_response) => {
                        let updated_resp =
                            routing_utils::UpdateSuccessRateWindowEventResponse::try_from(
                                &update_response,
                            )
                            .change_context(errors::RoutingError::RoutingEventsError { message: "Unable to convert to UpdateSuccessRateWindowEventResponse from UpdateSuccessRateWindowResponse".to_string(), status_code: 500 })?;
                        Ok(Some(updated_resp))
                    }
                    Err(err) => {
                        logger::error!(
                            "unable to update connector score in dynamic routing service: {:?}",
                            err.current_context()
                        );

                        Err(err)
                    }
                }
            };

            let events_response = routing_events_wrapper
                .construct_event_builder(
                    "SuccessRateCalculator.UpdateSuccessRateWindow".to_string(),
                    routing_events::RoutingEngine::IntelligentRouter,
                    routing_events::ApiMethod::Grpc,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "SR-Intelligent-Router: Failed to update success rate in Intelligent-Router",
                )?
                .trigger_event(state, closure)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "SR-Intelligent-Router: Failed to update success rate in Intelligent-Router",
                )?;

            let _response: routing_utils::UpdateSuccessRateWindowEventResponse = events_response
                .response
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "UpdateSuccessRateWindowEventResponse not found in RoutingEventResponse",
                )?;

            let mut routing_event = events_response
                .event
                .ok_or(errors::RoutingError::RoutingEventsError {
                    message:
                        "SR-Intelligent-Router: RoutingEvent not found in RoutingEventsResponse"
                            .to_string(),
                    status_code: 500,
                })
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "SR-Intelligent-Router: RoutingEvent not found in RoutingEventsResponse",
                )?;

            routing_event.set_status_code(200);
            routing_event.set_payment_connector(routable_connector); // we can do this inside the event wrap by implementing an interface on the req type
            state.event_handler().log_event(&routing_event);

            Ok(())
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

/// update window for elimination based dynamic routing
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn update_window_for_elimination_routing(
    state: &SessionState,
    payment_attempt: &storage::PaymentAttempt,
    profile_id: &id_type::ProfileId,
    dynamic_algo_ref: routing_types::DynamicRoutingAlgorithmRef,
    elimination_routing_configs_params_interpolator: DynamicRoutingConfigParamsInterpolator,
    gsm_error_category: common_enums::ErrorCategory,
) -> RouterResult<()> {
    if let Some(elimination_algo_ref) = dynamic_algo_ref.elimination_routing_algorithm {
        if elimination_algo_ref.enabled_feature != routing_types::DynamicRoutingFeatures::None {
            let client = &state
                .grpc_client
                .dynamic_routing
                .as_ref()
                .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "dynamic routing gRPC client not found".to_string(),
                })?
                .elimination_based_client;

            let elimination_routing_config = fetch_dynamic_routing_configs::<
                routing_types::EliminationRoutingConfig,
            >(
                state,
                profile_id,
                elimination_algo_ref
                    .algorithm_id_with_timestamp
                    .algorithm_id
                    .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
                        message: "elimination routing algorithm_id not found".to_string(),
                    })
                    .attach_printable(
                        "elimination_routing_algorithm_id not found in business_profile",
                    )?,
            )
            .await
            .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                message: "elimination based dynamic routing configs not found".to_string(),
            })
            .attach_printable("unable to retrieve success_rate based dynamic routing configs")?;

            let payment_connector = &payment_attempt.connector.clone().ok_or(
                errors::ApiErrorResponse::GenericNotFoundError {
                    message: "unable to derive payment connector from payment attempt".to_string(),
                },
            )?;

            let elimination_routing_config_params = elimination_routing_configs_params_interpolator
                .get_string_val(
                    elimination_routing_config
                        .params
                        .as_ref()
                        .ok_or(errors::RoutingError::EliminationBasedRoutingParamsNotFoundError)
                        .change_context(errors::ApiErrorResponse::InternalServerError)?,
                );

            let labels_with_bucket_name =
                vec![routing_types::RoutableConnectorChoiceWithBucketName::new(
                    routing_types::RoutableConnectorChoice {
                        choice_kind: api_models::routing::RoutableChoiceKind::FullStruct,
                        connector: common_enums::RoutableConnectors::from_str(
                            payment_connector.as_str(),
                        )
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("unable to infer routable_connector from connector")?,
                        merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
                    },
                    gsm_error_category.to_string(),
                )];

            let event_request = routing_utils::UpdateEliminationBucketEventRequest {
                id: profile_id.get_string_repr().to_string(),
                params: elimination_routing_config_params.clone(),
                labels_with_bucket_name: labels_with_bucket_name
                    .iter()
                    .map(|conn_choice| {
                        routing_utils::LabelWithBucketNameEventRequest::from(conn_choice)
                    })
                    .collect(),
                config: elimination_routing_config
                    .elimination_analyser_config
                    .as_ref()
                    .map(routing_utils::EliminationRoutingEventBucketConfig::from),
            };

            let routing_events_wrapper = routing_utils::RoutingEventsWrapper::new(
                state.tenant.tenant_id.clone(),
                state.request_id.clone(),
                payment_attempt.payment_id.get_string_repr().to_string(),
                profile_id.to_owned(),
                payment_attempt.merchant_id.to_owned(),
                "IntelligentRouter: UpdateEliminationBucket".to_string(),
                Some(event_request.clone()),
                true,
                false,
            );

            let closure = || async {
                let update_response_result = client
                .update_elimination_bucket_config(
                    profile_id.get_string_repr().to_string(),
                    elimination_routing_config_params,
                    labels_with_bucket_name,
                    elimination_routing_config.elimination_analyser_config,
                    state.get_grpc_headers(),
                )
                .await
                .change_context(errors::RoutingError::EliminationRoutingCalculationError)
                .attach_printable(
                    "unable to update elimination based routing buckets in dynamic routing service",
                );

                match update_response_result {
                    Ok(resp) => {
                        let updated_resp =
                            routing_utils::UpdateEliminationBucketEventResponse::try_from(&resp)
                            .change_context(errors::RoutingError::RoutingEventsError { message: "Unable to convert to UpdateEliminationBucketEventResponse from UpdateEliminationBucketResponse".to_string(), status_code: 500 })?;

                        Ok(Some(updated_resp))
                    }
                    Err(err) => {
                        logger::error!(
                            "unable to update elimination  score in dynamic routing service: {:?}",
                            err.current_context()
                        );

                        Err(err)
                    }
                }
            };

            let events_response = routing_events_wrapper.construct_event_builder( "EliminationAnalyser.UpdateEliminationBucket".to_string(),
            routing_events::RoutingEngine::IntelligentRouter,
            routing_events::ApiMethod::Grpc)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Elimination-Intelligent-Router: Failed to update elimination bucket in Intelligent-Router")?
            .trigger_event(state, closure)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Elimination-Intelligent-Router: Failed to update elimination bucket in Intelligent-Router")?;

            let _response: routing_utils::UpdateEliminationBucketEventResponse = events_response
                .response
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "UpdateEliminationBucketEventResponse not found in RoutingEventResponse",
                )?;

            let mut routing_event = events_response
                .event
                .ok_or(errors::RoutingError::RoutingEventsError {
                    message:
                        "Elimination-Intelligent-Router: RoutingEvent not found in RoutingEventsResponse"
                            .to_string(),
                    status_code: 500,
                })
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Elimination-Intelligent-Router: RoutingEvent not found in RoutingEventsResponse")?;

            routing_event.set_status_code(200);
            routing_event.set_payment_connector(routing_types::RoutableConnectorChoice {
                choice_kind: api_models::routing::RoutableChoiceKind::FullStruct,
                connector: common_enums::RoutableConnectors::from_str(payment_connector.as_str())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("unable to infer routable_connector from connector")?,
                merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
            });
            state.event_handler().log_event(&routing_event);
            Ok(())
        } else {
            Ok(())
        }
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
    if let Some(contract_routing_algo_ref) = dynamic_routing_algo_ref.contract_based_routing {
        if contract_routing_algo_ref.enabled_feature != routing_types::DynamicRoutingFeatures::None
        {
            let client = &state
                .grpc_client
                .dynamic_routing
                .as_ref()
                .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "dynamic routing gRPC client not found".to_string(),
                })?
                .contract_based_client;

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
                        .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
                            message: "contract_routing algorithm_id not found".to_string(),
                        })
                        .attach_printable(
                            "contract_based_routing_algorithm_id not found in business_profile",
                        )?,
                )
                .await
                .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "contract based dynamic routing configs not found".to_string(),
                })
                .attach_printable("unable to retrieve contract based dynamic routing configs")?;

            let mut existing_label_info = None;

            contract_based_routing_config
                .label_info
                .as_ref()
                .map(|label_info_vec| {
                    for label_info in label_info_vec {
                        if Some(&label_info.mca_id)
                            == payment_attempt.merchant_connector_id.as_ref()
                        {
                            existing_label_info = Some(label_info.clone());
                        }
                    }
                });

            let final_label_info = existing_label_info
                .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "LabelInformation from ContractBasedRoutingConfig not found"
                        .to_string(),
                })
                .attach_printable(
                    "unable to get LabelInformation from ContractBasedRoutingConfig",
                )?;

            logger::debug!(
                "contract based routing: matched LabelInformation - {:?}",
                final_label_info
            );

            let request_label_info = routing_types::LabelInformation {
                label: final_label_info.label.clone(),
                target_count: final_label_info.target_count,
                target_time: final_label_info.target_time,
                mca_id: final_label_info.mca_id.to_owned(),
            };

            let payment_status_attribute =
                get_desired_payment_status_for_dynamic_routing_metrics(payment_attempt.status);

            if payment_status_attribute == common_enums::AttemptStatus::Charged {
                let event_request = routing_utils::UpdateContractRequestEventRequest {
                    id: profile_id.get_string_repr().to_string(),
                    params: "".to_string(),
                    labels_information: vec![
                        routing_utils::ContractLabelInformationEventRequest::from(
                            &request_label_info,
                        ),
                    ],
                };

                let routing_events_wrapper = routing_utils::RoutingEventsWrapper::new(
                    state.tenant.tenant_id.clone(),
                    state.request_id.clone(),
                    payment_attempt.payment_id.get_string_repr().to_string(),
                    profile_id.to_owned(),
                    payment_attempt.merchant_id.to_owned(),
                    "IntelligentRouter: UpdateContractScore".to_string(),
                    Some(event_request.clone()),
                    true,
                    false,
                );

                let closure = || async {
                    let update_response_result = client
                    .update_contracts(
                        profile_id.get_string_repr().into(),
                        vec![request_label_info],
                        "".to_string(),
                        vec![],
                        1,
                        state.get_grpc_headers(),
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "unable to update contract based routing window in dynamic routing service",
                    );

                    match update_response_result {
                        Ok(resp) => {
                            let updated_resp =
                                routing_utils::UpdateContractEventResponse::try_from(&resp)
                                .change_context(errors::RoutingError::RoutingEventsError { message: "Unable to convert to UpdateContractEventResponse from UpdateContractResponse".to_string(), status_code: 500 })?;
                            Ok(Some(updated_resp))
                        }
                        Err(err) => {
                            logger::error!(
                                "unable to update elimination  score in dynamic routing service: {:?}",
                                err.current_context()
                            );

                            // have to refactor errors
                            Err(error_stack::report!(
                                errors::RoutingError::ContractScoreUpdationError
                            ))
                        }
                    }
                };

                let events_response = routing_events_wrapper.construct_event_builder( "ContractScoreCalculator.UpdateContract".to_string(),
                routing_events::RoutingEngine::IntelligentRouter,
                routing_events::ApiMethod::Grpc)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("ContractRouting-Intelligent-Router: Failed to construct RoutingEventsBuilder")?
                .trigger_event(state, closure)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("ContractRouting-Intelligent-Router: Failed to update contract scores in Intelligent-Router")?;

                let _response: routing_utils::UpdateContractEventResponse = events_response
                    .response
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "UpdateContractEventResponse not found in RoutingEventResponse",
                    )?;

                let mut routing_event = events_response
                    .event
                    .ok_or(errors::RoutingError::RoutingEventsError {
                        message:
                            "ContractRouting-Intelligent-Router: RoutingEvent not found in RoutingEventsResponse"
                                .to_string(),
                        status_code: 500,
                    })
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("ContractRouting-Intelligent-Router: RoutingEvent not found in RoutingEventsResponse")?;

                routing_event.set_payment_connector(routing_types::RoutableConnectorChoice {
                    choice_kind: api_models::routing::RoutableChoiceKind::FullStruct,
                    connector: common_enums::RoutableConnectors::from_str(
                        final_label_info.label.as_str(),
                    )
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("unable to infer routable_connector from connector")?,
                    merchant_connector_id: Some(final_label_info.mca_id.clone()),
                });
                routing_event.set_status_code(200);
                state.event_handler().log_event(&routing_event);
            }

            let contract_based_connectors = routable_connectors
                .into_iter()
                .filter(|conn| {
                    conn.merchant_connector_id.clone() == Some(final_label_info.mca_id.clone())
                })
                .collect::<Vec<_>>();

            let contract_scores = client
                .calculate_contract_score(
                    profile_id.get_string_repr().into(),
                    contract_based_routing_config.clone(),
                    "".to_string(),
                    contract_based_connectors,
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
                    "unable to split connector_name and mca_id from the first connector {first_contract_based_connector:?} obtained from dynamic routing service",

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
        | common_enums::AttemptStatus::PartialChargedAndChargeable
        | common_enums::AttemptStatus::PartiallyAuthorized => common_enums::AttemptStatus::Charged,
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
        | common_enums::AttemptStatus::VoidedPostCharge
        | common_enums::AttemptStatus::VoidInitiated
        | common_enums::AttemptStatus::CaptureInitiated
        | common_enums::AttemptStatus::VoidFailed
        | common_enums::AttemptStatus::AutoRefunded
        | common_enums::AttemptStatus::Unresolved
        | common_enums::AttemptStatus::Pending
        | common_enums::AttemptStatus::IntegrityFailure
        | common_enums::AttemptStatus::PaymentMethodAwaited
        | common_enums::AttemptStatus::ConfirmationAwaited
        | common_enums::AttemptStatus::DeviceDataCollectionPending
        | common_enums::AttemptStatus::Expired => common_enums::AttemptStatus::Pending,
    }
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
impl ForeignFrom<common_enums::AttemptStatus> for open_router::TxnStatus {
    fn foreign_from(attempt_status: common_enums::AttemptStatus) -> Self {
        match attempt_status {
            common_enums::AttemptStatus::Started => Self::Started,
            common_enums::AttemptStatus::AuthenticationFailed => Self::AuthenticationFailed,
            common_enums::AttemptStatus::RouterDeclined => Self::JuspayDeclined,
            common_enums::AttemptStatus::AuthenticationPending => Self::PendingVbv,
            common_enums::AttemptStatus::AuthenticationSuccessful => Self::VBVSuccessful,
            common_enums::AttemptStatus::Authorized
            | common_enums::AttemptStatus::PartiallyAuthorized => Self::Authorized,
            common_enums::AttemptStatus::AuthorizationFailed => Self::AuthorizationFailed,
            common_enums::AttemptStatus::Charged => Self::Charged,
            common_enums::AttemptStatus::Authorizing => Self::Authorizing,
            common_enums::AttemptStatus::CodInitiated => Self::CODInitiated,
            common_enums::AttemptStatus::Voided | common_enums::AttemptStatus::Expired => {
                Self::Voided
            }
            common_enums::AttemptStatus::VoidedPostCharge => Self::VoidedPostCharge,
            common_enums::AttemptStatus::VoidInitiated => Self::VoidInitiated,
            common_enums::AttemptStatus::CaptureInitiated => Self::CaptureInitiated,
            common_enums::AttemptStatus::CaptureFailed => Self::CaptureFailed,
            common_enums::AttemptStatus::VoidFailed => Self::VoidFailed,
            common_enums::AttemptStatus::AutoRefunded => Self::AutoRefunded,
            common_enums::AttemptStatus::PartialCharged => Self::PartialCharged,
            common_enums::AttemptStatus::PartialChargedAndChargeable => Self::ToBeCharged,
            common_enums::AttemptStatus::Unresolved => Self::Pending,
            common_enums::AttemptStatus::Pending
            | common_enums::AttemptStatus::IntegrityFailure => Self::Pending,
            common_enums::AttemptStatus::Failure => Self::Failure,
            common_enums::AttemptStatus::PaymentMethodAwaited => Self::Pending,
            common_enums::AttemptStatus::ConfirmationAwaited => Self::Pending,
            common_enums::AttemptStatus::DeviceDataCollectionPending => Self::Pending,
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
    let profile_id = business_profile.get_id().clone();
    let (algorithm_id, mut dynamic_routing_algorithm, cache_entries_to_redact) =
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
                        is_merchant_created_in_decision_engine: dynamic_routing_algo_ref
                            .is_merchant_created_in_decision_engine,
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
                        is_merchant_created_in_decision_engine: dynamic_routing_algo_ref
                            .is_merchant_created_in_decision_engine,
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
                        is_merchant_created_in_decision_engine: dynamic_routing_algo_ref
                            .is_merchant_created_in_decision_engine,
                    },
                    cache_entries_to_redact,
                )
            }
        };

    // Call to DE here
    if state.conf.open_router.dynamic_routing_enabled {
        disable_decision_engine_dynamic_routing_setup(
            state,
            business_profile.get_id(),
            dynamic_routing_type,
            &mut dynamic_routing_algorithm,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to disable dynamic routing setup in decision engine")?;
    }

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
    payload: Option<routing_types::DynamicRoutingPayload>,
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
                payload,
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
                payload,
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

#[allow(clippy::too_many_arguments)]
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn enable_specific_routing_algorithm<A>(
    state: &SessionState,
    key_store: domain::MerchantKeyStore,
    business_profile: domain::Profile,
    feature_to_enable: routing_types::DynamicRoutingFeatures,
    mut dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef,
    dynamic_routing_type: routing_types::DynamicRoutingType,
    algo_type: Option<A>,
    payload: Option<routing_types::DynamicRoutingPayload>,
) -> RouterResult<ApplicationResponse<routing_types::RoutingDictionaryRecord>>
where
    A: routing_types::DynamicRoutingAlgoAccessor + Clone + Debug,
{
    //Check for payload
    if let Some(payload) = payload {
        return create_specific_dynamic_routing_setup(
            state,
            key_store,
            business_profile,
            feature_to_enable,
            dynamic_routing_algo_ref,
            dynamic_routing_type,
            payload,
        )
        .await;
    }
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
        let routing_algorithm = db
            .find_routing_algorithm_by_profile_id_algorithm_id(&profile_id, &algo_type_algorithm_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
        let updated_routing_record = routing_algorithm.foreign_into();

        return Ok(ApplicationResponse::Json(updated_routing_record));
    };
    *algo_type_enabled_features = feature_to_enable;
    dynamic_routing_algo_ref.update_enabled_features(dynamic_routing_type, feature_to_enable);
    update_business_profile_active_dynamic_algorithm_ref(
        db,
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

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
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
    let profile_id = business_profile.get_id().clone();
    let merchant_id = business_profile.merchant_id.clone();
    let algorithm_id = common_utils::generate_routing_id_of_default_length();
    let timestamp = common_utils::date_time::now();

    let algo = match dynamic_routing_type {
        routing_types::DynamicRoutingType::SuccessRateBasedRouting => {
            let default_success_based_routing_config =
                if state.conf.open_router.dynamic_routing_enabled {
                    routing_types::SuccessBasedRoutingConfig::open_router_config_default()
                } else {
                    routing_types::SuccessBasedRoutingConfig::default()
                };

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
                decision_engine_routing_id: None,
            }
        }
        routing_types::DynamicRoutingType::EliminationRouting => {
            let default_elimination_routing_config =
                if state.conf.open_router.dynamic_routing_enabled {
                    routing_types::EliminationRoutingConfig::open_router_config_default()
                } else {
                    routing_types::EliminationRoutingConfig::default()
                };

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
                decision_engine_routing_id: None,
            }
        }

        routing_types::DynamicRoutingType::ContractBasedRouting => {
            return Err((errors::ApiErrorResponse::InvalidRequestData {
                message: "Contract routing cannot be set as default".to_string(),
            })
            .into())
        }
    };

    // Call to DE here
    // Need to map out the cases if this call should always be made or not
    if state.conf.open_router.dynamic_routing_enabled {
        enable_decision_engine_dynamic_routing_setup(
            state,
            business_profile.get_id(),
            dynamic_routing_type,
            &mut dynamic_routing_algo_ref,
            None,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to setup decision engine dynamic routing")?;
    }

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

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
#[instrument(skip_all)]
pub async fn create_specific_dynamic_routing_setup(
    state: &SessionState,
    key_store: domain::MerchantKeyStore,
    business_profile: domain::Profile,
    feature_to_enable: routing_types::DynamicRoutingFeatures,
    mut dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef,
    dynamic_routing_type: routing_types::DynamicRoutingType,
    payload: routing_types::DynamicRoutingPayload,
) -> RouterResult<ApplicationResponse<routing_types::RoutingDictionaryRecord>> {
    let db = state.store.as_ref();
    let profile_id = business_profile.get_id().clone();
    let merchant_id = business_profile.merchant_id.clone();
    let algorithm_id = common_utils::generate_routing_id_of_default_length();
    let timestamp = common_utils::date_time::now();

    let algo = match dynamic_routing_type {
        routing_types::DynamicRoutingType::SuccessRateBasedRouting => {
            let success_config = match &payload {
                routing_types::DynamicRoutingPayload::SuccessBasedRoutingPayload(config) => {
                    config.validate().change_context(
                        errors::ApiErrorResponse::InvalidRequestData {
                            message: "All fields in SuccessBasedRoutingConfig cannot be null"
                                .to_string(),
                        },
                    )?;
                    config
                }
                _ => {
                    return Err((errors::ApiErrorResponse::InvalidRequestData {
                        message: "Invalid payload type for Success Rate Based Routing".to_string(),
                    })
                    .into())
                }
            };

            routing_algorithm::RoutingAlgorithm {
                algorithm_id: algorithm_id.clone(),
                profile_id: profile_id.clone(),
                merchant_id,
                name: SUCCESS_BASED_DYNAMIC_ROUTING_ALGORITHM.to_string(),
                description: None,
                kind: diesel_models::enums::RoutingAlgorithmKind::Dynamic,
                algorithm_data: serde_json::json!(success_config),
                created_at: timestamp,
                modified_at: timestamp,
                algorithm_for: common_enums::TransactionType::Payment,
                decision_engine_routing_id: None,
            }
        }
        routing_types::DynamicRoutingType::EliminationRouting => {
            let elimination_config = match &payload {
                routing_types::DynamicRoutingPayload::EliminationRoutingPayload(config) => {
                    config.validate().change_context(
                        errors::ApiErrorResponse::InvalidRequestData {
                            message: "All fields in EliminationRoutingConfig cannot be null"
                                .to_string(),
                        },
                    )?;
                    config
                }
                _ => {
                    return Err((errors::ApiErrorResponse::InvalidRequestData {
                        message: "Invalid payload type for Elimination Routing".to_string(),
                    })
                    .into())
                }
            };

            routing_algorithm::RoutingAlgorithm {
                algorithm_id: algorithm_id.clone(),
                profile_id: profile_id.clone(),
                merchant_id,
                name: ELIMINATION_BASED_DYNAMIC_ROUTING_ALGORITHM.to_string(),
                description: None,
                kind: diesel_models::enums::RoutingAlgorithmKind::Dynamic,
                algorithm_data: serde_json::json!(elimination_config),
                created_at: timestamp,
                modified_at: timestamp,
                algorithm_for: common_enums::TransactionType::Payment,
                decision_engine_routing_id: None,
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

    dynamic_routing_algo_ref.update_feature(feature_to_enable, dynamic_routing_type);
    update_business_profile_active_dynamic_algorithm_ref(
        db,
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

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
#[instrument(skip_all)]
pub async fn enable_decision_engine_dynamic_routing_setup(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
    dynamic_routing_type: routing_types::DynamicRoutingType,
    dynamic_routing_algo_ref: &mut routing_types::DynamicRoutingAlgorithmRef,
    payload: Option<routing_types::DynamicRoutingPayload>,
) -> RouterResult<()> {
    logger::debug!(
        "performing call with open_router for profile {}",
        profile_id.get_string_repr()
    );

    let decision_engine_config_request = match dynamic_routing_type {
        routing_types::DynamicRoutingType::SuccessRateBasedRouting => {
            let success_based_routing_config = payload
                .and_then(|p| match p {
                    routing_types::DynamicRoutingPayload::SuccessBasedRoutingPayload(config) => {
                        Some(config)
                    }
                    _ => None,
                })
                .unwrap_or_else(
                    routing_types::SuccessBasedRoutingConfig::open_router_config_default,
                );

            open_router::DecisionEngineConfigSetupRequest {
                merchant_id: profile_id.get_string_repr().to_string(),
                config: open_router::DecisionEngineConfigVariant::SuccessRate(
                    success_based_routing_config
                        .get_decision_engine_configs()
                        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                            message: "Decision engine config not found".to_string(),
                        })
                        .attach_printable("Decision engine config not found")?,
                ),
            }
        }
        routing_types::DynamicRoutingType::EliminationRouting => {
            let elimination_based_routing_config = payload
                .and_then(|p| match p {
                    routing_types::DynamicRoutingPayload::EliminationRoutingPayload(config) => {
                        Some(config)
                    }
                    _ => None,
                })
                .unwrap_or_else(
                    routing_types::EliminationRoutingConfig::open_router_config_default,
                );

            open_router::DecisionEngineConfigSetupRequest {
                merchant_id: profile_id.get_string_repr().to_string(),
                config: open_router::DecisionEngineConfigVariant::Elimination(
                    elimination_based_routing_config
                        .get_decision_engine_configs()
                        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                            message: "Decision engine config not found".to_string(),
                        })
                        .attach_printable("Decision engine config not found")?,
                ),
            }
        }
        routing_types::DynamicRoutingType::ContractBasedRouting => {
            return Err((errors::ApiErrorResponse::InvalidRequestData {
                message: "Contract routing cannot be set as default".to_string(),
            })
            .into())
        }
    };

    // Create merchant in Decision Engine if it is not already created
    create_merchant_in_decision_engine_if_not_exists(state, profile_id, dynamic_routing_algo_ref)
        .await;

    routing_utils::ConfigApiClient::send_decision_engine_request::<_, serde_json::Value>(
        state,
        services::Method::Post,
        DECISION_ENGINE_RULE_CREATE_ENDPOINT,
        Some(decision_engine_config_request),
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Unable to setup decision engine dynamic routing")?;

    Ok(())
}

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
#[instrument(skip_all)]
pub async fn update_decision_engine_dynamic_routing_setup(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
    request: serde_json::Value,
    dynamic_routing_type: routing_types::DynamicRoutingType,
    dynamic_routing_algo_ref: &mut routing_types::DynamicRoutingAlgorithmRef,
) -> RouterResult<()> {
    logger::debug!(
        "performing call with open_router for profile {}",
        profile_id.get_string_repr()
    );

    let decision_engine_request = match dynamic_routing_type {
        routing_types::DynamicRoutingType::SuccessRateBasedRouting => {
            let success_rate_config: routing_types::SuccessBasedRoutingConfig = request
                .parse_value("SuccessBasedRoutingConfig")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("unable to deserialize SuccessBasedRoutingConfig")?;

            open_router::DecisionEngineConfigSetupRequest {
                merchant_id: profile_id.get_string_repr().to_string(),
                config: open_router::DecisionEngineConfigVariant::SuccessRate(
                    success_rate_config
                        .get_decision_engine_configs()
                        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                            message: "Decision engine config not found".to_string(),
                        })
                        .attach_printable("Decision engine config not found")?,
                ),
            }
        }
        routing_types::DynamicRoutingType::EliminationRouting => {
            let elimination_config: routing_types::EliminationRoutingConfig = request
                .parse_value("EliminationRoutingConfig")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("unable to deserialize EliminationRoutingConfig")?;

            open_router::DecisionEngineConfigSetupRequest {
                merchant_id: profile_id.get_string_repr().to_string(),
                config: open_router::DecisionEngineConfigVariant::Elimination(
                    elimination_config
                        .get_decision_engine_configs()
                        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                            message: "Decision engine config not found".to_string(),
                        })
                        .attach_printable("Decision engine config not found")?,
                ),
            }
        }
        routing_types::DynamicRoutingType::ContractBasedRouting => {
            return Err((errors::ApiErrorResponse::InvalidRequestData {
                message: "Contract routing cannot be set as default".to_string(),
            })
            .into())
        }
    };

    // Create merchant in Decision Engine if it is not already created
    create_merchant_in_decision_engine_if_not_exists(state, profile_id, dynamic_routing_algo_ref)
        .await;

    routing_utils::ConfigApiClient::send_decision_engine_request::<_, serde_json::Value>(
        state,
        services::Method::Post,
        DECISION_ENGINE_RULE_UPDATE_ENDPOINT,
        Some(decision_engine_request),
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Unable to update decision engine dynamic routing")?;

    Ok(())
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn get_decision_engine_active_dynamic_routing_algorithm(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
    dynamic_routing_type: open_router::DecisionEngineDynamicAlgorithmType,
) -> RouterResult<Option<open_router::DecisionEngineConfigSetupRequest>> {
    logger::debug!(
        "decision_engine_euclid: GET api call for decision active {:?} routing algorithm",
        dynamic_routing_type
    );
    let request = open_router::GetDecisionEngineConfigRequest {
        merchant_id: profile_id.get_string_repr().to_owned(),
        algorithm: dynamic_routing_type,
    };
    let response: Option<open_router::DecisionEngineConfigSetupRequest> =
        routing_utils::ConfigApiClient::send_decision_engine_request(
            state,
            services::Method::Post,
            DECISION_ENGINE_RULE_GET_ENDPOINT,
            Some(request),
            None,
            None,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get active dynamic algorithm from decision engine")?
        .response;

    Ok(response)
}

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
#[instrument(skip_all)]
pub async fn disable_decision_engine_dynamic_routing_setup(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
    dynamic_routing_type: routing_types::DynamicRoutingType,
    dynamic_routing_algo_ref: &mut routing_types::DynamicRoutingAlgorithmRef,
) -> RouterResult<()> {
    logger::debug!(
        "performing call with open_router for profile {}",
        profile_id.get_string_repr()
    );

    let decision_engine_request = open_router::FetchRoutingConfig {
        merchant_id: profile_id.get_string_repr().to_string(),
        algorithm: match dynamic_routing_type {
            routing_types::DynamicRoutingType::SuccessRateBasedRouting => {
                open_router::AlgorithmType::SuccessRate
            }
            routing_types::DynamicRoutingType::EliminationRouting => {
                open_router::AlgorithmType::Elimination
            }
            routing_types::DynamicRoutingType::ContractBasedRouting => {
                return Err((errors::ApiErrorResponse::InvalidRequestData {
                    message: "Contract routing is not enabled for decision engine".to_string(),
                })
                .into())
            }
        },
    };

    // Create merchant in Decision Engine if it is not already created
    create_merchant_in_decision_engine_if_not_exists(state, profile_id, dynamic_routing_algo_ref)
        .await;

    routing_utils::ConfigApiClient::send_decision_engine_request::<_, serde_json::Value>(
        state,
        services::Method::Post,
        DECISION_ENGINE_RULE_DELETE_ENDPOINT,
        Some(decision_engine_request),
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Unable to disable decision engine dynamic routing")?;

    Ok(())
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn create_merchant_in_decision_engine_if_not_exists(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
    dynamic_routing_algo_ref: &mut routing_types::DynamicRoutingAlgorithmRef,
) {
    if !dynamic_routing_algo_ref.is_merchant_created_in_decision_engine {
        logger::debug!(
            "Creating merchant_account in decision engine for profile {}",
            profile_id.get_string_repr()
        );

        create_decision_engine_merchant(state, profile_id)
            .await
            .map_err(|err| {
                logger::warn!("Merchant creation error in decision_engine: {err:?}");
            })
            .ok();

        // TODO: Update the status based on the status code or error message from the API call
        dynamic_routing_algo_ref.update_merchant_creation_status_in_decision_engine(true);
    }
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn create_decision_engine_merchant(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
) -> RouterResult<()> {
    let merchant_account_req = open_router::MerchantAccount {
        merchant_id: profile_id.get_string_repr().to_string(),
        gateway_success_rate_based_decider_input: None,
    };

    routing_utils::ConfigApiClient::send_decision_engine_request::<_, serde_json::Value>(
        state,
        services::Method::Post,
        DECISION_ENGINE_MERCHANT_CREATE_ENDPOINT,
        Some(merchant_account_req),
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to create merchant account on decision engine")?;

    Ok(())
}

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
#[instrument(skip_all)]
pub async fn delete_decision_engine_merchant(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
) -> RouterResult<()> {
    let path = format!(
        "{}/{}",
        DECISION_ENGINE_MERCHANT_BASE_ENDPOINT,
        profile_id.get_string_repr()
    );
    routing_utils::ConfigApiClient::send_decision_engine_request::<_, serde_json::Value>(
        state,
        services::Method::Delete,
        &path,
        None::<id_type::ProfileId>,
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to delete merchant account on decision engine")?;

    Ok(())
}

pub async fn redact_cgraph_cache(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    profile_id: &id_type::ProfileId,
) -> RouterResult<()> {
    let cgraph_payouts_key = format!(
        "cgraph_po_{}_{}",
        merchant_id.get_string_repr(),
        profile_id.get_string_repr(),
    );

    let cgraph_payments_key = format!(
        "cgraph_{}_{}",
        merchant_id.get_string_repr(),
        profile_id.get_string_repr(),
    );

    let config_payouts_key = cache::CacheKind::CGraph(cgraph_payouts_key.clone().into());
    let config_payments_key = cache::CacheKind::CGraph(cgraph_payments_key.clone().into());
    cache::redact_from_redis_and_publish(
        state.store.get_cache_store().as_ref(),
        [config_payouts_key, config_payments_key],
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to invalidate the cgraph cache")?;

    Ok(())
}

pub async fn redact_routing_cache(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    profile_id: &id_type::ProfileId,
) -> RouterResult<()> {
    let routing_payments_key = format!(
        "routing_config_{}_{}",
        merchant_id.get_string_repr(),
        profile_id.get_string_repr(),
    );
    let routing_payouts_key = format!(
        "routing_config_po_{}_{}",
        merchant_id.get_string_repr(),
        profile_id.get_string_repr(),
    );

    let routing_payouts_cache_key = cache::CacheKind::Routing(routing_payouts_key.clone().into());
    let routing_payments_cache_key = cache::CacheKind::CGraph(routing_payments_key.clone().into());
    cache::redact_from_redis_and_publish(
        state.store.get_cache_store().as_ref(),
        [routing_payouts_cache_key, routing_payments_cache_key],
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to invalidate the routing cache")?;

    Ok(())
}
