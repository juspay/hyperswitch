pub mod transformers;

use common_enums::{self, enums::PaymentConnectorCategory}; // Removed unused enums
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
        RefundsData, SetupMandateRequestData, ResponseId as RouterResponseId
    },
    router_response_types::{ConnectorInfo, PaymentsResponseData, RefundsResponseData, SupportedPaymentMethods},
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
use base64::Engine;
use transformers as spreedly;

use crate::{constants::headers, types::ResponseRouterData}; // Removed utils

#[derive(Clone)]
pub struct Spreedly {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Spreedly {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
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

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Spreedly
{
    fn get_headers(
        &self,
        req: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "/payment_methods"
        ))
    }

    fn build_request(
        &self,
        req: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let url = self.get_url(req, connectors)?;
        let headers = self.get_headers(req, connectors)?;
        // No request body needed for this operation
        
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&url)
            .headers(headers)
            .build();

        Ok(Some(request))
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Spreedly
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

impl ConnectorCommon for Spreedly {
    fn id(&self) -> &'static str {
        "spreedly"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.spreedly.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = transformers::SpreedlyAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        
        let auth_string = format!("{}:{}", auth.environment_key, auth.access_secret);
        let encoded_auth = base64::engine::general_purpose::STANDARD.encode(auth_string);
        
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Basic {}", encoded_auth).into(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: transformers::SpreedlyErrorResponse = res
            .response
            .parse_struct("SpreedlyErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let mut reasons = vec![];
        if let Some(errors) = response.errors {
            for error in errors {
                if let Some(message) = error.message {
                    reasons.push(message);
                }
            }
        }
        
        let message = reasons.join(" ");

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: res.status_code.to_string(),
            message,
            reason: None,
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        })
    }
}

