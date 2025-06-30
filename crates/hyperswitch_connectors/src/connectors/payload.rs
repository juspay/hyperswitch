pub mod transformers;

use std::sync::LazyLock;

use base64::Engine;
use common_enums::enums::{self, PaymentMethodType};
use common_utils::{
    consts::BASE64_ENGINE,
    errors::CustomResult,
    ext_traits::{BytesExt, ByteSliceExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse,
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
    router_response_types::{ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData, SupportedPaymentMethods, SupportedPaymentMethodsExt},
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
use api_models;
use masking::{ExposeInterface, Mask};
use transformers as payload;

use crate::{constants::headers, types::ResponseRouterData, utils};

static PAYLOAD_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::Manual,
        enums::CaptureMethod::ManualMultiple,
    ];

    let supported_card_network = vec![
        common_enums::CardNetwork::AmericanExpress,
        common_enums::CardNetwork::DinersClub,
        common_enums::CardNetwork::Discover,
        common_enums::CardNetwork::JCB,
        common_enums::CardNetwork::Maestro,
        common_enums::CardNetwork::Mastercard,
        common_enums::CardNetwork::Visa,
    ];

    let mut payload_supported_payment_methods = SupportedPaymentMethods::new();

    payload_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        PaymentMethodType::Credit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::NotSupported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    }
                }),
            ),
        },
    );

    payload_supported_payment_methods.add(
        enums::PaymentMethod::Card,
        PaymentMethodType::Debit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: Some(
                api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                    api_models::feature_matrix::CardSpecificFeatures {
                        three_ds: common_enums::FeatureStatus::NotSupported,
                        no_three_ds: common_enums::FeatureStatus::Supported,
                        supported_card_networks: supported_card_network.clone(),
                    }
                }),
            ),
        },
    );

    payload_supported_payment_methods
});

static PAYLOAD_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Payload",
    description: "Payload is a payment processing platform that provides secure, reliable payment infrastructure for businesses to accept card payments with comprehensive fraud protection and real-time transaction monitoring.",
    connector_type: enums::PaymentConnectorCategory::PaymentGateway,
};

#[derive(Clone)]
pub struct Payload {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Payload {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
    }

}

impl api::Payment for Payload {}
impl api::PaymentSession for Payload {}
impl api::ConnectorAccessToken for Payload {}
impl api::MandateSetup for Payload {}
impl api::PaymentAuthorize for Payload {}
impl api::PaymentSync for Payload {}
impl api::PaymentCapture for Payload {}
impl api::PaymentVoid for Payload {}
impl api::Refund for Payload {}
impl api::RefundExecute for Payload {}
impl api::RefundSync for Payload {}
impl api::PaymentToken for Payload {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Payload
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
        Ok(format!("{}/payment_methods", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = payload::PayloadTokenRequest::try_from(req)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::TokenizationType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::TokenizationType::get_headers(self, req, connectors)?)
                .set_body(types::TokenizationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>, errors::ConnectorError> {
        let response: payload::PayloadTokenResponse = res
            .response
            .parse_struct("Payload TokenResponse")
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

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Payload
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
        
        // Add enhanced idempotency key for payment operations to prevent duplicate transactions
        // Include timestamp to ensure uniqueness for legitimate separate transactions
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let idempotency_key = format!("{}-{}-{}", req.payment_id, req.attempt_id, timestamp);
        header.push((
            "X-Idempotency-Key".to_string(),
            idempotency_key.into(),
        ));
        
        Ok(header)
    }
}

impl ConnectorCommon for Payload {
    fn id(&self) -> &'static str {
        "payload"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        // Payload accepts amounts in minor units (cents)
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.payload.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = payload::PayloadAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let encoded_api_key = BASE64_ENGINE.encode(format!("{}:", auth.api_key.expose()));
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Basic {encoded_api_key}").into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: payload::PayloadErrorResponse = res
            .response
            .parse_struct("PayloadErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_type,
            message: response.error_description,
            reason: response.details.as_ref().map(|d| d.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        })
    }
}

impl ConnectorValidation for Payload {
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        payment_method: enums::PaymentMethod,
        pmt: Option<PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        
        // Use the feature matrix validation for consistent behavior
        let is_feature_supported = match self.get_supported_payment_methods() {
            Some(supported_payment_methods) => {
                // Get the payment method details from the feature matrix
                let payment_method_details = supported_payment_methods.get(&payment_method);
                
                match (payment_method_details, pmt) {
                    (Some(method_details), Some(payment_method_type)) => {
                        // Check if the specific payment method type is supported
                        method_details.get(&payment_method_type)
                            .map(|details| details.supported_capture_methods.contains(&capture_method))
                            .unwrap_or(false)
                    },
                    (Some(method_details), None) => {
                        // If no specific payment method type, check if any supported type supports this capture method
                        method_details.values()
                            .any(|details| details.supported_capture_methods.contains(&capture_method))
                    },
                    _ => false,
                }
            },
            None => {
                // Fallback to default behavior if no feature matrix is defined
                matches!(capture_method, enums::CaptureMethod::Automatic | enums::CaptureMethod::SequentialAutomatic)
            }
        };

