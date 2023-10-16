pub mod transformers;

use std::fmt::Debug;

use api_models::enums::AuthenticationType;
use common_utils::crypto;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::ExposeInterface;
use transformers as payme;

use crate::{
    configs::settings,
    connector::{utils as connector_utils, utils::PaymentsPreProcessingData},
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers,
    services::{self, request, ConnectorIntegration, ConnectorValidation},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        domain, ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Payme;

impl api::Payment for Payme {}
impl api::PaymentSession for Payme {}
impl api::PaymentsCompleteAuthorize for Payme {}
impl api::ConnectorAccessToken for Payme {}
impl api::MandateSetup for Payme {}
impl api::PaymentAuthorize for Payme {}
impl api::PaymentSync for Payme {}
impl api::PaymentCapture for Payme {}
impl api::PaymentVoid for Payme {}
impl api::Refund for Payme {}
impl api::RefundExecute for Payme {}
impl api::RefundSync for Payme {}
impl api::PaymentToken for Payme {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Payme
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let header = vec![(
            headers::CONTENT_TYPE.to_string(),
            Self::get_content_type(self).to_string().into(),
        )];
        Ok(header)
    }
}

impl ConnectorCommon for Payme {
    fn id(&self) -> &'static str {
        "payme"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.payme.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: payme::PaymeErrorResponse =
            res.response
                .parse_struct("PaymeErrorResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let status_code = match res.status_code {
            500..=511 => 200,
            _ => res.status_code,
        };
        Ok(ErrorResponse {
            status_code,
            code: response.status_error_code.to_string(),
            message: response.status_error_details.clone(),
            reason: Some(format!(
                "{}, additional info: {}",
                response.status_error_details, response.status_additional_info
            )),
        })
    }
}

impl ConnectorValidation for Payme {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_supported_error_report(capture_method, self.id()),
            ),
        }
    }
}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Payme
{
    fn get_headers(
        &self,
        req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}api/capture-buyer-token",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::TokenizationRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = payme::CaptureBuyerRequest::try_from(req)?;

        let payme_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payme::CaptureBuyerRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payme_req))
    }

    fn build_request(
        &self,
        req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(match req.auth_type {
            AuthenticationType::ThreeDs => Some(
                services::RequestBuilder::new()
                    .method(services::Method::Post)
                    .url(&types::TokenizationType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::TokenizationType::get_headers(self, req, connectors)?)
                    .body(types::TokenizationType::get_request_body(self, req)?)
                    .build(),
            ),
            AuthenticationType::NoThreeDs => None,
        })
    }

    fn handle_response(
        &self,
        data: &types::TokenizationRouterData,
        res: Response,
    ) -> CustomResult<types::TokenizationRouterData, errors::ConnectorError>
    where
        types::PaymentsResponseData: Clone,
    {
        let response: payme::CaptureBuyerResponse = res
            .response
            .parse_struct("Payme CaptureBuyerResponse")
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
        // we are always getting 500 in error scenarios
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Payme
{
}

impl api::PaymentsPreProcessing for Payme {}

impl
    ConnectorIntegration<
        api::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
    > for Payme
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
        _req: &types::PaymentsPreProcessingRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}api/generate-sale", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let amount = req.request.get_amount()?;
        let currency = req.request.get_currency()?;
        let connector_router_data =
            payme::PaymeRouterData::try_from((&self.get_currency_unit(), currency, amount, req))?;
        let req_obj = payme::GenerateSaleRequest::try_from(&connector_router_data)?;
        let payme_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payme::GenerateSaleRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payme_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let req = Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .attach_default_headers()
                .headers(types::PaymentsPreProcessingType::get_headers(
                    self, req, connectors,
                )?)
                .url(&types::PaymentsPreProcessingType::get_url(
                    self, req, connectors,
                )?)
                .body(types::PaymentsPreProcessingType::get_request_body(
                    self, req,
                )?)
                .build(),
        );
        Ok(req)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsPreProcessingRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsPreProcessingRouterData, errors::ConnectorError> {
        let response: payme::GenerateSaleResponse = res
            .response
            .parse_struct("Payme GenerateSaleResponse")
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
        // we are always getting 500 in error scenarios
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Payme
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Payme
{
}

impl services::ConnectorRedirectResponse for Payme {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        action: services::PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        match action {
            services::PaymentAction::PSync | services::PaymentAction::CompleteAuthorize => {
                Ok(payments::CallConnectorAction::Trigger)
            }
        }
    }
}

