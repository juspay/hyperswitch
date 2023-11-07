pub mod transformers;

use std::fmt::Debug;

use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};

use super::utils::RefundsRequestData;
use crate::{
    configs::settings,
    connector::utils as connector_utils,
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
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct DummyConnector<const T: u8>;

impl<const T: u8> api::Payment for DummyConnector<T> {}
impl<const T: u8> api::PaymentSession for DummyConnector<T> {}
impl<const T: u8> api::ConnectorAccessToken for DummyConnector<T> {}
impl<const T: u8> api::MandateSetup for DummyConnector<T> {}
impl<const T: u8> api::PaymentAuthorize for DummyConnector<T> {}
impl<const T: u8> api::PaymentSync for DummyConnector<T> {}
impl<const T: u8> api::PaymentCapture for DummyConnector<T> {}
impl<const T: u8> api::PaymentVoid for DummyConnector<T> {}
impl<const T: u8> api::Refund for DummyConnector<T> {}
impl<const T: u8> api::RefundExecute for DummyConnector<T> {}
impl<const T: u8> api::RefundSync for DummyConnector<T> {}
impl<const T: u8> api::PaymentToken for DummyConnector<T> {}

impl<const T: u8>
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for DummyConnector<T>
{
    // Not Implemented (R)
}

impl<const T: u8, Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response>
    for DummyConnector<T>
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl<const T: u8> ConnectorCommon for DummyConnector<T> {
    fn id(&self) -> &'static str {
        Into::<transformers::DummyConnectors>::into(T).get_dummy_connector_id()
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.dummyconnector.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = transformers::DummyConnectorAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: transformers::DummyConnectorErrorResponse = res
            .response
            .parse_struct("DummyConnectorErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error.code,
            message: response.error.message,
            reason: response.error.reason,
        })
    }
}

impl<const T: u8> ConnectorValidation for DummyConnector<T> {
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

impl<const T: u8>
    ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for DummyConnector<T>
{
    //TODO: implement sessions flow
}

impl<const T: u8>
    ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for DummyConnector<T>
{
}

impl<const T: u8>
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for DummyConnector<T>
{
}

impl<const T: u8>
    ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for DummyConnector<T>
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
        match req.payment_method {
            enums::PaymentMethod::Card => Ok(format!("{}/payment", self.base_url(connectors))),
            enums::PaymentMethod::Wallet => Ok(format!("{}/payment", self.base_url(connectors))),
            enums::PaymentMethod::PayLater => Ok(format!("{}/payment", self.base_url(connectors))),
            _ => Err(error_stack::report!(errors::ConnectorError::NotSupported {
                message: format!("The payment method {} is not supported", req.payment_method),
                connector: Into::<transformers::DummyConnectors>::into(T).get_dummy_connector_id(),
            })),
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = transformers::DummyConnectorPaymentsRequest::<T>::try_from(req)?;
        let dummmy_payments_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<transformers::DummyConnectorPaymentsRequest::<T>>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(dummmy_payments_request))
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
        let response: transformers::PaymentsResponse = res
            .response
            .parse_struct("DummyConnector PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        println!("handle_response: {:#?}", response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl<const T: u8>
    ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for DummyConnector<T>
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
        match req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
        {
            Ok(transaction_id) => Ok(format!(
                "{}/payments/{}",
                self.base_url(connectors),
                transaction_id
            )),
            Err(_) => Err(error_stack::report!(
                errors::ConnectorError::MissingConnectorTransactionID
            )),
        }
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
        let response: transformers::PaymentsResponse = res
            .response
            .parse_struct("dummyconnector PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl<const T: u8>
    ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for DummyConnector<T>
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
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
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
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: transformers::PaymentsResponse = res
            .response
            .parse_struct("transformers PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl<const T: u8>
    ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for DummyConnector<T>
{
}

impl<const T: u8> ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for DummyConnector<T>
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
            "{}/{}/refund",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_request = transformers::DummyConnectorRefundRequest::try_from(req)?;
        let dummmy_refund_request = types::RequestBody::log_and_get_request_body(
            &connector_request,
            utils::Encode::<transformers::DummyConnectorRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(dummmy_refund_request))
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
        let response: transformers::RefundResponse = res
            .response
            .parse_struct("transformers RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl<const T: u8> ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for DummyConnector<T>
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
            "{}/refunds/{}",
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
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: transformers::RefundResponse = res
            .response
            .parse_struct("transformers RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl<const T: u8> api::IncomingWebhook for DummyConnector<T> {
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
        Ok(api::IncomingWebhookEvent::EventNotSupported)
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
