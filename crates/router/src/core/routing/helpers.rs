//! Analysis for usage of all helper functions for use case of routing
//!
//! Functions that are used to perform the retrieval of merchant's
//! routing dict, configs, defaults
use std::sync::Arc;
use std::str::FromStr;

use api_models::routing as routing_types;
use common_utils::{
    ext_traits::{Encode, ValueExt},
    id_type,
    types::keymanager::KeyManagerState,
};
use diesel_models::configs;
use error_stack::ResultExt;
use external_services::grpc_client::dynamic_routing::SuccessBasedDynamicRouting;
use router_env::metrics::add_attributes;
use rustc_hash::FxHashSet;
use storage_impl::redis::cache;

#[cfg(feature = "v2")]
use crate::types::domain::MerchantConnectorAccount;
use crate::{
    core::{
        errors::{self, RouterResult},
        metrics as core_metrics,
    },
    db::StorageInterface,
    routes::{metrics, SessionState},
    types::{domain, storage},
    utils::StringExt,
};

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

    cache::publish_into_redact_channel(db.get_cache_store().as_ref(), [config_key])
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to invalidate the config cache")?;

    Ok(())
}

#[cfg(feature = "v2")]
pub async fn update_merchant_active_algorithm_ref(
    _state: &SessionState,
    _key_store: &domain::MerchantKeyStore,
    _config_key: cache::CacheKind<'_>,
    _algorithm_id: routing_types::RoutingAlgorithmRef,
) -> RouterResult<()> {
    // TODO: handle updating the active routing algorithm for v2 in merchant account
    todo!()
}

