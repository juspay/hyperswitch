pub mod transformers;

use std::sync::LazyLock;

use common_enums::{self, enums, CaptureMethod};
use common_utils::{
    errors::{CryptoError, CustomResult},
    ext_traits::{ByteSliceExt, BytesExt},
    id_type,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{
        AmountConvertor, MinorUnit, MinorUnitForConnector, StringMajorUnit,
        StringMajorUnitForConnector,
    },
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
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
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
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
use hyperswitch_masking::{ExposeInterface, Mask, PeekInterface, Secret};
use ring::aead::{self, UnboundKey};
use transformers as peachpayments;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self, RefundsRequestData},
};

const REFUND: &str = "Refund";
const APM_WEBHOOK_IV_HEADER: &str = "X-Initialization-Vector";
const APM_WEBHOOK_AUTH_TAG_HEADER: &str = "X-Authentication-Tag";

#[derive(Clone)]
pub struct Peachpayments {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
    apm_amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Peachpayments {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
            apm_amount_converter: &StringMajorUnitForConnector,
        }
    }

    /// APMs are processed by the Payments API (secondary base url), while
    /// cards and network tokens are processed by the card gateway (base url)
    fn apm_base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.peachpayments.secondary_base_url.as_ref()
    }

    /// Builds the Payments API status url; the Payments API only accepts
    /// authentication as query parameters on GET requests
    fn build_apm_status_url(
        &self,
        connectors: &Connectors,
        transaction_id: &str,
        auth_type: &ConnectorAuthType,
        connector_meta_data: &Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<String, errors::ConnectorError> {
        let authentication = peachpayments::ApmAuthentication::try_from_connector_data(
            auth_type,
            connector_meta_data,
        )?;
        let mut url = url::Url::parse(&format!(
            "{}/payments/{}",
            self.apm_base_url(connectors),
            transaction_id
        ))
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        url.query_pairs_mut()
            .append_pair("authentication.entityId", authentication.entity_id.peek())
            .append_pair("authentication.userId", authentication.user_id.peek())
            .append_pair("authentication.password", authentication.password.peek());
        Ok(url.to_string())
    }
}

/// Cards and network tokens go to the bankint card gateway; everything else
/// goes to the Payments API
fn is_apm(payment_method: enums::PaymentMethod) -> bool {
    !matches!(
        payment_method,
        enums::PaymentMethod::Card | enums::PaymentMethod::NetworkToken
    )
}

fn is_apm_webhook(request: &webhooks::IncomingWebhookRequestDetails<'_>) -> bool {
    request.headers.get(APM_WEBHOOK_IV_HEADER).is_some()
}

impl api::Payment for Peachpayments {}
impl api::PaymentSession for Peachpayments {}
impl api::ConnectorAccessToken for Peachpayments {}
impl api::MandateSetup for Peachpayments {}
impl api::PaymentAuthorize for Peachpayments {}
impl api::PaymentSync for Peachpayments {}
impl api::PaymentCapture for Peachpayments {}
impl api::PaymentVoid for Peachpayments {}
impl api::Refund for Peachpayments {}
impl api::RefundExecute for Peachpayments {}
impl api::RefundSync for Peachpayments {}
impl api::PaymentToken for Peachpayments {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Peachpayments
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Peachpayments
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Peachpayments {
    fn id(&self) -> &'static str {
        "peachpayments"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        // PeachPayments Card Gateway accepts amounts in cents (minor unit)
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.peachpayments.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let auth = peachpayments::PeachpaymentsAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![
            ("x-api-key".to_string(), auth.api_key.expose().into_masked()),
            (
                "x-tenant-id".to_string(),
                auth.tenant_id.expose().into_masked(),
            ),
            ("x-exi-auth-ver".to_string(), "v1".to_string().into_masked()),
        ])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        // The Payments API rate-limits status queries (2 per minute per transaction);
        // treat 429 as still pending so the attempt is not failed prematurely
        if res.status_code == 429 {
            return Ok(ErrorResponse {
                status_code: res.status_code,
                code: "RATE_LIMITED".to_string(),
                message: "Too many requests, the payment is still being processed".to_string(),
                reason: Some(
                    "Rate limited by Peach Payments, status will be updated via webhook or the next sync"
                        .to_string(),
                ),
                attempt_status: Some(enums::AttemptStatus::Pending),
                connector_transaction_id: None,
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            });
        }

