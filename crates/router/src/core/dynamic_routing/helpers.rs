#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use std::str::FromStr;
#[cfg(any(feature = "dynamic_routing", feature = "v1"))]
use std::sync::Arc;

use api_models::routing as routing_types;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use common_utils::ext_traits::ValueExt;
use common_utils::{ext_traits::Encode, id_type, types::keymanager::KeyManagerState};
#[cfg(feature = "v1")]
use diesel_models::routing_algorithm;
use error_stack::ResultExt;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use external_services::grpc_client::dynamic_routing::SuccessBasedDynamicRouting;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::api::ApplicationResponse;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use router_env::logger;
#[cfg(any(feature = "dynamic_routing", feature = "v1"))]
use router_env::{instrument, metrics::add_attributes, tracing};
use storage_impl::redis::cache;

#[cfg(feature = "v2")]
use crate::types::domain::MerchantConnectorAccount;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use crate::types::storage;
use crate::{
    core::errors::{self, RouterResult},
    db::StorageInterface,
    routes::SessionState,
    types::domain,
};
#[cfg(feature = "v1")]
use crate::{core::metrics as core_metrics, routes::metrics, types::transformers::ForeignInto};
pub const SUCCESS_BASED_DYNAMIC_ROUTING_ALGORITHM: &str =
    "Success rate based dynamic routing algorithm";

/// Retrieves cached success_based routing configs specific to tenant and profile
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn get_cached_success_based_routing_config_for_profile<'a>(
    state: &SessionState,
    key: &str,
) -> Option<Arc<routing_types::SuccessBasedRoutingConfig>> {
    cache::SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE
        .get_val::<Arc<routing_types::SuccessBasedRoutingConfig>>(cache::CacheKey {
            key: key.to_string(),
            prefix: state.tenant.redis_key_prefix.clone(),
        })
        .await
}