#[cfg(feature = "v1")]
pub async fn update_business_profile_active_algorithm_ref(
    db: &dyn StorageInterface,
    key_manager_state: &KeyManagerState,
    merchant_key_store: &domain::MerchantKeyStore,
    current_business_profile: domain::BusinessProfile,
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

    let business_profile_update = domain::BusinessProfileUpdate::RoutingAlgorithmUpdate {
        routing_algorithm,
        payout_routing_algorithm,
    };

    db.update_business_profile_by_profile_id(
        key_manager_state,
        merchant_key_store,
        current_business_profile,
        business_profile_update,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update routing algorithm ref in business profile")?;

    cache::publish_into_redact_channel(db.get_cache_store().as_ref(), [routing_cache_key])
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
    current_business_profile: domain::BusinessProfile,
    dynamic_routing_algorithm: routing_types::DynamicRoutingAlgorithmRef,
) -> RouterResult<()> {
    let ref_val = dynamic_routing_algorithm
        .encode_to_value()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert dynamic routing ref to value")?;
    let business_profile_update = domain::BusinessProfileUpdate::DynamicRoutingAlgorithmUpdate {
        dynamic_routing_algorithm: Some(ref_val),
    };
    db.update_business_profile_by_profile_id(
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
    pub FxHashSet<(&'a String, id_type::MerchantConnectorAccountId)>,
);
#[derive(Clone, Debug)]
pub struct ConnectNameForProfile<'a>(pub FxHashSet<&'a String>);

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
impl<'h> RoutingAlgorithmHelpers<'h> {
    fn connector_choice(
        &self,
        choice: &routing_types::RoutableConnectorChoice,
    ) -> RouterResult<()> {
        if let Some(ref mca_id) = choice.merchant_connector_id {
            error_stack::ensure!(
                    self.name_mca_id_set.0.contains(&(&choice.connector.to_string(), mca_id.clone())),
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
                self.name_set.0.contains(&choice.connector.to_string()),
                errors::ApiErrorResponse::InvalidRequestData {
                    message: format!(
                        "connector with name '{}' not found for the given profile",
                        choice.connector,
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

pub async fn get_dynamic_routing_cached_config_for_profile<'a>(
    state: &SessionState,
    key: &str,
) -> Option<Arc<routing_types::SuccessBasedRoutingConfig>> {
    cache::DYNAMIC_ALGORITHM_CACHE
        .get_val::<Arc<routing_types::SuccessBasedRoutingConfig>>(cache::CacheKey {
            key: key.to_string(),
            prefix: state.tenant.redis_key_prefix.clone(),
        })
        .await
}

pub async fn refresh_success_based_routing_cache(
    state: &SessionState,
    key: &str,
    success_based_routing_config: routing_types::SuccessBasedRoutingConfig,
) -> Arc<routing_types::SuccessBasedRoutingConfig> {
    let config = Arc::new(success_based_routing_config);
    cache::DYNAMIC_ALGORITHM_CACHE
        .push(
            cache::CacheKey {
                key: key.to_string(),
                prefix: state.tenant.redis_key_prefix.clone(),
            },
            config.clone(),
        )
        .await;
    config
}

pub async fn fetch_and_cache_dynamic_routing_configs(
    state: &SessionState,
    business_profile: &domain::BusinessProfile,
) -> RouterResult<routing_types::SuccessBasedRoutingConfig> {
    let dynamic_routing_algorithm = business_profile.dynamic_routing_algorithm.clone().ok_or(
        errors::ApiErrorResponse::GenericNotFoundError {
            message: "unable to find dynamic_routing_algorithm in business profile".to_string(),
        },
    )?;

    let dynamic_routing_algorithm_ref = dynamic_routing_algorithm
        .parse_value::<routing_types::DynamicRoutingAlgorithmRef>("DynamicRoutingAlgorithmRef")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to parse dynamic_algorithm_ref")?;

    let success_based_routing_id = dynamic_routing_algorithm_ref
        .clone()
        .success_based_algorithm
        .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
            message: "unable to find success_based_algorithm in dynamic algorithm ref".to_string(),
        })?
        .algorithm_id
        .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
            message: "unable to find algorithm id in success based algorithm".to_string(),
        })?;

    let key = format!(
        "{}_{}",
        business_profile.get_id().get_string_repr(),
        success_based_routing_id.get_string_repr()
    );

    if let Some(config) = get_dynamic_routing_cached_config_for_profile(state, key.as_str()).await {
        Ok(config.as_ref().clone())
    } else {
        let success_rate_algorithm = state
            .store
            .find_routing_algorithm_by_profile_id_algorithm_id(
                business_profile.get_id(),
                &success_based_routing_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::ResourceIdNotFound)
            .attach_printable("unable to find success_rate_algorithm for profile")?;

        let success_rate_config = success_rate_algorithm
            .algorithm_data
            .parse_value::<routing_types::SuccessBasedRoutingConfig>("SuccessBasedRoutingConfig")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to parse success_based_routing_config struct")?;

        refresh_success_based_routing_cache(state, key.as_str(), success_rate_config.clone()).await;

        Ok(success_rate_config)
    }
}

#[cfg(feature = "dynamic_routing")]
pub async fn metrics_for_success_based_routing(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    attempt_status: &common_enums::AttemptStatus,
    merchant_connector_id: &Option<id_type::MerchantConnectorAccountId>,
    profile_id: &id_type::ProfileId,
    attempt_connector: &Option<String>,
    routable_connectors: Vec<routing_types::RoutableConnectorChoice>,
) -> RouterResult<()> {
    let key_manager_state = &state.into();
    let business_profile = state
        .store
        .find_business_profile_by_profile_id(key_manager_state, key_store, profile_id)
        .await
        .change_context(errors::ApiErrorResponse::BusinessProfileNotFound {
            id: profile_id.clone().get_string_repr().to_owned(),
        })?;

    if let Some(payment_connector) = attempt_connector {
        if let Some(client) = state
            .grpc_client
            .dynamic_routing
            .success_rate_client
            .as_ref()
        {
            let default_success_based_routing_configs =
                fetch_and_cache_dynamic_routing_configs(state, &business_profile)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to derive the routing configs")?;

            let success_based_connectors = client
                .calculate_success_rate(
                    business_profile
                        .get_id()
                        .clone()
                        .get_string_repr()
                        .to_string(),
                    default_success_based_routing_configs.clone(),
                    routable_connectors.clone(),
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to get the success based connectors")?;

            let payment_status_attribute = match &attempt_status {
                common_enums::AttemptStatus::Charged | common_enums::AttemptStatus::Authorized => {
                    common_enums::AttemptStatus::Charged
                }
                common_enums::AttemptStatus::Failure
                | common_enums::AttemptStatus::AuthorizationFailed
                | common_enums::AttemptStatus::AuthenticationFailed
                | common_enums::AttemptStatus::CaptureFailed
                | common_enums::AttemptStatus::RouterDeclined => {
                    common_enums::AttemptStatus::Failure
                }
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
                | common_enums::AttemptStatus::PartialCharged
                | common_enums::AttemptStatus::PartialChargedAndChargeable
                | common_enums::AttemptStatus::Unresolved
                | common_enums::AttemptStatus::Pending
                | common_enums::AttemptStatus::PaymentMethodAwaited
                | common_enums::AttemptStatus::ConfirmationAwaited
                | common_enums::AttemptStatus::DeviceDataCollectionPending => {
                    common_enums::AttemptStatus::Pending
                }
            };

            let success_based_routing_first_connector_choice_attribute = &success_based_connectors
                .labels_with_score
                .first()
                .ok_or(errors::ApiErrorResponse::InternalServerError)?
                .label;
            let outcome = match payment_status_attribute {
                common_enums::AttemptStatus::Charged
                    if *success_based_routing_first_connector_choice_attribute
                        == *payment_connector =>
                {
                    common_enums::SuccessBasedRoutingConclusiveState::TruePositive
                }
                common_enums::AttemptStatus::Charged
                    if *success_based_routing_first_connector_choice_attribute
                        != *payment_connector =>
                {
                    common_enums::SuccessBasedRoutingConclusiveState::FalsePositive
                }
                common_enums::AttemptStatus::Failure
                    if *success_based_routing_first_connector_choice_attribute
                        == *payment_connector =>
                {
                    common_enums::SuccessBasedRoutingConclusiveState::TrueNegative
                }
                common_enums::AttemptStatus::Failure
                    if *success_based_routing_first_connector_choice_attribute
                        != *payment_connector =>
                {
                    common_enums::SuccessBasedRoutingConclusiveState::FalseNegative
                }
                _ => common_enums::SuccessBasedRoutingConclusiveState::NonDeterministic,
            };

            core_metrics::DYNAMIC_SUCCESS_BASED_ROUTING.add(
                &metrics::CONTEXT,
                1,
                &add_attributes([
                    (
                        "success_based_routing_connector",
                        success_based_routing_first_connector_choice_attribute.to_string(),
                    ),
                    ("payment_connector", payment_connector.to_string()),
                    ("payment_status", attempt_status.clone().to_string()),
                    ("conclusive_classification", outcome.to_string()),
                ]),
            );
            client
                .update_success_rate(
                    profile_id.clone().get_string_repr().to_string(),
                    default_success_based_routing_configs,
                    vec![routing_types::RoutableConnectorChoiceWithStatus::new(
                        routing_types::RoutableConnectorChoice {
                            choice_kind: api_models::routing::RoutableChoiceKind::FullStruct,
                            connector: common_enums::RoutableConnectors::from_str(
                                payment_connector.as_str(),
                            )
                            .change_context(errors::ApiErrorResponse::InternalServerError)?,
                            merchant_connector_id: merchant_connector_id.clone(),
                        },
                        payment_status_attribute == common_enums::AttemptStatus::Charged,
                    )],
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to get the success based connectors")?;
        }
    };
    Ok(())
}