        // Card gateway error shape
        if let Ok(response) = res
            .response
            .parse_struct::<peachpayments::PeachpaymentsErrorResponse>("PeachpaymentsErrorResponse")
        {
            event_builder.map(|i| i.set_response_body(&response));
            router_env::logger::info!(connector_response=?response);

            return Ok(ErrorResponse {
                status_code: res.status_code,
                code: response.error_ref.clone(),
                message: response.message.clone(),
                reason: Some(response.message.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            });
        }

        // Payments API (APM) error shape
        if let Ok(response) = res
            .response
            .parse_struct::<peachpayments::PeachpaymentsApmErrorResponse>(
                "PeachpaymentsApmErrorResponse",
            )
        {
            event_builder.map(|i| i.set_response_body(&response));
            router_env::logger::info!(connector_response=?response);

            return Ok(ErrorResponse {
                status_code: res.status_code,
                code: response.result.code.clone(),
                message: response
                    .result
                    .description
                    .clone()
                    .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                reason: peachpayments::build_apm_error_reason(&response.result),
                attempt_status: Some(peachpayments::map_apm_result_code_to_attempt_status(
                    &response.result.code,
                )),
                connector_transaction_id: response.id.clone(),
                connector_response_reference_id: response.merchant_transaction_id.clone(),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            });
        }

        // Fallback for unrecognised bodies; never infer a status from raw text
        let raw_body = String::from_utf8_lossy(&res.response).to_string();
        router_env::logger::info!(connector_response=?raw_body);
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: NO_ERROR_CODE.to_string(),
            message: NO_ERROR_MESSAGE.to_string(),
            reason: Some(raw_body),
            attempt_status: None,
            connector_transaction_id: None,
            connector_response_reference_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Peachpayments {
    fn validate_psync_reference_id(
        &self,
        _data: &PaymentsSyncData,
        _is_three_ds: bool,
        _status: enums::AttemptStatus,
        _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), errors::ConnectorError> {
        Ok(())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Peachpayments {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Peachpayments {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Peachpayments
{
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented(
            "Setup Mandate flow for Peachpayments".to_string(),
        )
        .into())
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Peachpayments
{
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        if is_apm(req.payment_method) {
            // The Payments API authenticates via the `authentication` object in the body
            return Ok(vec![(
                headers::CONTENT_TYPE.to_string(),
                self.common_get_content_type().to_string().into(),
            )]);
        }
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        if is_apm(req.payment_method) {
            return match req.request.capture_method.unwrap_or_default() {
                CaptureMethod::Automatic => {
                    Ok(format!("{}/payments", self.apm_base_url(connectors)))
                }
                CaptureMethod::Manual
                | CaptureMethod::ManualMultiple
                | CaptureMethod::Scheduled
                | CaptureMethod::SequentialAutomatic => {
                    Err(errors::ConnectorError::CaptureMethodNotSupported.into())
                }
            };
        }
        match req.request.capture_method.unwrap_or_default() {
            CaptureMethod::Automatic => Ok(format!(
                "{}/transactions/create-and-confirm",
                self.base_url(connectors)
            )),
            CaptureMethod::Manual => Ok(format!(
                "{}/transactions/authorization",
                self.base_url(connectors)
            )),
            CaptureMethod::ManualMultiple
            | CaptureMethod::Scheduled
            | CaptureMethod::SequentialAutomatic => {
                Err(errors::ConnectorError::CaptureMethodNotSupported.into())
            }
        }
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        if is_apm(req.payment_method) {
            let amount = utils::convert_amount(
                self.apm_amount_converter,
                req.request.minor_amount,
                req.request.currency,
            )?;

            let connector_router_data =
                peachpayments::PeachpaymentsApmRouterData::from((amount, req));
            let connector_req =
                peachpayments::PeachpaymentsApmPaymentsRequest::try_from(&connector_router_data)?;
            return Ok(RequestContent::Json(Box::new(connector_req)));
        }
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = peachpayments::PeachpaymentsRouterData::from((amount, req));
        let connector_req =
            peachpayments::PeachpaymentsPaymentsRequest::try_from(&connector_router_data)?;
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
        if is_apm(data.payment_method) {
            let response: peachpayments::PeachpaymentsApmPaymentsResponse = res
                .response
                .parse_struct("Peachpayments ApmPaymentsAuthorizeResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            event_builder.map(|i| i.set_response_body(&response));
            router_env::logger::info!(connector_response=?response);
            return RouterData::try_from(ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            });
        }
        let response: peachpayments::PeachpaymentsPaymentsResponse = res
            .response
            .parse_struct("Peachpayments PaymentsAuthorizeResponse")
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
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Peachpayments {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        if is_apm(req.payment_method) {
            return Ok(vec![(
                headers::CONTENT_TYPE.to_string(),
                self.common_get_content_type().to_string().into(),
            )]);
        }
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        if is_apm(req.payment_method) {
            let transaction_id = req
                .request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
            return self.build_apm_status_url(
                connectors,
                &transaction_id,
                &req.connector_auth_type,
                &req.connector_meta_data,
            );
        }
        let reference_id = req.connector_request_reference_id.clone();
        Ok(format!(
            "{}/transactions/by-reference/{}",
            self.base_url(connectors),
            reference_id
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
        if is_apm(data.payment_method) {
            let response: peachpayments::PeachpaymentsApmPaymentsResponse = res
                .response
                .parse_struct("Peachpayments ApmPaymentsSyncResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            event_builder.map(|i| i.set_response_body(&response));
            router_env::logger::info!(connector_response=?response);
            return RouterData::try_from(ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            });
        }
        let response: peachpayments::PeachpaymentsPaymentsResponse = res
            .response
            .parse_struct("peachpayments PaymentsSyncResponse")
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
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Peachpayments {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
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
        let connector_transaction_id = &req.request.connector_transaction_id;
        Ok(format!(
            "{}/transactions/authorization/{}/capture",
            self.base_url(connectors),
            connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount_to_capture,
            req.request.currency,
        )?;

        let connector_router_data = peachpayments::PeachpaymentsRouterData::from((amount, req));
        let connector_req =
            peachpayments::PeachpaymentsCaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        if is_apm(req.payment_method) {
            return Err(errors::ConnectorError::FlowNotSupported {
                flow: "Capture".to_string(),
                connector: "Peachpayments".to_string(),
            }
            .into());
        }
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
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
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: peachpayments::PeachpaymentsCaptureResponse = res
            .response
            .parse_struct("Peachpayments PaymentsCaptureResponse")
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
        let response: peachpayments::Peachpayments5xxErrorResponse = res
            .response
            .parse_struct("Peachpayments5xxErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        match response {
            peachpayments::Peachpayments5xxErrorResponse::Standard(error_response) => {
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: error_response.error_ref.clone(),
                    message: error_response.message.clone(),
                    reason: Some(error_response.message.clone()),
                    attempt_status: Some(enums::AttemptStatus::Authorized),
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            peachpayments::Peachpayments5xxErrorResponse::Detailed(decline_response) => {
                let (code, message, reason) = match &decline_response.response.response_code {
                    Some(peachpayments::ResponseCode::Text(text)) => {
                        (text.clone(), text.clone(), Some(text.clone()))
                    }
                    Some(peachpayments::ResponseCode::Structured {
                        value,
                        description,
                        explanation,
                        ..
                    }) => (value.clone(), description.clone(), explanation.clone()),
                    None => (
                        NO_ERROR_CODE.to_string(),
                        NO_ERROR_MESSAGE.to_string(),
                        None,
                    ),
                };
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code,
                    message,
                    reason,
                    attempt_status: Some(enums::AttemptStatus::Authorized),
                    connector_transaction_id: Some(
                        decline_response.response.transaction_id.clone(),
                    ),
                    connector_response_reference_id: Some(
                        decline_response.response.reference_id.clone(),
                    ),
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
        }
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Peachpayments {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_transaction_id = &req.request.connector_transaction_id;
        Ok(format!(
            "{}/transactions/authorization/{}/reverse",
            self.base_url(connectors),
            connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request
                .minor_amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "Amount",
                })?,
            req.request
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "Currency",
                })?,
        )?;

        let connector_router_data = peachpayments::PeachpaymentsRouterData::from((amount, req));

        let connector_req =
            peachpayments::PeachpaymentsVoidRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        if is_apm(req.payment_method) {
            return Err(errors::ConnectorError::FlowNotSupported {
                flow: "Void".to_string(),
                connector: "Peachpayments".to_string(),
            }
            .into());
        }
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
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
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: peachpayments::PeachpaymentsPaymentsResponse = res
            .response
            .parse_struct("Peachpayments PaymentsVoidResponse")
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
        let response: peachpayments::PeachpaymentsErrorResponse = res
            .response
            .parse_struct("PeachpaymentsErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_ref.clone(),
            message: response.message.clone(),
            reason: Some(response.message.clone()),
            attempt_status: Some(enums::AttemptStatus::Authorized),
            connector_transaction_id: None,
            connector_response_reference_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Peachpayments {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        if is_apm(req.payment_method) {
            return Ok(vec![(
                headers::CONTENT_TYPE.to_string(),
                self.common_get_content_type().to_string().into(),
            )]);
        }
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
        if is_apm(req.payment_method) {
            return Ok(format!(
                "{}/payments/{}",
                self.apm_base_url(connectors),
                req.request.connector_transaction_id
            ));
        }
        Ok(format!(
            "{}/transactions/{}/refund",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        if is_apm(req.payment_method) {
            let amount = utils::convert_amount(
                self.apm_amount_converter,
                req.request.minor_refund_amount,
                req.request.currency,
            )?;
            let connector_router_data =
                peachpayments::PeachpaymentsApmRouterData::from((amount, req));
            let connector_req =
                peachpayments::PeachpaymentsApmRefundRequest::try_from(&connector_router_data)?;
            return Ok(RequestContent::Json(Box::new(connector_req)));
        }
        let connector_req = peachpayments::PeachpaymentsRefundRequest::try_from(req)?;
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
        if is_apm(data.payment_method) {
            let response: peachpayments::PeachpaymentsApmPaymentsResponse = res
                .response
                .parse_struct("Peachpayments ApmRefundResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            event_builder.map(|i| i.set_response_body(&response));
            router_env::logger::info!(connector_response=?response);
            return RouterData::try_from(ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            });
        }
        let response: peachpayments::PeachpaymentsRefundResponse = res
            .response
            .parse_struct("PeachpaymentsRefundResponse")
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
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Peachpayments {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        if is_apm(req.payment_method) {
            return Ok(vec![(
                headers::CONTENT_TYPE.to_string(),
                self.common_get_content_type().to_string().into(),
            )]);
        }
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        if is_apm(req.payment_method) {
            let connector_refund_id = req.request.get_connector_refund_id()?;
            return self.build_apm_status_url(
                connectors,
                &connector_refund_id,
                &req.connector_auth_type,
                &req.connector_meta_data,
            );
        }
        let refund_id = req.request.refund_id.clone();
        Ok(format!(
            "{}/transactions/by-reference/{}",
            self.base_url(connectors),
            refund_id
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
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        if is_apm(data.payment_method) {
            let response: peachpayments::PeachpaymentsApmPaymentsResponse = res
                .response
                .parse_struct("Peachpayments ApmRsyncResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            event_builder.map(|i| i.set_response_body(&response));
            router_env::logger::info!(connector_response=?response);
            return RouterData::try_from(ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            });
        }
        let response: peachpayments::PeachpaymentsRsyncResponse = res
            .response
            .parse_struct("PeachpaymentsRsyncResponse")
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
        self.build_error_response(res, event_builder)
    }
}

/// Decrypts an AES-GCM encrypted Payments API webhook where the IV, auth
/// tag, and ciphertext are provided separately as hex strings (key from the
/// dashboard, IV and auth tag from the webhook request headers).
///
/// The Payments API documents AES-128-GCM (16 byte key); the cipher is picked
/// from the configured key length so a 32 byte key also keeps working.
fn decrypt_apm_webhook_payload(
    hex_key: &str,
    hex_iv: &str,
    hex_auth_tag: &str,
    hex_encrypted_body: &str,
) -> CustomResult<Vec<u8>, CryptoError> {
    let key_bytes = hex::decode(hex_key)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decode hex key")?;
    let iv_bytes = hex::decode(hex_iv)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decode hex IV")?;
    let auth_tag_bytes = hex::decode(hex_auth_tag)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decode hex auth tag")?;
    let encrypted_body_bytes = hex::decode(hex_encrypted_body)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decode hex encrypted body")?;
    let algorithm = match key_bytes.len() {
        16 => &aead::AES_128_GCM,
        32 => &aead::AES_256_GCM,
        _ => {
            return Err(CryptoError::InvalidKeyLength)
                .attach_printable("Key must be 16 bytes (AES-128-GCM) or 32 bytes (AES-256-GCM)");
        }
    };
    if iv_bytes.len() != aead::NONCE_LEN {
        return Err(CryptoError::InvalidIvLength)
            .attach_printable(format!("IV must be {} bytes for AES-GCM", aead::NONCE_LEN));
    }
    if auth_tag_bytes.len() != 16 {
        return Err(CryptoError::InvalidTagLength)
            .attach_printable("Auth tag must be 16 bytes for AES-GCM");
    }

    let unbound_key = UnboundKey::new(algorithm, &key_bytes)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to create unbound key")?;

    let less_safe_key = aead::LessSafeKey::new(unbound_key);

    let nonce_arr: [u8; aead::NONCE_LEN] = iv_bytes
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::InvalidIvLength)?;
    let nonce = aead::Nonce::assume_unique_for_key(nonce_arr);

    let mut ciphertext_and_tag = encrypted_body_bytes;
    ciphertext_and_tag.extend_from_slice(&auth_tag_bytes);

    less_safe_key
        .open_in_place(nonce, aead::Aad::empty(), &mut ciphertext_and_tag)
        .change_context(CryptoError::DecodingFailed)
        .attach_printable("Failed to decrypt webhook payload")?;

    let original_ciphertext_len = ciphertext_and_tag.len() - auth_tag_bytes.len();
    ciphertext_and_tag.truncate(original_ciphertext_len);

    Ok(ciphertext_and_tag)
}

impl Peachpayments {
    fn decrypt_apm_webhook(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let key_hex = String::from_utf8(connector_webhook_secrets.secret.to_vec())
            .map_err(|_| errors::ConnectorError::WebhookVerificationSecretInvalid)
            .attach_printable("Peachpayments webhook secret is not a valid UTF-8 string")?;

        let iv_hex = request
            .headers
            .get(APM_WEBHOOK_IV_HEADER)
            .ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)
            .attach_printable("Missing X-Initialization-Vector header")?
            .to_str()
            .map_err(|_| errors::ConnectorError::WebhookBodyDecodingFailed)
            .attach_printable("Invalid X-Initialization-Vector header value")?;

        let auth_tag_hex = request
            .headers
            .get(APM_WEBHOOK_AUTH_TAG_HEADER)
            .ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)
            .attach_printable("Missing X-Authentication-Tag header")?
            .to_str()
            .map_err(|_| errors::ConnectorError::WebhookBodyDecodingFailed)
            .attach_printable("Invalid X-Authentication-Tag header value")?;

        let body = String::from_utf8(request.body.to_vec())
            .map_err(|_| errors::ConnectorError::WebhookBodyDecodingFailed)
            .attach_printable("Failed to read encrypted webhook body as UTF-8")?;
        // The encrypted payload is either raw hex or wrapped as {"encryptedBody": "<hex>"}
        let encrypted_body_hex =
            serde_json::from_str::<peachpayments::ApmEncryptedWebhookBody>(&body)
                .map(|wrapped_body| wrapped_body.encrypted_body)
                .unwrap_or_else(|_| body.trim().to_string());

        decrypt_apm_webhook_payload(&key_hex, iv_hex, auth_tag_hex, &encrypted_body_hex)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
            .attach_printable("Failed to decrypt Peachpayments APM webhook payload")
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Peachpayments {
    async fn decode_webhook_body(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        merchant_id: &id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        connector_name: &str,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        if is_apm_webhook(request) {
            let connector_webhook_secrets = self
                .get_webhook_source_verification_merchant_secret(
                    merchant_id,
                    connector_name,
                    connector_webhook_details,
                )
                .await
                .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
            self.decrypt_apm_webhook(request, &connector_webhook_secrets)
        } else {
            Ok(request.body.to_vec())
        }
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        if is_apm_webhook(request) {
            let webhook_body: peachpayments::PeachpaymentsApmWebhook = request
                .body
                .parse_struct("PeachpaymentsApmWebhook")
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

            return if webhook_body.payment_type == Some(peachpayments::PeachApmPaymentType::RF) {
                Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                    api_models::webhooks::RefundIdType::ConnectorRefundId(webhook_body.id),
                ))
            } else {
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(webhook_body.id),
                ))
            };
        }
        let webhook_body: peachpayments::PeachpaymentsIncomingWebhook = request
            .body
            .parse_struct("PeachpaymentsIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let description = webhook_body
            .transaction
            .as_ref()
            .map(|txn| txn.transaction_type.description.clone());

        let reference_id = webhook_body
            .transaction
            .as_ref()
            .map(|txn| txn.reference_id.clone())
            .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        if description == Some(REFUND.to_string()) {
            Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::RefundId(reference_id),
            ))
        } else {
            Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::PaymentAttemptId(reference_id),
            ))
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        if is_apm_webhook(request) {
            let webhook_body: peachpayments::PeachpaymentsApmWebhook = request
                .body
                .parse_struct("PeachpaymentsApmWebhook")
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

            let is_refund =
                webhook_body.payment_type == Some(peachpayments::PeachApmPaymentType::RF);
            let code = webhook_body.result.code.as_str();

            return if code.starts_with("000.000.") || code.starts_with("000.100.1") {
                if is_refund {
                    Ok(api_models::webhooks::IncomingWebhookEvent::RefundSuccess)
                } else {
                    Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess)
                }
            } else if code.starts_with("000.200") {
                if is_refund {
                    Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported)
                } else {
                    Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing)
                }
            } else if code.starts_with("100.")
                || code.starts_with("200.")
                || code.starts_with("800.")
                || code.starts_with("900.")
            {
                if is_refund {
                    Ok(api_models::webhooks::IncomingWebhookEvent::RefundFailure)
                } else {
                    Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure)
                }
            } else {
                Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported)
            };
        }
        let webhook_body: peachpayments::PeachpaymentsIncomingWebhook = request
            .body
            .parse_struct("PeachpaymentsIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let description = webhook_body
            .transaction
            .as_ref()
            .map(|txn| txn.transaction_type.description.clone());

        match webhook_body.webhook_type.as_str() {
            "transaction" => {
                if let Some(transaction) = webhook_body.transaction {
                    match transaction.transaction_result {
                        peachpayments::PeachpaymentsPaymentStatus::Successful => {
                            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess)
                        }
                        peachpayments::PeachpaymentsPaymentStatus::ApprovedConfirmed => {
                            if description == Some(REFUND.to_string()) {
                                Ok(api_models::webhooks::IncomingWebhookEvent::RefundSuccess)
                            } else {
                                Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess)
                            }
                        }
                        // `FailedRetry` is returned by PeachPayments for 5xx capture failures.
                        // In this state, the transaction remains retryable and Capture/Void
                        // operations can be re-attempted.
                        peachpayments::PeachpaymentsPaymentStatus::Authorized
                        | peachpayments::PeachpaymentsPaymentStatus::Approved
                        | peachpayments::PeachpaymentsPaymentStatus::FailedRetry => {
                            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentAuthorizationSuccess)
                        }
                        peachpayments::PeachpaymentsPaymentStatus::Pending => {
                            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing)
                        }
                        peachpayments::PeachpaymentsPaymentStatus::Declined
                        | peachpayments::PeachpaymentsPaymentStatus::Failed => {
                            if description == Some(REFUND.to_string()) {
                                Ok(api_models::webhooks::IncomingWebhookEvent::RefundFailure)
                            } else {
                                Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure)
                            }
                        }
                        peachpayments::PeachpaymentsPaymentStatus::Voided
                        | peachpayments::PeachpaymentsPaymentStatus::Reversed => {
                            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelled)
                        }
                        peachpayments::PeachpaymentsPaymentStatus::ThreedsRequired => {
                            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentActionRequired)
                        }
                    }
                } else {
                    Err(errors::ConnectorError::WebhookEventTypeNotFound)
                }
            }
            _ => Err(errors::ConnectorError::WebhookEventTypeNotFound),
        }
        .change_context(errors::ConnectorError::WebhookEventTypeNotFound)
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        if is_apm_webhook(request) {
            let webhook_body: peachpayments::PeachpaymentsApmWebhook = request
                .body
                .parse_struct("PeachpaymentsApmWebhook")
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

            return Ok(Box::new(webhook_body));
        }
        let webhook_body: peachpayments::PeachpaymentsIncomingWebhook = request
            .body
            .parse_struct("PeachpaymentsIncomingWebhook")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(Box::new(webhook_body))
    }

    async fn verify_webhook_source(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        merchant_id: &id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: common_utils::crypto::Encryptable<Secret<serde_json::Value>>,
        connector_name: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        if is_apm_webhook(request) {
            // A successful AES-GCM decryption authenticates the webhook source,
            // since the tag verification fails for any other key
            let connector_webhook_secrets = self
                .get_webhook_source_verification_merchant_secret(
                    merchant_id,
                    connector_name,
                    connector_webhook_details,
                )
                .await
                .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
            if self
                .decrypt_apm_webhook(request, &connector_webhook_secrets)
                .is_ok()
            {
                return Ok(true);
            }
            // The body has usually already been decrypted by decode_webhook_body
            // by the time source verification runs; reaching this point with a
            // valid plaintext payload means the AES-GCM decryption succeeded
            return Ok(request
                .body
                .parse_struct::<peachpayments::PeachpaymentsApmWebhook>("PeachpaymentsApmWebhook")
                .is_ok());
        }
        Ok(false)
    }
}

