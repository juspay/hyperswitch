use api_models::superposition_sdk_config::SuperPositionConfigResponse;
use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;

use crate::{
    consts::superposition::DYNAMIC_FIELDS,
    core::errors::{self, RouterResponse},
    types::domain,
    SessionState,
};

pub async fn get_superposition_sdk_config(
    state: SessionState,
    _platform: domain::Platform,
) -> RouterResponse<SuperPositionConfigResponse> {
    // let resolved_configs = state
    //     .superposition_service
    //     .as_ref()
    //     .async_map(|sp| async move { sp.as_ref().resolve_full_config(None, None).await })
    //     .await
    //     .transpose()
    //     .change_context(errors::ApiErrorResponse::InternalServerError)
    //     .attach_printable("Failed to resolve superposition sdk config")?;

    let cached_configs = state
        .superposition_service
        .as_ref()
        .async_map(|sp| async move {
            sp.as_ref()
                .get_cached_config(Some(vec![DYNAMIC_FIELDS.to_string()]), None)
                .await
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get cached superposition sdk config")?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        SuperPositionConfigResponse {
            raw_configs: cached_configs,
            resolved_configs: None,
        },
    ))
}
