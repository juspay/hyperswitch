pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use common_utils::request::RequestContent;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface};
use ring::{digest, hmac};
use time::OffsetDateTime;
use transformers as cybersource;
use url::Url;

use super::utils::{PaymentsAuthorizeRequestData, RouterData};
use crate::{
    configs::settings,
    connector::{utils as connector_utils, utils::RefundsRequestData},
    consts,
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
    },
    utils::BytesExt,
};

#[derive(Debug, Clone)]
pub struct Cybersource;

impl Cybersource {
    pub fn generate_digest(&self, payload: &[u8]) -> String {
        let payload_digest = digest::digest(&digest::SHA256, payload);
        consts::BASE64_ENGINE.encode(payload_digest)
    }

    pub fn generate_signature(
        &self,
        auth: cybersource::CybersourceAuthType,
        host: String,
        resource: &str,
        payload: &String,
        date: OffsetDateTime,
        http_method: services::Method,
    ) -> CustomResult<String, errors::ConnectorError> {
        let cybersource::CybersourceAuthType {
            api_key,
            merchant_account,
            api_secret,
        } = auth;
        let is_post_method = matches!(http_method, services::Method::Post);
        let is_patch_method = matches!(http_method, services::Method::Patch);
        let is_delete_method = matches!(http_method, services::Method::Delete);
        let digest_str = if is_post_method || is_patch_method {
            "digest "
        } else {
            ""
        };
        let headers = format!("host date (request-target) {digest_str}v-c-merchant-id");
        let request_target = if is_post_method {
            format!("(request-target): post {resource}\ndigest: SHA-256={payload}\n")
        } else if is_patch_method {
            format!("(request-target): patch {resource}\ndigest: SHA-256={payload}\n")
        } else if is_delete_method {
            format!("(request-target): delete {resource}\n")
        } else {
            format!("(request-target): get {resource}\n")
        };
        let signature_string = format!(
            "host: {host}\ndate: {date}\n{request_target}v-c-merchant-id: {}",
            merchant_account.peek()
        );
        let key_value = consts::BASE64_ENGINE
            .decode(api_secret.expose())
            .into_report()
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "connector_account_details.api_secret",
            })?;
        let key = hmac::Key::new(hmac::HMAC_SHA256, &key_value);
        let signature_value =
            consts::BASE64_ENGINE.encode(hmac::sign(&key, signature_string.as_bytes()).as_ref());
        let signature_header = format!(
            r#"keyid="{}", algorithm="HmacSHA256", headers="{headers}", signature="{signature_value}""#,
            api_key.peek()
        );

        Ok(signature_header)
    }
}

