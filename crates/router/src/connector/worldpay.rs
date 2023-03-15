mod requests;
mod response;
mod transformers;

use std::fmt::Debug;

use error_stack::{IntoReport, ResultExt};
use storage_models::enums;
use transformers as worldpay;

use self::{requests::*, response::*};
use super::utils::RefundsRequestData;
use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    headers,
    services::{self, ConnectorIntegration},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Worldpay;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Worldpay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut headers = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }
}

impl ConnectorCommon for Worldpay {
    fn id(&self) -> &'static str {
        "worldpay"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/vnd.worldpay.payments-v6+json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.worldpay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth: worldpay::WorldpayAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::AUTHORIZATION.to_string(), auth.api_key)])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: WorldpayErrorResponse = res
            .response
            .parse_struct("WorldpayErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_name,
            message: response.message,
            reason: response.validation_errors.map(|e| e.to_string()),
        })
    }
}

impl api::Payment for Worldpay {}

impl api::PreVerify for Worldpay {}
impl ConnectorIntegration<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for Worldpay
{
}

impl api::PaymentVoid for Worldpay {}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Worldpay
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
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}payments/settlements/{}",
            self.base_url(connectors),
            connector_payment_id
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
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError>
    where
        api::Void: Clone,
        types::PaymentsCancelData: Clone,
        types::PaymentsResponseData: Clone,
    {
        match res.status_code {
            202 => {
                let response: WorldpayPaymentsResponse = res
                    .response
                    .parse_struct("Worldpay PaymentsResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                Ok(types::PaymentsCancelRouterData {
                    status: enums::AttemptStatus::Voided,
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::try_from(response.links)?,
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                    }),
                    ..data.clone()
                })
            }
            _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        }
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::ConnectorAccessToken for Worldpay {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Worldpay
{
}

impl api::PaymentSync for Worldpay {}
impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Worldpay
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
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}payments/events/{}",
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
        let response: WorldpayEventResponse =
            res.response
                .parse_struct("Worldpay EventResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(types::PaymentsSyncRouterData {
            status: enums::AttemptStatus::from(response.last_event),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: data.request.connector_transaction_id.clone(),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..data.clone()
        })
    }
}

impl api::PaymentCapture for Worldpay {}
impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Worldpay
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
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        match res.status_code {
            202 => {
                let response: WorldpayPaymentsResponse = res
                    .response
                    .parse_struct("Worldpay PaymentsResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                Ok(types::PaymentsCaptureRouterData {
                    status: enums::AttemptStatus::Charged,
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::try_from(response.links)?,
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                    }),
                    ..data.clone()
                })
            }
            _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        }
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}payments/settlements/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSession for Worldpay {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Worldpay
{
}

impl api::PaymentAuthorize for Worldpay {}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Worldpay
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
            "{}payments/authorizations",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let connector_req = WorldpayPaymentsRequest::try_from(req)?;
        let worldpay_req =
            utils::Encode::<WorldpayPaymentsRequest>::encode_to_string_of_json(&connector_req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(worldpay_req))
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
        let response: WorldpayPaymentsResponse = res
            .response
            .parse_struct("Worldpay PaymentsResponse")
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

impl api::Refund for Worldpay {}
impl api::RefundExecute for Worldpay {}
impl api::RefundSync for Worldpay {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Worldpay
{
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

    fn get_request_body(
        &self,
        req: &types::RefundExecuteRouterData,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let connector_req = WorldpayRefundRequest::try_from(req)?;
        let req = utils::Encode::<WorldpayRefundRequest>::encode_to_string_of_json(&connector_req)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(req))
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}payments/settlements/refunds/partials/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
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
        match res.status_code {
            202 => {
                let response: WorldpayPaymentsResponse = res
                    .response
                    .parse_struct("Worldpay PaymentsResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                Ok(types::RefundExecuteRouterData {
                    response: Ok(types::RefundsResponseData {
                        connector_refund_id: ResponseIdStr::try_from(response.links)?.id,
                        refund_status: enums::RefundStatus::Success,
                    }),
                    ..data.clone()
                })
            }
            _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        }
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Worldpay {
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
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
        Ok(format!(
            "{}payments/events/{}",
            self.base_url(connectors),
            req.request.get_connector_refund_id()?
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
        let response: WorldpayEventResponse =
            res.response
                .parse_struct("Worldpay EventResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::RefundSyncRouterData {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: data.request.refund_id.clone(),
                refund_status: enums::RefundStatus::from(response.last_event),
            }),
            ..data.clone()
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
impl api::IncomingWebhook for Worldpay {
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
