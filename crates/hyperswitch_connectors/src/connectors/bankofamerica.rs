pub mod transformers;

use std::fmt::Debug;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts,
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{
        PaymentsAuthorizeType, PaymentsCaptureType, PaymentsSyncType, PaymentsVoidType,
        RefundExecuteType, RefundSyncType, Response, SetupMandateType,
    },
    webhooks,
};
use masking::{ExposeInterface, Mask, Maskable, PeekInterface};
use ring::{digest, hmac};
use time::OffsetDateTime;
use transformers as bankofamerica;
use url::Url;

use crate::{
    constants::{self, headers},
    types::ResponseRouterData,
    utils::{self, PaymentMethodDataType, RefundsRequestData},
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
        http_method: Method,
    ) -> CustomResult<String, errors::ConnectorError> {
        let bankofamerica::BankOfAmericaAuthType {
            api_key,
            merchant_account,
            api_secret,
        } = auth;
        let is_post_method = matches!(http_method, Method::Post);
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

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Bankofamerica
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Bankofamerica
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let date = OffsetDateTime::now_utc();
        let boa_req = self.get_request_body(req, connectors)?;
        let http_method = self.get_http_method();
        let auth = bankofamerica::BankOfAmericaAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account = auth.merchant_account.clone();
        let base_url = connectors.bankofamerica.base_url.as_str();
        let boa_host =
            Url::parse(base_url).change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let host = boa_host
            .host_str()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let path: String = self
            .get_url(req, connectors)?
            .chars()
            .skip(base_url.len() - 1)
            .collect();
        let sha256 = self.generate_digest(boa_req.get_inner_value().expose().as_bytes());
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
        if matches!(http_method, Method::Post | Method::Put) {
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

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.bankofamerica.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaErrorResponse = res
            .response
            .parse_struct("BankOfAmerica ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let error_message = if res.status_code == 401 {
            constants::CONNECTOR_UNAUTHORIZED_ERROR
        } else {
            hyperswitch_interfaces::consts::NO_ERROR_MESSAGE
        };
        match response {
            transformers::BankOfAmericaErrorResponse::StandardError(response) => {
                let (code, message, reason) = match response.error_information {
                    Some(ref error_info) => {
                        let detailed_error_info = error_info.details.as_ref().map(|details| {
                            details
                                .iter()
                                .map(|det| format!("{} : {}", det.field, det.reason))
                                .collect::<Vec<_>>()
                                .join(", ")
                        });
                        (
                            error_info.reason.clone(),
                            error_info.reason.clone(),
                            transformers::get_error_reason(
                                Some(error_info.message.clone()),
                                detailed_error_info,
                                None,
                            ),
                        )
                    }
                    None => {
                        let detailed_error_info = response.details.map(|details| {
                            details
                                .iter()
                                .map(|det| format!("{} : {}", det.field, det.reason))
                                .collect::<Vec<_>>()
                                .join(", ")
                        });
                        (
                            response.reason.clone().map_or(
                                hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(),
                                |reason| reason.to_string(),
                            ),
                            response
                                .reason
                                .map_or(error_message.to_string(), |reason| reason.to_string()),
                            transformers::get_error_reason(
                                response.message,
                                detailed_error_info,
                                None,
                            ),
                        )
                    }
                };

                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code,
                    message,
                    reason,
                    attempt_status: None,
                    connector_transaction_id: None,
                    issuer_error_code: None,
                    issuer_error_message: None,
                })
            }
            transformers::BankOfAmericaErrorResponse::AuthenticationError(response) => {
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(),
                    message: response.response.rmsg.clone(),
                    reason: Some(response.response.rmsg),
                    attempt_status: None,
                    connector_transaction_id: None,
                    issuer_error_code: None,
                    issuer_error_message: None,
                })
            }
        }
    }
}

impl ConnectorValidation for Bankofamerica {
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _payment_method: enums::PaymentMethod,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic
            | enums::CaptureMethod::Manual
            | enums::CaptureMethod::SequentialAutomatic => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }

    fn validate_mandate_payment(
        &self,
        pm_type: Option<enums::PaymentMethodType>,
        pm_data: PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            PaymentMethodDataType::Card,
            PaymentMethodDataType::ApplePay,
            PaymentMethodDataType::GooglePay,
        ]);
        utils::is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Bankofamerica {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Bankofamerica {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Bankofamerica
{
    fn get_headers(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        _req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}pts/v2/payments/", self.base_url(connectors)))
    }
    fn get_request_body(
        &self,
        req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = bankofamerica::BankOfAmericaPaymentsRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&SetupMandateType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(SetupMandateType::get_headers(self, req, connectors)?)
                .set_body(SetupMandateType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &SetupMandateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SetupMandateRouterData, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaSetupMandatesResponse = res
            .response
            .parse_struct("BankOfAmericaSetupMandatesResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaServerErrorResponse = res
            .response
            .parse_struct("BankOfAmericaServerErrorResponse")
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
        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status,
            connector_transaction_id: None,
            issuer_error_code: None,
            issuer_error_message: None,
        })
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Bankofamerica
{
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}pts/v2/payments/",
            ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
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
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsAuthorizeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsAuthorizeType::get_headers(self, req, connectors)?)
                .set_body(PaymentsAuthorizeType::get_request_body(
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
        let response: bankofamerica::BankOfAmericaPaymentsResponse = res
            .response
            .parse_struct("Bankofamerica PaymentResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaServerErrorResponse = res
            .response
            .parse_struct("BankOfAmericaServerErrorResponse")
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
        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status,
            connector_transaction_id: None,
            issuer_error_code: None,
            issuer_error_message: None,
        })
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Bankofamerica {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_http_method(&self) -> Method {
        Method::Get
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
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
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaTransactionResponse = res
            .response
            .parse_struct("BankOfAmerica PaymentSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Bankofamerica {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{connector_payment_id}/captures",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
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
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsCaptureType::get_headers(self, req, connectors)?)
                .set_body(PaymentsCaptureType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaPaymentsResponse = res
            .response
            .parse_struct("BankOfAmerica PaymentResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaServerErrorResponse = res
            .response
            .parse_struct("BankOfAmericaServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
            issuer_error_code: None,
            issuer_error_message: None,
        })
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Bankofamerica {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
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
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
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
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(PaymentsVoidType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaPaymentsResponse = res
            .response
            .parse_struct("BankOfAmerica PaymentResponse")
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

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaServerErrorResponse = res
            .response
            .parse_struct("BankOfAmericaServerErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(error_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            reason: response.status.clone(),
            code: response
                .status
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
            message: response
                .message
                .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
            issuer_error_code: None,
            issuer_error_message: None,
        })
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Bankofamerica {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}pts/v2/payments/{connector_payment_id}/refunds",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
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
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(RefundExecuteType::get_headers(self, req, connectors)?)
            .set_body(RefundExecuteType::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaRefundResponse = res
            .response
            .parse_struct("bankofamerica RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Bankofamerica {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_http_method(&self) -> Method {
        Method::Get
    }

    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req.request.get_connector_refund_id()?;
        Ok(format!(
            "{}tss/v2/transactions/{refund_id}",
            self.base_url(connectors)
        ))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: bankofamerica::BankOfAmericaRsyncResponse = res
            .response
            .parse_struct("bankofamerica RefundSyncResponse")
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
impl webhooks::IncomingWebhook for Bankofamerica {
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
        Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported)
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

impl ConnectorSpecifications for Bankofamerica {}
