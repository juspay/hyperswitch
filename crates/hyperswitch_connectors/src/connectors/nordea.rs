mod requests;
mod responses;
pub mod transformers;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts, date_time,
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        PreProcessing,
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsPreProcessingData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsPreProcessingRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use lazy_static::lazy_static;
use masking::{ExposeInterface, Mask, PeekInterface};
use requests::{
    NordeaPaymentsConfirmRequest, NordeaPaymentsRequest, NordeaRefundRequest, NordeaRouterData,
};
use responses::{NordeaPaymentsResponse, NordeaRefundResponse};
use ring::{
    digest,
    signature::{RsaKeyPair, RSA_PKCS1_SHA256},
};
use transformers::{get_error_data, NordeaAuthType};
use url::Url;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self, PaymentsAuthorizeRequestData},
};

#[derive(Clone)]
pub struct Nordea {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

struct SignatureParams<'a> {
    content_type: &'a str,
    host: &'a str,
    path: &'a str,
    payload_digest: Option<&'a str>,
    date: &'a str,
    http_method: Method,
}

impl Nordea {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }

    pub fn generate_digest(&self, payload: &[u8]) -> String {
        let payload_digest = digest::digest(&digest::SHA256, &payload);
        format!("sha-256={}", consts::BASE64_ENGINE.encode(payload_digest))
    }

    pub fn generate_digest_from_request(&self, payload: &RequestContent) -> String {
        let payload_bytes = match payload {
            RequestContent::FormUrlEncoded(form_data) => {
                // Get the JSON representation and convert to raw form string
                let json_value = form_data.masked_serialize().unwrap_or_default();
                if let serde_json::Value::Object(map) = json_value {
                    map.iter()
                        .map(|(k, v)| {
                            // Remove quotes from string values
                            let value = match v {
                                serde_json::Value::String(s) => s.to_string(),
                                _ => v.to_string(),
                            };
                            format!("{}={}", k, value)
                        })
                        .collect::<Vec<_>>()
                        .join("&")
                        .into_bytes()
                } else {
                    payload.get_inner_value().expose().as_bytes().to_vec()
                }
            }
            _ => payload.get_inner_value().expose().as_bytes().to_vec(),
        };

        self.generate_digest(&payload_bytes)
    }

    // For non-production environments, signature generation can be skipped and instead `SKIP_SIGNATURE_VALIDATION_FOR_SANDBOX` can be passed.
    fn generate_signature(
        &self,
        auth: &NordeaAuthType,
        signature_params: SignatureParams<'_>,
    ) -> CustomResult<String, errors::ConnectorError> {
        const REQUEST_WITHOUT_CONTENT_HEADERS: &str =
            "(request-target) x-nordea-originating-host x-nordea-originating-date";
        const REQUEST_WITH_CONTENT_HEADERS: &str = "(request-target) x-nordea-originating-host x-nordea-originating-date content-type digest";

        let method_string = signature_params.http_method.to_string().to_lowercase();
        let mut normalized_string = format!(
            "(request-target) {} {}\nx-nordea-originating-host: {}\nx-nordea-originating-date: {}",
            method_string, signature_params.path, signature_params.host, signature_params.date
        );

        let headers = if matches!(
            signature_params.http_method,
            Method::Post | Method::Put | Method::Patch
        ) {
            let digest = signature_params.payload_digest.unwrap_or("");
            normalized_string.push_str(&format!(
                "\ncontent-type: {}\ndigest: {}",
                signature_params.content_type, digest
            ));
            REQUEST_WITH_CONTENT_HEADERS
        } else {
            REQUEST_WITHOUT_CONTENT_HEADERS
        };

        let signature_base64 = {
            let private_key_der = pem::parse(auth.eidas_private_key.clone().expose())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "eIDAS Private Key",
                })?;
            let private_key_der_contents = private_key_der.contents();

            let key_pair = RsaKeyPair::from_der(private_key_der_contents).change_context(
                errors::ConnectorError::InvalidConnectorConfig {
                    config: "eIDAS Private Key",
                },
            )?;

            let mut signature = vec![0u8; key_pair.public().modulus_len()];
            key_pair
                .sign(
                    &RSA_PKCS1_SHA256,
                    &ring::rand::SystemRandom::new(),
                    normalized_string.as_bytes(),
                    &mut signature,
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;

            consts::BASE64_ENGINE.encode(signature)
        };

        Ok(format!(
            r#"keyId="{}",algorithm="rsa-sha256",headers="{}",signature="{}""#,
            auth.client_id.peek(),
            headers,
            signature_base64
        ))
    }
}

