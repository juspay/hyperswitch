use std::collections::HashMap;

use api_models::{
    admin::PaymentMethodsEnabled,
    enums::{self as api_enums, Connector},
    payment_methods::RequestPaymentMethodTypes,
    superposition_sdk_config::{
        DynamicFields, PaymentMethodGroup, PaymentMethodTypeWithFields, SuperPositionConfigResponse,
    },
};
use common_utils::id_type::ProfileId;
use hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount;
use hyperswitch_masking::ExposeInterface;
use serde_json::Map;

use crate::{
    consts::superposition::DYNAMIC_FIELDS,
    core::errors::{self, RouterResponse, StorageErrorExt},
    routes::SessionState,
    types::domain,
};

/// Type alias for required fields grouped by payment method type.
type RequiredFieldsByPmType = HashMap<
    api_enums::PaymentMethodType,
    HashMap<String, api_models::payment_methods::RequiredFieldInfo>,
>;

/// Type alias for grouped payment method data.
type GroupedPaymentMethods = HashMap<api_enums::PaymentMethod, RequiredFieldsByPmType>;

/// Builds a superset of required fields from common, mandate, and non-mandate fields.
///
/// All field values are set to `None` for configuration responses.
fn build_superset_required_fields(
    common: &HashMap<String, api_models::payment_methods::RequiredFieldInfo>,
    mandate: &HashMap<String, api_models::payment_methods::RequiredFieldInfo>,
    non_mandate: &HashMap<String, api_models::payment_methods::RequiredFieldInfo>,
) -> HashMap<String, api_models::payment_methods::RequiredFieldInfo> {
    let mut superset = HashMap::with_capacity(common.len() + mandate.len() + non_mandate.len());

    for (key, field_info) in common {
        superset.insert(
            key.clone(),
            api_models::payment_methods::RequiredFieldInfo {
                required_field: field_info.required_field.clone(),
                display_name: field_info.display_name.clone(),
                field_type: field_info.field_type.clone(),
                value: None,
            },
        );
    }

    for (key, field_info) in mandate {
        superset.entry(key.clone()).or_insert_with(|| {
            api_models::payment_methods::RequiredFieldInfo {
                required_field: field_info.required_field.clone(),
                display_name: field_info.display_name.clone(),
                field_type: field_info.field_type.clone(),
                value: None,
            }
        });
    }

    for (key, field_info) in non_mandate {
        superset.entry(key.clone()).or_insert_with(|| {
            api_models::payment_methods::RequiredFieldInfo {
                required_field: field_info.required_field.clone(),
                display_name: field_info.display_name.clone(),
                field_type: field_info.field_type.clone(),
                value: None,
            }
        });
    }

    superset
}

/// Processes payment methods from an MCA and groups required fields.
///
/// # Arguments
/// * `mca` - The merchant connector account to process
/// * `required_fields_config` - Configuration containing required fields mappings
/// * `grouped_data` - Mutable reference to accumulate grouped data
fn process_mca_payment_methods(
    mca: &MerchantConnectorAccount,
    required_fields_config: &crate::configs::settings::RequiredFields,
    grouped_data: &mut GroupedPaymentMethods,
) {
    if let Some(payment_methods_enabled) = &mca.payment_methods_enabled {
        for pm_secret in payment_methods_enabled {
            match serde_json::from_value::<PaymentMethodsEnabled>(pm_secret.clone().expose()) {
                Ok(pm_enabled) => {
                    if let Some(pm_types) = &pm_enabled.payment_method_types {
                        for pm_type in pm_types {
                            process_payment_method_type(
                                pm_enabled.payment_method,
                                pm_type,
                                &mca.connector_name,
                                required_fields_config,
                                grouped_data,
                            );
                        }
                    }
                }
                Err(e) => {
                    router_env::logger::debug!(error=%e, "Failed to deserialize payment methods enabled");
                }
            }
        }
    }
}

