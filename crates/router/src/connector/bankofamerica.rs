pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use common_utils::request::RequestContent;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface};
use ring::{digest, hmac};
use time::OffsetDateTime;
use transformers as bankofamerica;
use url::Url;

use super::utils::{PaymentsAuthorizeRequestData, RouterData};
use crate::{
    configs::settings,
    connector::{utils as connector_utils, utils::RefundsRequestData},
    consts,
    core::errors::{self, CustomResult},
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::BytesExt,
};

pub const V_C_MERCHANT_ID: &str = "v-c-merchant-id";

#[derive(Debug, Clone)]
pub struct Bankofamerica;

impl api::Payment for Bankofamerica {}
impl api::PaymentSession for Bankofamerica {}
impl api::ConnectorAccessToken for Bankofamerica {}
impl api::MandateSetup for Bankofamerica {}
impl api::PaymentAuthorize for Bankofamerica {}
impl api::PaymentSync for Bankofamerica {}
impl api::PaymentCapture for Bankofamerica {}
impl api::PaymentVoid for Bankofamerica {}
impl api::Refund for Bankofamerica {}
impl api::RefundExecute for Bankofamerica {}
impl api::RefundSync for Bankofamerica {}
impl api::PaymentToken for Bankofamerica {}
impl api::PaymentsPreProcessing for Bankofamerica {}
impl api::PaymentsCompleteAuthorize for Bankofamerica {}

impl Bankofamerica {
    pub fn generate_digest(&self, payload: &[u8]) -> String {
        let payload_digest = digest::digest(&digest::SHA256, payload);
        consts::BASE64_ENGINE.encode(payload_digest)
    }

    pub fn generate_signature(
        &self,
        auth: bankofamerica::BankOfAmericaAuthType,
        host: String,
        resource: &str,
        payload: &String,
        date: OffsetDateTime,
        http_method: services::Method,
    ) -> CustomResult<String, errors::ConnectorError> {
        let bankofamerica::BankOfAmericaAuthType {
            api_key,
            merchant_account,
            api_secret,
        } = auth;
        let is_post_method = matches!(http_method, services::Method::Post);
        let digest_str = if is_post_method { "digest " } else { "" };
        let headers = format!("host date (request-target) {digest_str}{V_C_MERCHANT_ID}");
        let request_target = if is_post_method {
            format!("(request-target): post {resource}\ndigest: SHA-256={payload}\n")
        } else {
            format!("(request-target): get {resource}\n")
        };
        let signature_string = format!(
            "host: {host}\ndate: {date}\n{request_target}{V_C_MERCHANT_ID}: {}",
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

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Bankofamerica
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Bankofamerica
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
        let boa_req = self.get_request_body(req, connectors)?;
        let http_method = self.get_http_method();
        let auth = bankofamerica::BankOfAmericaAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account = auth.merchant_account.clone();
        let base_url = connectors.bankofamerica.base_url.as_str();
        let boa_host = Url::parse(base_url)
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let host = boa_host
            .host_str()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let path: String = self
            .get_url(req, connectors)?
            .chars()
            .skip(base_url.len() - 1)
            .collect();
        let sha256 = self.generate_digest(
            types::RequestBody::get_inner_value(boa_req)
                .expose()
                .as_bytes(),
        );
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
            (V_C_MERCHANT_ID.to_string(), merchant_account.into_masked()),
            ("Date".to_string(), date.to_string().into()),
            ("Host".to_string(), host.to_string().into()),
            ("Signature".to_string(), signature.into_masked()),
        ];
        if matches!(http_method, services::Method::Post | services::Method::Put) {
            headers.push((
                "Digest".to_string(),
                format!("SHA-256={sha256}").into_masked(),
            ));
        }
        Ok(headers)
    }
}

impl ConnectorCommon for Bankofamerica {
    fn id(&self) -> &'static str {
        "bankofamerica"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json;charset=utf-8"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.bankofamerica.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaErrorResponse = res
            .response
            .parse_struct("BankOfAmerica ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let error_message = if res.status_code == 401 {
            consts::CONNECTOR_UNAUTHORIZED_ERROR
        } else {
            consts::NO_ERROR_MESSAGE
        };
        match response {
            transformers::BankOfAmericaErrorResponse::StandardError(response) => {
                let (code, connector_reason) = match response.error_information {
                    Some(ref error_info) => (error_info.reason.clone(), error_info.message.clone()),
                    None => (
                        response
                            .reason
                            .map_or(consts::NO_ERROR_CODE.to_string(), |reason| {
                                reason.to_string()
                            }),
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

                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code,
                    message,
                    reason: Some(connector_reason),
                    attempt_status: None,
                    connector_transaction_id: None,
                })
            }
            transformers::BankOfAmericaErrorResponse::AuthenticationError(response) => {
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: consts::NO_ERROR_CODE.to_string(),
                    message: response.response.rmsg.clone(),
                    reason: Some(response.response.rmsg),
                    attempt_status: None,
                    connector_transaction_id: None,
                })
            }
        }
    }
}