impl api::Payment for Nordea {}
impl api::PaymentSession for Nordea {}
impl api::ConnectorAccessToken for Nordea {}
impl api::MandateSetup for Nordea {}
impl api::PaymentAuthorize for Nordea {}
impl api::PaymentSync for Nordea {}
impl api::PaymentCapture for Nordea {}
impl api::PaymentVoid for Nordea {}
impl api::Refund for Nordea {}
impl api::RefundExecute for Nordea {}
impl api::RefundSync for Nordea {}
impl api::PaymentToken for Nordea {}
impl api::PaymentsPreProcessing for Nordea {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Nordea
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Nordea
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let access_token = req
            .access_token
            .clone()
            .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth = NordeaAuthType::try_from(&req.connector_auth_type)?;
        let content_type = self.get_content_type().to_string();
        let http_method = self.get_http_method();

        // Extract host from base URL
        let nordea_host = Url::parse(self.base_url(connectors))
            .change_context(errors::ConnectorError::RequestEncodingFailed)?
            .host_str()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?
            .to_string();

        let nordea_origin_date = date_time::now_rfc7231_http_date()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let full_url = self.get_url(req, connectors)?;
        let url_parsed =
            Url::parse(&full_url).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let path = url_parsed.path();
        let path_with_query = if let Some(query) = url_parsed.query() {
            format!("{}?{}", path, query)
        } else {
            path.to_string()
        };

        let mut required_headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                content_type.clone().into(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", access_token.token.clone().peek()).into_masked(),
            ),
            (
                "X-IBM-Client-ID".to_string(),
                auth.client_id.clone().expose().into_masked(),
            ),
            (
                "X-IBM-Client-Secret".to_string(),
                auth.client_secret.clone().expose().into_masked(),
            ),
            (
                "X-Nordea-Originating-Date".to_string(),
                nordea_origin_date.clone().into_masked(),
            ),
            (
                "X-Nordea-Originating-Host".to_string(),
                nordea_host.clone().into_masked(),
            ),
        ];

        if matches!(http_method, Method::Post | Method::Put | Method::Patch) {
            let nordea_request = self.get_request_body(req, connectors)?;

            let sha256_digest = self.generate_digest_from_request(&nordea_request)?;

            // Add Digest header
            required_headers.push((
                "Digest".to_string(),
                format!("{}", sha256_digest).into_masked(),
            ));

            let signature = self.generate_signature(
                &auth,
                SignatureParams {
                    content_type: &content_type,
                    host: &nordea_host,
                    path,
                    payload_digest: Some(&sha256_digest),
                    date: &nordea_origin_date,
                    http_method,
                },
            )?;

            required_headers.push(("Signature".to_string(), signature.into_masked()));
        } else {
            // Generate signature without digest for GET requests
            let signature = self.generate_signature(
                &auth,
                SignatureParams {
                    content_type: &content_type,
                    host: &nordea_host,
                    path: &path_with_query,
                    payload_digest: None,
                    date: &nordea_origin_date,
                    http_method,
                },
            )?;

            required_headers.push(("Signature".to_string(), signature.into_masked()));
        }

        Ok(required_headers)
    }
}

impl ConnectorCommon for Nordea {
    fn id(&self) -> &'static str {
        "nordea"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.nordea.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: responses::NordeaErrorResponse = res
            .response
            .parse_struct("NordeaErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: get_error_data(response.error.as_ref())
                .and_then(|failure| failure.code.clone())
                .unwrap_or(NO_ERROR_CODE.to_string()),
            message: get_error_data(response.error.as_ref())
                .and_then(|failure| failure.description.clone())
                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
            reason: get_error_data(response.error.as_ref())
                .and_then(|failure| failure.failure_type.clone()),
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        })
    }
}

impl ConnectorValidation for Nordea {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Nordea {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Nordea {
    // TO-DO: Implement Access Token flow
    // Nordea has 2 types of access token flows:
    // 1. Redirect access authorization flow (2 steps) <- we're interested in this one given the simplicity and it requires a core change (may be, introduce a new flow)
    //      Initiation of access authorisation.
    //      Receipt of an authorisation code.
    //      Exchange of the authorisation code for an access token and a refresh token.
    //          The refresh token can be used to obtain a new access token after its expiry.
    // 2. Decoupled access authorization flow (6 steps)
    //      Initiation of customer authentication
    //      Polling of the authentication status
    //      Receipt of the first authorisation code
    //      Initiation of the access authorisation
    //      Receipt of the second authorisation code
    //      Exchange of the second authorisation code for an access token and a refresh token.
    //          The refresh token can be used to obtain a new access token after its expiry.

