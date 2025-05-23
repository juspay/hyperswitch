pub mod transformers;

use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Mask};
use base64::Engine; // For BASE64_ENGINE.encode

use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
};

use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{
            Authorize, Capture, PSync, PaymentMethodToken, Session,
            SetupMandate, Void,
        },
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData,
        PaymentsCaptureRouterData, PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorValidation, ConnectorSpecifications},
    configs::Connectors,
    consts, // Import for NO_ERROR_CODE, NO_ERROR_MESSAGE
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use crate::{
    constants::headers,
    types::ResponseRouterData,
};

use transformers as spreedly;

#[derive(Clone)]
pub struct Spreedly {}

impl Spreedly {
    pub fn new() -> &'static Self {
        &Self {}
    }
}

impl api::Payment for Spreedly {}
impl api::PaymentSession for Spreedly {}
impl api::ConnectorAccessToken for Spreedly {}
impl api::MandateSetup for Spreedly {}
impl api::PaymentAuthorize for Spreedly {}
impl api::PaymentSync for Spreedly {}
impl api::PaymentCapture for Spreedly {}
impl api::PaymentVoid for Spreedly {}
impl api::Refund for Spreedly {}
impl api::RefundExecute for Spreedly {}
impl api::RefundSync for Spreedly {}
impl api::PaymentToken for Spreedly {}

