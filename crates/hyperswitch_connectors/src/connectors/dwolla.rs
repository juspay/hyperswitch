pub mod transformers;

use error_stack::ResultExt;
use masking::{ExposeInterface, Mask};

use base64::Engine;
use common_utils::{
    consts::BASE64_ENGINE,
    errors::CustomResult,
    ext_traits::{BytesExt, ByteSliceExt},
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
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
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self, RefundsRequestData},
};

use transformers as dwolla;

#[derive(Clone)]
pub struct Dwolla {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync)
}

impl Dwolla {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector
        }
    }
}

impl api::Payment for Dwolla {}
impl api::PaymentSession for Dwolla {}
impl api::ConnectorAccessToken for Dwolla {}
impl api::MandateSetup for Dwolla {}
impl api::PaymentAuthorize for Dwolla {}
impl api::PaymentSync for Dwolla {}
impl api::PaymentCapture for Dwolla {}
impl api::PaymentVoid for Dwolla {}
impl api::Refund for Dwolla {}
impl api::RefundExecute for Dwolla {}
impl api::RefundSync for Dwolla {}
impl api::PaymentToken for Dwolla {}

impl
    ConnectorIntegration<
        PaymentMethodToken,
        PaymentMethodTokenizationData,
        PaymentsResponseData,
    > for Dwolla
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Dwolla
where
    Self: ConnectorIntegration<Flow, Request, Response>,{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::ACCEPT.to_string(),
                "application/vnd.dwolla.v1.hal+json".to_string().into(),
            ),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Dwolla {
    fn id(&self) -> &'static str {
        "dwolla"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        // Dwolla accepts amounts in major units (dollars, not cents)
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/vnd.dwolla.v1.hal+json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.dwolla.base_url.as_ref()
    }

    fn get_auth_header(&self, auth_type:&ConnectorAuthType)-> CustomResult<Vec<(String,masking::Maskable<String>)>,errors::ConnectorError> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => {
                // This means we have a Bearer token from OAuth
                let auth_header = format!("Bearer {}", api_key.clone().expose());
                Ok(vec![(headers::AUTHORIZATION.to_string(), auth_header.into_masked())])
            },
            ConnectorAuthType::BodyKey { api_key, key1: _ } => {
                // For BodyKey with Dwolla, api_key should contain the access token obtained from OAuth
                // This is the access token that should be used for API calls
                let auth_header = format!("Bearer {}", api_key.clone().expose());
                Ok(vec![(headers::AUTHORIZATION.to_string(), auth_header.into_masked())])
            },
            _ => {
                Err(errors::ConnectorError::FailedToObtainAuthType.into())
            }
        }
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: dwolla::DwollaErrorResponse = res
            .response
            .parse_struct("DwollaErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        // Use the new ACH return code mapping function
        let _mapped_error = dwolla::map_dwolla_error_to_hyperswitch(&response);
        
        // Extract ACH return code information if present
        let (network_decline_code, network_error_message) = if let Some(ach_code) = dwolla::extract_ach_return_code(&response.code) {
            (
                Some(response.code.clone()),
                Some(ach_code.get_error_reason().to_string()),
            )
        } else {
            (None, None)
        };

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response.reason,
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code,
            network_advice_code: None,
            network_error_message,
        })
    }
}

impl ConnectorValidation for Dwolla
{
    //TODO: implement functions when support enabled
}

