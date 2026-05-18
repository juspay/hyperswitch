use actix_web::{web, HttpRequest, Responder};
use common_enums::EntityType;
use error_stack::ResultExt;
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{api_locking, errors, webhooks::webhook_events},
    routes::AppState,
    services::{
        api,
        authentication::{self as auth, UserFromToken},
        authorization::{permissions::Permission, roles::RoleInfo},
    },
    types::api::webhook_events::{
        EventListConstraints, EventListRequestInternal, WebhookDeliveryAttemptListRequestInternal,
        WebhookDeliveryRetryRequestInternal,
    },
};

#[instrument(skip_all, fields(flow = ?Flow::WebhookEventInitialDeliveryAttemptList))]
pub async fn list_initial_webhook_delivery_attempts(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
    json_payload: web::Json<EventListConstraints>,
) -> impl Responder {
    let flow = Flow::WebhookEventInitialDeliveryAttemptList;
    let merchant_id = path.into_inner();
    let constraints = json_payload.into_inner();

    let request_internal = EventListRequestInternal {
        merchant_id: merchant_id.clone(),
        constraints,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request_internal,
        |state, _, request_internal, _| {
            webhook_events::list_initial_delivery_attempts(
                state,
                request_internal.merchant_id,
                request_internal.constraints,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantWebhookEventRead,
                allow_connected: true,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::WebhookEventInitialDeliveryAttemptList))]
pub async fn list_initial_webhook_delivery_attempts_with_jwtauth(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<EventListConstraints>,
) -> impl Responder {
    let flow = Flow::WebhookEventInitialDeliveryAttemptList;
    let constraints = json_payload.into_inner();

    let request_internal = EventListRequestInternal {
        merchant_id: common_utils::id_type::MerchantId::default(),
        constraints,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request_internal,
        |state, auth: UserFromToken, request_internal, _| async move {
            let role_info = RoleInfo::from_role_id_org_id_tenant_id(
                &state,
                &auth.role_id,
                &auth.org_id,
                auth.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch role info while listing webhook events")?;

            // Merchant-or-higher scopes search across all profiles in the merchant;
            // profile-scoped users stay confined to their JWT's profile_id.
            let request_internal = EventListRequestInternal {
                merchant_id: auth.merchant_id,
                constraints: EventListConstraints {
                    profile_id: (role_info.get_entity_type() == EntityType::Profile)
                        .then_some(auth.profile_id),
                    ..request_internal.constraints
                },
            };

            webhook_events::list_initial_delivery_attempts(
                state,
                request_internal.merchant_id,
                request_internal.constraints,
            )
            .await
        },
        &auth::JWTAuth {
            permission: Permission::ProfileWebhookEventRead,
            allow_connected: true,
            allow_platform: false,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::WebhookEventDeliveryAttemptList))]
pub async fn list_webhook_delivery_attempts(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
) -> impl Responder {
    let flow = Flow::WebhookEventDeliveryAttemptList;
    let (merchant_id, initial_attempt_id) = path.into_inner();

    let request_internal = WebhookDeliveryAttemptListRequestInternal {
        merchant_id: merchant_id.clone(),
        initial_attempt_id,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request_internal,
        |state, _, request_internal, _| {
            webhook_events::list_delivery_attempts(
                state,
                request_internal.merchant_id,
                request_internal.initial_attempt_id,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantWebhookEventRead,
                allow_connected: true,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::WebhookEventDeliveryRetry))]
#[cfg(feature = "v1")]
pub async fn retry_webhook_delivery_attempt(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
) -> impl Responder {
    let flow = Flow::WebhookEventDeliveryRetry;
    let (merchant_id, event_id) = path.into_inner();

    let request_internal = WebhookDeliveryRetryRequestInternal {
        merchant_id: merchant_id.clone(),
        event_id,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        request_internal,
        |state, _, request_internal, _| {
            webhook_events::retry_delivery_attempt(
                state,
                request_internal.merchant_id,
                request_internal.event_id,
            )
        },
        auth::auth_type(
            &auth::AdminApiAuth,
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: Permission::MerchantWebhookEventWrite,
                allow_connected: true,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