impl ConnectorCommon for Cybersource {
    fn id(&self) -> &'static str {
        "cybersource"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json;charset=utf-8"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.cybersource.base_url.as_ref()
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn build_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: cybersource::CybersourceErrorResponse = res
            .response
            .parse_struct("Cybersource ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let error_message = if res.status_code == 401 {
            consts::CONNECTOR_UNAUTHORIZED_ERROR
        } else {
            consts::NO_ERROR_MESSAGE
        };
        match response {
            transformers::CybersourceErrorResponse::StandardError(response) => {
                let (code, connector_reason) = match response.error_information {
                    Some(ref error_info) => (error_info.reason.clone(), error_info.message.clone()),
                    None => (
                        response
                            .reason
                            .map_or(consts::NO_ERROR_CODE.to_string(), |reason| reason),
                        response
                            .message
                            .map_or(error_message.to_string(), |message| message),
                    ),
                };
                let message = match response.details {
                    Some(details) => details
                        .iter()
                        .map(|det| format!("{} : {}", det.field, det.reason))
                        .collect::<Vec<_>>()
                        .join(", "),
                    None => connector_reason.clone(),
                };

                Ok(types::ErrorResponse {
                    status_code: res.status_code,
                    code,
                    message,
                    reason: Some(connector_reason),
                    attempt_status: None,
                    connector_transaction_id: None,
                })
            }
            transformers::CybersourceErrorResponse::AuthenticationError(response) => {
                Ok(types::ErrorResponse {
                    status_code: res.status_code,
                    code: consts::NO_ERROR_CODE.to_string(),
                    message: response.response.rmsg.clone(),
                    reason: Some(response.response.rmsg),
                    attempt_status: None,
                    connector_transaction_id: None,
                })
            }
            transformers::CybersourceErrorResponse::NotAvailableError(response) => {
                let error_response = response
                    .errors
                    .iter()
                    .map(|error_info| {
                        format!(
                            "{}: {}",
                            error_info.error_type.clone().unwrap_or("".to_string()),
                            error_info.message.clone().unwrap_or("".to_string())
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(" & ");
                Ok(types::ErrorResponse {
                    status_code: res.status_code,
                    code: consts::NO_ERROR_CODE.to_string(),
                    message: error_response.clone(),
                    reason: Some(error_response),
                    attempt_status: None,
                    connector_transaction_id: None,
                })
            }
        }
    }
}

impl ConnectorValidation for Cybersource {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Cybersource
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, services::request::Maskable<String>)>, errors::ConnectorError>
    {
        let date = OffsetDateTime::now_utc();
        let cybersource_req = self.get_request_body(req, connectors)?;
        let auth = cybersource::CybersourceAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account = auth.merchant_account.clone();
        let base_url = connectors.cybersource.base_url.as_str();
        let cybersource_host = Url::parse(base_url)
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let host = cybersource_host
            .host_str()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let path: String = self
            .get_url(req, connectors)?
            .chars()
            .skip(base_url.len() - 1)
            .collect();
        let sha256 = self.generate_digest(
            types::RequestBody::get_inner_value(cybersource_req)
                .expose()
                .as_bytes(),
        );
        let http_method = self.get_http_method();
        let signature = self.generate_signature(
            auth,
            host.to_string(),
            path.as_str(),
            &sha256,
            date,
            http_method,
        )?;

        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::ACCEPT.to_string(),
                "application/hal+json;charset=utf-8".to_string().into(),
            ),
            (
                "v-c-merchant-id".to_string(),
                merchant_account.into_masked(),
            ),
            ("Date".to_string(), date.to_string().into()),
            ("Host".to_string(), host.to_string().into()),
            ("Signature".to_string(), signature.into_masked()),
        ];
        if matches!(
            http_method,
            services::Method::Post | services::Method::Put | services::Method::Patch
        ) {
            headers.push((
                "Digest".to_string(),
                format!("SHA-256={sha256}").into_masked(),
            ));
        }
        Ok(headers)
    }
}

