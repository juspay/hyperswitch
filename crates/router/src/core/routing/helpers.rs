//! Analysis for usage of all helper functions for use case of routing
//!
//! Functions that are used to perform the retrieval of merchant's
//! routing dict, configs, defaults
use api_models::routing as routing_types;
use common_utils::ext_traits::Encode;
use diesel_models::{
    business_profile::{BusinessProfile, BusinessProfileUpdateInternal},
    configs,
};
use error_stack::ResultExt;
use rustc_hash::FxHashSet;

use crate::{
    core::errors::{self, RouterResult},
    db::StorageInterface,
    types::{domain, storage},
    utils::{self, StringExt},
};

/// provides the complete merchant routing dictionary that is basically a list of all the routing
/// configs a merchant configured with an active_id field that specifies the current active routing
/// config
pub async fn get_merchant_routing_dictionary(
    db: &dyn StorageInterface,
    merchant_id: &str,
) -> RouterResult<routing_types::RoutingDictionary> {
    let key = get_routing_dictionary_key(merchant_id);
    let maybe_dict = db.find_config_by_key(&key).await;

    match maybe_dict {
        Ok(config) => config
            .config
            .parse_struct("RoutingDictionary")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Merchant routing dictionary has invalid structure"),

        Err(e) if e.current_context().is_db_not_found() => {
            let new_dictionary = routing_types::RoutingDictionary {
                merchant_id: merchant_id.to_string(),
                active_id: None,
                records: Vec::new(),
            };

            let serialized =
                utils::Encode::<routing_types::RoutingDictionary>::encode_to_string_of_json(
                    &new_dictionary,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error serializing newly created merchant dictionary")?;

            let new_config = configs::ConfigNew {
                key,
                config: serialized,
            };

            db.insert_config(new_config)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error inserting new routing dictionary for merchant")?;

            Ok(new_dictionary)
        }

        Err(e) => Err(e)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error fetching routing dictionary for merchant"),
    }
}

/// Provides us with all the configured configs of the Merchant in the ascending time configured
/// manner and chooses the first of them
pub async fn get_merchant_default_config(
    db: &dyn StorageInterface,
    merchant_id: &str,
) -> RouterResult<Vec<routing_types::RoutableConnectorChoice>> {
    let key = get_default_config_key(merchant_id);
    let maybe_config = db.find_config_by_key(&key).await;

    match maybe_config {
        Ok(config) => config
            .config
            .parse_struct("Vec<RoutableConnectors>")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Merchant default config has invalid structure"),

        Err(e) if e.current_context().is_db_not_found() => {
            let new_config_conns = Vec::<routing_types::RoutableConnectorChoice>::new();
            let serialized =
                utils::Encode::<Vec<routing_types::RoutableConnectorChoice>>::encode_to_string_of_json(
                    &new_config_conns,
                )
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
) -> RouterResult<()> {
    let key = get_default_config_key(merchant_id);
    let config_str =
        Encode::<Vec<routing_types::RoutableConnectorChoice>>::encode_to_string_of_json(
            &connectors,
        )
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
    let dictionary_str =
        Encode::<routing_types::RoutingDictionary>::encode_to_string_of_json(&dictionary)
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

pub async fn update_routing_algorithm(
    db: &dyn StorageInterface,
    algorithm_id: String,
    algorithm: routing_types::RoutingAlgorithm,
) -> RouterResult<()> {
    let algorithm_str =
        Encode::<routing_types::RoutingAlgorithm>::encode_to_string_of_json(&algorithm)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to serialize routing algorithm to string")?;

    let config_update = configs::ConfigUpdate::Update {
        config: Some(algorithm_str),
    };

    db.update_config_by_key(&algorithm_id, config_update)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating the routing algorithm in DB")?;

    Ok(())
}

/// This will help make one of all configured algorithms to be in active state for a particular
/// merchant
pub async fn update_merchant_active_algorithm_ref(
    db: &dyn StorageInterface,
    key_store: &domain::MerchantKeyStore,
    algorithm_id: routing_types::RoutingAlgorithmRef,
) -> RouterResult<()> {
    let ref_value = Encode::<routing_types::RoutingAlgorithmRef>::encode_to_value(&algorithm_id)
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
    };

    db.update_specific_fields_in_merchant(
        &key_store.merchant_id,
        merchant_account_update,
        key_store,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update routing algorithm ref in merchant account")?;

    Ok(())
}

pub async fn update_business_profile_active_algorithm_ref(
    db: &dyn StorageInterface,
    current_business_profile: BusinessProfile,
    algorithm_id: routing_types::RoutingAlgorithmRef,
) -> RouterResult<()> {
    let ref_val = Encode::<routing_types::RoutingAlgorithmRef>::encode_to_value(&algorithm_id)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert routing ref to value")?;

    let business_profile_update = BusinessProfileUpdateInternal {
        profile_name: None,
        return_url: None,
        enable_payment_response_hash: None,
        payment_response_hash_key: None,
        redirect_to_merchant_with_http_post: None,
        webhook_details: None,
        metadata: None,
        routing_algorithm: Some(ref_val),
        intent_fulfillment_time: None,
        frm_routing_algorithm: None,
        payout_routing_algorithm: None,
        applepay_verified_domains: None,
        modified_at: None,
        is_recon_enabled: None,
    };
    db.update_business_profile_by_profile_id(current_business_profile, business_profile_update)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update routing algorithm ref in business profile")?;
    Ok(())
}

pub async fn get_merchant_connector_agnostic_mandate_config(
    db: &dyn StorageInterface,
    merchant_id: &str,
) -> RouterResult<Vec<routing_types::DetailedConnectorChoice>> {
    let key = get_pg_agnostic_mandate_config_key(merchant_id);
    let maybe_config = db.find_config_by_key(&key).await;

    match maybe_config {
        Ok(config) => config
            .config
            .parse_struct("Vec<DetailedConnectorChoice>")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("pg agnostic mandate config has invalid structure"),

        Err(e) if e.current_context().is_db_not_found() => {
            let new_mandate_config: Vec<routing_types::DetailedConnectorChoice> = Vec::new();

            let serialized =
                utils::Encode::<Vec<routing_types::DetailedConnectorChoice>>::encode_to_string_of_json(
                    &new_mandate_config,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("error serializing newly created pg agnostic mandate config")?;

            let new_config = configs::ConfigNew {
                key,
                config: serialized,
            };

            db.insert_config(new_config)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("error inserting new pg agnostic mandate config in db")?;

            Ok(new_mandate_config)
        }

        Err(e) => Err(e)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("error fetching pg agnostic mandate config for merchant from db"),
    }
}

pub async fn update_merchant_connector_agnostic_mandate_config(
    db: &dyn StorageInterface,
    merchant_id: &str,
    mandate_config: Vec<routing_types::DetailedConnectorChoice>,
) -> RouterResult<Vec<routing_types::DetailedConnectorChoice>> {
    let key = get_pg_agnostic_mandate_config_key(merchant_id);
    let mandate_config_str =
        Encode::<Vec<routing_types::DetailedConnectorChoice>>::encode_to_string_of_json(
            &mandate_config,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to serialize pg agnostic mandate config during update")?;

    let config_update = configs::ConfigUpdate::Update {
        config: Some(mandate_config_str),
    };

    db.update_config_by_key(&key, config_update)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error saving pg agnostic mandate config to db")?;

    Ok(mandate_config)
}

pub async fn validate_connectors_in_routing_config(
    db: &dyn StorageInterface,
    key_store: &domain::MerchantKeyStore,
    merchant_id: &str,
    profile_id: &str,
    routing_algorithm: &routing_types::RoutingAlgorithm,
) -> RouterResult<()> {
    let all_mcas = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            merchant_id,
            true,
            key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_id.to_string(),
        })?;

    #[cfg(feature = "connector_choice_mca_id")]
    let name_mca_id_set = all_mcas
        .iter()
        .filter(|mca| mca.profile_id.as_deref() == Some(profile_id))
        .map(|mca| (&mca.connector_name, &mca.merchant_connector_id))
        .collect::<FxHashSet<_>>();

    let name_set = all_mcas
        .iter()
        .filter(|mca| mca.profile_id.as_deref() == Some(profile_id))
        .map(|mca| &mca.connector_name)
        .collect::<FxHashSet<_>>();

    #[cfg(feature = "connector_choice_mca_id")]
    let check_connector_choice = |choice: &routing_types::RoutableConnectorChoice| {
        if let Some(ref mca_id) = choice.merchant_connector_id {
            error_stack::ensure!(
                name_mca_id_set.contains(&(&choice.connector.to_string(), mca_id)),
                errors::ApiErrorResponse::InvalidRequestData {
                    message: format!(
                        "connector with name '{}' and merchant connector account id '{}' not found for the given profile",
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

    #[cfg(not(feature = "connector_choice_mca_id"))]
    let check_connector_choice = |choice: &routing_types::RoutableConnectorChoice| {
        error_stack::ensure!(
            name_set.contains(&choice.connector.to_string()),
            errors::ApiErrorResponse::InvalidRequestData {
                message: format!(
                    "connector with name '{}' not found for the given profile",
                    choice.connector,
                )
            }
        );

        Ok(())
    };

    match routing_algorithm {
        routing_types::RoutingAlgorithm::Single(choice) => {
            check_connector_choice(choice)?;
        }

        routing_types::RoutingAlgorithm::Priority(list) => {
            for choice in list {
                check_connector_choice(choice)?;
            }
        }

        routing_types::RoutingAlgorithm::VolumeSplit(splits) => {
            for split in splits {
                check_connector_choice(&split.connector)?;
            }
        }

        routing_types::RoutingAlgorithm::Advanced(program) => {
            let check_connector_selection =
                |selection: &routing_types::ConnectorSelection| -> RouterResult<()> {
                    match selection {
                        routing_types::ConnectorSelection::VolumeSplit(splits) => {
                            for split in splits {
                                check_connector_choice(&split.connector)?;
                            }
                        }

                        routing_types::ConnectorSelection::Priority(list) => {
                            for choice in list {
                                check_connector_choice(choice)?;
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

/// Provides the identifier for the specific merchant's agnostic_mandate_config
#[inline(always)]
pub fn get_pg_agnostic_mandate_config_key(merchant_id: &str) -> String {
    format!("pg_agnostic_mandate_{merchant_id}")
}

/// Provides the identifier for the specific merchant's default_config
#[inline(always)]
pub fn get_default_config_key(merchant_id: &str) -> String {
    format!("routing_default_{merchant_id}")
}
pub fn get_payment_config_routing_id(merchant_id: &str) -> String {
    format!("payment_config_id_{merchant_id}")
}

pub fn get_payment_method_surcharge_routing_id(merchant_id: &str) -> String {
    format!("payment_method_surcharge_id_{merchant_id}")
}
