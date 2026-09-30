//! Monitoring policy shared by the identity and session endpoints.

use actix_web::{
    cookie::{Cookie, SameSite},
    http::header::{HeaderMap, HeaderValue, SET_COOKIE},
};
use api_models::observability::monitoring::{
    GrafanaAuthRequest, GrafanaAuthResponse, GrafanaSessionResponse,
};
use error_stack::report;
use hyperswitch_masking::{PeekInterface, Secret};

use super::response::ApplicationResponse;
use crate::{domain::monitoring::GrafanaLogin, errors::ObservabilityError, state::AppState};

const GRAFANA_PERMISSION: &str = "ProfileReconRuleRead";

pub async fn authorize(
    state: AppState,
    request: GrafanaAuthRequest,
) -> error_stack::Result<GrafanaAuthResponse, ObservabilityError> {
    let token = &request.token;
    let client = state
        .router_transport
        .as_deref()
        .ok_or_else(|| report!(ObservabilityError::RouterUnavailable))?;
    client
        .authorize_token(token, GRAFANA_PERMISSION)
        .await
        .map_err(ObservabilityError::from)?;
    let email = client
        .get_user_email(token)
        .await
        .map_err(ObservabilityError::from)?;
    let login = GrafanaLogin::from_router_email(&email)
        .ok_or_else(|| report!(ObservabilityError::RouterUnavailable))?;
    Ok(GrafanaAuthResponse {
        grafana_login: login.into_string(),
    })
}

/// Successful bootstrap sets a cookie only after Router validation; cache policy belongs to the gateway.
pub async fn session(
    state: AppState,
    (id, token): (
        String,
        error_stack::Result<Secret<String>, ObservabilityError>,
    ),
) -> error_stack::Result<ApplicationResponse<GrafanaSessionResponse>, ObservabilityError> {
    let token = token?;
    authorize(
        state.clone(),
        GrafanaAuthRequest {
            token: token.clone(),
        },
    )
    .await?;
    let embed_url = state
        .conf
        .monitoring
        .destinations
        .get(&id)
        .ok_or_else(|| report!(ObservabilityError::UnknownMonitoringDestination))?
        .to_string();
    let cookie = Cookie::build("grafana_token", token.peek().clone())
        .path("/api/observability-plane/grafana")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Strict)
        .finish();
    let mut value = HeaderValue::from_str(&cookie.to_string())
        .map_err(|_| report!(ObservabilityError::InternalServerError))?;
    value.set_sensitive(true);
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, value);
    Ok(ApplicationResponse::JsonWithHeaders {
        body: GrafanaSessionResponse { embed_url },
        headers,
    })
}
