pub mod transformers;

use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        Checkout, Fulfillment, RecordReturn, Sale, Transaction,
    },
    router_request_types::{
        fraud_check::{
            FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
            FraudCheckSaleData, FraudCheckTransactionData,
        },
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        fraud_check::FraudCheckResponseData, PaymentsResponseData, RefundsResponseData,
    },
};
use hyperswitch_interfaces::{
    api::{
        ConnectorAccessToken, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration,
        ConnectorSpecifications, ConnectorValidation, FraudCheck, FraudCheckCheckout,
        FraudCheckFulfillment, FraudCheckRecordReturn, FraudCheckSale, FraudCheckTransaction,
        MandateSetup, Payment, PaymentAuthorize, PaymentCapture, PaymentSession, PaymentSync,
        PaymentToken, PaymentVoid, Refund, RefundExecute, RefundSync,
    },
    configs::Connectors,
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors::ConnectorError,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks,
};
use hyperswitch_masking::{ExposeInterface, Mask, Maskable};
use transformers as sanlam_payshield;

use crate::{
    constants::headers,
    types::{FrmCheckoutRouterData, FrmCheckoutType, ResponseRouterData},
};

#[derive(Clone)]
pub struct SanlamPayshield;

impl SanlamPayshield {
    pub fn new() -> &'static Self {
        &Self
    }
}

impl<Flow, RequestData, ResponseData> ConnectorCommonExt<Flow, RequestData, ResponseData>
    for SanlamPayshield
where
    Self: ConnectorIntegration<Flow, RequestData, ResponseData>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, RequestData, ResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let mut headers = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        headers.append(&mut self.get_auth_header(&req.connector_auth_type)?);
        Ok(headers)
    }
}

impl ConnectorCommon for SanlamPayshield {
    fn id(&self) -> &'static str {
        "sanlam_payshield"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.sanlam_payshield.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let auth = sanlam_payshield::SanlamPayshieldAuthType::try_from(auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::X_API_KEY.to_string(),
            auth.api_key.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: sanlam_payshield::SanlamPayshieldErrorResponse = res
            .response
            .parse_struct("SanlamPayshieldErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error_code
                .map_or(NO_ERROR_CODE.to_string(), |v| v.to_string()),
            message: response
                .error_message
                .clone()
                .or(response.message.clone())
                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
            reason: response.reason(),
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

impl ConnectorIntegration<Checkout, FraudCheckCheckoutData, FraudCheckResponseData>
    for SanlamPayshield
{
    fn get_headers(
        &self,
        req: &FrmCheckoutRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &FrmCheckoutRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let merchant_id = _req.merchant_id.get_string_repr();
        Ok(format!(
            "{}/payshield/v1/check/{}",
            self.base_url(connectors).to_owned(),
            merchant_id
        ))
    }

    fn get_request_body(
        &self,
        req: &FrmCheckoutRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        Ok(RequestContent::Json(Box::new(
            sanlam_payshield::SanlamPayshieldCheckoutRequest::try_from(req)?,
        )))
    }

    fn build_request(
        &self,
        req: &FrmCheckoutRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&FrmCheckoutType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(FrmCheckoutType::get_headers(self, req, connectors)?)
                .set_body(FrmCheckoutType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &FrmCheckoutRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<FrmCheckoutRouterData, ConnectorError> {
        let response: sanlam_payshield::SanlamPayshieldCheckoutResponse = res
            .response
            .parse_struct("SanlamPayshieldCheckoutResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|event| event.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        FrmCheckoutRouterData::try_from(ResponseRouterData {
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

impl ConnectorIntegration<Sale, FraudCheckSaleData, FraudCheckResponseData> for SanlamPayshield {}
impl ConnectorIntegration<Transaction, FraudCheckTransactionData, FraudCheckResponseData>
    for SanlamPayshield
{
}
impl ConnectorIntegration<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>
    for SanlamPayshield
{
}
impl ConnectorIntegration<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>
    for SanlamPayshield
{
}

impl FraudCheck for SanlamPayshield {}
impl FraudCheckCheckout for SanlamPayshield {}
impl FraudCheckSale for SanlamPayshield {}
impl FraudCheckTransaction for SanlamPayshield {}
impl FraudCheckFulfillment for SanlamPayshield {}
impl FraudCheckRecordReturn for SanlamPayshield {}

impl Payment for SanlamPayshield {}
impl PaymentSession for SanlamPayshield {}
impl ConnectorAccessToken for SanlamPayshield {}
impl MandateSetup for SanlamPayshield {}
impl PaymentAuthorize for SanlamPayshield {}
impl PaymentSync for SanlamPayshield {}
impl PaymentCapture for SanlamPayshield {}
impl PaymentVoid for SanlamPayshield {}
impl Refund for SanlamPayshield {}
impl RefundExecute for SanlamPayshield {}
impl RefundSync for SanlamPayshield {}
impl PaymentToken for SanlamPayshield {}
impl ConnectorValidation for SanlamPayshield {}
impl ConnectorSpecifications for SanlamPayshield {}
#[async_trait::async_trait]
impl webhooks::IncomingWebhook for SanlamPayshield {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, ConnectorError> {
        Err(ConnectorError::WebhooksNotImplemented.into())
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, ConnectorError> {
        Err(ConnectorError::WebhooksNotImplemented.into())
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, ConnectorError> {
        Err(ConnectorError::WebhooksNotImplemented.into())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for SanlamPayshield {}
impl
    ConnectorIntegration<
        AccessTokenAuth,
        AccessTokenRequestData,
        hyperswitch_domain_models::router_data::AccessToken,
    > for SanlamPayshield
{
}
impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for SanlamPayshield
{
}
impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for SanlamPayshield
{
}
impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for SanlamPayshield {}
impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for SanlamPayshield {}
impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for SanlamPayshield {}
impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for SanlamPayshield
{
}
impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for SanlamPayshield {}
impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for SanlamPayshield {}