static PEACHPAYMENTS_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![CaptureMethod::Automatic, CaptureMethod::Manual];

        let supported_card_network = vec![
            common_enums::CardNetwork::Visa,
            common_enums::CardNetwork::Mastercard,
            common_enums::CardNetwork::AmericanExpress,
        ];

        let mut peachpayments_supported_payment_methods = SupportedPaymentMethods::new();

        peachpayments_supported_payment_methods.add(
            enums::PaymentMethod::Card,
            enums::PaymentMethodType::Credit,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: Some(
                    api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                        api_models::feature_matrix::CardSpecificFeatures {
                            three_ds: common_enums::FeatureStatus::NotSupported,
                            no_three_ds: common_enums::FeatureStatus::Supported,
                            supported_card_networks: supported_card_network.clone(),
                        }
                    }),
                ),
            },
        );

        peachpayments_supported_payment_methods.add(
            enums::PaymentMethod::Card,
            enums::PaymentMethodType::Debit,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: Some(
                    api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                        api_models::feature_matrix::CardSpecificFeatures {
                            three_ds: common_enums::FeatureStatus::NotSupported,
                            no_three_ds: common_enums::FeatureStatus::Supported,
                            supported_card_networks: supported_card_network.clone(),
                        }
                    }),
                ),
            },
        );

        peachpayments_supported_payment_methods.add(
            enums::PaymentMethod::NetworkToken,
            enums::PaymentMethodType::NetworkToken,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods,
                specific_features: None,
            },
        );

        // APMs via the Payments API support automatic capture only
        let apm_details = PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: vec![CaptureMethod::Automatic],
            specific_features: None,
        };

        for payment_method_type in [
            enums::PaymentMethodType::CapitecPay,
            enums::PaymentMethodType::PayShap,
            enums::PaymentMethodType::NedbankDirectEft,
            enums::PaymentMethodType::PeachEft,
        ] {
            peachpayments_supported_payment_methods.add(
                enums::PaymentMethod::BankTransfer,
                payment_method_type,
                apm_details.clone(),
            );
        }

        for payment_method_type in [
            enums::PaymentMethodType::Payflex,
            enums::PaymentMethodType::ZeroPay,
            enums::PaymentMethodType::Float,
            enums::PaymentMethodType::HappyPay,
            enums::PaymentMethodType::Mobicred,
            enums::PaymentMethodType::Rcs,
            enums::PaymentMethodType::APlus,
        ] {
            peachpayments_supported_payment_methods.add(
                enums::PaymentMethod::PayLater,
                payment_method_type,
                apm_details.clone(),
            );
        }

        for payment_method_type in [
            enums::PaymentMethodType::Mpesa,
            enums::PaymentMethodType::BlinkByEmtel,
            enums::PaymentMethodType::McbJuice,
            enums::PaymentMethodType::ScanToPay,
            enums::PaymentMethodType::Maucas,
        ] {
            peachpayments_supported_payment_methods.add(
                enums::PaymentMethod::Wallet,
                payment_method_type,
                apm_details.clone(),
            );
        }

        peachpayments_supported_payment_methods.add(
            enums::PaymentMethod::Voucher,
            enums::PaymentMethodType::OneForYou,
            apm_details.clone(),
        );

        peachpayments_supported_payment_methods.add(
            enums::PaymentMethod::Crypto,
            enums::PaymentMethodType::MoneyBadger,
            apm_details,
        );

        peachpayments_supported_payment_methods
    });