impl ConnectorValidation for Spreedly {
    // Validation methods have been removed as they aren't part of the current ConnectorValidation trait
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Spreedly {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Spreedly {
    fn get_headers(
        &self,
        req: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/oauth/token", self.base_url(connectors)))
    }
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Spreedly
{
    fn get_headers(
        &self,
        req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/mandates", self.base_url(connectors)))
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Spreedly {
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
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth = transformers::SpreedlyAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let gateway_token = auth.environment_key;
        
        Ok(format!("{}/gateways/{}/authorize.json", self.base_url(connectors), gateway_token))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = req.request.minor_amount.get_amount_as_i64(); // Changed to i64
        
        let router_data = transformers::SpreedlyRouterData {
            amount,
            router_data: req,
        };
        
        let spreedly_req = transformers::SpreedlyPaymentsRequest::try_from(&router_data)?;
        Ok(RequestContent::Json(Box::new(spreedly_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request_body = self.get_request_body(req, connectors)?;
        let url = self.get_url(req, connectors)?;
        let headers = self.get_headers(req, connectors)?;
        
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&url)
            .headers(headers)
            .set_body(request_body)
            .build();

        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: transformers::SpreedlyPaymentsResponse = res
            .response
            .parse_struct("SpreedlyPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        
        event_builder.map(|i| i.set_response_body(&transformers::SpreedlyPaymentsResponseWrapper::from(response.clone())));
        
        let transaction_status = match response.transaction.succeeded {
            true => match response.transaction.state.clone().unwrap_or_default().as_str() {
                "succeeded" => common_enums::AttemptStatus::Charged,
                _ => common_enums::AttemptStatus::Authorized,
            },
            false => common_enums::AttemptStatus::Failure,
        };
        
        let response_data = match transaction_status {
            common_enums::AttemptStatus::Failure => Err(ErrorResponse {
                status_code: res.status_code,
                code: response.transaction.message.clone().unwrap_or_else(|| "Failure".to_string()), // Using message as code if error object is not present
                message: response.transaction.message.clone().unwrap_or_else(|| "Transaction failed".to_string()),
                reason: response.transaction.message.clone(), // Using message as reason
                attempt_status: Some(transaction_status),
                connector_transaction_id: Some(response.transaction.token.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
            }),
            _ => {
                let connector_id = response.transaction.token.clone();
                let response_id = Some(connector_id.clone());
                
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: RouterResponseId::ConnectorTransactionId(connector_id),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: response_id,
                    incremental_authorization_allowed: None,
                    charges: None,
                })
            }
        };
        
        Ok(PaymentsAuthorizeRouterData {
            status: transaction_status,
            response: response_data,
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Spreedly {
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
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
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
        let response: spreedly::SpreedlyPaymentsResponse = res
            .response
            .parse_struct("spreedly PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        
        let transaction_status = match response.transaction.succeeded {
            true => match response.transaction.state.clone().unwrap_or_default().as_str() {
                "succeeded" => common_enums::AttemptStatus::Charged,
                _ => common_enums::AttemptStatus::Authorized,
            },
            false => common_enums::AttemptStatus::Failure,
        };
        
        let response_data = match transaction_status {
            common_enums::AttemptStatus::Failure => Err(ErrorResponse {
                status_code: res.status_code,
                code: response.transaction.message.clone().unwrap_or_else(|| "Failure".to_string()), // Using message as code if error object is not present
                message: response.transaction.message.clone().unwrap_or_else(|| "Transaction failed".to_string()),
                reason: response.transaction.message.clone(), // Using message as reason
                attempt_status: Some(transaction_status),
                connector_transaction_id: Some(response.transaction.token.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
            }),
            _ => {
                let connector_id = response.transaction.token.clone();
                let response_id = Some(connector_id.clone());
                
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: RouterResponseId::ConnectorTransactionId(connector_id),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: response_id,
                    incremental_authorization_allowed: None,
                    charges: None,
                })
            }
        };
        
        Ok(PaymentsSyncRouterData {
            status: transaction_status,
            response: response_data,
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Spreedly {
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
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
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
        let response: spreedly::SpreedlyPaymentsResponse = res
            .response
            .parse_struct("Spreedly PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        
        let transaction_status = match response.transaction.succeeded {
            true => match response.transaction.state.clone().unwrap_or_default().as_str() {
                "succeeded" => common_enums::AttemptStatus::Charged,
                _ => common_enums::AttemptStatus::Authorized,
            },
            false => common_enums::AttemptStatus::Failure,
        };
        
        let response_data = match transaction_status {
            common_enums::AttemptStatus::Failure => Err(ErrorResponse {
                status_code: res.status_code,
                code: response.transaction.message.clone().unwrap_or_else(|| "Failure".to_string()), // Using message as code if error object is not present
                message: response.transaction.message.clone().unwrap_or_else(|| "Transaction failed".to_string()),
                reason: response.transaction.message.clone(), // Using message as reason
                attempt_status: Some(transaction_status),
                connector_transaction_id: Some(response.transaction.token.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
            }),
            _ => {
                let connector_id = response.transaction.token.clone();
                let response_id = Some(connector_id.clone());
                
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: RouterResponseId::ConnectorTransactionId(connector_id),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: response_id,
                    incremental_authorization_allowed: None,
                    charges: None,
                })
            }
        };
        
        Ok(PaymentsCaptureRouterData {
            status: transaction_status,
            response: response_data,
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Spreedly {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Spreedly {
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
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = req.request.minor_refund_amount.get_amount_as_i64(); // Changed to i64

        let connector_router_data = spreedly::SpreedlyRouterData::from((refund_amount, req));
        let connector_req = spreedly::SpreedlyRefundRequest::try_from(&connector_router_data)?;
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
        let response: spreedly::RefundResponse = res
            .response
            .parse_struct("spreedly RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Spreedly {
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
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
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
                .set_body(types::RefundSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: spreedly::RefundResponse = res
            .response
            .parse_struct("spreedly RefundSyncResponse")
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

impl ConnectorSpecifications for Spreedly {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        static ABOUT: ConnectorInfo = ConnectorInfo {
            display_name: "Spreedly",
            description: "Spreedly Payment Orchestration Platform",
            connector_type: PaymentConnectorCategory::PaymentGateway, 
        };
        Some(&ABOUT)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        None
    }

}
