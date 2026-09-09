//! Global Payments RealEx (legacy Global Payments XML API).
//!
//! Payments for this connector are executed by the Unified Connector Service (UCS);
//! this module exists so the connector can be registered in the `Connector` enum,
//! resolved by `ConnectorData`, and validated on merchant connector account creation.
//! The request/response plumbing below is intentionally unimplemented.

pub mod transformers;

use std::sync::LazyLock;

use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
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
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
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
use hyperswitch_masking::{ExposeInterface, Mask, PeekInterface};
use transformers as globalpayments_realex;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self, PaymentsAuthorizeRequestData},
};

#[derive(Clone)]
pub struct GlobalpaymentsRealex {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl GlobalpaymentsRealex {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl api::Payment for GlobalpaymentsRealex {}
impl api::PaymentSession for GlobalpaymentsRealex {}
impl api::ConnectorAccessToken for GlobalpaymentsRealex {}
impl api::MandateSetup for GlobalpaymentsRealex {}
impl api::PaymentAuthorize for GlobalpaymentsRealex {}
impl api::PaymentSync for GlobalpaymentsRealex {}
impl api::PaymentCapture for GlobalpaymentsRealex {}
impl api::PaymentVoid for GlobalpaymentsRealex {}
impl api::Refund for GlobalpaymentsRealex {}
impl api::RefundExecute for GlobalpaymentsRealex {}
impl api::RefundSync for GlobalpaymentsRealex {}
impl api::PaymentToken for GlobalpaymentsRealex {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for GlobalpaymentsRealex
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for GlobalpaymentsRealex
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for GlobalpaymentsRealex {
    fn id(&self) -> &'static str {
        "globalpayments_realex"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.globalpayments_realex.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let auth = globalpayments_realex::GlobalpaymentsRealexAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.shared_secret.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: globalpayments_realex::GlobalpaymentsRealexErrorResponse = res
            .response
            .parse_struct("GlobalpaymentsRealexErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response.reason,
            attempt_status: None,
            connector_transaction_id: None,
            connector_response_reference_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for GlobalpaymentsRealex {
    fn validate_psync_reference_id(
        &self,
        _data: &PaymentsSyncData,
        _is_three_ds: bool,
        _status: enums::AttemptStatus,
        _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), errors::ConnectorError> {
        Ok(())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData>
    for GlobalpaymentsRealex
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for GlobalpaymentsRealex
{
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for GlobalpaymentsRealex
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for GlobalpaymentsRealex
{
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
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

        let connector_router_data =
            globalpayments_realex::GlobalpaymentsRealexRouterData::from((amount, req));
        let connector_req = globalpayments_realex::GlobalpaymentsRealexPaymentsRequest::try_from(
            &connector_router_data,
        )?;
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
        let response: globalpayments_realex::GlobalpaymentsRealexPaymentsResponse = res
            .response
            .parse_struct("GlobalpaymentsRealex PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for GlobalpaymentsRealex {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
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
        let response: globalpayments_realex::GlobalpaymentsRealexPaymentsResponse = res
            .response
            .parse_struct("globalpayments_realex PaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData>
    for GlobalpaymentsRealex
{
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
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
        let response: globalpayments_realex::GlobalpaymentsRealexPaymentsResponse = res
            .response
            .parse_struct("GlobalpaymentsRealex PaymentsCaptureResponse")
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for GlobalpaymentsRealex {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for GlobalpaymentsRealex {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
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
        let refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data =
            globalpayments_realex::GlobalpaymentsRealexRouterData::from((refund_amount, req));
        let connector_req = globalpayments_realex::GlobalpaymentsRealexRefundRequest::try_from(
            &connector_router_data,
        )?;
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
        let response: globalpayments_realex::RefundResponse = res
            .response
            .parse_struct("globalpayments_realex RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for GlobalpaymentsRealex {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
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
        let response: globalpayments_realex::RefundResponse = res
            .response
            .parse_struct("globalpayments_realex RefundSyncResponse")
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
impl webhooks::IncomingWebhook for GlobalpaymentsRealex {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

static GLOBALPAYMENTS_REALEX_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::Manual,
        ];

        let supported_card_networks = vec![
            common_enums::CardNetwork::Visa,
            common_enums::CardNetwork::Mastercard,
            common_enums::CardNetwork::AmericanExpress,
            common_enums::CardNetwork::JCB,
            common_enums::CardNetwork::DinersClub,
            common_enums::CardNetwork::Discover,
            common_enums::CardNetwork::UnionPay,
        ];

        let mut globalpayments_realex_supported_payment_methods = SupportedPaymentMethods::new();

        globalpayments_realex_supported_payment_methods.add(
            enums::PaymentMethod::Card,
            enums::PaymentMethodType::Credit,
            PaymentMethodDetails {
                // Mandates / MIT / recurring are deliberately out of scope: SetupMandate,
                // RepeatPayment and MandateRevoke are all in the connector's
                // `not_implemented` list on the UCS side.
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: Some(
                    api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                        api_models::feature_matrix::CardSpecificFeatures {
                            // 3DS2 is fully implemented (UCS PreAuthenticate / Authenticate /
                            // PostAuthenticate against the Global Payments 3DS2 JSON API) and
                            // HS routing into pre-authentication is verified, but not one 3DS
                            // request has ever completed against the gateway: the sandbox
                            // account is not provisioned for 3-D Secure
                            // (`403 That Account ID is not configured for 3D Secure`; the legacy
                            // XML `3ds-verifyenrolled` likewise returns `503 No mpi details
                            // setup`). Declared NotSupported until a 3DS payment is proven end
                            // to end; flip to Supported once Global Payments enable it.
                            three_ds: common_enums::FeatureStatus::NotSupported,
                            no_three_ds: common_enums::FeatureStatus::Supported,
                            supported_card_networks: supported_card_networks.clone(),
                        }
                    }),
                ),
            },
        );

        globalpayments_realex_supported_payment_methods.add(
            enums::PaymentMethod::Card,
            enums::PaymentMethodType::Debit,
            PaymentMethodDetails {
                // Mandates / MIT / recurring are deliberately out of scope: SetupMandate,
                // RepeatPayment and MandateRevoke are all in the connector's
                // `not_implemented` list on the UCS side.
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: Some(
                    api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                        api_models::feature_matrix::CardSpecificFeatures {
                            // 3DS2 is fully implemented (UCS PreAuthenticate / Authenticate /
                            // PostAuthenticate against the Global Payments 3DS2 JSON API) and
                            // HS routing into pre-authentication is verified, but not one 3DS
                            // request has ever completed against the gateway: the sandbox
                            // account is not provisioned for 3-D Secure
                            // (`403 That Account ID is not configured for 3D Secure`; the legacy
                            // XML `3ds-verifyenrolled` likewise returns `503 No mpi details
                            // setup`). Declared NotSupported until a 3DS payment is proven end
                            // to end; flip to Supported once Global Payments enable it.
                            three_ds: common_enums::FeatureStatus::NotSupported,
                            no_three_ds: common_enums::FeatureStatus::Supported,
                            supported_card_networks,
                        }
                    }),
                ),
            },
        );

        globalpayments_realex_supported_payment_methods
    });

static GLOBALPAYMENTS_REALEX_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Global Payments RealEx",
    description: "Global Payments RealEx is the legacy Global Payments XML (HPP/Remote) gateway, offering card authorisation, settlement, void, rebate and transaction query over the `epage-remote.cgi` endpoint.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Beta,
};

static GLOBALPAYMENTS_REALEX_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for GlobalpaymentsRealex {
    /// Check if pre-authentication flow is required.
    ///
    /// The 3DS2 authentication for this connector is driven by the Unified Connector Service
    /// over the dedicated `PreAuthenticate` / `Authenticate` / `PostAuthenticate` flows, so a
    /// merchant-initiated `three_ds` card authorization must first go through pre-authentication.
    /// When the caller already supplies `authentication_data` from an external authentication,
    /// the connector keeps its existing consume-only behaviour and goes straight to authorize.
    fn is_pre_authentication_flow_required(&self, current_flow: api::CurrentFlowInfo) -> bool {
        match current_flow {
            api::CurrentFlowInfo::Authorize {
                auth_type,
                request_data,
            } => {
                auth_type.is_three_ds()
                    && request_data.is_card()
                    && request_data.authentication_data.is_none()
            }
            // No alternate flow for complete authorize
            api::CurrentFlowInfo::CompleteAuthorize { .. } => false,
            api::CurrentFlowInfo::SetupMandate { .. } => false,
            api::CurrentFlowInfo::Psync { .. }
            | api::CurrentFlowInfo::UpdatePostConfirm { .. }
            | api::CurrentFlowInfo::ConnectorWebhookRegister { .. } => false,
        }
    }

    /// Check if authentication flow is required.
    ///
    /// The device data collection (3DS Method) notification URL carries a query string, so a
    /// return with non-empty `params` is the device profiling leg and must run `Authenticate`.
    fn is_authentication_flow_required(&self, current_flow: api::CurrentFlowInfo) -> bool {
        match current_flow {
            api::CurrentFlowInfo::Authorize { .. } => {
                // during authorize flow, there is no authentication call needed
                false
            }
            api::CurrentFlowInfo::CompleteAuthorize { request_data, .. } => {
                let redirection_params = request_data
                    .redirect_response
                    .as_ref()
                    .and_then(|redirect_response| redirect_response.params.as_ref());
                match redirection_params {
                    Some(param) if !param.peek().is_empty() => true,
                    Some(_) | None => false,
                }
            }
            api::CurrentFlowInfo::SetupMandate { .. } => false,
            api::CurrentFlowInfo::Psync { .. }
            | api::CurrentFlowInfo::UpdatePostConfirm { .. }
            | api::CurrentFlowInfo::ConnectorWebhookRegister { .. } => false,
        }
    }

    /// Check if post-authentication flow is required.
    ///
    /// The challenge notification URL carries no query string, so a return with empty or absent
    /// `params` is the ACS challenge result and must run `PostAuthenticate`.
    fn is_post_authentication_flow_required(&self, current_flow: api::CurrentFlowInfo) -> bool {
        match current_flow {
            api::CurrentFlowInfo::Authorize { .. } => {
                // during authorize flow, there is no post_authentication call needed
                false
            }
            api::CurrentFlowInfo::CompleteAuthorize { request_data, .. } => {
                let redirection_params = request_data
                    .redirect_response
                    .as_ref()
                    .and_then(|redirect_response| redirect_response.params.as_ref());
                match redirection_params {
                    Some(param) if !param.peek().is_empty() => false,
                    Some(_) | None => true,
                }
            }
            api::CurrentFlowInfo::SetupMandate { .. } => false,
            api::CurrentFlowInfo::Psync { .. }
            | api::CurrentFlowInfo::UpdatePostConfirm { .. }
            | api::CurrentFlowInfo::ConnectorWebhookRegister { .. } => false,
        }
    }

    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&GLOBALPAYMENTS_REALEX_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*GLOBALPAYMENTS_REALEX_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&GLOBALPAYMENTS_REALEX_SUPPORTED_WEBHOOK_FLOWS)
    }
}
