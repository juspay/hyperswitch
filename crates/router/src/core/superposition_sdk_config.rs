use api_models::superposition_sdk_config::SuperPositionConfigResponse;
use common_enums::ConnectorType;
use error_stack::ResultExt;
use serde_json::Map;

use crate::{
    consts::superposition::DYNAMIC_FIELDS,
    core::errors::{self, RouterResponse},
    types::domain,
    SessionState,
};

pub async fn get_superposition_sdk_config(
    state: SessionState,
    platform: domain::Platform,
    profile_id: common_utils::id_type::ProfileId,
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

    // Fetch enabled connector accounts for the profile
    let enabled_connectors = state
        .store
        .list_enabled_connector_accounts_by_profile_id(
            &profile_id,
            key_store,
            ConnectorType::PaymentProcessor,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch enabled connector accounts")?;

    // Extract unique connector names from enabled connectors
    let active_connectors: Vec<String> = enabled_connectors
        .into_iter()
        .map(|mca| mca.get_connector_name_as_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

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
    dimension_filter.insert(
        "connector".to_string(),
        serde_json::Value::Array(
            active_connectors
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        ),
    );

    let cached_configs = state
        .superposition_service
        .get_cached_config(
            Some(vec![DYNAMIC_FIELDS.to_string()]),
            Some(dimension_filter.clone()),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get cached superposition sdk config")?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        SuperPositionConfigResponse {
            raw_configs: Some(cached_configs),
            resolved_configs: None,
            context_used: dimension_filter,
        },
    ))
}