impl
    ConnectorIntegration<
        Session,
        PaymentsSessionData,
        PaymentsResponseData,
    > for Dwolla
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for Dwolla
{
    fn get_headers(
        &self,
        req: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = dwolla::DwollaAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        
        // Create Basic Auth header for OAuth token request
        let auth_val = format!("{}:{}", auth.client_id.expose(), auth.client_secret.expose());
        let auth_header = format!("Basic {}", BASE64_ENGINE.encode(auth_val));
        
        Ok(vec![
            (headers::CONTENT_TYPE.to_string(), "application/x-www-form-urlencoded".to_string().into()),
            (headers::AUTHORIZATION.to_string(), auth_header.into()),
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
        Ok(format!("{}/token", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        _req: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let token_request = dwolla::DwollaTokenRequest {
            grant_type: "client_credentials".to_string(),
        };
        
        // Use the struct directly for form encoding
        Ok(RequestContent::FormUrlEncoded(Box::new(token_request)))
    }

    fn build_request(
        &self,
        req: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
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
        data: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>, errors::ConnectorError> {
        let response: dwolla::DwollaTokenResponse = res
            .response
            .parse_struct("DwollaTokenResponse")
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
        // Try to parse as OAuth error first, then fall back to regular error
        if let Ok(oauth_error) = res.response.parse_struct::<dwolla::DwollaOAuthErrorResponse>("DwollaOAuthErrorResponse") {
            event_builder.map(|i| i.set_response_body(&oauth_error));
            router_env::logger::info!(connector_response=?oauth_error);
            
            // Convert OAuth error to standard error format
            let standard_error: dwolla::DwollaErrorResponse = oauth_error.into();
            
            Ok(ErrorResponse {
                status_code: res.status_code,
                code: standard_error.code,
                message: standard_error.message,
                reason: standard_error.reason,
                attempt_status: None,
                connector_transaction_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
            })
        } else {
            // Fall back to regular error handling
            self.build_error_response(res, event_builder)
        }
    }
}

impl
    ConnectorIntegration<
        SetupMandate,
        SetupMandateRequestData,
        PaymentsResponseData,
    > for Dwolla
{
}

impl
    ConnectorIntegration<
        Authorize,
        PaymentsAuthorizeData,
        PaymentsResponseData,
    > for Dwolla {
    fn get_headers(&self, req: &PaymentsAuthorizeRouterData, connectors: &Connectors,) -> CustomResult<Vec<(String, masking::Maskable<String>)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &PaymentsAuthorizeRouterData, connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
        // For Dwolla, we'll use the transfers endpoint for ACH payments
        // The actual flow involves multiple steps: customer creation, funding source creation, then transfer
        // For now, we'll use the transfers endpoint as the main URL
        Ok(format!("{}/transfers", self.base_url(connectors)))
    }

    fn get_request_body(&self, req: &PaymentsAuthorizeRouterData, _connectors: &Connectors,) -> CustomResult<RequestContent, errors::ConnectorError> {
        let _amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        // Convert StringMinorUnit to StringMajorUnit for Dwolla (dollars instead of cents)
        let amount_i64 = req.request.minor_amount.get_amount_as_i64();
        let _major_amount = utils::to_currency_base_unit_with_zero_decimal_check(
            amount_i64,
            req.request.currency,
        )?;

        // Use the StringMajorUnitForConnector to convert the amount properly
        let amount_converter = common_utils::types::StringMajorUnitForConnector;
        let major_unit_amount = amount_converter.convert(
            common_utils::types::MinorUnit::new(amount_i64),
            req.request.currency,
        ).change_context(errors::ConnectorError::AmountConversionFailed)?;

        // Create a simple transfer request for Dwolla API
        let transfer_request = dwolla::DwollaTransferRequest {
            links: dwolla::DwollaTransferLinks {
                source: dwolla::DwollaLink {
                    href: "".to_string(), // This should be populated with actual funding source URL
                },
                destination: dwolla::DwollaLink {
                    href: "".to_string(), // This should be populated with actual funding source URL
                },
            },
            amount: dwolla::DwollaAmount {
                currency: req.request.currency.to_string(),
                value: major_unit_amount.get_amount_as_string(),
            },
            metadata: Some(dwolla::DwollaMetadata {
                order_id: req.connector_request_reference_id.clone(),
                customer_reference: req.payment_id.clone(),
            }),
            clearing: None,
            correlation_id: None,
        };

        Ok(RequestContent::Json(Box::new(transfer_request)))
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
        let response: dwolla::DwollaPaymentsResponse = res.response.parse_struct("Dwolla PaymentsAuthorizeResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
    for Dwolla
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
        // For payment sync, we need to get the transfer status using the transfer ID
        let connector_transaction_id = req.request.connector_transaction_id.get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        
        Ok(format!("{}/transfers/{}", self.base_url(connectors), connector_transaction_id))
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
        let response: dwolla:: DwollaPaymentsResponse = res
            .response
            .parse_struct("dwolla PaymentsSyncResponse")
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
    > for Dwolla
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
        let response: dwolla::DwollaPaymentsResponse = res
            .response
            .parse_struct("Dwolla PaymentsCaptureResponse")
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
    > for Dwolla
{}

impl
    ConnectorIntegration<
        Execute,
        RefundsData,
        RefundsResponseData,
    > for Dwolla {
    fn get_headers(&self, req: &RefundsRouterData<Execute>, connectors: &Connectors,) -> CustomResult<Vec<(String,masking::Maskable<String>)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &RefundsRouterData<Execute>, connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
        // For refunds, we also use the transfers endpoint but with reversed source/destination
        // This creates a credit transfer (refund) from merchant to customer
        Ok(format!("{}/transfers", self.base_url(connectors)))
    }

    fn get_request_body(&self, req: &RefundsRouterData<Execute>, _connectors: &Connectors,) -> CustomResult<RequestContent, errors::ConnectorError> {
        let _refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        // Convert StringMinorUnit to StringMajorUnit for Dwolla (dollars instead of cents)
        let refund_amount_i64 = req.request.minor_refund_amount.get_amount_as_i64();
        let _major_refund_amount = utils::to_currency_base_unit_with_zero_decimal_check(
            refund_amount_i64,
            req.request.currency,
        )?;

        // Use the StringMajorUnitForConnector to convert the refund amount properly
        let amount_converter = common_utils::types::StringMajorUnitForConnector;
        let major_unit_refund_amount = amount_converter.convert(
            common_utils::types::MinorUnit::new(refund_amount_i64),
            req.request.currency,
        ).change_context(errors::ConnectorError::AmountConversionFailed)?;

        let connector_router_data =
            dwolla::DwollaRouterData::from((
                major_unit_refund_amount,
                req,
            ));
        let connector_req = dwolla::DwollaRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(&self, req: &RefundsRouterData<Execute>, connectors: &Connectors,) -> CustomResult<Option<Request>,errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
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
        let response: dwolla::RefundResponse = res.response.parse_struct("dwolla RefundResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
    ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Dwolla {
    fn get_headers(&self, req: &RefundSyncRouterData,connectors: &Connectors,) -> CustomResult<Vec<(String, masking::Maskable<String>)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, req: &RefundSyncRouterData, connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
        // For refund sync, we need to get the refund transfer status using the refund ID
        let connector_refund_id = req.request.get_connector_refund_id()?;
        Ok(format!("{}/transfers/{}", self.base_url(connectors), connector_refund_id))
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
        let response: dwolla::RefundResponse = res.response.parse_struct("dwolla RefundSyncResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
impl webhooks::IncomingWebhook for Dwolla {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn common_utils::crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(common_utils::crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let signature_header = request
            .headers
            .get("X-Request-Signature-SHA-256")
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?
            .to_str()
            .map_err(|_| errors::ConnectorError::WebhookSignatureNotFound)?;

        // Dwolla sends the signature as a hex-encoded string
        hex::decode(signature_header)
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        // For Dwolla, the message to verify is the raw request body
        Ok(request.body.to_vec())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook_body: dwolla::DwollaWebhookEvent = request
            .body
            .parse_struct("DwollaWebhookEvent")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        // Use the enhanced webhook processing function
        dwolla::get_webhook_object_reference_id(&webhook_body)
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook_body: dwolla::DwollaWebhookEvent = request
            .body
            .parse_struct("DwollaWebhookEvent")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        // Use the enhanced webhook event processing function
        Ok(dwolla::process_webhook_event_type(&webhook_body))
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook_body: dwolla::DwollaWebhookEvent = request
            .body
            .parse_struct("DwollaWebhookEvent")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(Box::new(webhook_body))
    }
}

impl ConnectorSpecifications for Dwolla {}

// Bank Account Verification Helper Methods
impl Dwolla {
    /// Initiate micro-deposit verification for a funding source
    pub async fn initiate_micro_deposits(
        &self,
        funding_source_url: &str,
        _connectors: &Connectors,
        auth_header: Vec<(String, masking::Maskable<String>)>,
    ) -> CustomResult<dwolla::DwollaMicroDepositResponse, errors::ConnectorError> {
        let micro_deposit_request = dwolla::DwollaMicroDepositRequest {
            links: dwolla::DwollaMicroDepositLinks {
                funding_source: dwolla::DwollaLink {
                    href: funding_source_url.to_string(),
                },
            },
        };

        let url = format!("{}/micro-deposits", self.base_url(_connectors));
        
        let _request = RequestBuilder::new()
            .method(Method::Post)
            .url(&url)
            .attach_default_headers()
            .headers(auth_header)
            .set_body(RequestContent::Json(Box::new(micro_deposit_request)))
            .build();

        // This would need to be called with proper HTTP client
        Err(errors::ConnectorError::NotImplemented("initiate_micro_deposits method".to_string()).into())
    }

    /// Verify micro-deposits with the amounts provided by the customer
    pub async fn verify_micro_deposits(
        &self,
        funding_source_url: &str,
        amount1: dwolla::DwollaAmount,
        amount2: dwolla::DwollaAmount,
        _connectors: &Connectors,
        auth_header: Vec<(String, masking::Maskable<String>)>,
    ) -> CustomResult<dwolla::DwollaVerificationResponse, errors::ConnectorError> {
        let verification_request = dwolla::DwollaVerificationRequest {
            amount1,
            amount2,
        };

        let url = format!("{}/micro-deposits", funding_source_url);
        
        let _request = RequestBuilder::new()
            .method(Method::Post)
            .url(&url)
            .attach_default_headers()
            .headers(auth_header)
            .set_body(RequestContent::Json(Box::new(verification_request)))
            .build();

        // This would need to be called with proper HTTP client
        Err(errors::ConnectorError::NotImplemented("verify_micro_deposits method".to_string()).into())
    }

    /// Get funding source status and verification details
    pub async fn get_funding_source_status(
        &self,
        funding_source_url: &str,
        _connectors: &Connectors,
        auth_header: Vec<(String, masking::Maskable<String>)>,
    ) -> CustomResult<dwolla::DwollaFundingSourceResponse, errors::ConnectorError> {
        let _request = RequestBuilder::new()
            .method(Method::Get)
            .url(funding_source_url)
            .attach_default_headers()
            .headers(auth_header)
            .build();

        // This would need to be called with proper HTTP client
        Err(errors::ConnectorError::NotImplemented("get_funding_source_status method".to_string()).into())
    }

    /// Check if a funding source requires verification
    pub fn requires_verification(funding_source: &dwolla::DwollaFundingSourceResponse) -> bool {
        matches!(funding_source.status.as_str(), "unverified")
    }

    /// Check if verification is in progress
    pub fn verification_in_progress(funding_source: &dwolla::DwollaFundingSourceResponse) -> bool {
        matches!(funding_source.status.as_str(), "pending")
    }

    /// Check if funding source is verified and ready for use
    pub fn is_verified(funding_source: &dwolla::DwollaFundingSourceResponse) -> bool {
        matches!(funding_source.status.as_str(), "verified")
    }
}
