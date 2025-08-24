pub mod transformers;
use std::sync::LazyLock;

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        AccessTokenAuth, Authorize, Capture, Execute, PSync, PaymentMethodToken, PostProcessing,
        RSync, Session, SetupMandate, Void,
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsPostProcessingData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{PaymentsAuthorizeRouterData, PaymentsSyncRouterData},
};
use hyperswitch_interfaces::{
    api::{
        ConnectorAccessToken, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration,
        ConnectorSpecifications, ConnectorValidation, CurrencyUnit, MandateSetup, Payment,
        PaymentAuthorize, PaymentCapture, PaymentSession, PaymentSync, PaymentToken, PaymentVoid,
        PaymentsPostProcessing, Refund, RefundExecute, RefundSync,
    },
    configs::Connectors,
    consts::NO_ERROR_CODE,
    errors::ConnectorError,
    events::connector_api_logs::ConnectorEvent,
    types::{PaymentsAuthorizeType, PaymentsPostProcessingType, PaymentsSyncType, Response},
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use masking::{Mask as _, Maskable};
use transformers as plaid;

use crate::{
    constants::headers,
    types::{PaymentsPostProcessingRouterData, ResponseRouterData},
    utils::convert_amount,
};

#[derive(Clone)]
pub struct Plaid {
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl Plaid {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &FloatMajorUnitForConnector,
        }
    }
}

impl Payment for Plaid {}
impl PaymentSession for Plaid {}
impl ConnectorAccessToken for Plaid {}
impl MandateSetup for Plaid {}
impl PaymentAuthorize for Plaid {}
impl PaymentSync for Plaid {}
impl PaymentCapture for Plaid {}
impl PaymentVoid for Plaid {}
impl Refund for Plaid {}
impl RefundExecute for Plaid {}
impl RefundSync for Plaid {}
impl PaymentToken for Plaid {}
impl PaymentsPostProcessing for Plaid {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Plaid
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Plaid
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut auth = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut auth);
        Ok(header)
    }
}

impl ConnectorCommon for Plaid {
    fn id(&self) -> &'static str {
        "plaid"
    }

    fn get_currency_unit(&self) -> CurrencyUnit {
        CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.plaid.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let auth = plaid::PlaidAuthType::try_from(auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        let client_id = auth.client_id.into_masked();
        let secret = auth.secret.into_masked();

        Ok(vec![
            ("PLAID-CLIENT-ID".to_string(), client_id),
            ("PLAID-SECRET".to_string(), secret),
        ])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: plaid::PlaidErrorResponse =
            res.response
                .parse_struct("PlaidErrorResponse")
                .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error_code.unwrap_or(NO_ERROR_CODE.to_string()),
            message: response.error_message,
            reason: response.display_message,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Plaid {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Plaid {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Plaid {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Plaid {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Plaid {
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
            "{}/payment_initiation/payment/create",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        let connector_router_data = plaid::PlaidRouterData::from((amount, req));
        let connector_req = plaid::PlaidPaymentsRequest::try_from(&connector_router_data)?;
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
        let response: plaid::PlaidPaymentsResponse = res
            .response
            .parse_struct("PlaidPaymentsResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Plaid {
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

    fn get_request_body(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = plaid::PlaidSyncRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn get_url(
        &self,
        _req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!(
            "{}/payment_initiation/payment/get",
            self.base_url(connectors)
        ))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsSyncType::get_headers(self, req, connectors)?)
                .set_body(PaymentsSyncType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, ConnectorError> {
        let response: plaid::PlaidSyncResponse = res
            .response
            .parse_struct("PlaidSyncResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PostProcessing, PaymentsPostProcessingData, PaymentsResponseData>
    for Plaid
{
    fn get_headers(
        &self,
        req: &PaymentsPostProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsPostProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}/link/token/create", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &PaymentsPostProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let connector_req = plaid::PlaidLinkTokenRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsPostProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsPostProcessingType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsPostProcessingType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(PaymentsPostProcessingType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsPostProcessingRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsPostProcessingRouterData, ConnectorError> {
        let response: plaid::PlaidLinkTokenResponse = res
            .response
            .parse_struct("PlaidLinkTokenResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Plaid {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Plaid {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Plaid {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Plaid {}

#[async_trait::async_trait]
impl IncomingWebhook for Plaid {
    fn get_webhook_object_reference_id(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, ConnectorError> {
        Err((ConnectorError::WebhooksNotImplemented).into())
    }

    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        Err((ConnectorError::WebhooksNotImplemented).into())
    }

    fn get_webhook_resource_object(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, ConnectorError> {
        Err((ConnectorError::WebhooksNotImplemented).into())
    }
}

static PLAID_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods = vec![
        enums::CaptureMethod::Automatic,
        enums::CaptureMethod::SequentialAutomatic,
    ];

    let mut plaid_supported_payment_methods = SupportedPaymentMethods::new();

    plaid_supported_payment_methods.add(
        enums::PaymentMethod::OpenBanking,
        enums::PaymentMethodType::OpenBankingPIS,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods,
            specific_features: None,
        },
    );

    plaid_supported_payment_methods
});

static PLAID_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Plaid",
    description: "Plaid is a data network that helps millions connect their financial accounts to apps like Venmo, SoFi, and Betterment. It powers tools used by Fortune 500 companies, major banks, and leading fintechs to enable easier, smarter financial lives.",
    connector_type: enums::HyperswitchConnectorCategory::AuthenticationProvider,
    integration_status: enums::ConnectorIntegrationStatus::Beta,
};

static PLAID_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Plaid {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&PLAID_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*PLAID_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&PLAID_SUPPORTED_WEBHOOK_FLOWS)
    }
}
