pub mod transformers;

use std::sync::LazyLock;

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use common_enums::enums;
use common_utils::{
    crypto,
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
        CreateConnectorCustomer,
    },
    router_request_types::{
        AccessTokenRequestData, ConnectorCustomerData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        ConnectorCustomerRouterData, PaymentsAuthorizeRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData, SetupMandateRouterData, TokenizationRouterData,
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
    types::{self, PaymentsSyncType, Response},
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails, WebhookContext},
};
use hyperswitch_masking::{Mask, PeekInterface};
use transformers as gocardless;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self},
};

#[derive(Clone)]
pub struct Gocardless {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Gocardless {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl api::Payment for Gocardless {}
impl api::PaymentSession for Gocardless {}
impl api::ConnectorAccessToken for Gocardless {}
impl api::MandateSetup for Gocardless {}
impl api::PaymentAuthorize for Gocardless {}
impl api::PaymentSync for Gocardless {}
impl api::PaymentCapture for Gocardless {}
impl api::PaymentVoid for Gocardless {}
impl api::Refund for Gocardless {}
impl api::RefundExecute for Gocardless {}
impl api::RefundSync for Gocardless {}
impl api::PaymentToken for Gocardless {}
impl api::ConnectorCustomer for Gocardless {}

const GOCARDLESS_VERSION: &str = "2015-07-06";
const GOCARDLESS_VERSION_HEADER: &str = "GoCardless-Version";

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Gocardless
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                GOCARDLESS_VERSION_HEADER.to_string(),
                GOCARDLESS_VERSION.to_string().into(),
            ),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Gocardless {
    fn id(&self) -> &'static str {
        "gocardless"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.gocardless.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let auth = gocardless::GocardlessAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", auth.access_token.peek()).into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: gocardless::GocardlessErrorResponse = res
            .response
            .parse_struct("GocardlessErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let error_iter = response.error.errors.iter();
        let mut error_reason: Vec<String> = Vec::new();
        for error in error_iter {
            let reason = error.field.clone().map_or(error.message.clone(), |field| {
                format!("{} {}", field, error.message)
            });
            error_reason.push(reason)
        }
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error.code.to_string(),
            message: response.error.error_type,
            reason: Some(error_reason.join("; ")),
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

impl ConnectorIntegration<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>
    for Gocardless
{
    fn get_headers(
        &self,
        req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/customers", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &ConnectorCustomerRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = gocardless::GocardlessCustomerRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &ConnectorCustomerRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::ConnectorCustomerType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::ConnectorCustomerType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::ConnectorCustomerType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &ConnectorCustomerRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        RouterData<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>,
        errors::ConnectorError,
    >
    where
        CreateConnectorCustomer: Clone,
        ConnectorCustomerData: Clone,
        PaymentsResponseData: Clone,
    {
        let response: gocardless::GocardlessCustomerResponse = res
            .response
            .parse_struct("GocardlessCustomerResponse")
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

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Gocardless
{
    fn get_headers(
        &self,
        req: &TokenizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &TokenizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/customer_bank_accounts",
            self.base_url(connectors),
        ))
    }

    fn get_request_body(
        &self,
        req: &TokenizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = gocardless::GocardlessBankAccountRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &TokenizationRouterData,
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
        data: &TokenizationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<TokenizationRouterData, errors::ConnectorError>
    where
        PaymentMethodToken: Clone,
        PaymentMethodTokenizationData: Clone,
        PaymentsResponseData: Clone,
    {
        let response: gocardless::GocardlessBankAccountResponse = res
            .response
            .parse_struct("GocardlessBankAccountResponse")
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

impl ConnectorValidation for Gocardless {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Gocardless {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Gocardless {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Gocardless
{
    fn get_headers(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/mandates", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &SetupMandateRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = gocardless::GocardlessMandateRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &SetupMandateRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        // Preprocessing flow is to create mandate, which should to be called only in case of First mandate
        if req.request.setup_mandate_details.is_some() {
            Ok(Some(
                RequestBuilder::new()
                    .method(Method::Post)
                    .url(&types::SetupMandateType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::SetupMandateType::get_headers(self, req, connectors)?)
                    .set_body(types::SetupMandateType::get_request_body(
                        self, req, connectors,
                    )?)
                    .build(),
            ))
        } else {
            Ok(None)
        }
    }

    fn handle_response(
        &self,
        data: &SetupMandateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<SetupMandateRouterData, errors::ConnectorError> {
        let response: gocardless::GocardlessMandateResponse = res
            .response
            .parse_struct("GocardlessMandateResponse")
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

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Gocardless {
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
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/payments", self.base_url(connectors)))
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

        let connector_router_data = gocardless::GocardlessRouterData::from((amount, req));
        let connector_req =
            gocardless::GocardlessPaymentsRequest::try_from(&connector_router_data)?;
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
        let response: gocardless::GocardlessPaymentsResponse = res
            .response
            .parse_struct("GocardlessPaymentsResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Gocardless {
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
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/payments/{}",
            self.base_url(connectors),
            req.request
                .connector_transaction_id
                .get_connector_transaction_id()
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
                .url(&PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: gocardless::GocardlessPaymentsResponse = res
            .response
            .parse_struct("GocardlessPaymentsResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Gocardless {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Gocardless {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Gocardless {
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
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/refunds", self.base_url(connectors)))
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

        let connector_router_data = gocardless::GocardlessRouterData::from((refund_amount, req));

        let connector_req = gocardless::GocardlessRefundRequest::try_from(&connector_router_data)?;
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
        let response: gocardless::RefundResponse = res
            .response
            .parse_struct("gocardless RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Gocardless {
    fn build_request(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(None)
    }
}

#[async_trait::async_trait]
impl IncomingWebhook for Gocardless {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let signature = request
            .headers
            .get("Webhook-Signature")
            .map(|header_value| {
                header_value
                    .to_str()
                    .map(String::from)
                    .map_err(|_| errors::ConnectorError::WebhookSignatureNotFound)
            })
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)??;

        hex::decode(signature).change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(format!("{}", String::from_utf8_lossy(request.body))
            .as_bytes()
            .to_vec())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, errors::ConnectorError> {
        let details: gocardless::GocardlessWebhookEvent = request
            .body
            .parse_struct("GocardlessWebhookEvent")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let first_event = details
            .events
            .first()
            .ok_or_else(|| errors::ConnectorError::WebhookReferenceIdNotFound)?;
        let reference_id = match first_event {
            transformers::WebhookEvent::Payments(event) => {
                let payment_id = api_models::payments::PaymentIdType::ConnectorTransactionId(
                    event.links.payment.to_owned(),
                );
                ObjectReferenceId::PaymentId(payment_id)
            }
            transformers::WebhookEvent::Refunds(event) => {
                let refund_id = api_models::webhooks::RefundIdType::ConnectorRefundId(
                    event.links.refund.to_owned(),
                );
                ObjectReferenceId::RefundId(refund_id)
            }
            transformers::WebhookEvent::Mandates(event) => {
                let mandate_id = api_models::webhooks::MandateIdType::ConnectorMandateId(
                    event.links.mandate.to_owned(),
                );
                ObjectReferenceId::MandateId(mandate_id)
            }
        };
        Ok(reference_id)
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _context: Option<&WebhookContext>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        let details: gocardless::GocardlessWebhookEvent = request
            .body
            .parse_struct("GocardlessWebhookEvent")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let first_event = details
            .events
            .first()
            .ok_or_else(|| errors::ConnectorError::WebhookReferenceIdNotFound)?;
        let event_type = match first_event {
            transformers::WebhookEvent::Payments(event) => match event.action {
                transformers::PaymentsAction::Created
                | transformers::PaymentsAction::Submitted
                | transformers::PaymentsAction::CustomerApprovalGranted => {
                    IncomingWebhookEvent::PaymentIntentProcessing
                }
                transformers::PaymentsAction::CustomerApprovalDenied
                | transformers::PaymentsAction::Failed
                | transformers::PaymentsAction::Cancelled
                | transformers::PaymentsAction::LateFailureSettled => {
                    IncomingWebhookEvent::PaymentIntentFailure
                }
                transformers::PaymentsAction::Confirmed | transformers::PaymentsAction::PaidOut => {
                    IncomingWebhookEvent::PaymentIntentSuccess
                }
                transformers::PaymentsAction::SurchargeFeeDebited
                | transformers::PaymentsAction::ResubmissionRequested => {
                    IncomingWebhookEvent::EventNotSupported
                }
            },
            transformers::WebhookEvent::Refunds(event) => match event.action {
                transformers::RefundsAction::Failed => IncomingWebhookEvent::RefundFailure,
                transformers::RefundsAction::Paid => IncomingWebhookEvent::RefundSuccess,
                transformers::RefundsAction::RefundSettled
                | transformers::RefundsAction::FundsReturned
                | transformers::RefundsAction::Created => IncomingWebhookEvent::EventNotSupported,
            },
            transformers::WebhookEvent::Mandates(event) => match event.action {
                transformers::MandatesAction::Active | transformers::MandatesAction::Reinstated => {
                    IncomingWebhookEvent::MandateActive
                }
                transformers::MandatesAction::Expired
                | transformers::MandatesAction::Cancelled
                | transformers::MandatesAction::Failed
                | transformers::MandatesAction::Consumed => IncomingWebhookEvent::MandateRevoked,
                transformers::MandatesAction::Created
                | transformers::MandatesAction::CustomerApprovalGranted
                | transformers::MandatesAction::CustomerApprovalSkipped
                | transformers::MandatesAction::Transferred
                | transformers::MandatesAction::Submitted
                | transformers::MandatesAction::ResubmissionRequested
                | transformers::MandatesAction::Replaced
                | transformers::MandatesAction::Blocked => IncomingWebhookEvent::EventNotSupported,
            },
        };
        Ok(event_type)
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        let details: gocardless::GocardlessWebhookEvent = request
            .body
            .parse_struct("GocardlessWebhookEvent")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let first_event = details
            .events
            .first()
            .ok_or_else(|| errors::ConnectorError::WebhookReferenceIdNotFound)?
            .clone();
        match &first_event {
            transformers::WebhookEvent::Payments(event) => Ok(Box::new(
                gocardless::GocardlessPaymentsResponse::try_from(event)?,
            )),
            transformers::WebhookEvent::Refunds(_) | transformers::WebhookEvent::Mandates(_) => {
                Ok(Box::new(first_event))
            }
        }
    }
}

static GOCARDLESS_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::SequentialAutomatic,
        ];

        let mut gocardless_supported_payment_methods = SupportedPaymentMethods::new();

        gocardless_supported_payment_methods.add(
            enums::PaymentMethod::BankDebit,
            enums::PaymentMethodType::Ach,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::Supported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        gocardless_supported_payment_methods.add(
            enums::PaymentMethod::BankDebit,
            enums::PaymentMethodType::Becs,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::Supported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        gocardless_supported_payment_methods.add(
            enums::PaymentMethod::BankDebit,
            enums::PaymentMethodType::Sepa,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::Supported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        gocardless_supported_payment_methods
    });

static GOCARDLESS_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "GoCardless",
    description: "GoCardless is a fintech company that specialises in bank payments including recurring payments.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Sandbox,
};

static GOCARDLESS_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 3] = [
    enums::EventClass::Payments,
    enums::EventClass::Refunds,
    enums::EventClass::Mandates,
];

impl ConnectorSpecifications for Gocardless {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&GOCARDLESS_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*GOCARDLESS_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&GOCARDLESS_SUPPORTED_WEBHOOK_FLOWS)
    }

    fn should_call_connector_customer(
        &self,
        #[cfg(feature = "v1")]
        _payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> api::ConnectorCustomerAction {
        api::ConnectorCustomerAction::CallConnectorCustomer
    }
}

#[cfg(test)]
mod tests {
    use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId, RefundIdType};
    use hyperswitch_interfaces::webhooks::{IncomingWebhook, IncomingWebhookRequestDetails};

    use super::Gocardless;

    fn webhook_body(resource_type: &str, action: &str, link_key: &str) -> Vec<u8> {
        serde_json::json!({
            "events": [{
                "id": "EV000",
                "resource_type": resource_type,
                "action": action,
                "links": { link_key: "ID000" }
            }]
        })
        .to_string()
        .into_bytes()
    }

    fn event_type_for(resource_type: &str, action: &str, link_key: &str) -> IncomingWebhookEvent {
        let body = webhook_body(resource_type, action, link_key);
        let headers = actix_web::http::header::HeaderMap::new();
        let request = IncomingWebhookRequestDetails {
            method: http::Method::POST,
            uri: http::Uri::from_static("/webhooks"),
            headers: &headers,
            body: &body,
            query_params: String::new(),
        };
        Gocardless::new()
            .get_webhook_event_type(&request, None)
            .expect("webhook event type should be derived")
    }

    #[test]
    fn refund_actions_shared_with_payments_map_to_refund_events() {
        assert_eq!(
            event_type_for("refunds", "failed", "refund"),
            IncomingWebhookEvent::RefundFailure
        );
        assert_eq!(
            event_type_for("refunds", "created", "refund"),
            IncomingWebhookEvent::EventNotSupported
        );
        assert_eq!(
            event_type_for("refunds", "paid", "refund"),
            IncomingWebhookEvent::RefundSuccess
        );
    }

    #[test]
    fn mandate_actions_shared_with_payments_map_to_mandate_events() {
        assert_eq!(
            event_type_for("mandates", "cancelled", "mandate"),
            IncomingWebhookEvent::MandateRevoked
        );
        assert_eq!(
            event_type_for("mandates", "failed", "mandate"),
            IncomingWebhookEvent::MandateRevoked
        );
        for action in ["created", "submitted", "customer_approval_granted"] {
            assert_eq!(
                event_type_for("mandates", action, "mandate"),
                IncomingWebhookEvent::EventNotSupported,
                "mandate action `{action}`"
            );
        }
    }

    #[test]
    fn payment_actions_map_to_payment_events() {
        assert_eq!(
            event_type_for("payments", "failed", "payment"),
            IncomingWebhookEvent::PaymentIntentFailure
        );
        assert_eq!(
            event_type_for("payments", "confirmed", "payment"),
            IncomingWebhookEvent::PaymentIntentSuccess
        );
        assert_eq!(
            event_type_for("payments", "resubmission_requested", "payment"),
            IncomingWebhookEvent::EventNotSupported
        );
    }

    #[test]
    fn refund_webhook_references_the_refund() {
        let body = webhook_body("refunds", "failed", "refund");
        let headers = actix_web::http::header::HeaderMap::new();
        let request = IncomingWebhookRequestDetails {
            method: http::Method::POST,
            uri: http::Uri::from_static("/webhooks"),
            headers: &headers,
            body: &body,
            query_params: String::new(),
        };

        let reference_id = Gocardless::new()
            .get_webhook_object_reference_id(&request)
            .expect("webhook reference id should be derived");

        assert!(matches!(
            reference_id,
            ObjectReferenceId::RefundId(RefundIdType::ConnectorRefundId(ref id)) if id == "ID000"
        ));
    }
}
