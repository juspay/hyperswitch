use actix_web::{http::header, web, HttpRequest, HttpResponse};
use api_models::hierarchical_resources as api_resources;
use base64::Engine;
use common_utils::id_type;
use error_stack::ResultExt;
use router_env::{instrument, tracing, Flow};

use super::app::{AppState, SessionState};
use crate::{
    core::{api_locking, errors, hierarchical_resources as resources_core},
    headers,
    services::{
        api, authentication as auth, authentication::HeaderMapStruct,
        authorization::permissions::Permission,
    },
};

/// `/resources` is JWT-only (dashboard session) and mandates `X-Profile-Id`, resolving the
/// `Profile` the same way `ApiKeyAuth`/`payment_methods_session_create` do — the fetch itself,
/// scoped to the JWT-authenticated merchant, is the authorization check: a `NotFound` means the
/// header names a profile that doesn't belong to this session's merchant.
fn mandatory_profile_id_from_header(req: &HttpRequest) -> Result<id_type::ProfileId, HttpResponse> {
    HeaderMapStruct::new(req.headers())
        .get_id_type_from_header::<id_type::ProfileId>(headers::X_PROFILE_ID)
        .map_err(api::log_and_return_error_response)
}

async fn resolve_profile_from_header(
    state: &SessionState,
    auth_data: &auth::AuthenticationData,
    profile_id: &id_type::ProfileId,
) -> Result<crate::types::domain::Profile, error_stack::Report<errors::ApiErrorResponse>> {
    let processor = auth_data.platform.get_processor();
    state
        .store
        .find_business_profile_by_merchant_id_profile_id(
            processor.get_key_store(),
            processor.get_account().get_id(),
            profile_id,
        )
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)
        .attach_printable(
            "X-Profile-Id does not name a profile belonging to this session's merchant",
        )
}

fn parse_upload_certificate_body(
    req: &HttpRequest,
    body: &[u8],
) -> Result<api_resources::UploadCertificateRequest, HttpResponse> {
    let is_json = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with(mime::APPLICATION_JSON.as_ref()));

    if is_json {
        serde_json::from_slice(body).map_err(|error| {
            api::log_and_return_error_response(error_stack::report!(
                errors::ApiErrorResponse::InvalidRequestData {
                    message: format!("invalid JSON body: {error}"),
                }
            ))
        })
    } else {
        Ok(api_resources::UploadCertificateRequest {
            certificate: crate::consts::BASE64_ENGINE.encode(body),
        })
    }
}

#[instrument(skip_all, fields(flow = ?Flow::HierarchicalResourcesGenerate))]
pub async fn generate_hierarchical_resource(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_resources::GenerateHierarchicalResourceRequest>,
) -> HttpResponse {
    let flow = Flow::HierarchicalResourcesGenerate;
    let payload = json_payload.into_inner();
    let profile_id = match mandatory_profile_id_from_header(&req) {
        Ok(id) => id,
        Err(error_response) => return error_response,
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        move |state, auth_data: auth::AuthenticationData, req, _| {
            let profile_id = profile_id.clone();
            async move {
                resolve_profile_from_header(&state, &auth_data, &profile_id).await?;
                resources_core::generate_hierarchical_resource(
                    state,
                    auth_data.platform.get_processor().clone(),
                    req,
                )
                .await
            }
        },
        &auth::JWTAndEmbeddedAuth {
            merchant_id_from_route: None,
            permission: Some(Permission::MerchantConnectorWrite),
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::HierarchicalResourcesUpload))]
pub async fn upload_hierarchical_resource(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::ResourceId>,
    body: web::Bytes,
) -> HttpResponse {
    let flow = Flow::HierarchicalResourcesUpload;
    let payload = match parse_upload_certificate_body(&req, &body) {
        Ok(payload) => payload,
        Err(error_response) => return error_response,
    };
    let resource_id = path.into_inner();
    let profile_id = match mandatory_profile_id_from_header(&req) {
        Ok(id) => id,
        Err(error_response) => return error_response,
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        move |state, auth_data: auth::AuthenticationData, req, _| {
            let resource_id = resource_id.clone();
            let profile_id = profile_id.clone();
            async move {
                resolve_profile_from_header(&state, &auth_data, &profile_id).await?;
                resources_core::upload_hierarchical_resource(
                    state,
                    auth_data.platform.get_processor().clone(),
                    resource_id,
                    req,
                )
                .await
            }
        },
        &auth::JWTAndEmbeddedAuth {
            merchant_id_from_route: None,
            permission: Some(Permission::MerchantConnectorWrite),
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::HierarchicalResourcesList))]
pub async fn list_hierarchical_resources(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_resources::ListHierarchicalResourcesRequest>,
) -> HttpResponse {
    let flow = Flow::HierarchicalResourcesList;
    let payload = json_payload.into_inner();
    let profile_id = match mandatory_profile_id_from_header(&req) {
        Ok(id) => id,
        Err(error_response) => return error_response,
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        move |state, auth_data: auth::AuthenticationData, req, _| {
            let profile_id = profile_id.clone();
            async move {
                resolve_profile_from_header(&state, &auth_data, &profile_id).await?;
                resources_core::list_hierarchical_resources(
                    state,
                    auth_data.platform.get_processor().clone(),
                    req,
                )
                .await
            }
        },
        &auth::JWTAndEmbeddedAuth {
            merchant_id_from_route: None,
            permission: Some(Permission::MerchantConnectorRead),
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::HierarchicalResourcesLink))]
pub async fn link_hierarchical_resource(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::ResourceId>,
    json_payload: web::Json<api_resources::LinkHierarchicalResourceRequest>,
) -> HttpResponse {
    let flow = Flow::HierarchicalResourcesLink;
    let payload = json_payload.into_inner();
    let resource_id = path.into_inner();
    let profile_id = match mandatory_profile_id_from_header(&req) {
        Ok(id) => id,
        Err(error_response) => return error_response,
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        move |state, auth_data: auth::AuthenticationData, req, _| {
            let resource_id = resource_id.clone();
            let profile_id = profile_id.clone();
            async move {
                resolve_profile_from_header(&state, &auth_data, &profile_id).await?;
                resources_core::link_hierarchical_resource(
                    state,
                    auth_data.platform.get_processor().clone(),
                    resource_id,
                    req,
                )
                .await
            }
        },
        &auth::JWTAndEmbeddedAuth {
            merchant_id_from_route: None,
            permission: Some(Permission::MerchantConnectorWrite),
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