        if is_feature_supported {
            Ok(())
        } else {
            Err(errors::ConnectorError::NotSupported {
                message: format!("{} is not supported", capture_method),
                connector: self.id(),
            }.into())
        }
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Payload {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Payload {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Payload {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Payload {
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
        Ok(format!("{}/transactions", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = payload::PayloadRouterData::from((amount, req));
        let connector_req = payload::PayloadPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
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
        let response: payload::PayloadPaymentsResponse = res
            .response
            .parse_struct("Payload PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        
        if let Some(event_builder) = event_builder {
            event_builder.set_response_body(&response);
        }
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Payload {
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
        Ok(format!(
            "{}/transactions/{}",
            self.base_url(connectors),
            req.request.connector_transaction_id.get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
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
        let response: payload::PayloadPaymentsResponse = res
            .response
            .parse_struct("payload PaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Payload {
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
        Ok(format!(
            "{}/transactions/{}",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = payload::PayloadRouterData::from((Default::default(), req));
        let connector_req = payload::PayloadCaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Put)
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
        let response: payload::PayloadPaymentsResponse = res
            .response
            .parse_struct("Payload PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Payload {
    fn get_headers(
        &self,
        req: &RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/transactions/{}",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = payload::PayloadRouterData::from((Default::default(), req));
        let connector_req = payload::PayloadCancelRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Put)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(types::PaymentsVoidType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RouterData<Void, PaymentsCancelData, PaymentsResponseData>, errors::ConnectorError> {
        let response: payload::PayloadPaymentsResponse = res
            .response
            .parse_struct("Payload PaymentsCancelResponse")
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

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Payload {
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
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/transactions", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data = payload::PayloadRouterData::from((refund_amount, req));
        let connector_req = payload::PayloadRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
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
        let response: payload::RefundResponse = res
            .response
            .parse_struct("payload RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Payload {
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
        let connector_refund_id = req.request.connector_refund_id.as_ref()
            .ok_or_else(|| errors::ConnectorError::MissingConnectorRefundID)?;
        Ok(format!(
            "{}/transactions/{}",
            self.base_url(connectors),
            connector_refund_id
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
        let response: payload::RefundResponse = res
            .response
            .parse_struct("payload RefundSyncResponse")
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
impl webhooks::IncomingWebhook for Payload {
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook_body: payload::PayloadWebhookBody = request
            .body
            .parse_struct("PayloadWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        if let Some(transaction) = &webhook_body.data.transaction {
            Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(
                    transaction.id.clone(),
                ),
            ))
        } else if let Some(refund) = &webhook_body.data.refund {
            Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::ConnectorRefundId(refund.id.clone()),
            ))
        } else {
            Err(errors::ConnectorError::WebhookResourceObjectNotFound)?
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook_body: payload::PayloadWebhookBody = request
            .body
            .parse_struct("PayloadWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        match webhook_body.event.as_str() {
            "transaction.authorized" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentAuthorizationSuccess),
            "transaction.processed" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess),
            "transaction.cancelled" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelled),
            "transaction.failed" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelled),
            "refund.processed" => Ok(api_models::webhooks::IncomingWebhookEvent::RefundSuccess),
            "refund.failed" => Ok(api_models::webhooks::IncomingWebhookEvent::RefundFailure),
            _ => Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported),
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook_body: payload::PayloadWebhookBody = request
            .body
            .parse_struct("PayloadWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        if let Some(transaction) = webhook_body.data.transaction {
            Ok(Box::new(transaction))
        } else if let Some(refund) = webhook_body.data.refund {
            Ok(Box::new(refund))
        } else {
            Err(errors::ConnectorError::WebhookResourceObjectNotFound)?
        }
    }

    fn get_webhook_api_response(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _webhook_flow_error: Option<webhooks::IncomingWebhookFlowError>,
    ) -> CustomResult<ApplicationResponse<serde_json::Value>, errors::ConnectorError> {
        Ok(ApplicationResponse::Json(serde_json::json!({
            "status": "received"
        })))
    }
}

impl ConnectorSpecifications for Payload {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&PAYLOAD_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*PAYLOAD_SUPPORTED_PAYMENT_METHODS)
    }
}
