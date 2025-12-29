pub mod transformers;

use std::sync::LazyLock;

use api_models::payments::PaymentIdType;
use api_models::webhooks::IncomingWebhookEvent;
use common_enums::enums;
use common_utils::request::{Method, Request, RequestContent};
use common_utils::{
    crypto,
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::RequestBuilder,
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use hyperswitch_domain_models::router_data::{
    AccessToken, ConnectorAuthType, ErrorResponse, RouterData,
};
use hyperswitch_domain_models::router_flow_types::*;
use hyperswitch_domain_models::{router_request_types::*, router_response_types::*, types::*};
use hyperswitch_interfaces::api::*;
use hyperswitch_interfaces::webhooks::{IncomingWebhook, IncomingWebhookRequestDetails};
use hyperswitch_interfaces::{
    api::{ConnectorCommonExt, ConnectorIntegration},
    configs::Connectors,
    errors::ConnectorError,
    events::connector_api_logs::ConnectorEvent,
    types::*,
};

use error_stack::ResultExt;
use masking::{Mask, Maskable};

use transformers::*;

use crate::connectors::worldpaymodular::transformers::request::{
    WorldpaymodularPartialRequest, WorldpaymodularPaymentsRequest,
};
use crate::types::ResponseRouterData;
use crate::utils::RefundsRequestData as _;
use crate::{
    constants::headers,
    utils::{self},
};

#[derive(Clone)]
pub struct Worldpaymodular {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Worldpaymodular {
    pub const fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Worldpaymodular
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut headers = vec![
            (
                headers::ACCEPT.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }
}

impl ConnectorCommon for Worldpaymodular {
    fn id(&self) -> &'static str {
        "worldpaymodular"
    }

    fn get_currency_unit(&self) -> CurrencyUnit {
        CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/vnd.worldpay.payments-v7+json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.worldpaymodular.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let auth = WorldpaymodularAuthType::try_from(auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response = if !res.response.is_empty() {
            res.response
                .parse_struct("WorldpaymodularErrorResponse")
                .change_context(ConnectorError::ResponseDeserializationFailed)?
        } else {
            WorldpaymodularErrorResponse::default(res.status_code)
        };

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_name,
            message: response.message,
            reason: response.validation_errors.map(|e| e.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Worldpaymodular {
    // fn validate_capture_method(
    //     &self,
    //     capture_method: Option<enums::CaptureMethod>,
    //     _pmt: Option<enums::PaymentMethodType>,
    // ) -> CustomResult<(), ConnectorError> {
    //     let capture_method = capture_method.unwrap_or_default();
    //     match capture_method {
    //         enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
    //         enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
    //             construct_not_implemented_error_report(capture_method, self.id()),
    //         ),
    //     }
    // }
}

impl Payment for Worldpaymodular {}

impl MandateSetup for Worldpaymodular {}
impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Worldpaymodular
{
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Err(
            ConnectorError::NotImplemented("Setup Mandate flow for Worldpaymodular".to_string())
                .into(),
        )
    }
}

impl PaymentToken for Worldpaymodular {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Worldpaymodular
{
    // Not Implemented (R)
}

impl PaymentVoid for Worldpaymodular {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Worldpaymodular {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}payments/authorizations/cancellations/{connector_payment_id}",
            self.base_url(connectors),
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsVoidType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, ConnectorError>
    where
        Void: Clone,
        PaymentsCancelData: Clone,
        PaymentsResponseData: Clone,
    {
        match res.status_code {
            202 => {
                let response: WorldpaymodularPaymentsResponse = res
                    .response
                    .parse_struct("Worldpaymodular PaymentsResponse")
                    .change_context(ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                Ok(PaymentsCancelRouterData {
                    status: enums::AttemptStatus::Voided,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: response.links.get_resource_id()?,
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    ..data.clone()
                })
            }
            _ => Err(ConnectorError::ResponseHandlingFailed)?,
        }
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorAccessToken for Worldpaymodular {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for Worldpaymodular
{
}

impl PaymentSync for Worldpaymodular {}
impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Worldpaymodular {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}payments/events/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, ConnectorError> {
        let response: WorldpaymodularEventResponse = res
            .response
            .parse_struct("Worldpaymodular EventResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let attempt_status = data.status;
        let worldpay_status = response.last_event;
        let status = match (attempt_status, worldpay_status.clone()) {
            (
                enums::AttemptStatus::Authorizing
                | enums::AttemptStatus::Authorized
                | enums::AttemptStatus::CaptureInitiated
                | enums::AttemptStatus::Pending
                | enums::AttemptStatus::VoidInitiated,
                EventType::Authorized,
            ) => attempt_status,
            _ => enums::AttemptStatus::from(&worldpay_status),
        };

        Ok(PaymentsSyncRouterData {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: data.request.connector_transaction_id.clone(),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..data.clone()
        })
    }
}

impl PaymentCapture for Worldpaymodular {}
impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Worldpaymodular {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}payments/settlements/partials/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req =
            WorldpaymodularPartialRequest::try_from((req, req.request.minor_amount_to_capture))?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
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
    ) -> CustomResult<PaymentsCaptureRouterData, ConnectorError> {
        match res.status_code {
            202 => {
                let response: WorldpaymodularPaymentsResponse = res
                    .response
                    .parse_struct("Worldpaymodular PaymentsResponse")
                    .change_context(ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                Ok(PaymentsCaptureRouterData {
                    status: enums::AttemptStatus::Pending,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: response.links.get_resource_id()?,
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    ..data.clone()
                })
            }
            _ => Err(ConnectorError::ResponseHandlingFailed)?,
        }
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl PaymentSession for Worldpaymodular {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Worldpaymodular {}

impl PaymentAuthorize for Worldpaymodular {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Worldpaymodular
{
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}cardPayments/customerInitiatedTransactions",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_router_data = WorldpaymodularRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.minor_amount,
            req,
        ))?;
        let auth = WorldpaymodularAuthType::try_from(&req.connector_auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        let connector_req =
            WorldpaymodularPaymentsRequest::try_from((&connector_router_data, &auth.entity_id))?;
        println!(
            "hjkhjklhjkljkl {:?} ",
            serde_json::to_string_pretty(&connector_req)
        );
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
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
    ) -> CustomResult<PaymentsAuthorizeRouterData, ConnectorError> {
        let response: WorldpaymodularPaymentsResponse = res
            .response
            .parse_struct("Worldpaymodular PaymentsResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl Refund for Worldpaymodular {}
impl RefundExecute for Worldpaymodular {}
impl RefundSync for Worldpaymodular {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Worldpaymodular {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_request_body(
        &self,
        req: &RefundExecuteRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req =
            WorldpaymodularPartialRequest::try_from((req, req.request.minor_refund_amount))?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}payments/settlements/refunds/partials/{}",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
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
    ) -> CustomResult<RefundsRouterData<Execute>, ConnectorError> {
        match res.status_code {
            202 => {
                let response: WorldpaymodularPaymentsResponse = res
                    .response
                    .parse_struct("Worldpaymodular PaymentsResponse")
                    .change_context(ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                Ok(RefundExecuteRouterData {
                    response: Ok(RefundsResponseData {
                        connector_refund_id: response.links.get_response_id_str()?.id,
                        refund_status: enums::RefundStatus::Pending,
                    }),
                    ..data.clone()
                })
            }
            _ => Err(ConnectorError::ResponseHandlingFailed)?,
        }
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Worldpaymodular {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}payments/events/{}",
            self.base_url(connectors),
            req.request.get_connector_refund_id()?
        ))
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
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
    ) -> CustomResult<RefundSyncRouterData, ConnectorError> {
        let response: WorldpaymodularEventResponse = res
            .response
            .parse_struct("Worldpaymodular EventResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        Ok(RefundSyncRouterData {
            response: Ok(RefundsResponseData {
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl IncomingWebhook for Worldpaymodular {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, ConnectorError> {
        Ok(Box::new(crypto::Sha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        let event_signature =
            utils::get_header_key_value("Event-Signature", request.headers)?.split(',');
        let sign_header = event_signature
            .last()
            .ok_or(ConnectorError::WebhookSignatureNotFound)?;
        let signature = sign_header
            .split('/')
            .last()
            .ok_or(ConnectorError::WebhookSignatureNotFound)?;
        hex::decode(signature).change_context(ConnectorError::WebhookResponseEncodingFailed)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        let secret_str = std::str::from_utf8(&connector_webhook_secrets.secret)
            .change_context(ConnectorError::WebhookBodyDecodingFailed)?;
        let to_sign = format!(
            "{}{}",
            secret_str,
            std::str::from_utf8(request.body)
                .change_context(ConnectorError::WebhookBodyDecodingFailed)?
        );
        Ok(to_sign.into_bytes())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, ConnectorError> {
        let body: WorldpaymodularWebhookTransactionId = request
            .body
            .parse_struct("WorldpayWebhookTransactionId")
            .change_context(ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            PaymentIdType::ConnectorTransactionId(body.event_details.transaction_reference),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        let body: WorldpaymodularWebhookEventType = request
            .body
            .parse_struct("WorldpaymodularWebhookEventType")
            .change_context(ConnectorError::WebhookReferenceIdNotFound)?;
        match body.event_details.event_type {
            EventType::Authorized => Ok(IncomingWebhookEvent::PaymentIntentAuthorizationSuccess),
            EventType::SentForSettlement => Ok(IncomingWebhookEvent::PaymentIntentProcessing),
            EventType::Settled => Ok(IncomingWebhookEvent::PaymentIntentSuccess),
            EventType::Error | EventType::Expired | EventType::SettlementFailed => {
                Ok(IncomingWebhookEvent::PaymentIntentFailure)
            }
            EventType::Unknown
            | EventType::SentForAuthorization
            | EventType::Cancelled
            | EventType::Refused
            | EventType::Refunded
            | EventType::SentForRefund
            | EventType::RefundFailed => Ok(IncomingWebhookEvent::EventNotSupported),
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, ConnectorError> {
        let body: WorldpaymodularWebhookEventType = request
            .body
            .parse_struct("WorldpayWebhookEventType")
            .change_context(ConnectorError::WebhookResourceObjectNotFound)?;
        let psync_body = WorldpaymodularEventResponse::try_from(body)?;
        Ok(Box::new(psync_body))
    }
}

static WORLDPAY_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::Manual,
            enums::CaptureMethod::SequentialAutomatic,
        ];

        let mut worldpay_supported_payment_methods = SupportedPaymentMethods::new();

        worldpay_supported_payment_methods.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::GooglePay,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::Supported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        worldpay_supported_payment_methods.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::ApplePay,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::Supported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        worldpay_supported_payment_methods
    });

static WORLDPAY_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Worldpaymodular",
    description: "Worldpaymodular is a payment gateway and PSP enabling secure online transactions, It utilizes modular Api's of worldpay",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Live,
};

static WORLDPAY_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] = [enums::EventClass::Payments];

impl ConnectorSpecifications for Worldpaymodular {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&WORLDPAY_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*WORLDPAY_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&WORLDPAY_SUPPORTED_WEBHOOK_FLOWS)
    }
}
