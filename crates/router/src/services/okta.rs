use error_stack::ResultExt;
use oidc::TokenResponse;
use openidconnect::{self as oidc, core as oidc_core};
use redis_interface::RedisConnectionPool;

use crate::{
    consts,
    core::errors::{UserErrors, UserResult},
    routes::SessionState,
    services::api::client,
};

pub struct ControlCentreRedirectURL {
    base_url: &'static str,
    sso_id: String,
}

pub async fn get_authorization_url(
    state: SessionState,
    redirect_url: String,
) -> UserResult<url::Url> {
    let discovery_document =
        get_discovery_document("https://dev-28418517.okta.com".to_string(), &state).await?;
    let client_id = oidc::ClientId::new("0oahmmwdmuFvv2pFo5d7".to_string());

    let redirect_url = oidc::RedirectUrl::new("http://localhost:8080/health".to_string()).unwrap();

    let (csrf, nounce) = get_csrf_and_nounce(&state, None).await?;

    let (auth_url, csrf_token, nonce) =
        oidc_core::CoreClient::from_provider_metadata(discovery_document, client_id, None)
            .set_redirect_uri(redirect_url)
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

pub async fn authorize_code(
    state: SessionState,
    authorization_code: String,
    csrf: String,
) -> UserResult<()> {
    let (csrf, nounce) = get_csrf_and_nounce(&state, Some(csrf)).await?;

    let discovery_document =
        get_discovery_document("https://dev-28418517.okta.com".to_string(), &state).await?;

    let client_id = oidc::ClientId::new("0oahmmwdmuFvv2pFo5d7".to_string());
    let client_secret = oidc::ClientSecret::new(
        "-VIrZZeN_A0SdSpFykAUZ0iMJNpSYQyILcfUmYlmZaLaFK7uRayrEuSvhs-Um5IR".to_string(),
    );

    let redirect_url = oidc::RedirectUrl::new("http://localhost:8080/health".to_string()).unwrap();
    let client = oidc_core::CoreClient::from_provider_metadata(
        discovery_document,
        client_id,
        Some(client_secret),
    )
    .set_redirect_uri(redirect_url);

    let nounce_clone = nounce.clone();
    client.authorize_url(
        oidc_core::CoreAuthenticationFlow::AuthorizationCode,
        || csrf,
        || nounce_clone,
    );

    let token_response = client
        .exchange_code(oidc::AuthorizationCode::new(authorization_code))
        .request_async(|req| get_oidc_client(&state, req))
        .await
        .unwrap();

    let id_token = token_response.id_token().unwrap();
    println!("{:?}", id_token);
    let claims = id_token
        .claims(&client.id_token_verifier(), &nounce)
        .unwrap();
    println!("{:?}", claims);
    Ok(())
}

// TODO: Cache Discovery Document
async fn get_discovery_document(
    base_url: String,
    state: &SessionState,
) -> UserResult<oidc_core::CoreProviderMetadata> {
    let issuer_url =
        oidc::IssuerUrl::new(base_url).change_context(UserErrors::InternalServerError)?;
    oidc_core::CoreProviderMetadata::discover_async(issuer_url, |req| get_oidc_client(&state, req))
        .await
        .change_context(UserErrors::InternalServerError)
}

async fn get_csrf_and_nounce(
    state: &SessionState,
    redirect_state: Option<String>,
) -> UserResult<(oidc::CsrfToken, oidc::Nonce)> {
    let redis_connection = get_redis_connection(&state)?;
    if let Some(redirect_state) = redirect_state {
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

async fn get_oidc_client(
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