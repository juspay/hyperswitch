pub mod transformers;

// Constants
pub const BASE_URL: &str = "https://api.monexgroup.com/v1";
pub const SANDBOX_URL: &str = "https://sandbox.api.monexgroup.com/v1";
pub const PAYMENTS_URL: &str = "/payments/authorize";
pub const CAPTURES_URL: &str = "/payments/capture";
pub const PAYMENTS_SYNC_URL: &str = "/payments";
pub const REFUNDS_URL: &str = "/payments/refund";

use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
};
use error_stack::{report, ResultExt};
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
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use masking::{ExposeInterface, Mask};
use transformers as monex;

use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct Monex {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Monex {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
    }
}

impl api::Payment for Monex {}
impl api::PaymentSession for Monex {}
impl api::ConnectorAccessToken for Monex {}
impl api::MandateSetup for Monex {}
impl api::PaymentAuthorize for Monex {}
impl api::PaymentSync for Monex {}
impl api::PaymentCapture for Monex {}
impl api::PaymentVoid for Monex {}
impl api::Refund for Monex {}
impl api::RefundExecute for Monex {}
impl api::RefundSync for Monex {}
impl api::PaymentToken for Monex {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Monex
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Monex
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
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

impl ConnectorCommon for Monex {
    fn id(&self) -> &'static str {
        "monex"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        // Based on Monex documentation, it processes amount in minor units (cents for USD)
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.monex.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        // First check if we have a stored access token
        if let ConnectorAuthType::BodyKey { api_key, .. } = auth_type {
            return Ok(vec![(
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", api_key.clone().expose()).into_masked(),
            )]);
        }
        
        // Fall back to API key if access token not available
        let auth = monex::MonexAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", auth.api_key.clone().expose()).into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        // Try to parse as detailed error response first
        let detailed_response = res.response.parse_struct::<monex::MonexDetailedErrorResponse>("MonexDetailedErrorResponse");
        
        let response = match detailed_response {
            Ok(detailed) if !detailed.errors.is_empty() => {
                // Use the first error from the detailed error response
                event_builder.map(|i| i.set_response_body(&detailed));
                router_env::logger::info!(connector_response=?detailed);
                
                let first_error = &detailed.errors[0];
                ErrorResponse {
                    status_code: res.status_code,
                    code: first_error.code.clone(),
                    message: first_error.message.clone(),
                    reason: first_error.reason.clone(),
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                }
            },
            _ => {
                // Try to parse as standard error response
                let standard_response = res.response
                    .parse_struct::<monex::MonexErrorResponse>("MonexErrorResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                
                event_builder.map(|i| i.set_response_body(&standard_response));
                router_env::logger::info!(connector_response=?standard_response);
                
                // Map status code to appropriate attempt status
                let attempt_status = match res.status_code {
                    400..=499 => match standard_response.code.as_str() {
                        "card_declined" => Some(common_enums::AttemptStatus::Failure),
                        "insufficient_funds" => Some(common_enums::AttemptStatus::Failure),
                        "invalid_card" => Some(common_enums::AttemptStatus::Failure),
                        _ => None
                    },
                    500..=599 => Some(common_enums::AttemptStatus::Pending),
                    _ => None,
                };

                ErrorResponse {
                    status_code: res.status_code,
                    code: standard_response.code,
                    message: standard_response.message,
                    reason: standard_response.reason,
                    attempt_status,
                    connector_transaction_id: None,
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                }
            }
        };

        Ok(response)
    }
}

impl ConnectorValidation for Monex {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Monex {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Monex {
    fn get_headers(
        &self,
        _req: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                "application/x-www-form-urlencoded".to_string().into(),
            ),
        ])
    }