impl
    ConnectorIntegration<
        PaymentMethodToken,
        PaymentMethodTokenizationData,
        PaymentsResponseData,
    > for Spreedly
{
    fn get_headers(&self, req: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>, connectors: &Connectors,) -> CustomResult<Vec<(String, masking::Maskable<String>)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>, connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
        Ok(format!(
            "{}/payment_methods.json",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(&self, req: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>, _connectors: &Connectors,) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = spreedly::SpreedlyTokenizeRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&self.get_url(req, connectors)?)
                .attach_default_headers()
                .headers(self.get_headers(req, connectors)?)
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,errors::ConnectorError> {
        let response: spreedly::SpreedlyTokenizeResponse = res.response.parse_struct("SpreedlyTokenizeResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(&self, res: Response, event_builder: Option<&mut ConnectorEvent>) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Spreedly
where
    Self: ConnectorIntegration<Flow, Request, Response>,{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Spreedly {
    fn id(&self) -> &'static str {
        "spreedly"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        // As per Spreedly documentation, amounts are in major units (e.g., "10.00" for $10.00)
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.spreedly.base_url.as_ref()
    }

    fn get_auth_header(&self, auth_type:&ConnectorAuthType)-> CustomResult<Vec<(String,masking::Maskable<String>)>,errors::ConnectorError> {
        let auth =  spreedly::SpreedlyAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        // Basic auth for Spreedly: environment_key:access_secret
        let basic_auth_val = format!("{}:{}", auth.environment_key.expose(), auth.access_secret.expose());
        Ok(vec![(headers::AUTHORIZATION.to_string(), format!("Basic {}", common_utils::consts::BASE64_ENGINE.encode(basic_auth_val.as_bytes())).into_masked())])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        // Attempt to parse as SpreedlyErrorResponse which might contain a nested transaction error
        let parsed_response: Result<spreedly::SpreedlyErrorResponse, _> = res.response.parse_struct("SpreedlyErrorResponse");

        let (code, message, reason) = match parsed_response {
            Ok(spreedly_error_response) => {
                event_builder.map(|i| i.set_response_body(&spreedly_error_response));
                router_env::logger::info!(connector_response=?spreedly_error_response);

                if let Some(transaction_error) = spreedly_error_response.transaction {
                    // Prefer details from the nested transaction error if available
                    let detailed_error_message = transaction_error.errors.as_ref()
                        .and_then(|errors| errors.first())
                        .map(|e| e.message.clone());
                    let error_key = transaction_error.errors.as_ref()
                        .and_then(|errors| errors.first())
                        .and_then(|e| e.key.clone());
                    
                    (
                        error_key.unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                        transaction_error.message.or(detailed_error_message).unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                        None // Spreedly's detailed errors might not map directly to a single "reason"
                    )
                } else if let Some(general_errors) = spreedly_error_response.errors {
                     // Use general errors if no transaction error
                    let detailed_error_message = general_errors.first().map(|e| e.message.clone());
                    let error_key = general_errors.first().and_then(|e| e.key.clone());
                    (
                        error_key.unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                        detailed_error_message.or(spreedly_error_response.message).unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                        None
                    )
                }
                 else {
                    // Fallback to top-level message if no specific error details
                    (
                        spreedly_error_response.code.unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                        spreedly_error_response.message.unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                        spreedly_error_response.reason,
                    )
                }
            }
            Err(_) => {
                // If parsing SpreedlyErrorResponse fails, use the raw response as message
                let error_message = String::from_utf8(res.response.to_vec())
                    .unwrap_or_else(|_| "Failed to parse error response and decode as UTF-8".to_string());
                router_env::logger::error!(raw_error_response=?error_message);
                (
                    consts::NO_ERROR_CODE.to_string(),
                    error_message,
                    None,
                )
            }
        };

        Ok(ErrorResponse {
            status_code: res.status_code,
            code,
            message,
            reason,
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        })
    }
}

impl ConnectorValidation for Spreedly
{
    //TODO: implement functions when support enabled
}

impl
    ConnectorIntegration<
        Session,
        PaymentsSessionData,
        PaymentsResponseData,
    > for Spreedly
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for Spreedly
{
}

impl
    ConnectorIntegration<
        SetupMandate,
        SetupMandateRequestData,
        PaymentsResponseData,
    > for Spreedly
{
}

impl
    ConnectorIntegration<
        Authorize,
        PaymentsAuthorizeData,
        PaymentsResponseData,
    > for Spreedly {
    fn get_headers(&self, req: &PaymentsAuthorizeRouterData, connectors: &Connectors,) -> CustomResult<Vec<(String, masking::Maskable<String>)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, req: &PaymentsAuthorizeRouterData, connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
        let auth_type = spreedly::SpreedlyAuthType::try_from(&req.connector_auth_type)?;
        Ok(format!(
            "{}/gateways/{}/transactions.json",
            self.base_url(connectors),
            auth_type.gateway_token.expose()
        ))
    }

    fn get_request_body(&self, req: &PaymentsAuthorizeRouterData, _connectors: &Connectors,) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data =
            spreedly::SpreedlyRouterData::from((
                req.request.minor_amount, // Pass MinorUnit directly
                req,
            ));
        let connector_req = spreedly::SpreedlyAuthorizeRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Put) // Spreedly uses PUT for Authorize
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsAuthorizeType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData,errors::ConnectorError> {
        let response: spreedly::SpreedlyAuthorizeResponse = res.response.parse_struct("SpreedlyAuthorizeResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(&self, res: Response, event_builder: Option<&mut ConnectorEvent>) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData>
    for Spreedly
{
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        let transaction_token = req.request.connector_transaction_id.clone().get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}/transactions/{}.json",
            self.base_url(connectors),
            transaction_token
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
        // Assuming PSync response for a single transaction is similar to Authorize's
        let response: spreedly::SpreedlyAuthorizeResponse = res
            .response
            .parse_struct("SpreedlyPSyncResponse") // Or SpreedlyAuthorizeResponse if structure matches
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
        event_builder: Option<&mut ConnectorEvent>
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    ConnectorIntegration<
        Capture,
        PaymentsCaptureData,
        PaymentsResponseData,
    > for Spreedly
{
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        let auth_type = spreedly::SpreedlyAuthType::try_from(&req.connector_auth_type)?;
        let transaction_token = req.request.connector_transaction_id.clone(); // This is already a String
        Ok(format!(
            "{}/gateways/{}/transactions/{}/capture.json",
            self.base_url(connectors),
            auth_type.gateway_token.expose(),
            transaction_token
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data =
            spreedly::SpreedlyRouterData::from((
                req.request.minor_amount_to_capture, // Pass MinorUnit directly for amount to capture
                req,
            ));
        let connector_req = spreedly::SpreedlyCaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Put) // Spreedly uses PUT for Capture
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsCaptureType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        // Assuming Capture response is similar to Authorize's
        let response: spreedly::SpreedlyAuthorizeResponse = res
            .response
            .parse_struct("SpreedlyCaptureResponse") // Or SpreedlyAuthorizeResponse if structure matches
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
        event_builder: Option<&mut ConnectorEvent>
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    ConnectorIntegration<
        Void,
        PaymentsCancelData,
        PaymentsResponseData,
    > for Spreedly
{}

impl
    ConnectorIntegration<
        Execute,
        RefundsData,
        RefundsResponseData,
    > for Spreedly {
    fn get_headers(&self, req: &RefundsRouterData<Execute>, connectors: &Connectors,) -> CustomResult<Vec<(String,masking::Maskable<String>)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, req: &RefundsRouterData<Execute>, connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
        let auth_type = spreedly::SpreedlyAuthType::try_from(&req.connector_auth_type)?;
        let transaction_token = req.request.connector_transaction_id.clone(); // This is already a String
        Ok(format!(
            "{}/gateways/{}/transactions/{}/credit.json",
            self.base_url(connectors),
            auth_type.gateway_token.expose(),
            transaction_token
        ))
    }

    fn get_request_body(&self, req: &RefundsRouterData<Execute>, _connectors: &Connectors,) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data =
            spreedly::SpreedlyRouterData::from((
                req.request.minor_refund_amount, // Pass MinorUnit directly
                req,
            ));
        let connector_req = spreedly::SpreedlyRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(&self, req: &RefundsRouterData<Execute>, connectors: &Connectors,) -> CustomResult<Option<Request>,errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Put) // Spreedly uses PUT for Refund
            .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefundExecuteType::get_headers(self, req, connectors)?)
            .set_body(types::RefundExecuteType::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>,errors::ConnectorError> {
        let response: spreedly::RefundResponse = res.response.parse_struct("spreedly RefundResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(&self, res: Response, event_builder: Option<&mut ConnectorEvent>) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Spreedly {
    fn get_headers(&self, req: &RefundSyncRouterData,connectors: &Connectors,) -> CustomResult<Vec<(String, masking::Maskable<String>)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, req: &RefundSyncRouterData,connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
        // RSync uses the connector_refund_id which is the Spreedly transaction token for the refund
        let transaction_token = req.request.connector_refund_id.clone()
            .ok_or_else(|| errors::ConnectorError::MissingRequiredField {field_name: "connector_refund_id"})?;
        Ok(format!(
            "{}/transactions/{}.json",
            self.base_url(connectors),
            transaction_token
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
                .set_body(types::RefundSyncType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData,errors::ConnectorError,> {
        let response: spreedly::RefundResponse = res.response.parse_struct("spreedly RefundSyncResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(&self, res: Response, event_builder: Option<&mut ConnectorEvent>) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Spreedly {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

impl ConnectorSpecifications for Spreedly {}
