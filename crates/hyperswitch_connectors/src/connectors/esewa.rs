pub mod transformers;

use std::sync::LazyLock;

use common_enums::{enums, CallConnectorAction, PaymentAction};
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
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
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorRedirectResponse,
        ConnectorSpecifications, ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use hyperswitch_masking::Maskable;
use transformers as esewa;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self, PaymentsSyncRequestData},
};

const CONNECTOR_NAME: &str = "eSewa";

#[derive(Clone)]
pub struct Esewa {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Esewa {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }

    fn not_supported(flow: &str) -> error_stack::Report<errors::ConnectorError> {
        report!(errors::ConnectorError::NotSupported {
            message: flow.to_string(),
            connector: CONNECTOR_NAME.into(),
        })
    }
}

impl api::Payment for Esewa {}
impl api::PaymentSession for Esewa {}
impl api::ConnectorAccessToken for Esewa {}
impl api::MandateSetup for Esewa {}
impl api::PaymentAuthorize for Esewa {}
impl api::PaymentSync for Esewa {}
impl api::PaymentCapture for Esewa {}
impl api::PaymentVoid for Esewa {}
impl api::Refund for Esewa {}
impl api::RefundExecute for Esewa {}
impl api::RefundSync for Esewa {}
impl api::PaymentToken for Esewa {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Esewa
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Esewa
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        // eSewa authenticates by the request signature, not by a header.
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )])
    }
}

impl ConnectorCommon for Esewa {
    fn id(&self) -> &'static str {
        "esewa"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.esewa.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        Ok(Vec::new())
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: esewa::EsewaErrorResponse =
            res.response
                .parse_struct("EsewaErrorResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(response.into_error_response(res.status_code, None))
    }
}

impl ConnectorValidation for Esewa {}

impl ConnectorRedirectResponse for Esewa {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        action: PaymentAction,
    ) -> CustomResult<CallConnectorAction, errors::ConnectorError> {
        // The `data` param eSewa appends on return is not trusted; always confirm with the
        // status API.
        match action {
            PaymentAction::PSync
            | PaymentAction::CompleteAuthorize
            | PaymentAction::PaymentAuthenticateCompleteAuthorize => {
                Ok(CallConnectorAction::Trigger)
            }
        }
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Esewa {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Esewa {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Esewa {
    fn build_request(
        &self,
        _req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(Self::not_supported("Setup mandate"))
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Esewa {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
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
            "{}api/epay/main/v2/form",
            self.base_url(connectors)
        ))
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

        let connector_router_data = esewa::EsewaRouterData::from((amount, req));
        let connector_req = esewa::EsewaPaymentsRequest::try_from(&connector_router_data)?;
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
        // A valid form gets a 302 to eSewa's payment page; there is no body to parse.
        let location = res
            .headers
            .as_ref()
            .and_then(|headers| utils::get_http_header("Location", headers).ok())
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)
            .attach_printable("eSewa form response has no Location header")?;
        let response = esewa::EsewaRedirectResponse {
            payment_page_url: url::Url::parse(location)
                .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
        };
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Esewa {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
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
        let auth = esewa::EsewaAuthType::try_from(&req.connector_auth_type)?;
        let query = esewa::EsewaSyncQuery {
            product_code: auth.product_code,
            total_amount: utils::convert_amount(
                self.amount_converter,
                req.request.amount,
                req.request.currency,
            )?,
            transaction_uuid: req.request.get_connector_transaction_id()?,
        };
        let query = serde_urlencoded::to_string(&query)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(format!(
            "{}api/epay/transaction/status/?{query}",
            self.base_url(connectors)
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
        let response: esewa::EsewaSyncResponse = res
            .response
            .parse_struct("Esewa PaymentsSyncResponse")
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

// ePay v2 has no capture, void or refund API: payments are captured on completion, and
// refunds are made from the eSewa merchant portal.
impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Esewa {
    fn build_request(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(Self::not_supported("Capture"))
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Esewa {
    fn build_request(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(Self::not_supported("Void"))
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Esewa {
    fn build_request(
        &self,
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(Self::not_supported("Refunds"))
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Esewa {
    fn build_request(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(Self::not_supported("Refund sync"))
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Esewa {
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

static ESEWA_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let mut esewa_supported_payment_methods = SupportedPaymentMethods::new();

    esewa_supported_payment_methods.add(
        enums::PaymentMethod::Wallet,
        enums::PaymentMethodType::Esewa,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods: vec![
                enums::CaptureMethod::Automatic,
                enums::CaptureMethod::SequentialAutomatic,
            ],
            specific_features: None,
        },
    );

    esewa_supported_payment_methods
});

static ESEWA_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "eSewa",
    description: "eSewa is a Nepali digital wallet. Customers pay on eSewa's hosted page and the payment is confirmed with eSewa's status API.",
    connector_type: enums::HyperswitchConnectorCategory::AlternativePaymentMethod,
    integration_status: enums::ConnectorIntegrationStatus::Alpha,
};

static ESEWA_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Esewa {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&ESEWA_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*ESEWA_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&ESEWA_SUPPORTED_WEBHOOK_FLOWS)
    }
}