/// Refreshes the cached success_based routing configs specific to tenant and profile
#[cfg(feature = "v1")]
pub async fn refresh_success_based_routing_cache(
    state: &SessionState,
    key: &str,
    success_based_routing_config: routing_types::SuccessBasedRoutingConfig,
) -> Arc<routing_types::SuccessBasedRoutingConfig> {
    let config = Arc::new(success_based_routing_config);
    cache::SUCCESS_BASED_DYNAMIC_ALGORITHM_CACHE
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

/// Checked fetch of success based routing configs
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn fetch_success_based_routing_configs(
    state: &SessionState,
    business_profile: &domain::Profile,
    success_based_routing_id: id_type::RoutingId,
) -> RouterResult<routing_types::SuccessBasedRoutingConfig> {
    let key = format!(
        "{}_{}",
        business_profile.get_id().get_string_repr(),
        success_based_routing_id.get_string_repr()
    );

    if let Some(config) =
        get_cached_success_based_routing_config_for_profile(state, key.as_str()).await
    {
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
            .attach_printable("unable to retrieve success_rate_algorithm for profile from db")?;

        let success_rate_config = success_rate_algorithm
            .algorithm_data
            .parse_value::<routing_types::SuccessBasedRoutingConfig>("SuccessBasedRoutingConfig")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to parse success_based_routing_config struct")?;

        refresh_success_based_routing_cache(state, key.as_str(), success_rate_config.clone()).await;

        Ok(success_rate_config)
    }
}

#[cfg(feature = "v1")]
pub async fn update_business_profile_active_dynamic_algorithm_ref(
    db: &dyn StorageInterface,
    key_manager_state: &KeyManagerState,
    merchant_key_store: &domain::MerchantKeyStore,
    current_business_profile: domain::Profile,
    dynamic_routing_algorithm: routing_types::DynamicRoutingAlgorithmRef,
) -> RouterResult<()> {
    let ref_val = dynamic_routing_algorithm
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

/// metrics for success based dynamic routing
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn push_metrics_with_update_window_for_success_based_routing(
    state: &SessionState,
    payment_attempt: &storage::PaymentAttempt,
    routable_connectors: Vec<routing_types::RoutableConnectorChoice>,
    business_profile: &domain::Profile,
    success_based_routing_config_params_interpolator: SuccessBasedRoutingConfigParamsInterpolator,
) -> RouterResult<()> {
    let success_based_dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef =
        business_profile
            .dynamic_routing_algorithm
            .clone()
            .map(|val| val.parse_value("DynamicRoutingAlgorithmRef"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to deserialize DynamicRoutingAlgorithmRef from JSON")?
            .unwrap_or_default();

    let success_based_algo_ref = success_based_dynamic_routing_algo_ref
        .success_based_algorithm
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("success_based_algorithm not found in dynamic_routing_algorithm from business_profile table")?;

    if success_based_algo_ref.enabled_feature != routing_types::SuccessBasedRoutingFeatures::None {
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

        let success_based_routing_configs = fetch_success_based_routing_configs(
            state,
            business_profile,
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

        let tenant_business_profile_id = generate_tenant_business_profile_id(
            &state.tenant.redis_key_prefix,
            business_profile.get_id().get_string_repr(),
        );

        let success_based_routing_config_params = success_based_routing_config_params_interpolator
            .get_string_val(
                success_based_routing_configs
                    .params
                    .as_ref()
                    .ok_or(errors::RoutingError::SuccessBasedRoutingParamsNotFoundError)
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
            );

        let success_based_connectors = client
            .calculate_success_rate(
                tenant_business_profile_id.clone(),
                success_based_routing_configs.clone(),
                success_based_routing_config_params.clone(),
                routable_connectors.clone(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to calculate/fetch success rate from dynamic routing service",
            )?;

        let payment_status_attribute =
            get_desired_payment_status_for_success_routing_metrics(&payment_attempt.status);

        let first_success_based_connector_label = &success_based_connectors
            .labels_with_score
            .first()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to fetch the first connector from list of connectors obtained from dynamic routing service",
            )?
            .label
            .to_string();

        let (first_success_based_connector, merchant_connector_id) = first_success_based_connector_label
            .split_once(':')
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!(
                "unable to split connector_name and mca_id from the first connector {:?} obtained from dynamic routing service",
                first_success_based_connector_label
            ))?;

        let outcome = get_success_based_metrics_outcome_for_payment(
            &payment_status_attribute,
            payment_connector.to_string(),
            first_success_based_connector.to_string(),
        );

        core_metrics::DYNAMIC_SUCCESS_BASED_ROUTING.add(
            &metrics::CONTEXT,
            1,
            &add_attributes([
                ("tenant", state.tenant.tenant_id.clone()),
                (
                    "merchant_id",
                    payment_attempt.merchant_id.get_string_repr().to_string(),
                ),
                (
                    "profile_id",
                    payment_attempt.profile_id.get_string_repr().to_string(),
                ),
                ("merchant_connector_id", merchant_connector_id.to_string()),
                (
                    "payment_id",
                    payment_attempt.payment_id.get_string_repr().to_string(),
                ),
                (
                    "success_based_routing_connector",
                    first_success_based_connector.to_string(),
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
            ]),
        );
        logger::debug!("successfully pushed success_based_routing metrics");

        client
            .update_success_rate(
                tenant_business_profile_id,
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

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
fn get_desired_payment_status_for_success_routing_metrics(
    attempt_status: &common_enums::AttemptStatus,
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
fn get_success_based_metrics_outcome_for_payment(
    payment_status_attribute: &common_enums::AttemptStatus,
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

/// generates cache key with tenant's redis key prefix and profile_id
pub fn generate_tenant_business_profile_id(
    redis_key_prefix: &str,
    business_profile_id: &str,
) -> String {
    format!("{}:{}", redis_key_prefix, business_profile_id)
}

/// default config setup for success_based_routing
#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn default_success_based_routing_setup(
    state: &SessionState,
    key_store: domain::MerchantKeyStore,
    business_profile: domain::Profile,
    feature_to_enable: routing_types::SuccessBasedRoutingFeatures,
    merchant_id: id_type::MerchantId,
    mut success_based_dynamic_routing_algo: routing_types::DynamicRoutingAlgorithmRef,
) -> RouterResult<ApplicationResponse<routing_types::RoutingDictionaryRecord>> {
    let db = state.store.as_ref();
    let key_manager_state = &state.into();
    let profile_id = business_profile.get_id().to_owned();
    let default_success_based_routing_config = routing_types::SuccessBasedRoutingConfig::default();
    let algorithm_id = common_utils::generate_routing_id_of_default_length();
    let timestamp = common_utils::date_time::now();
    let algo = routing_algorithm::RoutingAlgorithm {
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
    };

    let record = db
        .insert_routing_algorithm(algo)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to insert record in routing algorithm table")?;

    success_based_dynamic_routing_algo.update_algorithm_id(algorithm_id, feature_to_enable);
    update_business_profile_active_dynamic_algorithm_ref(
        db,
        key_manager_state,
        &key_store,
        business_profile,
        success_based_dynamic_routing_algo,
    )
    .await?;

    let new_record = record.foreign_into();

    core_metrics::ROUTING_CREATE_SUCCESS_RESPONSE.add(
        &metrics::CONTEXT,
        1,
        &add_attributes([("profile_id", profile_id.get_string_repr().to_string())]),
    );
    Ok(ApplicationResponse::Json(new_record))
}

pub struct SuccessBasedRoutingConfigParamsInterpolator {
    pub payment_method: Option<common_enums::PaymentMethod>,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub authentication_type: Option<common_enums::AuthenticationType>,
    pub currency: Option<common_enums::Currency>,
    pub country: Option<common_enums::CountryAlpha2>,
    pub card_network: Option<String>,
    pub card_bin: Option<String>,
}

impl SuccessBasedRoutingConfigParamsInterpolator {
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
        params: &Vec<routing_types::SuccessBasedRoutingConfigParams>,
    ) -> String {
        let mut parts: Vec<String> = Vec::new();
        for param in params {
            let val = match param {
                routing_types::SuccessBasedRoutingConfigParams::PaymentMethod => self
                    .payment_method
                    .as_ref()
                    .map_or(String::new(), |pm| pm.to_string()),
                routing_types::SuccessBasedRoutingConfigParams::PaymentMethodType => self
                    .payment_method_type
                    .as_ref()
                    .map_or(String::new(), |pmt| pmt.to_string()),
                routing_types::SuccessBasedRoutingConfigParams::AuthenticationType => self
                    .authentication_type
                    .as_ref()
                    .map_or(String::new(), |at| at.to_string()),
                routing_types::SuccessBasedRoutingConfigParams::Currency => self
                    .currency
                    .as_ref()
                    .map_or(String::new(), |cur| cur.to_string()),
                routing_types::SuccessBasedRoutingConfigParams::Country => self
                    .country
                    .as_ref()
                    .map_or(String::new(), |cn| cn.to_string()),
                routing_types::SuccessBasedRoutingConfigParams::CardNetwork => {
                    self.card_network.clone().unwrap_or_default()
                }
                routing_types::SuccessBasedRoutingConfigParams::CardBin => {
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
