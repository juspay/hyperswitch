mod requests;
mod response;
pub mod transformers;

use std::fmt::Debug;

use common_utils::{crypto, ext_traits::ByteSliceExt, request::RequestContent};
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use transformers as worldpay;

use self::{requests::*, response::*};
use super::utils::{self, RefundsRequestData};
use crate::{
    configs::settings,
    connector::utils as connector_utils,
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
        ErrorResponse, Response,
    },
    utils::BytesExt,
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
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
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

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
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
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: worldpay::WorldpayAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: WorldpayErrorResponse = res
            .response
            .parse_struct("WorldpayErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_error_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_name,
            message: response.message,
            reason: response.validation_errors.map(|e| e.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Worldpay {
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

impl api::Payment for Worldpay {}

impl api::MandateSetup for Worldpay {}
impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Worldpay
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
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Worldpay".to_string())
                .into(),
        )
    }
}

impl api::PaymentToken for Worldpay {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Worldpay
{
    // Not Implemented (R)
}

impl api::PaymentVoid for Worldpay {}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Worldpay
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
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
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
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                Ok(types::PaymentsCancelRouterData {
                    status: enums::AttemptStatus::Voided,
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::try_from(response.links)?,
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
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
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: WorldpayEventResponse =
            res.response
                .parse_struct("Worldpay EventResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(types::PaymentsSyncRouterData {
            status: enums::AttemptStatus::from(response.last_event),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: data.request.connector_transaction_id.clone(),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
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
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        match res.status_code {
            202 => {
                let response: WorldpayPaymentsResponse = res
                    .response
                    .parse_struct("Worldpay PaymentsResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                Ok(types::PaymentsCaptureRouterData {
                    status: enums::AttemptStatus::Charged,
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::try_from(response.links)?,
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
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
        Ok(format!(
            "{}payments/authorizations",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = worldpay::WorldpayRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = WorldpayPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
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
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: WorldpayPaymentsResponse = res
            .response
            .parse_struct("Worldpay PaymentsResponse")
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
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
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
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &types::RefundExecuteRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = WorldpayRefundRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
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
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        match res.status_code {
            202 => {
                let response: WorldpayPaymentsResponse = res
                    .response
                    .parse_struct("Worldpay PaymentsResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Worldpay {
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
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: WorldpayEventResponse =
            res.response
                .parse_struct("Worldpay EventResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Worldpay {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::Sha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let event_signature =
            utils::get_header_key_value("Event-Signature", request.headers)?.split(',');
        let sign_header = event_signature
            .last()
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        let signature = sign_header
            .split('/')
            .last()
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        hex::decode(signature)
            .into_report()
            .change_context(errors::ConnectorError::WebhookResponseEncodingFailed)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let secret_str = std::str::from_utf8(&connector_webhook_secrets.secret)
            .into_report()
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let to_sign = format!(
            "{}{}",
            secret_str,
            std::str::from_utf8(request.body)
                .into_report()
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?
        );
        Ok(to_sign.into_bytes())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let body: WorldpayWebhookTransactionId = request
            .body
            .parse_struct("WorldpayWebhookTransactionId")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            types::api::PaymentIdType::ConnectorTransactionId(
                body.event_details.transaction_reference,
            ),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let body: WorldpayWebhookEventType = request
            .body
            .parse_struct("WorldpayWebhookEventType")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        match body.event_details.event_type {
            EventType::SentForSettlement | EventType::Charged => {
                Ok(api::IncomingWebhookEvent::PaymentIntentSuccess)
            }
            EventType::Error | EventType::Expired => {
                Ok(api::IncomingWebhookEvent::PaymentIntentFailure)
            }
            EventType::Unknown
            | EventType::Authorized
            | EventType::Cancelled
            | EventType::Refused
            | EventType::Refunded
            | EventType::SentForRefund
            | EventType::CaptureFailed
            | EventType::RefundFailed => Ok(api::IncomingWebhookEvent::EventNotSupported),
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let body: WorldpayWebhookEventType = request
            .body
            .parse_struct("WorldpayWebhookEventType")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        let psync_body = WorldpayEventResponse::try_from(body)?;
        Ok(Box::new(psync_body))
    }
}