impl api::Payment for Cybersource {}
impl api::PaymentAuthorize for Cybersource {}
impl api::PaymentSync for Cybersource {}
impl api::PaymentVoid for Cybersource {}
impl api::PaymentCapture for Cybersource {}
impl api::PaymentIncrementalAuthorization for Cybersource {}
impl api::MandateSetup for Cybersource {}
impl api::ConnectorAccessToken for Cybersource {}
impl api::PaymentToken for Cybersource {}
impl api::PaymentsPreProcessing for Cybersource {}
impl api::PaymentsCompleteAuthorize for Cybersource {}
impl api::ConnectorMandateRevoke for Cybersource {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Cybersource
{
    // Not Implemented (R)
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Cybersource
{
    fn get_headers(
        &self,
        req: &types::SetupMandateRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        _req: &types::SetupMandateRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}pts/v2/payments/", self.base_url(connectors)))
    }
    fn get_request_body(
        &self,
        req: &types::SetupMandateRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = cybersource::CybersourceZeroMandateRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::SetupMandateRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<common_utils::request::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::SetupMandateType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::SetupMandateType::get_headers(self, req, connectors)?)
                .set_body(types::SetupMandateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::SetupMandateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::SetupMandateRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourceSetupMandatesResponse = res
            .response
            .parse_struct("CybersourceSetupMandatesResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    ConnectorIntegration<
        api::MandateRevoke,
        types::MandateRevokeRequestData,
        types::MandateRevokeResponseData,
    > for Cybersource
{
    fn get_headers(
        &self,
        req: &types::MandateRevokeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_http_method(&self) -> services::Method {
        services::Method::Delete
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        req: &types::MandateRevokeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}tms/v1/paymentinstruments/{}",
            self.base_url(connectors),
            connector_utils::RevokeMandateRequestData::get_connector_mandate_id(&req.request)?
        ))
    }
    fn build_request(
        &self,
        req: &types::MandateRevokeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Delete)
                .url(&types::MandateRevokeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::MandateRevokeType::get_headers(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &types::MandateRevokeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::MandateRevokeRouterData, errors::ConnectorError> {
        if matches!(res.status_code, 204) {
            event_builder.map(|i| i.set_response_body(&serde_json::json!({"mandate_status": common_enums::MandateStatus::Revoked.to_string()})));
            Ok(types::MandateRevokeRouterData {
                response: Ok(types::MandateRevokeResponseData {
                    mandate_status: common_enums::MandateStatus::Revoked,
                }),
                ..data.clone()
            })
        } else {
            // If http_code != 204 || http_code != 4xx, we dont know any other response scenario yet.
            let response_value: serde_json::Value = serde_json::from_slice(&res.response)
                .into_report()
                .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
            let response_string = response_value.to_string();

            event_builder.map(|i| {
                i.set_response_body(
                    &serde_json::json!({"response_string": response_string.clone()}),
                )
            });
            router_env::logger::info!(connector_response=?response_string);

            Ok(types::MandateRevokeRouterData {
                response: Err(types::ErrorResponse {
                    code: consts::NO_ERROR_CODE.to_string(),
                    message: response_string.clone(),
                    reason: Some(response_string),
                    status_code: res.status_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..data.clone()
            })
        }
    }
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}
impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Cybersource
{
    // Not Implemented (R)
}

impl api::PaymentSession for Cybersource {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Cybersource
{
}

impl
    ConnectorIntegration<
        api::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
    > for Cybersource
{
    fn get_headers(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let redirect_response = req.request.redirect_response.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "redirect_response",
            },
        )?;
        match redirect_response.params {
            Some(param) if !param.clone().peek().is_empty() => Ok(format!(
                "{}risk/v1/authentications",
                self.base_url(connectors)
            )),
            Some(_) | None => Ok(format!(
                "{}risk/v1/authentication-results",
                self.base_url(connectors)
            )),
        }
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = cybersource::CybersourceRouterData::try_from((
            &self.get_currency_unit(),
            req.request
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?,
            req.request
                .amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "amount",
                })?,
            req,
        ))?;
        let connector_req =
            cybersource::CybersourcePreProcessingRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        data: &types::PaymentsPreProcessingRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsPreProcessingRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourcePreProcessingResponse = res
            .response
            .parse_struct("Cybersource AuthEnrollmentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{}/captures",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = cybersource::CybersourceRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount_to_capture,
            req,
        ))?;
        let connector_req =
            cybersource::CybersourcePaymentsCaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsCaptureType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>,
        errors::ConnectorError,
    > {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: cybersource::CybersourceServerErrorResponse = res
            .response
            .parse_struct("CybersourceServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        Ok(types::ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response.status.unwrap_or(consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, services::request::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_http_method(&self) -> services::Method {
        services::Method::Get
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}tss/v2/transactions/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourceTransactionResponse = res
            .response
            .parse_struct("Cybersource PaymentSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        if req.is_three_ds()
            && req.request.is_card()
            && req.request.connector_mandate_id().is_none()
        {
            Ok(format!(
                "{}risk/v1/authentication-setups",
                api::ConnectorCommon::base_url(self, connectors)
            ))
        } else {
            Ok(format!(
                "{}pts/v2/payments/",
                api::ConnectorCommon::base_url(self, connectors)
            ))
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = cybersource::CybersourceRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        if req.is_three_ds()
            && req.request.is_card()
            && req.request.connector_mandate_id().is_none()
        {
            let connector_req =
                cybersource::CybersourceAuthSetupRequest::try_from(&connector_router_data)?;
            Ok(RequestContent::Json(Box::new(connector_req)))
        } else {
            let connector_req =
                cybersource::CybersourcePaymentsRequest::try_from(&connector_router_data)?;
            Ok(RequestContent::Json(Box::new(connector_req)))
        }
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsAuthorizeType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(types::PaymentsAuthorizeType::get_headers(
                self, req, connectors,
            )?)
            .set_body(self.get_request_body(req, connectors)?)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        if data.is_three_ds()
            && data.request.is_card()
            && data.request.connector_mandate_id().is_none()
        {
            let response: cybersource::CybersourceAuthSetupResponse = res
                .response
                .parse_struct("Cybersource AuthSetupResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            event_builder.map(|i| i.set_response_body(&response));
            router_env::logger::info!(connector_response=?response);
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        } else {
            let response: cybersource::CybersourcePaymentsResponse = res
                .response
                .parse_struct("Cybersource PaymentResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            event_builder.map(|i| i.set_response_body(&response));
            router_env::logger::info!(connector_response=?response);
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: cybersource::CybersourceServerErrorResponse = res
            .response
            .parse_struct("CybersourceServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        let attempt_status = match response.reason {
            Some(reason) => match reason {
                transformers::Reason::SystemError => Some(enums::AttemptStatus::Failure),
                transformers::Reason::ServerTimeout | transformers::Reason::ServiceTimeout => None,
            },
            None => None,
        };
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response.status.unwrap_or(consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status,
            connector_transaction_id: None,
        })
    }
}

impl
    ConnectorIntegration<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for Cybersource
{
    fn get_headers(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        _req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}pts/v2/payments/",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = cybersource::CybersourceRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req =
            cybersource::CybersourcePaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCompleteAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsCompleteAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsCompleteAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCompleteAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCompleteAuthorizeRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: cybersource::CybersourceServerErrorResponse = res
            .response
            .parse_struct("CybersourceServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        let attempt_status = match response.reason {
            Some(reason) => match reason {
                transformers::Reason::SystemError => Some(enums::AttemptStatus::Failure),
                transformers::Reason::ServerTimeout | transformers::Reason::ServiceTimeout => None,
            },
            None => None,
        };
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response.status.unwrap_or(consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{connector_payment_id}/reversals",
            self.base_url(connectors)
        ))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = cybersource::CybersourceRouterData::try_from((
            &self.get_currency_unit(),
            req.request
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "Currency",
                })?,
            req.request
                .amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "Amount",
                })?,
            req,
        ))?;
        let connector_req = cybersource::CybersourceVoidRequest::try_from(&connector_router_data)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourcePaymentsResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: cybersource::CybersourceServerErrorResponse = res
            .response
            .parse_struct("CybersourceServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        Ok(types::ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response.status.unwrap_or(consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl api::Refund for Cybersource {}
impl api::RefundExecute for Cybersource {}
impl api::RefundSync for Cybersource {}

#[allow(dead_code)]
impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &types::RefundExecuteRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundExecuteRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{}/refunds",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundExecuteRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = cybersource::CybersourceRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let connector_req =
            cybersource::CybersourceRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &types::RefundExecuteRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundExecuteType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundExecuteRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::RefundExecuteRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourceRefundResponse = res
            .response
            .parse_struct("Cybersource RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[allow(dead_code)]
impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Cybersource
{
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_http_method(&self) -> services::Method {
        services::Method::Get
    }
    fn get_url(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req.request.get_connector_refund_id()?;
        Ok(format!(
            "{}tss/v2/transactions/{}",
            self.base_url(connectors),
            refund_id
        ))
    }
    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: cybersource::CybersourceRsyncResponse = res
            .response
            .parse_struct("Cybersource RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    ConnectorIntegration<
        api::IncrementalAuthorization,
        types::PaymentsIncrementalAuthorizationData,
        types::PaymentsResponseData,
    > for Cybersource
{
    fn get_headers(
        &self,
        req: &types::PaymentsIncrementalAuthorizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_http_method(&self) -> services::Method {
        services::Method::Patch
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsIncrementalAuthorizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsIncrementalAuthorizationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = cybersource::CybersourceRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.additional_amount,
            req,
        ))?;
        let connector_request =
            cybersource::CybersourcePaymentsIncrementalAuthorizationRequest::try_from(
                &connector_router_data,
            )?;
        Ok(RequestContent::Json(Box::new(connector_request)))
    }
    fn build_request(
        &self,
        req: &types::PaymentsIncrementalAuthorizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Patch)
                .url(&types::IncrementalAuthorizationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::IncrementalAuthorizationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::IncrementalAuthorizationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &types::PaymentsIncrementalAuthorizationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<
            api::IncrementalAuthorization,
            types::PaymentsIncrementalAuthorizationData,
            types::PaymentsResponseData,
        >,
        errors::ConnectorError,
    > {
        let response: cybersource::CybersourcePaymentsIncrementalAuthorizationResponse = res
            .response
            .parse_struct("Cybersource PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            true,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Cybersource {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Ok(api::IncomingWebhookEvent::EventNotSupported)
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
