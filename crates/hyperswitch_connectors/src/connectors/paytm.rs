pub mod transformers;

use api_models::webhooks::{self, IncomingWebhookEvent};
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector},
};
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, RouterData},
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
    consts::NO_ERROR_CODE,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use lazy_static::lazy_static;
use masking::{ExposeInterface, Mask};
use router_env::logger;
use transformers as paytm;

use crate::{
    core::errors,
    routes::AppState,
    services,
    types::{self, domain, transformers::ForeignTryFrom},
};

pub struct Paytm {
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl Paytm {
    pub fn new() -> &'static Self {
        lazy_static! {
            static ref CONNECTOR: Paytm = Paytm {
                amount_converter: &FloatMajorUnitForConnector,
            };
        }
        &CONNECTOR
    }
}

impl api::Payment for Paytm {}
impl api::PaymentSession for Paytm {}
impl api::ConnectorAccessToken for Paytm {}
impl api::MandateSetup for Paytm {}
impl api::PaymentAuthorize for Paytm {}
impl api::PaymentSync for Paytm {}
impl api::PaymentCapture for Paytm {}
impl api::PaymentVoid for Paytm {}
impl api::Refund for Paytm {}
impl api::RefundExecute for Paytm {}
impl api::RefundSync for Paytm {}
impl api::PaymentToken for Paytm {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Paytm
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = paytm::PaytmAuthType::try_from(&connectors.paytm)?;
        Ok(vec![(
            "Authorization".to_string(),
            format!("Bearer {}", auth.api_key.expose()).into(),
        )])
    }
}

impl ConnectorCommon for Paytm {
    fn id(&self) -> &'static str {
        "paytm"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.paytm.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: paytm::PaytmErrorResponse =
            res.response
                .parse_struct("PaytmErrorResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response(Some(response.clone())));
        event_builder.map(|i| i.set_response_body(Some(response.clone())));

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error_code
                .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
            message: response.error_message,
            reason: None,
        })
    }
}

impl ConnectorValidation for Paytm {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Paytm {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Paytm {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Paytm {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Paytm {
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
        Ok(format!(
            "{}/v1/initiateTransaction",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = paytm::PaytmRouterData::try_from((&req.get_amount()?, req))?;
        let connector_req = paytm::PaytmPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&Self::get_url(self, req, connectors)?)
            .headers(Self::get_headers(self, req, connectors)?)
            .body(Self::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: paytm::PaytmPaymentsResponse = res
            .response
            .parse_struct("PaytmPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(Some(response.clone())));

        let router_data = RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })?;
        Ok(router_data)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Paytm {
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
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/v1/processTransaction",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = paytm::PaytmRouterData::try_from((&req.get_amount()?, req))?;
        let connector_req = paytm::PaytmSyncRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&Self::get_url(self, req, connectors)?)
            .headers(Self::get_headers(self, req, connectors)?)
            .body(Self::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: paytm::PaytmSyncResponse = res
            .response
            .parse_struct("PaytmSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(Some(response.clone())));

        let router_data = RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })?;
        Ok(router_data)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Paytm {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Paytm {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Paytm {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Paytm {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Paytm
{
}

impl IncomingWebhook for Paytm {
    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<webhooks::ObjectReferenceId, errors::ConnectorError> {
        let payload: paytm::PaytmWebhookPayload = request
            .body
            .parse_struct("PaytmWebhookPayload")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(webhooks::ObjectReferenceId::PaymentId(
            payload.payment.entity.id,
        ))
    }

    async fn verify_webhook_source(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: common_utils::crypto::Encryptable<
            masking::Secret<serde_json::Value>,
        >,
        _connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        Ok(true)
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        let payload: paytm::PaytmWebhookPayload = request
            .body
            .parse_struct("PaytmWebhookPayload")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(IncomingWebhookEvent::PaymentIntent)
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let payload: paytm::PaytmWebhookPayload = request
            .body
            .parse_struct("PaytmWebhookPayload")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(Box::new(payload))
    }
}

impl ConnectorSpecifications for Paytm {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&ConnectorInfo {
            name: "Paytm",
            description: "Paytm Payment Gateway",
            logo: None,
            supported_payment_methods: Some(&SupportedPaymentMethods::default()),
            supported_webhook_flows: Some(&[enums::EventClass::PaymentIntent]),
        })
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&SupportedPaymentMethods::default())
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&[enums::EventClass::PaymentIntent])
    }
}
