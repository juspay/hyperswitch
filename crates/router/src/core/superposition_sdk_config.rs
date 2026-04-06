use api_models::superposition_sdk_config::SuperPositionConfigResponse;
use common_utils::ext_traits::AsyncExt;
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

    let cached_configs = state
        .superposition_service
        .as_ref()
        .async_map(|sp| {
            sp.as_ref().get_cached_config(
                Some(vec![DYNAMIC_FIELDS.to_string()]),
                Some(dimension_filter.clone()),
            )
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get cached superposition sdk config")?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        SuperPositionConfigResponse {
            raw_configs: cached_configs,
            resolved_configs: None,
            context_used: dimension_filter,
        },
    ))
}