    // Redirect access authorization flow:
    // 1. Hit `/authorize` endpoint to get a `code` (one time use only and is never needed once used to get `access_token`)
    // 2. Use the `code` to hit `authorize/token` endpoint to get an `access_token` and `refresh_token` depending on the grant_type (`authorization_code` & `refresh_token`) that is passed.
    //    `grant_type` should be `authorization_code` for the first step and `refresh_token` for the subsequent calls.
    //
    // FAQ:
    // Access is valid for 3600 seconds
    // Refresh token is long lived (depends on `duration` field in minutes passed in `/authorize` call), can only be used once
    //
    // Flow would be like (POST):
    // 1. Check if we already have a refresh token and is not expired (depends on the duration passed during the retrieval of `code` -- max 180 days).
    //      1.1. If yes, use it to get an access token after access token expires.
    //      1.2. If no, hit `/authorize` endpoint to get a `code`.
    //          1.2.1 If `code` is received, use it to hit `authorize/token` endpoint to get an `access_token` and `refresh_token`.
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Nordea {}

impl ConnectorIntegration<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>
    for Nordea
{
    fn get_headers(
        &self,
        req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsPreProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(_connectors),
            "/personal/v5/payments/sepa-credit-transfer"
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsPreProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let minor_amount =
            req.request
                .minor_amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "minor_amount",
                })?;
        let currency =
            req.request
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?;

        let amount = utils::convert_amount(self.amount_converter, minor_amount, currency)?;
        let connector_router_data = NordeaRouterData::from((amount, req));
        let connector_req = NordeaPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsPreProcessingType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsPreProcessingType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsPreProcessingType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsPreProcessingRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsPreProcessingRouterData, errors::ConnectorError> {
        let response: NordeaPaymentsResponse = res
            .response
            .parse_struct("NordeaPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Nordea {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = self.build_headers(req, connectors)?;

        // Add the X-Nordea-Originating-User-Ip header for authorize (connector confirm) calls
        let user_ip = req.request.get_ip_address()?;

        headers.push((
            "X-Nordea-Originating-User-Ip".to_string(),
            user_ip.peek().clone().into_masked(),
        ));

        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(_connectors),
            "/personal/v5/payments"
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = NordeaRouterData::from((amount, req));
        let connector_req = NordeaPaymentsConfirmRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: NordeaPaymentsResponse = res
            .response
            .parse_struct("NordeaPaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Nordea {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.clone();
        let connector_transaction_id = id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;

        Ok(format!(
            "{}{}{}",
            self.base_url(_connectors),
            "/personal/v5/payments/",
            connector_transaction_id
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: NordeaPaymentsResponse = res
            .response
            .parse_struct("NordeaPaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Nordea {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Nordea {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Nordea {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data = NordeaRouterData::from((refund_amount, req));
        let connector_req = NordeaRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundExecuteType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::RefundExecuteType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: NordeaRefundResponse = res
            .response
            .parse_struct("NordeaRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Nordea {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .set_body(types::RefundSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: NordeaRefundResponse = res
            .response
            .parse_struct("NordeaRefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Nordea {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

lazy_static! {
    static ref NORDEA_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
        display_name:
            "Nordea",
        description:
            "Nordea is one of the leading financial services group in the Nordics and the preferred choice for millions across the region.",
        connector_type: enums::PaymentConnectorCategory::PaymentGateway,
    };
    static ref NORDEA_SUPPORTED_PAYMENT_METHODS: SupportedPaymentMethods = {
        let nordea_supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::SequentialAutomatic,
        ];

        let mut nordea_supported_payment_methods = SupportedPaymentMethods::new();

        nordea_supported_payment_methods.add(
            enums::PaymentMethod::BankDebit,
            enums::PaymentMethodType::Sepa,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::NotSupported,
                // Supported only in corporate API (corporate accounts)
                refunds: common_enums::FeatureStatus::NotSupported,
                supported_capture_methods: nordea_supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        nordea_supported_payment_methods
    };
    static ref NORDEA_SUPPORTED_WEBHOOK_FLOWS: Vec<enums::EventClass> = Vec::new();
}

impl ConnectorSpecifications for Nordea {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&*NORDEA_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*NORDEA_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&*NORDEA_SUPPORTED_WEBHOOK_FLOWS)
    }
}
