use actix_web::{http::header, web, HttpRequest, HttpResponse};
use api_models::resources as api_resources;
use base64::Engine;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, resources as resources_core},
    services::{api, authentication as auth, authorization::permissions::Permission},
};

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
                crate::core::errors::ApiErrorResponse::InvalidRequestData {
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

#[instrument(skip_all, fields(flow = ?Flow::ResourcesGenerate))]
pub async fn generate_resource(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_resources::GenerateResourceRequest>,
) -> HttpResponse {
    let flow = Flow::ResourcesGenerate;
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth_data: auth::AuthenticationData, req, _| {
            resources_core::generate_resource(
                state,
                auth_data.platform.get_processor().clone(),
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            }),
            &auth::JWTAndEmbeddedAuth {
                merchant_id_from_route: None,
                permission: Some(Permission::MerchantConnectorWrite),
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ResourcesUpload))]
pub async fn upload_apple_pay_certificate(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ResourceId>,
    body: web::Bytes,
) -> HttpResponse {
    let flow = Flow::ResourcesUpload;
    let payload = match parse_upload_certificate_body(&req, &body) {
        Ok(payload) => payload,
        Err(error_response) => return error_response,
    };
    let resource_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        move |state, auth_data: auth::AuthenticationData, req, _| {
            resources_core::upload_apple_pay_certificate(
                state,
                auth_data.platform.get_processor().clone(),
                resource_id.clone(),
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            }),
            &auth::JWTAndEmbeddedAuth {
                merchant_id_from_route: None,
                permission: Some(Permission::MerchantConnectorWrite),
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ResourcesList))]
pub async fn list_resources(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_resources::ListResourcesRequest>,
) -> HttpResponse {
    let flow = Flow::ResourcesList;
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth_data: auth::AuthenticationData, req, _| {
            let organization_id = auth_data
                .platform
                .get_processor()
                .get_account()
                .organization_id
                .clone();
            resources_core::list_resources(state, organization_id, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            }),
            &auth::JWTAndEmbeddedAuth {
                merchant_id_from_route: None,
                permission: Some(Permission::MerchantConnectorRead),
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ResourcesLink))]
pub async fn link_resource(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ResourceId>,
    json_payload: web::Json<api_resources::LinkResourceRequest>,
) -> HttpResponse {
    let flow = Flow::ResourcesLink;
    let payload = json_payload.into_inner();
    let resource_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        move |state, auth_data: auth::AuthenticationData, req, _| {
            resources_core::link_resource(
                state,
                auth_data.platform.get_processor().clone(),
                resource_id.clone(),
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: true,
                allow_platform_self_operation: true,
            }),
            &auth::JWTAndEmbeddedAuth {
                merchant_id_from_route: None,
                permission: Some(Permission::MerchantConnectorWrite),
                allow_connected: true,
                allow_platform: true,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
