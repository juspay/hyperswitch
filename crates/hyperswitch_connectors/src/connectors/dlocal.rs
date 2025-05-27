pub mod transformers;

// Removed: use std::fmt::Debug;

use api_models::webhooks::IncomingWebhookEvent;
use common_enums::enums;
use common_utils::{
    crypto::{self, SignMessage},
    date_time,
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    // Removed AmountConvertor, StringMinorUnit, StringMinorUnitForConnector
};
use error_stack::{report, ResultExt};
use hex::encode;
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
        ConnectorInfo,
        PaymentMethodDetails,
        PaymentsResponseData,
        RefundsResponseData,
        SupportedPaymentMethods,
        SupportedPaymentMethodsExt, // Added SupportedPaymentMethodsExt
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
    consts, errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use lazy_static::lazy_static;
use masking::{Mask, Maskable, PeekInterface}; // Removed ExposeInterface, Secret
use transformers as dlocal;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::RefundsRequestData, // Removed RouterData as _
};

#[derive(Clone, Debug)] // Added Debug back as the problematic field is removed
pub struct Dlocal; // Removed amount_converter field

impl Dlocal {
    pub fn new() -> &'static Self {
        &Self {} // Removed initialization of amount_converter
    }
}

impl api::Payment for Dlocal {}
impl api::PaymentToken for Dlocal {}
impl api::PaymentSession for Dlocal {}
impl api::ConnectorAccessToken for Dlocal {}
impl api::MandateSetup for Dlocal {}
impl api::PaymentAuthorize for Dlocal {}
impl api::PaymentSync for Dlocal {}
impl api::PaymentCapture for Dlocal {}
impl api::PaymentVoid for Dlocal {}
impl api::Refund for Dlocal {}
impl api::RefundExecute for Dlocal {}
impl api::RefundSync for Dlocal {}

impl<Flow, Req, Resp> ConnectorCommonExt<Flow, Req, Resp> for Dlocal
where
    Self: ConnectorIntegration<Flow, Req, Resp>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Req, Resp>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let request_content = self.get_request_body(req, connectors)?;
        // Use get_inner_value().peek().to_owned() to get the request body string
        // This handles various RequestContent types and returns a String.
        let request_body_str = request_content.get_inner_value().peek().to_owned();

        let date = date_time::date_as_yyyymmddthhmmssmmmz()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let auth = dlocal::DlocalAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let sign_payload = format!("{}{}{}", auth.x_login.peek(), date, request_body_str);

        let signature = crypto::HmacSha256::sign_message(
            &crypto::HmacSha256,
            auth.secret_key.peek().as_bytes(), // Assuming DlocalAuthType has secret_key
            sign_payload.as_bytes(),
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to sign the message")?;
        let auth_string = format!("V2-HMAC-SHA256, Signature: {}", encode(signature));

        let mut headers_vec = vec![
            (
                headers::AUTHORIZATION.to_string(),
                auth_string.into_masked(),
            ),
            (
                headers::X_LOGIN.to_string(),
                auth.x_login.clone().into_masked(),
            ),
            (
                headers::X_TRANS_KEY.to_string(),
                auth.x_trans_key.clone().into_masked(),
            ),
            (headers::X_VERSION.to_string(), "2.1".to_string().into()),
            (headers::X_DATE.to_string(), date.into_masked()),
        ];

        if !request_body_str.is_empty() {
            headers_vec.push((
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ));
        }
        Ok(headers_vec)
    }
}

impl ConnectorCommon for Dlocal {
    fn id(&self) -> &'static str {
        "dlocal"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.dlocal.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        // Dlocal uses custom headers X-Login, X-Trans-Key, X-Date, X-Version, and Authorization
        // These are constructed in `build_headers` as part of ConnectorCommonExt
        Ok(Vec::new())
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: dlocal::DlocalErrorResponse = res
            .response
            .parse_struct("DlocalErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .code
                .map_or_else(|| consts::NO_ERROR_CODE.to_string(), |c| c.to_string()),
            message: response
                .message
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.reason, // Changed from response.param to response.reason
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
        })
    }
}

