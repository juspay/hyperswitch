//! Consists of all the common functions to use the Keymanager.

use core::fmt::Debug;
use std::str::FromStr;

use base64::Engine;
use error_stack::ResultExt;
use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use hyperswitch_masking::{PeekInterface, StrongSecret};
use once_cell::sync::OnceCell;
use router_env::{
    global_meter, histogram_metric_f64, instrument, logger, metric_attributes, tracing,
};
use time::OffsetDateTime;

#[cfg(feature = "ext_services_latency")]
use crate::consts::EXTERNAL_CALL_TAG;
use crate::{
    consts::{BASE64_ENGINE, TENANT_HEADER},
    errors,
    external_service::ExternalServiceCall,
    types::keymanager::{
        BatchDecryptDataRequest, DataKeyCreateResponse, DecryptDataRequest,
        EncryptionCreateRequest, EncryptionTransferRequest, GetKeymanagerTenant, KeyManagerState,
        TransientBatchDecryptDataRequest, TransientDecryptDataRequest,
    },
};

const CONTENT_TYPE: &str = "Content-Type";
static ENCRYPTION_API_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();
static DEFAULT_ENCRYPTION_VERSION: &str = "v1";
#[cfg(any(feature = "km_forward_x_request_id", feature = "ext_services_latency"))]
const X_REQUEST_ID: &str = "X-Request-Id";

global_meter!(KEY_MANAGER_METER, "COMMON_UTILS_KEY_MANAGER");
histogram_metric_f64!(
    ENCRYPTION_SERVICE_CALL_DURATION,
    KEY_MANAGER_METER,
    name: "encryption_service.call.duration",
    description: "Duration of completed encryption-service call attempts",
    unit: "s",
);

fn encryption_service_operation(endpoint: &str) -> &'static str {
    match endpoint {
        "data/encrypt" => "encrypt",
        "data/decrypt" => "decrypt",
        "key/create" => "key_create",
        "key/transfer" => "key_transfer",
        _ => "other",
    }
}

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
    endpoint: &str,
) -> errors::CustomResult<reqwest::Response, errors::KeyManagerClientError>
where
    T: ConvertRaw,
{
    let client = get_api_encryption_client(state)?;
    let url = reqwest::Url::parse(&url)
        .change_context(errors::KeyManagerClientError::UrlEncodingFailed)?;

    let method_str = method.to_string();
    let start_time = std::time::Instant::now();

    let response_result = client
        .request(method, url)
        .json(&ConvertRaw::convert_raw(request_body)?)
        .headers(headers)
        .send()
        .await;

    let elapsed = start_time.elapsed();
    if let Some(context) = &state.metrics_context {
        let outcome = response_result.as_ref().map_or("failure", |response| {
            if response.status().is_success() {
                "success"
            } else {
                "failure"
            }
        });
        ENCRYPTION_SERVICE_CALL_DURATION.record(
            elapsed.as_secs_f64(),
            metric_attributes!(
                ("operation", encryption_service_operation(endpoint)),
                ("merchant_mode", context.merchant_mode),
                ("outcome", outcome),
            ),
        );
    }

    let response =
        response_result.change_context(errors::KeyManagerClientError::RequestNotSent(
            "Unable to send request to encryption service".to_string(),
        ))?;

    let latency_ms = elapsed.as_millis();
    let created_at_timestamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
    #[cfg(feature = "ext_services_latency")]
    {
        let downstream_request_id = response
            .headers()
            .get(X_REQUEST_ID)
            .and_then(|value| value.to_str().ok());

        logger::info!(
            tag = EXTERNAL_CALL_TAG,
            operation = endpoint,
            method = method_str,
            status_code = response.status().as_u16(),
            latency_ms = elapsed.as_secs_f64() * 1000.0,
            downstream_request_id,
        );
    }

    if let Some(request_id) = &state.request_id {
        state
            .event_emitter
            .emit_external_service_call(ExternalServiceCall {
                service_name: "keymanager".to_string(),
                endpoint: endpoint.to_string(),
                method: method_str.clone(),
                request_id: request_id.clone(),
                status_code: response.status().as_u16(),
                success: response.status().is_success(),
                latency_ms,
                created_at_timestamp,
            });
    } else {
        logger::warn!("KeyManager call made without emitting event: request_id missing");
    }

    Ok(response)
}

