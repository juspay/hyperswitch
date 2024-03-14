pub mod transformers;

use std::fmt::Debug;

use common_utils::{ext_traits::ByteSliceExt, request::RequestContent};
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use transformers as shift4;

use super::utils::{self as connector_utils, RefundsRequestData};
use crate::{
    configs::settings,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    events::connector_api_logs::ConnectorEvent,
    headers, routes,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse,
    },
    utils::BytesExt,
};

#[derive(Debug, Clone)]
pub struct Shift4;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Shift4
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::ACCEPT.to_string(),
                self.get_content_type().to_string().into(),
            ),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }
}
impl ConnectorCommon for Shift4 {
    fn id(&self) -> &'static str {
        "shift4"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.shift4.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: shift4::Shift4AuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: shift4::ErrorResponse = res
            .response
            .parse_struct("Shift4 ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response.error.message,
            reason: None,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Shift4 {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_supported_error_report(capture_method, self.id()),
            ),
        }
    }

    fn validate_mandate_payment(
        &self,
        pm_type: Option<types::storage::enums::PaymentMethodType>,
        pm_data: api_models::payments::PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        match pm_data {
            api_models::payments::PaymentMethodData::Card(_)
            | api_models::payments::PaymentMethodData::Wallet(_)
            | api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::BankRedirect(_)
            | api_models::payments::PaymentMethodData::BankDebit(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::MandatePayment
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::GiftCard(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::CardToken(_) => Err(
                connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
            ),
        }
    }
}

impl api::Payment for Shift4 {}
impl api::PaymentVoid for Shift4 {}
impl api::PaymentSync for Shift4 {}
impl api::PaymentCapture for Shift4 {}
impl api::PaymentSession for Shift4 {}
impl api::PaymentAuthorize for Shift4 {}
impl api::PaymentsCompleteAuthorize for Shift4 {}
impl api::Refund for Shift4 {}
impl api::RefundExecute for Shift4 {}
impl api::RefundSync for Shift4 {}
impl api::PaymentToken for Shift4 {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Shift4
{
    // Not Implemented (R)
}

impl api::ConnectorAccessToken for Shift4 {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Shift4
{
    // Not Implemented (R)
}

impl api::MandateSetup for Shift4 {}
impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Shift4
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
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Shift4".to_string())
                .into(),
        )
    }
}

#[async_trait::async_trait]
impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Shift4
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
        Ok(format!("{}charges", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = shift4::Shift4PaymentsRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    async fn execute_pretasks(
        &self,
        router_data: &mut types::PaymentsAuthorizeRouterData,
        app_state: &routes::AppState,
    ) -> CustomResult<(), errors::ConnectorError> {
        if router_data.auth_type == diesel_models::enums::AuthenticationType::ThreeDs
            && router_data.payment_method == diesel_models::enums::PaymentMethod::Card
        {
            let integ: Box<
                &(dyn ConnectorIntegration<
                    api::InitPayment,
                    types::PaymentsAuthorizeData,
                    types::PaymentsResponseData,
                > + Send
                      + Sync
                      + 'static),
            > = Box::new(&Self);
            let init_data = &types::PaymentsInitRouterData::from((
                &router_data.to_owned(),
                router_data.request.clone(),
            ));
            let init_resp = services::execute_connector_processing_step(
                app_state,
                integ,
                init_data,
                payments::CallConnectorAction::Trigger,
                None,
            )
            .await?;
            if init_resp.request.enrolled_for_3ds {
                router_data.response = init_resp.response;
                router_data.status = init_resp.status;
            } else {
                router_data.request.enrolled_for_3ds = false;
            }
        }
        Ok(())
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
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: shift4::Shift4NonThreeDsResponse = res
            .response
            .parse_struct("Shift4NonThreeDsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Shift4
{
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Shift4
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
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}charges/{}",
            self.base_url(connectors),
            connector_payment_id
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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: shift4::Shift4NonThreeDsResponse = res
            .response
            .parse_struct("Shift4NonThreeDsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Shift4
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
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: shift4::Shift4NonThreeDsResponse = res
            .response
            .parse_struct("Shift4NonThreeDsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}charges/{}/capture",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Shift4
{
    //TODO: implement sessions flow
}

impl
    ConnectorIntegration<
        api::InitPayment,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Shift4
{
    fn get_headers(
        &self,
        req: &types::PaymentsInitRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                "application/x-www-form-urlencoded".to_string().into(),
            ),
            (
                headers::ACCEPT.to_string(),
                self.common_get_content_type().to_string().into(),
            ),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsInitRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}3d-secure", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsInitRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = shift4::Shift4PaymentsRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsInitRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsInitType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsInitType::get_headers(self, req, connectors)?)
                .set_body(types::PaymentsInitType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsInitRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsInitRouterData, errors::ConnectorError> {
        let response: shift4::Shift4ThreeDsResponse = res
            .response
            .parse_struct("Shift4ThreeDsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    ConnectorIntegration<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for Shift4
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
        Ok(format!("{}charges", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = shift4::Shift4PaymentsRequest::try_from(req)?;
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
        let response: shift4::Shift4NonThreeDsResponse = res
            .response
            .parse_struct("Shift4NonThreeDsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Shift4 {
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
        Ok(format!("{}refunds", self.base_url(connectors),))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = shift4::Shift4RefundRequest::try_from(req)?;
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
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: shift4::RefundResponse = res
            .response
            .parse_struct("RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Shift4 {
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
            "{}refunds/{}",
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
        let response: shift4::RefundResponse =
            res.response
                .parse_struct("shift4 RefundResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Shift4 {
    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let details: shift4::Shift4WebhookObjectId = request
            .body
            .parse_struct("Shift4WebhookObjectId")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        if shift4::is_transaction_event(&details.event_type) {
            Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(details.data.id),
            ))
        } else if shift4::is_refund_event(&details.event_type) {
            Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::ConnectorRefundId(
                    details
                        .data
                        .refunds
                        .and_then(|refund| {
                            refund.first().map(|refund_object| refund_object.id.clone())
                        })
                        .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                ),
            ))
        } else {
            Err(errors::ConnectorError::WebhookReferenceIdNotFound).into_report()
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let details: shift4::Shift4WebhookObjectEventType = request
            .body
            .parse_struct("Shift4WebhookObjectEventType")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        Ok(api::IncomingWebhookEvent::from(details.event_type))
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let details: shift4::Shift4WebhookObjectResource = request
            .body
            .parse_struct("Shift4WebhookObjectResource")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        // Ideally this should be a strict type that has type information
        // PII information is likely being logged here when this response will be logged
        Ok(Box::new(details.data))
    }
}