impl ConnectorValidation for Dlocal {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Dlocal
{
    // Not Implemented
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Dlocal {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Dlocal {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Dlocal {
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Dlocal".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Dlocal {
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
        Ok(format!("{}secure_payments", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = dlocal::DlocalRouterData::from((
            req.request.minor_amount.get_amount_as_i64(), // Assuming DlocalRouterData::from takes (i64, &RouterData)
            req,
        ));
        let connector_req = dlocal::DlocalPaymentsRequest::try_from(&connector_router_data)?;
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
        router_env::logger::debug!(dlocal_payments_authorize_response=?res);
        let response: dlocal::DlocalPaymentsResponse = res
            .response
            .parse_struct("Dlocal PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Dlocal {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        // GET request, no content type
        ""
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        // Assuming DlocalPaymentsSyncRequest::try_from(req) exists in transformers
        // and extracts the connector_transaction_id as authz_id or similar.
        let payment_id_str = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}payments/{}/status",
            self.base_url(connectors),
            payment_id_str
        ))
    }

    fn get_request_body(
        &self,
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Ok(RequestContent::Json(Box::new(serde_json::Value::Null))) // Boxed serde_json::Value::Null
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
        router_env::logger::debug!(dlocal_payment_sync_response=?res);
        let response: dlocal::DlocalPaymentsResponse = res
            .response
            .parse_struct("Dlocal PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Dlocal {
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
        _req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        // DLocal's API captures by creating a new payment with 'authorization_id'
        Ok(format!("{}payments", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        // Assuming DlocalCaptureRequest exists in transformers
        let connector_req = dlocal::DlocalCaptureRequest::try_from(req)?;
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
        router_env::logger::debug!(dlocal_payments_capture_response=?res);
        let response: dlocal::DlocalPaymentsResponse = res
            .response
            .parse_struct("Dlocal PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Dlocal {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        // POST request, but DLocal Void might not need a body or specific content type if path based
        // Aligning with real-codebase which sets content-type in build_headers if body is present.
        // If Void has no body, build_headers won't add content-type.
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        // Assuming DlocalPaymentsCancelRequest::try_from(req) exists in transformers
        // and extracts the connector_transaction_id.
        let payment_id_str = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}payments/{}/cancel",
            self.base_url(connectors),
            payment_id_str
        ))
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        // DLocal Void (Cancel) is a POST to /payments/{payment_id}/cancel, likely no body.
        Ok(RequestContent::Json(Box::new(serde_json::Value::Null))) // Boxed serde_json::Value::Null
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post) // Void is typically a POST
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(types::PaymentsVoidType::get_request_body(
                    self, req, connectors,
                )?) // Will be NoContent
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        router_env::logger::debug!(dlocal_payments_cancel_response=?res);
        let response: dlocal::DlocalPaymentsResponse = res // Assuming Void returns a similar structure
            .response
            .parse_struct("Dlocal PaymentsCancelResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        RouterData::try_from(ResponseRouterData {
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

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Dlocal {
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
        _req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}refunds", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = dlocal::DlocalRouterData::from((
            req.request.minor_refund_amount.get_amount_as_i64(), // Assuming DlocalRouterData::from takes (i64, &RouterData)
            req,
        ));
        let connector_req = dlocal::DlocalRefundRequest::try_from(&connector_router_data)?;
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
        router_env::logger::debug!(dlocal_refund_response=?res);
        let response: dlocal::DlocalRefundResponse = res // Changed from dlocal::RefundResponse
            .response
            .parse_struct("Dlocal RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?; // Was RequestEncodingFailed
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Dlocal {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        // GET request, no content type
        ""
    }

    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        // Assuming DlocalRefundSyncRequest::try_from(req) exists in transformers
        // and extracts the connector_refund_id.
        let refund_id = req
            .request
            .get_connector_refund_id()
            .change_context(errors::ConnectorError::MissingConnectorRefundID)?;
        Ok(format!(
            "{}refunds/{}", // DLocal doc says /refunds/{refund_id}/status, but real-codebase implies /refunds/{refund_id}
            self.base_url(connectors),
            refund_id
        ))
    }

    fn get_request_body(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Ok(RequestContent::Json(Box::new(serde_json::Value::Null))) // Boxed serde_json::Value::Null
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
        router_env::logger::debug!(dlocal_refund_sync_response=?res);
        let response: dlocal::DlocalRefundResponse = res // Changed from dlocal::RefundResponse
            .response
            .parse_struct("Dlocal RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
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

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Dlocal {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        Ok(IncomingWebhookEvent::EventNotSupported) // Align with real-codebase
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

lazy_static! {
    static ref DLOCAL_SUPPORTED_PAYMENT_METHODS: SupportedPaymentMethods = {
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::Manual,
            // DLocal docs don't explicitly mention sequential automatic,
            // but it's often a subset of manual/automatic capabilities.
            // For now, aligning with real-codebase which might have broader assumptions.
            // enums::CaptureMethod::SequentialAutomatic,
        ];

        let supported_card_network = vec![
            common_enums::CardNetwork::Visa,
            common_enums::CardNetwork::Mastercard,
            common_enums::CardNetwork::AmericanExpress,
            common_enums::CardNetwork::Discover,
            common_enums::CardNetwork::JCB,
            common_enums::CardNetwork::DinersClub,
            // Elo, Hipercard are also mentioned for Brazil by DLocal.
            // UnionPay, Interac, CartesBancaires are more generic.
        ];

        let mut dlocal_supported_payment_methods = SupportedPaymentMethods::new();

        // Cards
        // Changed add_multiple to individual add calls
        dlocal_supported_payment_methods.add(
            enums::PaymentMethod::Card,
            enums::PaymentMethodType::Credit, // Removed Some()
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::NotSupported, // DLocal docs focus on one-time payments
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: Some(
                    api_models::feature_matrix::PaymentMethodSpecificFeatures::Card(
                        api_models::feature_matrix::CardSpecificFeatures {
                            three_ds: common_enums::FeatureStatus::Supported, // DLocal supports 3DS
                            no_three_ds: common_enums::FeatureStatus::Supported,
                            supported_card_networks: supported_card_network.clone(),
                        },
                    ),
                ),
            },
        );
        dlocal_supported_payment_methods.add(
            enums::PaymentMethod::Card,
            enums::PaymentMethodType::Debit, // Removed Some()
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::NotSupported, // DLocal docs focus on one-time payments
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: Some(
                    api_models::feature_matrix::PaymentMethodSpecificFeatures::Card(
                        api_models::feature_matrix::CardSpecificFeatures {
                            three_ds: common_enums::FeatureStatus::Supported, // DLocal supports 3DS
                            no_three_ds: common_enums::FeatureStatus::Supported,
                            supported_card_networks: supported_card_network.clone(),
                        },
                    ),
                ),
            },
        );

        // TODO: Add other payment methods supported by DLocal like Bank Transfer, Cash Payments, E-wallets
        // based on their documentation and connector capabilities.
        // Example:
        // dlocal_supported_payment_methods.add(
        //     enums::PaymentMethod::BankTransfer,
        //     enums::PaymentMethodType::Pix, // Example for Brazil
        //     PaymentMethodDetails {
        //         mandates: common_enums::FeatureStatus::NotSupported,
        //         refunds: common_enums::FeatureStatus::Supported, // Check DLocal docs
        //         supported_capture_methods: vec![enums::CaptureMethod::Automatic], // Typically automatic
        //         specific_features: None,
        //     },
        // );

        dlocal_supported_payment_methods
    };

    static ref DLOCAL_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
        display_name: "DLOCAL",
        description:
            "Dlocal is a cross-border payment processor enabling businesses to accept and send payments in emerging markets worldwide.",
        connector_type: enums::PaymentConnectorCategory::PaymentGateway, // As per real-codebase
    };

    // DLocal webhooks are for payment status notifications (PAID, REJECTED, CANCELLED)
    // These would map to events like PAYMENT_SUCCEEDED, PAYMENT_FAILED, etc.
    // For now, aligning with real-codebase which has an empty Vec.
    static ref DLOCAL_SUPPORTED_WEBHOOK_FLOWS: Vec<enums::EventClass> = Vec::new();
}

impl ConnectorSpecifications for Dlocal {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        // Renamed from get_connector_info
        Some(&*DLOCAL_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*DLOCAL_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&*DLOCAL_SUPPORTED_WEBHOOK_FLOWS)
    }
    // Removed get_payment_method_details_if_supported method
}
