pub mod transformers;

use std::fmt::Debug;

use api_models::enums::enums;
use common_utils::{crypto, ext_traits::ByteSliceExt, request::RequestContent};
use error_stack::ResultExt;
use masking::PeekInterface;
use transformers as gocardless;

use crate::{
    configs::settings,
    connector::utils::{self as connector_utils, PaymentMethodDataType},
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::BytesExt,
};

#[derive(Debug, Clone)]
pub struct Gocardless;

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
impl api::PaymentsPreProcessing for Gocardless {}

const GOCARDLESS_VERSION: &str = "2015-07-06";
const GOCARDLESS_VERSION_HEADER: &str = "GoCardless-Version";

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Gocardless
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
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

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.gocardless.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
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
        })
    }
}

impl
    ConnectorIntegration<
        api::CreateConnectorCustomer,
        types::ConnectorCustomerData,
        types::PaymentsResponseData,
    > for Gocardless
{
    fn get_headers(
        &self,
        req: &types::ConnectorCustomerRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &types::ConnectorCustomerRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/customers", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::ConnectorCustomerRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = gocardless::GocardlessCustomerRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::ConnectorCustomerRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<common_utils::request::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        data: &types::ConnectorCustomerRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        types::RouterData<
            api::CreateConnectorCustomer,
            types::ConnectorCustomerData,
            types::PaymentsResponseData,
        >,
        errors::ConnectorError,
    >
    where
        api::CreateConnectorCustomer: Clone,
        types::ConnectorCustomerData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: gocardless::GocardlessCustomerResponse = res
            .response
            .parse_struct("GocardlessCustomerResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
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

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Gocardless
{
    fn get_headers(
        &self,
        req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/customer_bank_accounts",
            self.base_url(connectors),
        ))
    }

    fn get_request_body(
        &self,
        req: &types::TokenizationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = gocardless::GocardlessBankAccountRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<common_utils::request::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        data: &types::TokenizationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::TokenizationRouterData, errors::ConnectorError>
    where
        api::PaymentMethodToken: Clone,
        types::PaymentMethodTokenizationData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: gocardless::GocardlessBankAccountResponse = res
            .response
            .parse_struct("GocardlessBankAccountResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
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

impl
    ConnectorIntegration<
        api::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
    > for Gocardless
{
}

impl ConnectorValidation for Gocardless {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic => Ok(()),
            enums::CaptureMethod::Manual
            | enums::CaptureMethod::ManualMultiple
            | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }
    fn validate_mandate_payment(
        &self,
        pm_type: Option<types::storage::enums::PaymentMethodType>,
        pm_data: types::domain::payments::PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        let mandate_supported_pmd = std::collections::HashSet::from([
            PaymentMethodDataType::SepaBankDebit,
            PaymentMethodDataType::AchBankDebit,
            PaymentMethodDataType::BecsBankDebit,
            PaymentMethodDataType::MandatePayment,
        ]);
        connector_utils::is_mandate_supported(pm_data, pm_type, mandate_supported_pmd, self.id())
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Gocardless
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Gocardless
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Gocardless
{
    fn get_headers(
        &self,
        req: &types::SetupMandateRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_url(
        &self,
        _req: &types::SetupMandateRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/mandates", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::SetupMandateRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = gocardless::GocardlessMandateRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::SetupMandateRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<common_utils::request::Request>, errors::ConnectorError> {
        // Preprocessing flow is to create mandate, which should to be called only in case of First mandate
        if req.request.setup_mandate_details.is_some() {
            Ok(Some(
                services::RequestBuilder::new()
                    .method(services::Method::Post)
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
        data: &types::SetupMandateRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::SetupMandateRouterData, errors::ConnectorError> {
        let response: gocardless::GocardlessMandateResponse = res
            .response
            .parse_struct("GocardlessMandateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
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

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Gocardless
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/payments", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = gocardless::GocardlessRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req =
            gocardless::GocardlessPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
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
        data: &types::PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: gocardless::GocardlessPaymentsResponse = res
            .response
            .parse_struct("GocardlessPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
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

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Gocardless
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
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
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: gocardless::GocardlessPaymentsResponse = res
            .response
            .parse_struct("GocardlessPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
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

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Gocardless
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Gocardless
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Gocardless
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/refunds", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = gocardless::GocardlessRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let connector_req = gocardless::GocardlessRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
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
        data: &types::RefundsRouterData<api::Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: gocardless::RefundResponse = res
            .response
            .parse_struct("gocardless RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
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

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Gocardless
{
    fn build_request(
        &self,
        _req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(None)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Gocardless {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
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
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(format!("{}", String::from_utf8_lossy(request.body))
            .as_bytes()
            .to_vec())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let details: gocardless::GocardlessWebhookEvent = request
            .body
            .parse_struct("GocardlessWebhookEvent")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let first_event = details
            .events
            .first()
            .ok_or_else(|| errors::ConnectorError::WebhookReferenceIdNotFound)?;
        let reference_id = match &first_event.links {
            transformers::WebhooksLink::PaymentWebhooksLink(link) => {
                let payment_id = api_models::payments::PaymentIdType::ConnectorTransactionId(
                    link.payment.to_owned(),
                );
                api::webhooks::ObjectReferenceId::PaymentId(payment_id)
            }
            transformers::WebhooksLink::RefundWebhookLink(link) => {
                let refund_id =
                    api_models::webhooks::RefundIdType::ConnectorRefundId(link.refund.to_owned());
                api::webhooks::ObjectReferenceId::RefundId(refund_id)
            }
            transformers::WebhooksLink::MandateWebhookLink(link) => {
                let mandate_id = api_models::webhooks::MandateIdType::ConnectorMandateId(
                    link.mandate.to_owned(),
                );
                api::webhooks::ObjectReferenceId::MandateId(mandate_id)
            }
        };
        Ok(reference_id)
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let details: gocardless::GocardlessWebhookEvent = request
            .body
            .parse_struct("GocardlessWebhookEvent")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let first_event = details
            .events
            .first()
            .ok_or_else(|| errors::ConnectorError::WebhookReferenceIdNotFound)?;
        let event_type = match &first_event.action {
            transformers::WebhookAction::PaymentsAction(action) => match action {
                transformers::PaymentsAction::Created
                | transformers::PaymentsAction::Submitted
                | transformers::PaymentsAction::CustomerApprovalGranted => {
                    api::IncomingWebhookEvent::PaymentIntentProcessing
                }
                transformers::PaymentsAction::CustomerApprovalDenied
                | transformers::PaymentsAction::Failed
                | transformers::PaymentsAction::Cancelled
                | transformers::PaymentsAction::LateFailureSettled => {
                    api::IncomingWebhookEvent::PaymentIntentFailure
                }
                transformers::PaymentsAction::Confirmed | transformers::PaymentsAction::PaidOut => {
                    api::IncomingWebhookEvent::PaymentIntentSuccess
                }
                transformers::PaymentsAction::SurchargeFeeDebited
                | transformers::PaymentsAction::ResubmissionRequired => {
                    api::IncomingWebhookEvent::EventNotSupported
                }
            },
            transformers::WebhookAction::RefundsAction(action) => match action {
                transformers::RefundsAction::Failed => api::IncomingWebhookEvent::RefundFailure,
                transformers::RefundsAction::Paid => api::IncomingWebhookEvent::RefundSuccess,
                transformers::RefundsAction::RefundSettled
                | transformers::RefundsAction::FundsReturned
                | transformers::RefundsAction::Created => {
                    api::IncomingWebhookEvent::EventNotSupported
                }
            },
            transformers::WebhookAction::MandatesAction(action) => match action {
                transformers::MandatesAction::Active | transformers::MandatesAction::Reinstated => {
                    api::IncomingWebhookEvent::MandateActive
                }
                transformers::MandatesAction::Expired
                | transformers::MandatesAction::Cancelled
                | transformers::MandatesAction::Failed
                | transformers::MandatesAction::Consumed => {
                    api::IncomingWebhookEvent::MandateRevoked
                }
                transformers::MandatesAction::Created
                | transformers::MandatesAction::CustomerApprovalGranted
                | transformers::MandatesAction::CustomerApprovalSkipped
                | transformers::MandatesAction::Transferred
                | transformers::MandatesAction::Submitted
                | transformers::MandatesAction::ResubmissionRequested
                | transformers::MandatesAction::Replaced
                | transformers::MandatesAction::Blocked => {
                    api::IncomingWebhookEvent::EventNotSupported
                }
            },
        };
        Ok(event_type)
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let details: gocardless::GocardlessWebhookEvent = request
            .body
            .parse_struct("GocardlessWebhookEvent")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let first_event = details
            .events
            .first()
            .ok_or_else(|| errors::ConnectorError::WebhookReferenceIdNotFound)?
            .clone();
        match first_event.resource_type {
            transformers::WebhookResourceType::Payments => Ok(Box::new(
                gocardless::GocardlessPaymentsResponse::try_from(&first_event)?,
            )),
            transformers::WebhookResourceType::Refunds
            | transformers::WebhookResourceType::Mandates => Ok(Box::new(first_event)),
        }
    }
}