    fn get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn get_url(
        &self,
        _req: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/oauth/token", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        router_env::logger::debug!("Creating OAuth token request for Monex");
        let auth = monex::MonexAuthType::try_from(&req.connector_auth_type)?;
        let connector_req = monex::MonexOAuthRequest {
            grant_type: "client_credentials".to_string(),
            client_id: auth.client_id,
            client_secret: auth.client_secret,
        };
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&types::RefreshTokenType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::RefreshTokenType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::RefreshTokenType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>, errors::ConnectorError> {
        let response: monex::MonexOAuthResponse = res
            .response
            .parse_struct("MonexOAuthResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        // Calculate expiry time
        let expires_in = response.expires_in;
        let expiry_time = time::OffsetDateTime::now_utc().unix_timestamp() + expires_in;

        Ok(RouterData {
            response: Ok(AccessToken {
                token: response.access_token,
                expires: expiry_time,
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

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Monex {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Monex {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        Ok(format!("{}{}", self.base_url(connectors), PAYMENTS_URL))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        router_env::logger::debug!(
            payment_id=?req.payment_id,
            "Creating payment authorization request for Monex"
        );
        
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        
        router_env::logger::debug!(
            payment_id=?req.payment_id,
            amount=?amount,
            currency=?req.request.currency,
            "Payment amount converted for Monex"
        );

        let connector_router_data = monex::MonexRouterData::from((amount, req));
        let connector_req = monex::MonexPaymentsRequest::try_from(&connector_router_data)?;
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
        router_env::logger::debug!(
            payment_id=?data.payment_id,
            status_code=?res.status_code,
            "Received payment authorization response from Monex"
        );
        
        let response: monex::MonexPaymentsResponse = res
            .response
            .parse_struct("Monex PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            
        event_builder.map(|i| i.set_response_body(&response));
        
        router_env::logger::info!(
            payment_id=?data.payment_id,
            connector_response=?response,
            connector_payment_id=?response.id,
            status=?response.status,
            "Payment authorization processed"
        );
        
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Monex {
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
        let payment_id = req.request.connector_transaction_id.clone();
        let payment_id_str = match payment_id {
            hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(id) => id,
            _ => Err(errors::ConnectorError::RequestEncodingFailed)?,
        };
        Ok(format!("{}{}/{}", self.base_url(connectors), PAYMENTS_SYNC_URL, payment_id_str))
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
        let response: monex::MonexPaymentsResponse = res
            .response
            .parse_struct("monex PaymentsSyncResponse")
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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Monex {
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
        // Format the URL with the payment ID from the connector transaction ID
        let payment_id = &req.request.connector_transaction_id;
        Ok(format!("{}{}/{}", self.base_url(connectors), CAPTURES_URL, payment_id))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        // Convert amount_to_capture to MinorUnit
        let minor_amount = common_utils::types::MinorUnit::new(req.request.amount_to_capture);
            
        let amount = utils::convert_amount(
            self.amount_converter,
            minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = monex::MonexRouterData::from((amount, req));
        let connector_req = monex::MonexPaymentsCaptureRequest::try_from(&connector_router_data)?;
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
        let response: monex::MonexPaymentsResponse = res
            .response
            .parse_struct("Monex PaymentsCaptureResponse")
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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Monex {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Monex {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        // Format the URL with the payment ID
        let payment_id = req.request.connector_transaction_id.clone();
        Ok(format!("{}{}/{}", self.base_url(connectors), REFUNDS_URL, payment_id))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        router_env::logger::debug!(
            payment_id=?req.payment_id,
            refund_id=?req.request.refund_id,
            "Creating refund request for Monex"
        );
        
        let refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;
        
        router_env::logger::debug!(
            payment_id=?req.payment_id,
            refund_id=?req.request.refund_id,
            amount=?refund_amount,
            currency=?req.request.currency,
            "Refund amount converted for Monex"
        );

        let connector_router_data = monex::MonexRouterData::from((refund_amount, req));
        let connector_req = monex::MonexRefundRequest::try_from(&connector_router_data)?;
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
        router_env::logger::debug!(
            payment_id=?data.payment_id,
            refund_id=?data.request.refund_id,
            status_code=?res.status_code,
            "Received refund response from Monex"
        );
        
        let response: monex::MonexRefundResponse = res
            .response
            .parse_struct("Monex RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            
        event_builder.map(|i| i.set_response_body(&response));
        
        router_env::logger::info!(
            payment_id=?data.payment_id,
            refund_id=?data.request.refund_id,
            connector_response=?response,
            connector_refund_id=?response.id,
            status=?response.status,
            "Refund processed"
        );
        
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Monex {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        // Extract the refund ID from the connector_refund_id
        let refund_id = req.request.connector_refund_id.clone()
            .ok_or(errors::ConnectorError::MissingConnectorRefundID)?;
            
        // Format the URL with the refund ID
        Ok(format!("{}/refunds/{}", self.base_url(connectors), refund_id))
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
        let response: monex::MonexRefundResponse = res
            .response
            .parse_struct("Monex RefundSyncResponse")
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
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Monex {
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

impl ConnectorSpecifications for Monex {}
