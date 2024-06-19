use once_cell::sync::OnceCell;

#[cfg(feature = "keymanager_mtls")]
use masking::PeekInterface;

use http::{HeaderMap, HeaderName, HeaderValue};
use std::str::FromStr;

use error_stack::ResultExt;

use crate::{errors, headers, types::domain::EncryptionCreateRequest, SessionState};

static ENCRYPTION_API_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

#[allow(unused_mut)]
pub fn get_api_encryption_client(
    state: &SessionState,
) -> errors::CustomResult<reqwest::Client, errors::ApiClientError> {
    let proxy = &state.conf.proxy;

    let get_client = || {
        let mut client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .pool_idle_timeout(std::time::Duration::from_secs(
                proxy.idle_pool_connection_timeout.unwrap_or_default(),
            ));

        #[cfg(feature = "keymanager_mtls")]
        {
            let ca = state.conf.key_manager.ca.clone();
            let cert = state.conf.key_manager.cert.clone();

            let identity = reqwest::Identity::from_pem(cert.peek().as_ref())
                .change_context(errors::ApiClientError::ClientConstructionFailed)?;
            let ca_cert = reqwest::Certificate::from_pem(ca.peek().as_ref())
                .change_context(errors::ApiClientError::ClientConstructionFailed)?;

            client = client
                .use_rustls_tls()
                .identity(identity)
                .add_root_certificate(ca_cert)
                .https_only(true);
        }

        client
            .build()
            .change_context(errors::ApiClientError::ClientConstructionFailed)
    };

    Ok(ENCRYPTION_API_CLIENT.get_or_try_init(get_client)?.clone())
}

pub async fn send_encryption_request<T>(
    state: &SessionState,
    headers: Vec<(String, String)>,
    url: String,
    request_body: T,
) -> errors::CustomResult<reqwest::Response, errors::ApiClientError>
where
    T: serde::Serialize,
{
    let client = get_api_encryption_client(state)?;
    let url =
        reqwest::Url::parse(&url).change_context(errors::ApiClientError::UrlEncodingFailed)?;

    let headers = headers.into_iter().try_fold(
        HeaderMap::new(),
        |mut header_map, (header_name, header_value)| {
            let header_name = HeaderName::from_str(&header_name)
                .change_context(errors::ApiClientError::HeaderMapConstructionFailed)?;
            let header_value = HeaderValue::from_str(&header_value)
                .change_context(errors::ApiClientError::HeaderMapConstructionFailed)?;
            header_map.append(header_name, header_value);
            Ok::<_, error_stack::Report<errors::ApiClientError>>(header_map)
        },
    )?;

    client
        .post(url)
        .json(&request_body)
        .headers(headers)
        .send()
        .await
        .change_context(errors::ApiClientError::RequestNotSent(
            "Unable to send request to encryption service".to_string(),
        ))
}

pub async fn call_encryption_service<T>(
    state: &SessionState,
    endpoint: &str,
    request_body: T,
) -> errors::CustomResult<reqwest::Response, errors::ApiClientError>
where
    T: serde::Serialize + Send + Sync + 'static,
{
    let url = format!("{}/{}", &state.conf.key_manager.url, endpoint);

    send_encryption_request(
        state,
        vec![(
            headers::CONTENT_TYPE.to_string(),
            "application/json".to_string(),
        )],
        url,
        request_body,
    )
    .await
}

pub async fn create_key_in_key_manager(
    state: &SessionState,
    request_body: EncryptionCreateRequest,
) -> errors::CustomResult<(), errors::ApiClientError> {
    let _ = call_encryption_service(state, "key/create", request_body).await?;

    Ok(())
}