/// Generic function to call the Keymanager and parse the response back
#[instrument(skip_all)]
// Deja: the one seam every keymanager operation crosses. Substituted on replay
// so recorded ciphertext comes back verbatim and replay does not depend on a
// live keymanager.
#[cfg_attr(
    feature = "deja",
    deja::boundary(
        boundary = "km",
        component = "common_utils::keymanager",
        operation = "call_encryption_service",
        replay = Substitute,
        effect = Http,
        codec = deja::codec::ResultCodec::<R, errors::KeyManagerClientError>,
        // Identity is the request body via `ConvertRaw::wire_image`, not a
        // masked `Debug` — a masked capture collapses every encrypt/decrypt of
        // one merchant into a single args_hash.
        args = serde_json::json!({
            "method": method.as_str(),
            "endpoint": endpoint,
            "request": request_body.wire_image(),
        }),
    )
)]
pub async fn call_encryption_service<T, R>(
    state: &KeyManagerState,
    method: Method,
    endpoint: &str,
    request_body: T,
) -> errors::CustomResult<R, errors::KeyManagerClientError>
where
    T: GetKeymanagerTenant + ConvertRaw + Send + Sync + 'static + Debug,
    R: serde::de::DeserializeOwned + serde::Serialize,
{
    let url = format!("{}/{endpoint}", state.url);

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
        endpoint,
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
    /// The JSON body this request becomes on the wire, read without consuming
    /// it. Must match `convert_raw`'s output; each impl derives both from one
    /// mapping so they cannot drift.
    fn wire_image(&self) -> serde_json::Value;
}

/// `to_value` at a capture site; a serialization failure names itself so it
/// cannot collide with other failures.
fn capture_json<T: serde::Serialize + ?Sized>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or_else(|error| {
        serde_json::json!({
            "capture_failed": error.to_string(),
            "type": std::any::type_name::<T>(),
        })
    })
}

/// The text form keymanager expects for an already-encrypted payload: the bytes
/// as UTF-8 when they are text, otherwise `{version}:{base64}`.
fn encrypted_payload_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => format!(
            "{DEFAULT_ENCRYPTION_VERSION}:{}",
            BASE64_ENGINE.encode(bytes)
        ),
    }
}

impl<T: serde::Serialize> ConvertRaw for T {
    type Output = T;
    fn convert_raw(self) -> Result<Self::Output, errors::KeyManagerClientError> {
        Ok(self)
    }
    fn wire_image(&self) -> serde_json::Value {
        capture_json(self)
    }
}

impl TransientDecryptDataRequest {
    /// The wire request this transient form becomes; by reference so
    /// `convert_raw` and `wire_image` share one definition.
    fn to_wire(&self) -> DecryptDataRequest {
        DecryptDataRequest {
            identifier: self.identifier.clone(),
            data: StrongSecret::new(encrypted_payload_text(self.data.peek())),
        }
    }
}

impl ConvertRaw for TransientDecryptDataRequest {
    type Output = DecryptDataRequest;
    fn convert_raw(self) -> Result<Self::Output, errors::KeyManagerClientError> {
        Ok(self.to_wire())
    }
    fn wire_image(&self) -> serde_json::Value {
        capture_json(&self.to_wire())
    }
}

impl TransientBatchDecryptDataRequest {
    /// The wire request this transient form becomes. See
    /// [`TransientDecryptDataRequest::to_wire`].
    fn to_wire(&self) -> BatchDecryptDataRequest {
        BatchDecryptDataRequest {
            identifier: self.identifier.clone(),
            data: self
                .data
                .iter()
                .map(|(key, value)| {
                    (
                        key.to_owned(),
                        StrongSecret::new(encrypted_payload_text(value.peek())),
                    )
                })
                .collect(),
        }
    }
}

impl ConvertRaw for TransientBatchDecryptDataRequest {
    type Output = BatchDecryptDataRequest;
    fn convert_raw(self) -> Result<Self::Output, errors::KeyManagerClientError> {
        Ok(self.to_wire())
    }
    fn wire_image(&self) -> serde_json::Value {
        capture_json(&self.to_wire())
    }
}

/// A function to create the key in keymanager
#[instrument(skip_all)]
pub async fn create_key_in_key_manager(
    state: &KeyManagerState,
    request_body: EncryptionCreateRequest,
) -> errors::CustomResult<Option<DataKeyCreateResponse>, errors::KeyManagerError> {
    if !state.is_encryption_service_enabled() {
        logger::info!(
            "Encryption service is disabled, skipping key creation for identifier: {:?}",
            request_body.identifier
        );
        Ok(None)
    } else {
        call_encryption_service(state, Method::POST, "key/create", request_body)
            .await
            .map(Some)
            .change_context(errors::KeyManagerError::KeyAddFailed)
    }
}

/// A function to transfer the key in keymanager
#[instrument(skip_all)]
pub async fn transfer_key_to_key_manager(
    state: &KeyManagerState,
    request_body: EncryptionTransferRequest,
) -> errors::CustomResult<Option<DataKeyCreateResponse>, errors::KeyManagerError> {
    if !state.is_encryption_service_enabled() {
        logger::info!(
            "Encryption service is disabled, skipping key transfer for identifier: {:?}",
            request_body.identifier
        );
        Ok(None)
    } else {
        call_encryption_service(state, Method::POST, "key/transfer", request_body)
            .await
            .map(Some)
            .change_context(errors::KeyManagerError::KeyTransferFailed)
    }
}
