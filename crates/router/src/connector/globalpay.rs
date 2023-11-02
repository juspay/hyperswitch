mod requests;
mod response;
pub mod transformers;

use std::fmt::Debug;

use ::common_utils::{errors::ReportSwitchExt, ext_traits::ByteSliceExt};
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use serde_json::Value;

use self::{
    requests::{GlobalpayPaymentsRequest, GlobalpayRefreshTokenRequest},
    response::{
        GlobalpayPaymentsResponse, GlobalpayRefreshTokenErrorResponse,
        GlobalpayRefreshTokenResponse,
    },
};
use super::utils::RefundsRequestData;
use crate::{
    configs::settings,
    connector::{utils as connector_utils, utils as conn_utils},
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt, PaymentsCompleteAuthorize},
        ErrorResponse,
    },
    utils::{self, crypto, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Globalpay;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Globalpay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let access_token = req
            .access_token
            .clone()
            .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;

        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            ("X-GP-Version".to_string(), "2021-03-22".to_string().into()),
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", access_token.token.peek()).into_masked(),
            ),
        ])
    }
}

impl ConnectorCommon for Globalpay {
    fn id(&self) -> &'static str {
        "globalpay"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.globalpay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: transformers::GlobalpayErrorResponse = res
            .response
            .parse_struct("Globalpay ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.detailed_error_description,
            reason: None,
        })
    }
}

impl ConnectorValidation for Globalpay {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic
            | enums::CaptureMethod::Manual
            | enums::CaptureMethod::ManualMultiple => Ok(()),
            enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }
}

impl PaymentsCompleteAuthorize for Globalpay {}

impl
    ConnectorIntegration<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for Globalpay
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
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}transactions/{}/confirmation",
            self.base_url(connectors),
            req.request
                .connector_transaction_id
                .clone()
                .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?
        ))
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsCompleteAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let globalpay_req = types::RequestBody::log_and_get_request_body("{}".to_string(), Ok)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(globalpay_req))
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
        res: types::Response,
    ) -> CustomResult<types::PaymentsCompleteAuthorizeRouterData, errors::ConnectorError> {
        let response: GlobalpayPaymentsResponse = res
            .response
            .parse_struct("Globalpay PaymentsResponse")
            .switch()?;

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::ConnectorAccessToken for Globalpay {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Globalpay
{
    fn get_headers(
        &self,
        _req: &types::RefreshTokenRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::RefreshTokenType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            ("X-GP-Version".to_string(), "2021-03-22".to_string().into()),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefreshTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "accesstoken"))
    }

    fn build_request(
        &self,
        req: &types::RefreshTokenRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::RefreshTokenType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefreshTokenType::get_headers(self, req, connectors)?)
                .body(types::RefreshTokenType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefreshTokenRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = GlobalpayRefreshTokenRequest::try_from(req)?;
        let globalpay_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<GlobalpayPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(globalpay_req))
    }

    fn handle_response(
        &self,
        data: &types::RefreshTokenRouterData,
        res: types::Response,
    ) -> CustomResult<types::RefreshTokenRouterData, errors::ConnectorError> {
        let response: GlobalpayRefreshTokenResponse = res
            .response
            .parse_struct("Globalpay PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: GlobalpayRefreshTokenErrorResponse = res
            .response
            .parse_struct("Globalpay ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.detailed_error_description,
            reason: None,
        })
    }
}

impl api::Payment for Globalpay {}