/// Processes a single payment method type and adds its required fields to the grouped data.
fn process_payment_method_type(
    payment_method: api_enums::PaymentMethod,
    pm_type: &RequestPaymentMethodTypes,
    connector_name: &str,
    required_fields_config: &crate::configs::settings::RequiredFields,
    grouped_data: &mut GroupedPaymentMethods,
) {
    let payment_method_type = pm_type.payment_method_type;

    match connector_name.parse::<Connector>() {
        Ok(connector) => {
            if let Some(required_field_final) = required_fields_config
                .0
                .get(&payment_method)
                .and_then(|pm_type_map| pm_type_map.0.get(&payment_method_type))
                .and_then(|connector_fields| connector_fields.fields.get(&connector))
            {
                let superset = build_superset_required_fields(
                    &required_field_final.common,
                    &required_field_final.mandate,
                    &required_field_final.non_mandate,
                );

                grouped_data
                    .entry(payment_method)
                    .or_default()
                    .entry(payment_method_type)
                    .or_default()
                    .extend(superset);
            } else {
                router_env::logger::debug!(
                    payment_method=?payment_method,
                    payment_method_type=?payment_method_type,
                    connector=%connector_name,
                    "No required fields found in config"
                );
            }
        }
        Err(e) => {
            router_env::logger::warn!(
                connector=%connector_name,
                error=%e,
                "Failed to parse connector name"
            );
        }
    }
}

/// Converts grouped payment method data into the response structure.
fn convert_to_response(grouped_data: GroupedPaymentMethods) -> DynamicFields {
    let payment_methods = grouped_data
        .into_iter()
        .map(|(payment_method, pm_types_map)| PaymentMethodGroup {
            payment_method,
            payment_method_types: pm_types_map
                .into_iter()
                .map(
                    |(payment_method_type, fields_map)| PaymentMethodTypeWithFields {
                        payment_method_type,
                        required_fields: fields_map,
                    },
                )
                .collect(),
        })
        .collect();

    DynamicFields { payment_methods }
}

pub async fn get_superposition_sdk_config(
    state: SessionState,
    platform: domain::Platform,
    profile_id: ProfileId,
) -> RouterResponse<SuperPositionConfigResponse> {
    // we want resolve config with filters which is not yet available in any version of superposition yet. so we are commenting it for future usecase

    // let resolved_configs = state
    //     .superposition_service
    //     .as_ref()
    //     .async_map(|sp| async move { sp.as_ref().resolve_full_config(None, None).await })
    //     .await
    //     .transpose()
    //     .change_context(errors::ApiErrorResponse::InternalServerError)
    //     .attach_printable("Failed to resolve superposition sdk config")?;

    let merchant_account = platform.get_processor().get_account();
    let key_store = platform.get_processor().get_key_store();

    // Fetch all enabled merchant connector accounts for the merchant
    let all_mcas = state
        .store
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            merchant_account.get_id(),
            false,
            key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    // Group required fields by payment method
    let mut grouped_data: GroupedPaymentMethods = HashMap::new();

    for mca in all_mcas.iter() {
        // Filter MCAs by profile ID
        if mca.profile_id == profile_id {
            process_mca_payment_methods(mca, &state.conf.required_fields, &mut grouped_data);
        }
    }

    let dynamic_fields = convert_to_response(grouped_data);

    // Build dimension filter for superposition context
    let mut dimension_filter = Map::new();
    dimension_filter.insert(
        "profile_id".to_string(),
        serde_json::Value::String(profile_id.get_string_repr().to_string()),
    );
    dimension_filter.insert(
        "merchant_id".to_string(),
        serde_json::Value::String(merchant_account.get_id().get_string_repr().to_string()),
    );
    dimension_filter.insert(
        "organization_id".to_string(),
        serde_json::Value::String(merchant_account.get_org_id().get_string_repr().to_string()),
    );

    let raw_configs = state
        .superposition_service
        .get_cached_config(
            Some(vec![DYNAMIC_FIELDS.to_string()]),
            Some(dimension_filter.clone()),
        )
        .await
        .map_err(|err| {
            router_env::logger::warn!(error=%err, "Failed to fetch cached superposition config");
            err
        })
        .ok();

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        SuperPositionConfigResponse {
            raw_configs,
            resolved_configs: None,
            context_used: dimension_filter,
            dynamic_fields: Some(dynamic_fields),
        },
    ))
}
