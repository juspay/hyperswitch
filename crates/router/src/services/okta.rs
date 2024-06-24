use error_stack::ResultExt;
use oidc::TokenResponse;
use openidconnect::{self as oidc, core as oidc_core};

use masking::{ExposeInterface, Secret};
use redis_interface::RedisConnectionPool;

use crate::{
    consts,
    core::errors::{UserErrors, UserResult},
    routes::SessionState,
    services::api::client,
    types::domain::user::UserEmail,
};

pub struct ControlCentreRedirectURL {
    base_url: &'static str,
    sso_id: String,
}

pub async fn get_authorization_url(
    state: SessionState,
    redirect_url: String,
    base_url: Secret<String>,
    client_id: Secret<String>,
) -> UserResult<url::Url> {
    let discovery_document = get_discovery_document(base_url, &state).await?;
    let (csrf, nounce) = get_csrf_and_nounce(&state, None).await?;

    let (auth_url, csrf_token, nonce) =
        get_oidc_core_client(discovery_document, client_id, None, redirect_url)?
            .authorize_url(
                oidc_core::CoreAuthenticationFlow::AuthorizationCode,
                || csrf,
                || nounce,
            )
            .url();

    // Save csrf & nounce as key value respectively
    let key = get_csrf_redis_prefix(csrf_token.secret());
    get_redis_connection(&state)?
        .set_key_with_expiry(&key, nonce.secret(), 500)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to save csrf-nounce in redis")?;

    Ok(auth_url)
}

pub async fn get_user_email_from_oidc_provider(
    state: SessionState,
    redirect_url: String,
    base_url: Secret<String>,
    client_id: Secret<String>,
    authorization_code: Secret<String>,
    csrf: Secret<String>,
    client_secret: Secret<String>,
) -> UserResult<UserEmail> {
    let (csrf, nounce) = get_csrf_and_nounce(&state, Some(csrf)).await?;
    let discovery_document = get_discovery_document(base_url, &state).await?;
    let client = get_oidc_core_client(
        discovery_document,
        client_id,
        Some(client_secret),
        redirect_url,
    )?;

    let nounce_clone = nounce.clone();
    client.authorize_url(
        oidc_core::CoreAuthenticationFlow::AuthorizationCode,
        || csrf,
        || nounce_clone,
    );

    // Send request to OpenId provider with authorization code
    let token_response = client
        .exchange_code(oidc::AuthorizationCode::new(authorization_code.expose()))
        .request_async(|req| get_oidc_reqwest_client(&state, req))
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to exhange code and fetch oidc token")?;

    // Fetch id token from response
    let id_token = token_response
        .id_token()
        .ok_or(UserErrors::InternalServerError)
        .attach_printable("Id Token not provided in token response")?;

    // Verify id token
    let id_token_claims = id_token
        .claims(&client.id_token_verifier(), &nounce)
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to verfiy id token")?;

    // Get email from token
    let email_from_token = id_token_claims
        .email()
        .map(|email| email.to_string())
        .ok_or(UserErrors::InternalServerError)
        .attach_printable("OpenID Provider Didnt provide email")?;

    UserEmail::new(Secret::new(email_from_token))
        .change_context(UserErrors::InternalServerError)
        .attach("Failed to create email type")
}

// TODO: Cache Discovery Document
async fn get_discovery_document(
    base_url: Secret<String>,
    state: &SessionState,
) -> UserResult<oidc_core::CoreProviderMetadata> {
    let issuer_url =
        oidc::IssuerUrl::new(base_url.expose()).change_context(UserErrors::InternalServerError)?;
    oidc_core::CoreProviderMetadata::discover_async(issuer_url, |req| {
        get_oidc_reqwest_client(&state, req)
    })
    .await
    .change_context(UserErrors::InternalServerError)
}

fn get_oidc_core_client(
    discovery_document: oidc_core::CoreProviderMetadata,
    client_id: Secret<String>,
    client_secret: Option<Secret<String>>,
    redirect_url: String,
) -> UserResult<oidc_core::CoreClient> {
    let client_id = oidc::ClientId::new(client_id.expose());
    let client_secret = client_secret.map(|secret| oidc::ClientSecret::new(secret.expose()));
    let redirect_url = oidc::RedirectUrl::new(redirect_url)
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error creating redirect URL type")?;

    Ok(
        oidc_core::CoreClient::from_provider_metadata(discovery_document, client_id, client_secret)
            .set_redirect_uri(redirect_url),
    )
}

async fn get_csrf_and_nounce(
    state: &SessionState,
    redirect_state: Option<Secret<String>>,
) -> UserResult<(oidc::CsrfToken, oidc::Nonce)> {
    let redis_connection = get_redis_connection(&state)?;
    if let Some(redirect_state) = redirect_state {
        let redirect_state = redirect_state.expose();
        let key = get_csrf_redis_prefix(&redirect_state);
        redis_connection
            .get_key::<Option<String>>(&key)
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error Fetching CSRF from redis")?
            .map(|nounce| {
                (
                    oidc::CsrfToken::new(redirect_state),
                    oidc::Nonce::new(nounce),
                )
            })
            .ok_or(UserErrors::SSOFailed)
            .attach_printable("Cannot find csrf in redis. Csrf invalid or expired")
    } else {
        Ok((oidc::CsrfToken::new_random(), oidc::Nonce::new_random()))
    }
}

async fn get_oidc_reqwest_client(
    state: &SessionState,
    request: oidc::HttpRequest,
) -> Result<oidc::HttpResponse, reqwest::Error> {
    let client = client::create_client(&state.conf.proxy, false, None, None).unwrap();

    let mut request_builder = client
        .request(request.method, request.url)
        .body(request.body);
    for (name, value) in &request.headers {
        request_builder = request_builder.header(name.as_str(), value.as_bytes());
    }

    let request = request_builder.build()?;
    let response = client.execute(request).await?;

    Ok(oidc::HttpResponse {
        status_code: response.status(),
        headers: response.headers().to_owned(),
        body: response.bytes().await?.to_vec(),
    })
}

fn get_csrf_redis_prefix(csrf: &str) -> String {
    format!("{}OKTA_{:?}", consts::user::REDIS_SSO_PREFIX, csrf)
}

fn get_redis_connection(state: &SessionState) -> UserResult<std::sync::Arc<RedisConnectionPool>> {
    state
        .store
        .get_redis_conn()
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to get redis connection")
}
