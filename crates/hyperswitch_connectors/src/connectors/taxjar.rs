pub mod transformers;

use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, FloatMajorUnit, FloatMajorUnitForConnector, MinorUnit},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{
            Authorize, CalculateTax, Capture, PSync, PaymentMethodToken, Session, SetupMandate,
            Void,
        },
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        PaymentsTaxCalculationData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        PaymentsResponseData, RefundsResponseData, TaxCalculationResponseData,
    },
    types::PaymentsTaxCalculationRouterData,
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
use masking::{Mask, PeekInterface};
use transformers as taxjar;

use crate::{constants::headers, types::ResponseRouterData, utils};

#[derive(Clone)]
pub struct Taxjar {
    amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl Taxjar {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &FloatMajorUnitForConnector,
        }
    }
}

impl api::Payment for Taxjar {}
impl api::PaymentSession for Taxjar {}
impl api::ConnectorAccessToken for Taxjar {}
impl api::MandateSetup for Taxjar {}
impl api::PaymentAuthorize for Taxjar {}
impl api::PaymentSync for Taxjar {}
impl api::PaymentCapture for Taxjar {}
impl api::PaymentVoid for Taxjar {}
impl api::Refund for Taxjar {}
impl api::RefundExecute for Taxjar {}
impl api::RefundSync for Taxjar {}
impl api::PaymentToken for Taxjar {}
impl api::TaxCalculation for Taxjar {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Taxjar
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Taxjar
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

impl ConnectorCommon for Taxjar {
    fn id(&self) -> &'static str {
        "taxjar"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.taxjar.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = taxjar::TaxjarAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", auth.api_key.peek()).into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: taxjar::TaxjarErrorResponse = res
            .response
            .parse_struct("TaxjarErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error.clone(),
            message: response.detail.clone(),
            reason: Some(response.detail),
            attempt_status: None,
            connector_transaction_id: None,
            issuer_error_code: None,
            issuer_error_message: None,
        })
    }
}

impl ConnectorValidation for Taxjar {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Taxjar {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Taxjar {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Taxjar {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Taxjar {}

impl ConnectorIntegration<CalculateTax, PaymentsTaxCalculationData, TaxCalculationResponseData>
    for Taxjar
{
    fn get_headers(
        &self,
        req: &PaymentsTaxCalculationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsTaxCalculationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}taxes", self.base_url(connectors)))
    }
    fn get_request_body(
        &self,
        req: &PaymentsTaxCalculationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.amount,
            req.request.currency,
        )?;

        let shipping = utils::convert_amount(
            self.amount_converter,
            req.request.shipping_cost.unwrap_or(MinorUnit::zero()),
            req.request.currency,
        )?;

        let order_amount = utils::convert_amount(
            self.amount_converter,
            req.request
                .order_details
                .as_ref()
                .map(|details| details.iter().map(|item| item.amount).sum())
                .unwrap_or(MinorUnit::zero()),
            req.request.currency,
        )?;

        let connector_router_data =
            taxjar::TaxjarRouterData::from((amount, order_amount, shipping, req));
        let connector_req = taxjar::TaxjarPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &PaymentsTaxCalculationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsTaxCalculationType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsTaxCalculationType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsTaxCalculationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsTaxCalculationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsTaxCalculationRouterData, errors::ConnectorError> {
        let response: taxjar::TaxjarPaymentsResponse = res
            .response
            .parse_struct("Taxjar PaymentsResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Taxjar {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Taxjar {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Taxjar {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Taxjar {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Taxjar {}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Taxjar {
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

impl ConnectorSpecifications for Taxjar {}
