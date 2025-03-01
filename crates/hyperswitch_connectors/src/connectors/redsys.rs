pub mod transformers;

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
        PreProcessing,
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData, PaymentsPreProcessingData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData, PaymentsPreProcessingRouterData,
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
    types::{self, Response, PaymentsPreProcessingType},
    webhooks,
};
use masking::{ExposeInterface, Mask};
use transformers as redsys;

use crate::{constants::headers, types::ResponseRouterData, utils as connector_utils};

#[derive(Clone)]
pub struct Redsys {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
}

impl Redsys {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector,
        }
    }
}

impl api::Payment for Redsys {}
impl api::PaymentSession for Redsys {}
impl api::ConnectorAccessToken for Redsys {}
impl api::MandateSetup for Redsys {}
impl api::PaymentAuthorize for Redsys {}
impl api::PaymentSync for Redsys {}
impl api::PaymentCapture for Redsys {}
impl api::PaymentVoid for Redsys {}
impl api::Refund for Redsys {}
impl api::RefundExecute for Redsys {}
impl api::RefundSync for Redsys {}
impl api::PaymentToken for Redsys {}
impl api::PaymentsPreProcessing for Redsys {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Redsys
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
        Ok(header)
    }
}

impl ConnectorCommon for Redsys {
    fn id(&self) -> &'static str {
        "redsys"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.redsys.base_url.as_ref()
    } 

    // fn get_auth_header(
    //     &self,
    //     auth_type: &ConnectorAuthType,
    // ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
    //     let auth = redsys::RedsysAuthType::try_from(auth_type)
    //         .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
    //     Ok(vec![(
    //         headers::AUTHORIZATION.to_string(),
    //         auth.api_key.expose().into_masked(),
    //     )])
    // }

    // fn build_error_response(
    //     &self,
    //     res: Response,
    //     event_builder: Option<&mut ConnectorEvent>,
    // ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
    //     let response: redsys::RedsysErrorResponse = res
    //         .response
    //         .parse_struct("RedsysErrorResponse")
    //         .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

    //     event_builder.map(|i| i.set_response_body(&response));
    //     router_env::logger::info!(connector_response=?response);

    //     Ok(ErrorResponse {
    //         status_code: res.status_code,
    //         code: response.code,
    //         message: response.message,
    //         reason: response.reason,
    //         attempt_status: None,
    //         connector_transaction_id: None,
    //     })
    // }
}

impl ConnectorValidation for Redsys {}
impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Redsys {}
impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Redsys {}
impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Redsys {}

impl ConnectorIntegration<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>
    for Redsys
{
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        _req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/iniciaPeticionREST",
            self.base_url(connectors)
        ))
    }
    fn get_request_body(
        &self,
        req: &PaymentsPreProcessingRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> { 
        let minor_amount=  req.request
            .minor_amount
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "minor_amount",
            })?;
    let currency =
        req.request
            .currency
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "currency",
            })?;

        let amount = connector_utils::convert_amount(
            self.amount_converter,
            minor_amount,
            currency,
        )?;
        let connector_router_data = redsys::RedsysRouterData::from((amount, req, currency));
        let auth = redsys::RedsysAuthType::try_from(&req.connector_auth_type)?;
        let connector_req_data = redsys::IniciaPeticionRequest::try_from((&connector_router_data, &auth))?;
        let connector_req = redsys::RedsysRequest::try_from((&connector_req_data, &auth))?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    fn build_request(
        &self,
        req: &PaymentsPreProcessingRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&PaymentsPreProcessingType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .set_body(PaymentsPreProcessingType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    // fn handle_response(
    //     &self,
    //     data: &PaymentsPreProcessingRouterData,
    //     event_builder: Option<&mut ConnectorEvent>,
    //     res: Response,
    // ) -> CustomResult<PaymentsPreProcessingRouterData, errors::ConnectorError> {
    //     let response: cybersource::CybersourcePreProcessingResponse = res
    //         .response
    //         .parse_struct("Cybersource AuthEnrollmentResponse")
    //         .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
    //     event_builder.map(|i| i.set_response_body(&response));
    //     router_env::logger::info!(connector_response=?response);
    //     RouterData::try_from(ResponseRouterData {
    //         response,
    //         data: data.clone(),
    //         http_code: res.status_code,
    //     })
    // }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}


impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Redsys {
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Redsys {
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Redsys {
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Redsys {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Redsys {
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Redsys {
}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Redsys
{}


#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Redsys {
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

impl ConnectorSpecifications for Redsys {}
