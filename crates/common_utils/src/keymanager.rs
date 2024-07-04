//! Consists of all the common functions to use the Keymanager.

use std::str::FromStr;

use error_stack::ResultExt;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
#[cfg(feature = "keymanager_mtls")]
use masking::PeekInterface;
use once_cell::sync::OnceCell;

use crate::{
    errors,
    types::keymanager::{
        DataKeyCreateResponse, EncryptionCreateRequest, EncryptionTransferRequest, KeyManagerState,
    },
};

const CONTENT_TYPE: &str = "Content-Type";
static ENCRYPTION_API_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

/// Get keymanager client constructed from the url and state
#[allow(unused_mut)]
fn get_api_encryption_client(
    state: &KeyManagerState,
) -> errors::CustomResult<reqwest::Client, errors::KeyManagerClientError> {
    let get_client = || {
        let mut client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .pool_idle_timeout(std::time::Duration::from_secs(
                state.client_idle_timeout.unwrap_or_default(),
            ));

        #[cfg(feature = "keymanager_mtls")]
        {
            let cert = state.cert.clone();
            let ca = state.ca.clone();

            let identity = reqwest::Identity::from_pem(cert.peek().as_ref())
                .change_context(errors::KeyManagerClientError::ClientConstructionFailed)?;
            let ca_cert = reqwest::Certificate::from_pem(ca.peek().as_ref())
                .change_context(errors::KeyManagerClientError::ClientConstructionFailed)?;

            client = client
                .use_rustls_tls()
                .identity(identity)
                .add_root_certificate(ca_cert)
                .https_only(true);
        }

        client
            .build()
            .change_context(errors::KeyManagerClientError::ClientConstructionFailed)
    };

    Ok(ENCRYPTION_API_CLIENT.get_or_try_init(get_client)?.clone())
}

/// Generic function to send the request to keymanager
pub async fn send_encryption_request<T>(
    state: &KeyManagerState,
    headers: HeaderMap,
    url: String,
    request_body: T,
) -> errors::CustomResult<reqwest::Response, errors::KeyManagerClientError>
where
    T: serde::Serialize,
{
    let client = get_api_encryption_client(state)?;
    let url = reqwest::Url::parse(&url)
        .change_context(errors::KeyManagerClientError::UrlEncodingFailed)?;

    client
        .post(url)
        .json(&request_body)
        .headers(headers)
        .send()
        .await
        .change_context(errors::KeyManagerClientError::RequestNotSent(
            "Unable to send request to encryption service".to_string(),
        ))
}

/// Generic function to call the Keymanager and parse the response back
pub async fn call_encryption_service<T, R>(
    state: &KeyManagerState,
    endpoint: &str,
    request_body: T,
) -> errors::CustomResult<R, errors::KeyManagerClientError>
where
    T: serde::Serialize + Send + Sync + 'static,
    R: serde::de::DeserializeOwned,
{
    let url = format!("{}/{endpoint}", &state.url);

    let response = send_encryption_request(
        state,
        HeaderMap::from_iter(
            vec![(
                HeaderName::from_str(CONTENT_TYPE)
                    .change_context(errors::KeyManagerClientError::FailedtoConstructHeader)?,
                HeaderValue::from_str("application/json")
                    .change_context(errors::KeyManagerClientError::FailedtoConstructHeader)?,
            )]
            .into_iter(),
        ),
        url,
        request_body,
    )
    .await
    .map_err(|err| err.change_context(errors::KeyManagerClientError::RequestSendFailed))?;

    match response.status() {
        StatusCode::OK => response
            .json::<R>()
            .await
            .change_context(errors::KeyManagerClientError::ResponseDecodingFailed),
        StatusCode::INTERNAL_SERVER_ERROR => {
            Err(errors::KeyManagerClientError::InternalServerError(
                response
                    .bytes()
                    .await
                    .change_context(errors::KeyManagerClientError::ResponseDecodingFailed)?,
            )
            .into())
        }
        StatusCode::BAD_REQUEST => Err(errors::KeyManagerClientError::BadRequest(
            response
                .bytes()
                .await
                .change_context(errors::KeyManagerClientError::ResponseDecodingFailed)?,
        )
        .into()),
        _ => Err(errors::KeyManagerClientError::Unexpected(
            response
                .bytes()
                .await
                .change_context(errors::KeyManagerClientError::ResponseDecodingFailed)?,
        )
        .into()),
    }
}

/// A function to create the key in keymanager
pub async fn create_key_in_key_manager(
    state: &KeyManagerState,
    request_body: EncryptionCreateRequest,
) -> errors::CustomResult<DataKeyCreateResponse, errors::KeyManagerError> {
    call_encryption_service(state, "key/create", request_body)
        .await
        .change_context(errors::KeyManagerError::KeyAddFailed)
}

/// A function to transfer the key in keymanager
pub async fn transfer_key_to_key_manager(
    state: &KeyManagerState,
    request_body: EncryptionTransferRequest,
) -> errors::CustomResult<DataKeyCreateResponse, errors::KeyManagerError> {
    call_encryption_service(state, "key/transfer", request_body)
        .await
        .change_context(errors::KeyManagerError::KeyTransferFailed)
}
