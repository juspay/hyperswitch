//! Consists of all the common functions to use the Keymanager.

use core::fmt::Debug;
use std::str::FromStr;

use base64::Engine;
use error_stack::ResultExt;
use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use masking::{PeekInterface, StrongSecret};
use once_cell::sync::OnceCell;
use router_env::{instrument, logger, tracing};

use crate::{
    consts::{BASE64_ENGINE, TENANT_HEADER},
    errors,
    types::keymanager::{
        BatchDecryptDataRequest, DataKeyCreateResponse, DecryptDataRequest,
        EncryptionCreateRequest, EncryptionTransferRequest, GetKeymanagerTenant, KeyManagerState,
        TransientBatchDecryptDataRequest, TransientDecryptDataRequest,
    },
};

const CONTENT_TYPE: &str = "Content-Type";
static ENCRYPTION_API_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();
static DEFAULT_ENCRYPTION_VERSION: &str = "v1";
#[cfg(feature = "km_forward_x_request_id")]
const X_REQUEST_ID: &str = "X-Request-Id";

/// Get keymanager client constructed from the url and state
#[instrument(skip_all)]
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
#[instrument(skip_all)]
pub async fn send_encryption_request<T>(
    state: &KeyManagerState,
    headers: HeaderMap,
    url: String,
    method: Method,
    request_body: T,
) -> errors::CustomResult<reqwest::Response, errors::KeyManagerClientError>
where
    T: ConvertRaw,
{
    let client = get_api_encryption_client(state)?;
    let url = reqwest::Url::parse(&url)
        .change_context(errors::KeyManagerClientError::UrlEncodingFailed)?;

    client
        .request(method, url)
        .json(&ConvertRaw::convert_raw(request_body)?)
        .headers(headers)
        .send()
        .await
        .change_context(errors::KeyManagerClientError::RequestNotSent(
            "Unable to send request to encryption service".to_string(),
        ))
}

/// Generic function to call the Keymanager and parse the response back
#[instrument(skip_all)]
pub async fn call_encryption_service<T, R>(
    state: &KeyManagerState,
    method: Method,
    endpoint: &str,
    request_body: T,
) -> errors::CustomResult<R, errors::KeyManagerClientError>
where
    T: GetKeymanagerTenant + ConvertRaw + Send + Sync + 'static + Debug,
    R: serde::de::DeserializeOwned,
{
    let url = format!("{}/{endpoint}", &state.url);

    logger::info!(key_manager_request=?request_body);
    let mut header = vec![];
    header.push((
        HeaderName::from_str(CONTENT_TYPE)
            .change_context(errors::KeyManagerClientError::FailedtoConstructHeader)?,
        HeaderValue::from_str("application/json")
            .change_context(errors::KeyManagerClientError::FailedtoConstructHeader)?,
    ));
    #[cfg(feature = "km_forward_x_request_id")]
    if let Some(ref request_id) = state.request_id {
        header.push((
            HeaderName::from_str(X_REQUEST_ID)
                .change_context(errors::KeyManagerClientError::FailedtoConstructHeader)?,
            HeaderValue::from_str(request_id.as_str())
                .change_context(errors::KeyManagerClientError::FailedtoConstructHeader)?,
        ))
    }

    //Add Tenant ID
    header.push((
        HeaderName::from_str(TENANT_HEADER)
            .change_context(errors::KeyManagerClientError::FailedtoConstructHeader)?,
        HeaderValue::from_str(request_body.get_tenant_id(state).get_string_repr())
            .change_context(errors::KeyManagerClientError::FailedtoConstructHeader)?,
    ));

    let response = send_encryption_request(
        state,
        HeaderMap::from_iter(header.into_iter()),
        url,
        method,
        request_body,
    )
    .await
    .map_err(|err| err.change_context(errors::KeyManagerClientError::RequestSendFailed))?;

    logger::info!(key_manager_response=?response);

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

/// Trait to convert the raw data to the required format for encryption service request
pub trait ConvertRaw {
    /// Return type of the convert_raw function
    type Output: serde::Serialize;
    /// Function to convert the raw data to the required format for encryption service request
    fn convert_raw(self) -> Result<Self::Output, errors::KeyManagerClientError>;
}

impl<T: serde::Serialize> ConvertRaw for T {
    type Output = T;
    fn convert_raw(self) -> Result<Self::Output, errors::KeyManagerClientError> {
        Ok(self)
    }
}

impl ConvertRaw for TransientDecryptDataRequest {
    type Output = DecryptDataRequest;
    fn convert_raw(self) -> Result<Self::Output, errors::KeyManagerClientError> {
        let data = match String::from_utf8(self.data.peek().clone()) {
            Ok(data) => data,
            Err(_) => {
                let data = BASE64_ENGINE.encode(self.data.peek().clone());
                format!("{DEFAULT_ENCRYPTION_VERSION}:{data}")
            }
        };
        Ok(DecryptDataRequest {
            identifier: self.identifier,
            data: StrongSecret::new(data),
        })
    }
}

impl ConvertRaw for TransientBatchDecryptDataRequest {
    type Output = BatchDecryptDataRequest;
    fn convert_raw(self) -> Result<Self::Output, errors::KeyManagerClientError> {
        let data = self
            .data
            .iter()
            .map(|(k, v)| {
                let value = match String::from_utf8(v.peek().clone()) {
                    Ok(data) => data,
                    Err(_) => {
                        let data = BASE64_ENGINE.encode(v.peek().clone());
                        format!("{DEFAULT_ENCRYPTION_VERSION}:{data}")
                    }
                };
                (k.to_owned(), StrongSecret::new(value))
            })
            .collect();
        Ok(BatchDecryptDataRequest {
            data,
            identifier: self.identifier,
        })
    }
}

/// A function to create the key in keymanager
#[instrument(skip_all)]
pub async fn create_key_in_key_manager(
    state: &KeyManagerState,
    request_body: EncryptionCreateRequest,
) -> errors::CustomResult<DataKeyCreateResponse, errors::KeyManagerError> {
    call_encryption_service(state, Method::POST, "key/create", request_body)
        .await
        .change_context(errors::KeyManagerError::KeyAddFailed)
}

/// A function to transfer the key in keymanager
#[instrument(skip_all)]
pub async fn transfer_key_to_key_manager(
    state: &KeyManagerState,
    request_body: EncryptionTransferRequest,
) -> errors::CustomResult<DataKeyCreateResponse, errors::KeyManagerError> {
    call_encryption_service(state, Method::POST, "key/transfer", request_body)
        .await
        .change_context(errors::KeyManagerError::KeyTransferFailed)
}
