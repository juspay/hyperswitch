mod transformers;

use std::fmt::Debug;

use error_stack::{IntoReport, ResultExt};
use transformers as nuvei;

use crate::{
    configs::settings,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, logger,
    services::{self, ConnectorIntegration},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response, RouterData,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Nuvei;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Nuvei
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let headers = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string(),
        )];
        Ok(headers)
    }
}

impl ConnectorCommon for Nuvei {
    fn id(&self) -> &'static str {
        "nuvei"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.nuvei.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(vec![])
    }
}

impl api::Payment for Nuvei {}

impl api::PreVerify for Nuvei {}
impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Nuvei
{
}

impl api::PaymentVoid for Nuvei {}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Nuvei
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/voidTransaction.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiPaymentFlowRequest::try_from(req)?;
        let req =
            utils::Encode::<nuvei::NuveiPaymentFlowRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
            .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
            .body(types::PaymentsVoidType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        logger::debug!(target: "router::connector::nuvei", response=?res);
        let response: nuvei::NuveiPaymentsResponse = res
            .response
            .parse_struct("nuvei NuveiPaymentsResponse")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        types::PaymentsCancelRouterData::try_from(types::ResponseRouterData {
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

impl api::ConnectorAccessToken for Nuvei {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Nuvei
{
}

impl api::PaymentSync for Nuvei {}
impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Nuvei
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/getPaymentStatus.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSyncRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiPaymentSyncRequest::try_from(req)?;
        let req =
            utils::Encode::<nuvei::NuveiPaymentSyncRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
    }
    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .body(types::PaymentsSyncType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        logger::debug!(payment_sync_response=?res);
        let response: nuvei::NuveiPaymentsResponse = res
            .response
            .parse_struct("nuvei PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl api::PaymentCapture for Nuvei {}
impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Nuvei
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
        Ok(format!(
            "{}ppp/api/v1/settleTransaction.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiPaymentFlowRequest::try_from(req)?;
        let req =
            utils::Encode::<nuvei::NuveiPaymentFlowRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
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
        let response: nuvei::NuveiPaymentsResponse = res
            .response
            .parse_struct("Nuvei PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(nuveipayments_create_response=?response);
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

impl api::PaymentSession for Nuvei {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Nuvei
{
}

impl api::PaymentAuthorize for Nuvei {}

#[async_trait::async_trait]
impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Nuvei
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
        Ok(format!(
            "{}ppp/api/v1/payment.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    async fn execute_pretasks(
        &self,
        router_data: &mut types::PaymentsAuthorizeRouterData,
        app_state: &crate::routes::AppState,
    ) -> CustomResult<(), errors::ConnectorError> {
        let integ: Box<
            &(dyn ConnectorIntegration<
                api::AuthorizeSessionToken,
                types::AuthorizeSessionTokenData,
                types::PaymentsResponseData,
            > + Send
                  + Sync
                  + 'static),
        > = Box::new(&Self);
        let authorize_data = &types::PaymentsAuthorizeSessionTokenRouterData::from(&router_data);
        let resp = services::execute_connector_processing_step(
            app_state,
            integ,
            authorize_data,
            payments::CallConnectorAction::Trigger,
        )
        .await?;
        router_data.session_token = resp.session_token;
        Ok(())
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiPaymentsRequest::try_from(req)?;
        let req = utils::Encode::<nuvei::NuveiPaymentsRequest>::encode_to_string_of_json(&req_obj)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
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
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: nuvei::NuveiPaymentsResponse = res
            .response
            .parse_struct("nuvei NuveiPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(nuveipayments_create_response=?response);
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

impl
    ConnectorIntegration<
        api::AuthorizeSessionToken,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,
    > for Nuvei
{
    fn get_headers(
        &self,
        req: &RouterData<
            types::api::payments::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RouterData<
            types::api::payments::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}ppp/api/v1/getSessionToken.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &RouterData<
            types::api::payments::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiSessionRequest::try_from(req)?;
        let req = utils::Encode::<nuvei::NuveiSessionRequest>::encode_to_string_of_json(&req_obj)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
    }

    fn build_request(
        &self,
        req: &RouterData<
            types::api::payments::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsPreAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .headers(types::PaymentsPreAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsPreAuthorizeType::get_request_body(
                    self, req,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RouterData<
            api::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
        res: Response,
    ) -> CustomResult<
        RouterData<
            api::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
        errors::ConnectorError,
    >
    where
        api::AuthorizeSessionToken: Clone,
        types::AuthorizeSessionTokenData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: nuvei::NuveiSessionResponse = res
            .response
            .parse_struct("nuvei NuveiSessionResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(nuvei_session_response=?response);

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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Nuvei {}
impl api::RefundExecute for Nuvei {}
impl api::RefundSync for Nuvei {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Nuvei {
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
        Ok(format!(
            "{}ppp/api/v1/refundTransaction.do",
            api::ConnectorCommon::base_url(self, connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let req_obj = nuvei::NuveiPaymentFlowRequest::try_from(req)?;
        let req =
            utils::Encode::<nuvei::NuveiPaymentFlowRequest>::encode_to_string_of_json(&req_obj)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
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
        logger::debug!(target: "router::connector::nuvei", response=?res);
        let response: nuvei::NuveiPaymentsResponse = res
            .response
            .parse_struct("nuvei NuveiPaymentsResponse")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Nuvei {}

#[async_trait::async_trait]
impl api::IncomingWebhook for Nuvei {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<String, errors::ConnectorError> {
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
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}

impl services::ConnectorRedirectResponse for Nuvei {
    fn get_flow_type(
        &self,
        _query_params: &str,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}