impl
    ConnectorIntegration<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for Payme
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
        Ok(format!("{}api/pay-sale", self.base_url(connectors)))
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = payme::Pay3dsRequest::try_from(req)?;
        let payme_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payme::PayRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payme_req))
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
                .body(types::PaymentsCompleteAuthorizeType::get_request_body(
                    self, req,
                )?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &types::PaymentsCompleteAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCompleteAuthorizeRouterData, errors::ConnectorError> {
        let response: payme::PaymePaySaleResponse = res
            .response
            .parse_struct("Payme PaymePaySaleResponse")
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
        self.build_error_response(res)
    }
}

impl
    ConnectorIntegration<
        api::InitPayment,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Payme
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Payme
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
        if req.request.mandate_id.is_some() {
            // For recurring mandate payments
            Ok(format!("{}api/generate-sale", self.base_url(connectors)))
        } else {
            // For Normal & first mandate payments
            Ok(format!("{}api/pay-sale", self.base_url(connectors)))
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = payme::PaymeRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let req_obj = payme::PaymePaymentRequest::try_from(&connector_router_data)?;
        let payme_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payme::PayRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payme_req))
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
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: payme::PaymePaySaleResponse = res
            .response
            .parse_struct("Payme PaymentsAuthorizeResponse")
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
        // we are always getting 500 in error scenarios
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Payme
{
    fn get_url(
        &self,
        _req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}api/get-sales", self.base_url(connectors)))
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_headers(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = payme::PaymeQuerySaleRequest::try_from(req)?;
        let payme_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payme::PayRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payme_req))
    }

    fn build_request(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .body(types::PaymentsSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        res: Response,
    ) -> CustomResult<
        types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        errors::ConnectorError,
    >
    where
        api::PSync: Clone,
        types::PaymentsSyncData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: payme::PaymePaymentsResponse = res
            .response
            .parse_struct("PaymePaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        // we are always getting 500 in error scenarios
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Payme
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
        _req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}api/capture-sale", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = payme::PaymeRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount_to_capture,
            req,
        ))?;
        let req_obj = payme::PaymentCaptureRequest::try_from(&connector_router_data)?;
        let payme_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payme::PaymentCaptureRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payme_req))
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
                .body(types::PaymentsCaptureType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: payme::PaymePaySaleResponse = res
            .response
            .parse_struct("Payme PaymentsCaptureResponse")
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
        // we are always getting 500 in error scenarios
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Payme
{
    fn build_request(
        &self,
        _req: &types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Void".to_string(),
            connector: "Payme".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Payme {
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
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}api/refund-sale", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = payme::PaymeRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let req_obj = payme::PaymeRefundRequest::try_from(&connector_router_data)?;
        let payme_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payme::PaymeRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payme_req))
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
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: payme::PaymeRefundResponse = res
            .response
            .parse_struct("PaymeRefundResponse")
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
        // we are always getting 500 in error scenarios
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Payme {
    fn get_url(
        &self,
        _req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}api/get-transactions", self.base_url(connectors)))
    }

    fn get_headers(
        &self,
        req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = payme::PaymeQueryTransactionRequest::try_from(req)?;
        let payme_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<payme::PayRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(payme_req))
    }

    fn build_request(
        &self,
        req: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundSyncType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
            .body(types::RefundSyncType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        res: Response,
    ) -> CustomResult<
        types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        errors::ConnectorError,
    >
    where
        api::RSync: Clone,
        types::RefundsData: Clone,
        types::RefundsResponseData: Clone,
    {
        let response: payme::PaymeQueryTransactionResponse = res
            .response
            .parse_struct("GetSalesResponse")
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
        // we are always getting 500 in error scenarios
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Payme {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::Md5))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let resource =
            serde_urlencoded::from_bytes::<payme::WebhookEventDataResourceSignature>(request.body)
                .into_report()
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(resource.payme_signature.expose().into_bytes())
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let resource =
            serde_urlencoded::from_bytes::<payme::WebhookEventDataResource>(request.body)
                .into_report()
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(format!(
            "{}{}{}",
            String::from_utf8_lossy(&connector_webhook_secrets.secret),
            resource.payme_transaction_id,
            resource.payme_sale_id
        )
        .as_bytes()
        .to_vec())
    }

    async fn verify_webhook_source(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        merchant_account: &domain::MerchantAccount,
        merchant_connector_account: domain::MerchantConnectorAccount,
        connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let algorithm = self
            .get_webhook_source_verification_algorithm(request)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_account,
                connector_label,
                merchant_connector_account,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let mut message = self
            .get_webhook_source_verification_message(
                request,
                &merchant_account.merchant_id,
                &connector_webhook_secrets,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let mut message_to_verify = connector_webhook_secrets
            .additional_secret
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)
            .into_report()
            .attach_printable("Failed to get additional secrets")?
            .expose()
            .as_bytes()
            .to_vec();

        message_to_verify.append(&mut message);

        let signature_to_verify = hex::decode(signature)
            .into_report()
            .change_context(errors::ConnectorError::WebhookResponseEncodingFailed)?;
        algorithm
            .verify_signature(
                &connector_webhook_secrets.secret,
                &signature_to_verify,
                &message_to_verify,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let resource =
            serde_urlencoded::from_bytes::<payme::WebhookEventDataResource>(request.body)
                .into_report()
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let id = match resource.notify_type {
            transformers::NotifyType::SaleComplete
            | transformers::NotifyType::SaleAuthorized
            | transformers::NotifyType::SaleFailure
            | transformers::NotifyType::SaleChargeback
            | transformers::NotifyType::SaleChargebackRefund => {
                api::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(
                        resource.payme_sale_id,
                    ),
                )
            }
            transformers::NotifyType::Refund => api::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::ConnectorRefundId(
                    resource.payme_transaction_id,
                ),
            ),
        };
        Ok(id)
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let resource =
            serde_urlencoded::from_bytes::<payme::WebhookEventDataResourceEvent>(request.body)
                .into_report()
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(api::IncomingWebhookEvent::from(resource.notify_type))
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let resource =
            serde_urlencoded::from_bytes::<payme::WebhookEventDataResource>(request.body)
                .into_report()
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let res_json = match resource.notify_type {
            transformers::NotifyType::SaleComplete
            | transformers::NotifyType::SaleAuthorized
            | transformers::NotifyType::SaleFailure => {
                serde_json::to_value(payme::PaymePaySaleResponse::from(resource))
                    .into_report()
                    .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
            }
            transformers::NotifyType::Refund => {
                serde_json::to_value(payme::PaymeQueryTransactionResponse::from(resource))
                    .into_report()
                    .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
            }
            transformers::NotifyType::SaleChargeback
            | transformers::NotifyType::SaleChargebackRefund => serde_json::to_value(resource)
                .into_report()
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed),
        }?;

        Ok(res_json)
    }

    fn get_dispute_details(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::disputes::DisputePayload, errors::ConnectorError> {
        let webhook_object =
            serde_urlencoded::from_bytes::<payme::WebhookEventDataResource>(request.body)
                .into_report()
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(api::disputes::DisputePayload {
            amount: webhook_object.price.to_string(),
            currency: webhook_object.currency.to_string(),
            dispute_stage: api_models::enums::DisputeStage::Dispute,
            connector_dispute_id: webhook_object.payme_transaction_id,
            connector_reason: None,
            connector_reason_code: None,
            challenge_required_by: None,
            connector_status: webhook_object.sale_status.to_string(),
            created_at: None,
            updated_at: None,
        })
    }
}
