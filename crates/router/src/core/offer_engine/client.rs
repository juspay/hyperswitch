use base64::Engine;
use common_utils::{
    errors::CustomResult,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use external_services::http_client;
use hyperswitch_masking::{Mask, PeekInterface};

use super::types::{OfferEngineError, ResolvedOfferEngineConfig};
use crate::{consts::BASE64_ENGINE, routes::SessionState};

const OFFERS_LIST_PATH: &str = "v1/offers/list";

#[derive(Debug, Clone)]
pub struct OfferEngineClient {
    config: ResolvedOfferEngineConfig,
}

impl OfferEngineClient {
    pub fn new(config: ResolvedOfferEngineConfig) -> Self {
        Self { config }
    }

    fn build_url(&self, path: &str) -> CustomResult<url::Url, OfferEngineError> {
        let mut base_url = self.config.base_url.clone();
        if !base_url.path().ends_with('/') {
            let with_slash = format!("{}/", base_url.path());
            base_url.set_path(&with_slash);
        }
        base_url
            .join(path)
            .change_context(OfferEngineError::RequestFailed)
            .attach_printable_lazy(|| format!("Failed to build Offer Engine URL for path: {path}"))
    }

    pub async fn send<Req, Resp>(
        &self,
        state: &SessionState,
        method: Method,
        path: &str,
        body: Option<Req>,
    ) -> CustomResult<Resp, OfferEngineError>
    where
        Req: serde::Serialize + Send + 'static,
        Resp: serde::de::DeserializeOwned,
    {
        let url = self.build_url(path)?;

        let auth_value = format!(
            "Basic {}",
            BASE64_ENGINE.encode(format!("{}:", self.config.api_key.peek()))
        );

        let mut request_builder = RequestBuilder::new()
            .method(method)
            .url(url.as_str())
            .attach_default_headers()
            .headers(vec![(
                "Authorization".to_string(),
                auth_value.into_masked(),
            )]);

        if let Some(body) = body {
            request_builder = request_builder.set_body(RequestContent::Json(Box::new(body)));
        }

        let request = request_builder.build();

        http_client::send_request(&state.conf.proxy, request, None)
            .await
            .change_context(OfferEngineError::RequestFailed)
            .attach_printable("Error while sending request to Offer Engine")?
            .json::<Resp>()
            .await
            .change_context(OfferEngineError::ResponseParseFailed)
            .attach_printable("Error while deserializing Offer Engine response")
    }

    pub async fn check_connectivity(&self, state: &SessionState) -> ConnectivityResult {
        let url = match self.build_url(OFFERS_LIST_PATH) {
            Ok(url) => url,
            Err(err) => {
                return ConnectivityResult {
                    reachable: false,
                    status_code: None,
                    detail: format!("Failed to build Offer Engine URL: {err:?}"),
                }
            }
        };

        let auth_value = format!(
            "Basic {}",
            BASE64_ENGINE.encode(format!("{}:", self.config.api_key.peek()))
        );

        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(url.as_str())
            .attach_default_headers()
            .headers(vec![(
                "Authorization".to_string(),
                auth_value.into_masked(),
            )])
            .set_body(RequestContent::Json(Box::new(serde_json::json!({}))))
            .build();

        match http_client::send_request(&state.conf.proxy, request, None).await {
            Ok(response) => {
                let status = response.status().as_u16();
                let detail = match status {
                    401 | 403 => "Reached Offer Engine, but authentication failed".to_string(),
                    _ => format!("Reached Offer Engine (HTTP {status})"),
                };
                ConnectivityResult {
                    reachable: true,
                    status_code: Some(status),
                    detail,
                }
            }
            Err(err) => ConnectivityResult {
                reachable: false,
                status_code: None,
                detail: format!(
                    "Could not reach Offer Engine (network / allowlisting issue): {err:?}"
                ),
            },
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectivityResult {
    pub reachable: bool,
    pub status_code: Option<u16>,
    pub detail: String,
}