static PEACHPAYMENTS_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Peach Payments",
    description: "The secure African payment gateway with easy integrations, 365-day support, and advanced orchestration.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Live,
};

static PEACHPAYMENTS_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 2] =
    [enums::EventClass::Payments, enums::EventClass::Refunds];

impl ConnectorSpecifications for Peachpayments {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&PEACHPAYMENTS_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*PEACHPAYMENTS_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&PEACHPAYMENTS_SUPPORTED_WEBHOOK_FLOWS)
    }

    #[cfg(feature = "v1")]
    fn generate_connector_request_reference_id(
        &self,
        _payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
        is_config_enabled_to_send_payment_id_as_connector_request_id: bool,
    ) -> String {
        match payment_attempt.payment_method {
            // The Payments API requires merchantTransactionId to be 8-16
            // alphanumeric characters; APM transactions correlate on the Peach
            // unique id, so a generated reference is sufficient
            Some(payment_method) if is_apm(payment_method) => {
                utils::generate_alphanumeric_code(16, 16)
            }
            // Card gateway: preserve the default reference behaviour
            _ => {
                if is_config_enabled_to_send_payment_id_as_connector_request_id {
                    payment_attempt.payment_id.get_string_repr().to_owned()
                } else {
                    payment_attempt.attempt_id.to_owned()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encrypt_for_test(
        algorithm: &'static aead::Algorithm,
        key_bytes: &[u8],
        iv_bytes: [u8; aead::NONCE_LEN],
        plaintext: &[u8],
    ) -> (Vec<u8>, Vec<u8>) {
        let unbound_key = UnboundKey::new(algorithm, key_bytes).expect("failed to create key");
        let sealing_key = aead::LessSafeKey::new(unbound_key);
        let nonce = aead::Nonce::assume_unique_for_key(iv_bytes);
        let mut ciphertext = plaintext.to_vec();
        let tag = sealing_key
            .seal_in_place_separate_tag(nonce, aead::Aad::empty(), &mut ciphertext)
            .expect("failed to encrypt");
        (ciphertext, tag.as_ref().to_vec())
    }

    // The Payments API documents AES-128-GCM webhook encryption
    #[test]
    fn test_decrypt_apm_webhook_payload_aes_128_round_trip() {
        let key_bytes = [0x42_u8; 16];
        let iv_bytes = [0x24_u8; aead::NONCE_LEN];
        let plaintext = br#"{"id":"8ac7a4a09c8b9c8e019c8ba47c0000aa","paymentType":"DB","result":{"code":"000.000.000","description":"Transaction succeeded"}}"#;

        let (ciphertext, tag) =
            encrypt_for_test(&aead::AES_128_GCM, &key_bytes, iv_bytes, plaintext);

        let decrypted = decrypt_apm_webhook_payload(
            &hex::encode(key_bytes),
            &hex::encode(iv_bytes),
            &hex::encode(tag),
            &hex::encode(&ciphertext),
        )
        .expect("failed to decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_apm_webhook_payload_aes_256_round_trip() {
        let key_bytes = [0x42_u8; 32];
        let iv_bytes = [0x24_u8; aead::NONCE_LEN];
        let plaintext = br#"{"id":"8ac7a4a09c8b9c8e019c8ba47c0000aa","paymentType":"DB","result":{"code":"000.000.000","description":"Transaction succeeded"}}"#;

        let (ciphertext, tag) =
            encrypt_for_test(&aead::AES_256_GCM, &key_bytes, iv_bytes, plaintext);

        let decrypted = decrypt_apm_webhook_payload(
            &hex::encode(key_bytes),
            &hex::encode(iv_bytes),
            &hex::encode(tag),
            &hex::encode(&ciphertext),
        )
        .expect("failed to decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_apm_webhook_payload_rejects_wrong_key() {
        let key_bytes = [0x42_u8; 32];
        let wrong_key_bytes = [0x43_u8; 32];
        let iv_bytes = [0x24_u8; aead::NONCE_LEN];
        let plaintext = b"payload";

        let unbound_key =
            UnboundKey::new(&aead::AES_256_GCM, &key_bytes).expect("failed to create key");
        let sealing_key = aead::LessSafeKey::new(unbound_key);
        let nonce = aead::Nonce::assume_unique_for_key(iv_bytes);
        let mut ciphertext = plaintext.to_vec();
        let tag = sealing_key
            .seal_in_place_separate_tag(nonce, aead::Aad::empty(), &mut ciphertext)
            .expect("failed to encrypt");

        assert!(decrypt_apm_webhook_payload(
            &hex::encode(wrong_key_bytes),
            &hex::encode(iv_bytes),
            &hex::encode(tag.as_ref()),
            &hex::encode(&ciphertext),
        )
        .is_err());
    }
}