impl api::PaymentToken for Globalpay {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Globalpay
{
    // Not Implemented (R)
}

impl api::MandateSetup for Globalpay {}
impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Globalpay
{
}

impl api::PaymentVoid for Globalpay {}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Globalpay
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/transactions/{}/reversal",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
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
                .body(types::PaymentsVoidType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = requests::GlobalpayCancelRequest::try_from(req)?;
        let globalpay_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<GlobalpayPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(globalpay_req))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: GlobalpayPaymentsResponse = res
            .response
            .parse_struct("Globalpay PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSync for Globalpay {}
impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Globalpay
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

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}transactions/{}",
            self.base_url(connectors),
            req.request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
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

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: GlobalpayPaymentsResponse = res
            .response
            .parse_struct("globalpay PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let is_multiple_capture_sync = match data.request.sync_type {
            types::SyncRequestType::MultipleCaptureSync(_) => true,
            types::SyncRequestType::SinglePaymentSync => false,
        };
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            is_multiple_capture_sync,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
    fn get_multiple_capture_sync_method(
        &self,
    ) -> CustomResult<services::CaptureSyncMethod, errors::ConnectorError> {
        Ok(services::CaptureSyncMethod::Individual)
    }
}

impl api::PaymentCapture for Globalpay {}
impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Globalpay
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
        Ok(format!(
            "{}/transactions/{}/capture",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = requests::GlobalpayCaptureRequest::try_from(req)?;
        let globalpay_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<GlobalpayPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(globalpay_req))
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
        res: types::Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: GlobalpayPaymentsResponse = res
            .response
            .parse_struct("Globalpay PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSession for Globalpay {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Globalpay
{
}

impl api::PaymentAuthorize for Globalpay {}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Globalpay
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
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}transactions", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = GlobalpayPaymentsRequest::try_from(req)?;
        let globalpay_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<GlobalpayPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(globalpay_req))
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
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: GlobalpayPaymentsResponse = res
            .response
            .parse_struct("Globalpay PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Globalpay {}
impl api::RefundExecute for Globalpay {}
impl api::RefundSync for Globalpay {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Globalpay
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
        Ok(format!(
            "{}transactions/{}/refund",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = requests::GlobalpayRefundRequest::try_from(req)?;
        let globalpay_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<requests::GlobalpayRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(globalpay_req))
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
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: GlobalpayPaymentsResponse = res
            .response
            .parse_struct("globalpay RefundResponse")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Globalpay
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

    fn get_url(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let refund_id = req.request.get_connector_refund_id()?;
        Ok(format!(
            "{}transactions/{}",
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
                .body(types::RefundSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        res: types::Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: GlobalpayPaymentsResponse = res
            .response
            .parse_struct("globalpay RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Globalpay {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::Sha512))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let signature = conn_utils::get_header_key_value("x-gp-signature", request.headers)?;
        Ok(signature.as_bytes().to_vec())
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let payload: Value = request.body.parse_struct("GlobalpayWebhookBody").switch()?;
        let mut payload_str = serde_json::to_string(&payload)
            .into_report()
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let sec = std::str::from_utf8(&connector_webhook_secrets.secret)
            .into_report()
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        payload_str.push_str(sec);
        Ok(payload_str.into_bytes())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let details: response::GlobalpayWebhookObjectId = request
            .body
            .parse_struct("GlobalpayWebhookObjectId")
            .switch()?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(details.id),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let details: response::GlobalpayWebhookObjectEventType = request
            .body
            .parse_struct("GlobalpayWebhookObjectEventType")
            .switch()?;
        Ok(match details.status {
            response::GlobalpayWebhookStatus::Declined => {
                api::IncomingWebhookEvent::PaymentIntentFailure
            }
            response::GlobalpayWebhookStatus::Captured => {
                api::IncomingWebhookEvent::PaymentIntentSuccess
            }
            response::GlobalpayWebhookStatus::Unknown => {
                api::IncomingWebhookEvent::EventNotSupported
            }
        })
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Value, errors::ConnectorError> {
        let details = std::str::from_utf8(request.body)
            .into_report()
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let res_json = serde_json::from_str(details)
            .into_report()
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(res_json)
    }
}

impl services::ConnectorRedirectResponse for Globalpay {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<Value>,
        action: services::PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        match action {
            services::PaymentAction::PSync | services::PaymentAction::CompleteAuthorize => {
                Ok(payments::CallConnectorAction::Trigger)
            }
        }
    }
}