impl ConnectorValidation for Bankofamerica {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
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

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Bankofamerica
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Bankofamerica
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Bankofamerica
{
    fn build_request(
        &self,
        _req: &types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented(
            "Setup Mandate flow for Bankofamerica".to_string(),
        )
        .into())
    }
}

impl
    ConnectorIntegration<
        api::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
    > for Bankofamerica
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
        let connector_router_data = bankofamerica::BankOfAmericaRouterData::try_from((
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
            bankofamerica::BankOfAmericaPreProcessingRequest::try_from(&connector_router_data)?;
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
        res: Response,
    ) -> CustomResult<types::PaymentsPreProcessingRouterData, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaPreProcessingResponse = res
            .response
            .parse_struct("BankOfAmerica AuthEnrollmentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Bankofamerica
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
        if req.is_three_ds() && req.request.is_card() {
            Ok(format!(
                "{}risk/v1/authentication-setups",
                self.base_url(connectors)
            ))
        } else {
            Ok(format!("{}pts/v2/payments/", self.base_url(connectors)))
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = bankofamerica::BankOfAmericaRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        if req.is_three_ds() && req.request.is_card() {
            let connector_req =
                bankofamerica::BankOfAmericaAuthSetupRequest::try_from(&connector_router_data)?;
            Ok(RequestContent::Json(Box::new(connector_req)))
        } else {
            let connector_req =
                bankofamerica::BankOfAmericaPaymentsRequest::try_from(&connector_router_data)?;
            Ok(RequestContent::Json(Box::new(connector_req)))
        }
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        if data.is_three_ds() && data.request.is_card() {
            let response: bankofamerica::BankOfAmericaAuthSetupResponse = res
                .response
                .parse_struct("Bankofamerica AuthSetupResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        } else {
            let response: bankofamerica::BankOfAmericaPaymentsResponse = res
                .response
                .parse_struct("Bankofamerica PaymentResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
        }
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaServerErrorResponse = res
            .response
            .parse_struct("BankOfAmericaServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let attempt_status = match response.reason {
            Some(reason) => match reason {
                transformers::Reason::SystemError => Some(enums::AttemptStatus::Failure),
                transformers::Reason::ServerTimeout | transformers::Reason::ServiceTimeout => None,
            },
            None => None,
        };
        Ok(ErrorResponse {
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
    > for Bankofamerica
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
        Ok(format!("{}pts/v2/payments/", self.base_url(connectors)))
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = bankofamerica::BankOfAmericaRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req =
            bankofamerica::BankOfAmericaPaymentsRequest::try_from(&connector_router_data)?;
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
        res: Response,
    ) -> CustomResult<types::PaymentsCompleteAuthorizeRouterData, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaPaymentsResponse = res
            .response
            .parse_struct("BankOfAmerica PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaServerErrorResponse = res
            .response
            .parse_struct("BankOfAmericaServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let attempt_status = match response.reason {
            Some(reason) => match reason {
                transformers::Reason::SystemError => Some(enums::AttemptStatus::Failure),
                transformers::Reason::ServerTimeout | transformers::Reason::ServiceTimeout => None,
            },
            None => None,
        };
        Ok(ErrorResponse {
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

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Bankofamerica
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
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
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}tss/v2/transactions/{connector_payment_id}",
            self.base_url(connectors)
        ))
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
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaTransactionResponse = res
            .response
            .parse_struct("BankOfAmerica PaymentSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Bankofamerica
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
            "{}pts/v2/payments/{connector_payment_id}/captures",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = bankofamerica::BankOfAmericaRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount_to_capture,
            req,
        ))?;
        let connector_req =
            bankofamerica::BankOfAmericaCaptureRequest::try_from(&connector_router_data)?;
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
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaPaymentsResponse = res
            .response
            .parse_struct("BankOfAmerica PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaServerErrorResponse = res
            .response
            .parse_struct("BankOfAmericaServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
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

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Bankofamerica
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
        let connector_router_data = bankofamerica::BankOfAmericaRouterData::try_from((
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
        let connector_req =
            bankofamerica::BankOfAmericaVoidRequest::try_from(&connector_router_data)?;

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
                .set_body(types::PaymentsVoidType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaPaymentsResponse = res
            .response
            .parse_struct("BankOfAmerica PaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaServerErrorResponse = res
            .response
            .parse_struct("BankOfAmericaServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
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

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Bankofamerica
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{connector_payment_id}/refunds",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = bankofamerica::BankOfAmericaRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let connector_req =
            bankofamerica::BankOfAmericaRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
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
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaRefundResponse = res
            .response
            .parse_struct("bankofamerica RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Bankofamerica
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
            "{}tss/v2/transactions/{refund_id}",
            self.base_url(connectors)
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
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaRsyncResponse = res
            .response
            .parse_struct("bankofamerica RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Bankofamerica {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
