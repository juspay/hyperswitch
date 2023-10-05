pub mod transformers;

use std::fmt::Debug;

use api_models::enums::enums;
use common_utils::{crypto, ext_traits::ByteSliceExt};
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use transformers as gocardless;

use crate::{
    configs::settings,
    connector::utils as connector_utils,
    core::errors::{self, CustomResult},
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
    utils::{self, BytesExt},
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: gocardless::GocardlessErrorResponse = res
            .response
            .parse_struct("GocardlessErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = gocardless::GocardlessCustomerRequest::try_from(req)?;
        let gocardless_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<gocardless::GocardlessCustomerRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(gocardless_req))
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
                .body(types::ConnectorCustomerType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::ConnectorCustomerRouterData,
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
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
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
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = gocardless::GocardlessBankAccountRequest::try_from(req)?;
        let gocardless_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<gocardless::GocardlessBankAccountRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(gocardless_req))
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
                .body(types::TokenizationType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::TokenizationRouterData,
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
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
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
        capture_method: Option<api_models::enums::CaptureMethod>,
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
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let req_obj = gocardless::GocardlessMandateRequest::try_from(req)?;
        let gocardless_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<gocardless::GocardlessMandateRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(gocardless_req))
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
                    .body(types::SetupMandateType::get_request_body(self, req)?)
                    .build(),
            ))
        } else {
            Ok(None)
        }
    }

    fn handle_response(
        &self,
        data: &types::SetupMandateRouterData,
        res: Response,
    ) -> CustomResult<types::SetupMandateRouterData, errors::ConnectorError> {
        let response: gocardless::GocardlessMandateResponse = res
            .response
            .parse_struct("GocardlessMandateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
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
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = gocardless::GocardlessRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let req_obj = gocardless::GocardlessPaymentsRequest::try_from(&connector_router_data)?;
        let gocardless_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<gocardless::GocardlessPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(gocardless_req))
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
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: gocardless::GocardlessPaymentsResponse = res
            .response
            .parse_struct("GocardlessPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
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
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: gocardless::GocardlessPaymentsResponse = res
            .response
            .parse_struct("GocardlessPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
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
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = gocardless::GocardlessRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let req_obj = gocardless::GocardlessRefundRequest::try_from(&connector_router_data)?;
        let gocardless_req = types::RequestBody::log_and_get_request_body(
            &req_obj,
            utils::Encode::<gocardless::GocardlessRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(gocardless_req))
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
            .body(types::RefundExecuteType::get_request_body(self, req)?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: gocardless::RefundResponse = res
            .response
            .parse_struct("gocardless RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
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
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let signature = request
            .headers
            .get("Webhook-Signature")
            .map(|header_value| {
                header_value
                    .to_str()
                    .map(String::from)
                    .map_err(|_| errors::ConnectorError::WebhookSignatureNotFound)
                    .into_report()
            })
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)
            .into_report()??;

        hex::decode(signature)
            .into_report()
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _secret: &[u8],
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
        };
        Ok(event_type)
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let details: gocardless::GocardlessWebhookEvent = request
            .body
            .parse_struct("GocardlessWebhookEvent")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let first_event = details
            .events
            .first()
            .ok_or_else(|| errors::ConnectorError::WebhookReferenceIdNotFound)?;
        match first_event.resource_type {
            transformers::WebhookResourceType::Payments => serde_json::to_value(
                gocardless::GocardlessPaymentsResponse::try_from(first_event)?,
            )
            .into_report()
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed),
            transformers::WebhookResourceType::Refunds => serde_json::to_value(first_event)
                .into_report()
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed),
        }
    }
}
