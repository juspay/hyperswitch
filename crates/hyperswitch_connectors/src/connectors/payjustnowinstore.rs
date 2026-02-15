pub mod transformers;

use std::sync::LazyLock;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    crypto,
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt, ValueExt},
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
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    consts::NO_ERROR_MESSAGE,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use masking::{ExposeInterface, Mask, PeekInterface};
use ring::hmac;
use transformers as payjustnowinstore;

use crate::{constants::headers, types::ResponseRouterData, utils};

const PAYJUSTNOWINSTORE_MERCHANT_TERMINAL_ID: &str = "X-PayJustNow-Merchant-Terminal-ID";
const SIGNATURE: &str = "X-Signature";
const MERCHANT_REFERENCE_NON_UNIQUE: &str = "X-Merchant-Reference-Non-Unique";

#[derive(Clone)]
pub struct Payjustnowinstore {
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Payjustnowinstore {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &MinorUnitForConnector,
        }
    }
}

impl api::Payment for Payjustnowinstore {}
impl api::PaymentSession for Payjustnowinstore {}
impl api::ConnectorAccessToken for Payjustnowinstore {}
impl api::MandateSetup for Payjustnowinstore {}
impl api::PaymentAuthorize for Payjustnowinstore {}
impl api::PaymentSync for Payjustnowinstore {}
impl api::PaymentCapture for Payjustnowinstore {}
impl api::PaymentVoid for Payjustnowinstore {}
impl api::Refund for Payjustnowinstore {}
impl api::RefundExecute for Payjustnowinstore {}
impl api::RefundSync for Payjustnowinstore {}
impl api::PaymentToken for Payjustnowinstore {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Payjustnowinstore
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Payjustnowinstore
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let request_body = Self::get_request_body(self, req, connectors)?;

        let request_body_string =
            String::from_utf8(request_body.get_inner_value().peek().as_bytes().to_vec())
                .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;

        let request_body_string_without_whitespace =
            request_body_string.replace(char::is_whitespace, "");

        let auth = payjustnowinstore::PayjustnowinstoreAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let key = hmac::Key::new(hmac::HMAC_SHA256, auth.merchant_api_key.expose().as_bytes());

        let signature = hmac::sign(&key, request_body_string_without_whitespace.as_bytes());

        let signature_hex = hex::encode(signature.as_ref());

        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (SIGNATURE.to_string(), signature_hex.into_masked()),
            (
                MERCHANT_REFERENCE_NON_UNIQUE.to_string(),
                "true".to_string().into(),
            ),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Payjustnowinstore {
    fn id(&self) -> &'static str {
        "payjustnowinstore"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.payjustnowinstore.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = payjustnowinstore::PayjustnowinstoreAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            PAYJUSTNOWINSTORE_MERCHANT_TERMINAL_ID.to_string(),
            auth.merchant_terminal_id.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: payjustnowinstore::PayjustnowinstoreErrorResponse = res
            .response
            .parse_struct("PayjustnowinstoreErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let error_message = match res.status_code {
            400 => "bad_request",
            401 => "unauthorized",
            403 => "forbidden",
            404 => "not_found",
            _ => NO_ERROR_MESSAGE,
        };

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: res.status_code.to_string(),
            message: response.error.clone().unwrap_or(error_message.to_string()),
            reason: response.error,
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

impl ConnectorValidation for Payjustnowinstore {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData>
    for Payjustnowinstore
{
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for Payjustnowinstore
{
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Payjustnowinstore
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Payjustnowinstore
{
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
            "{}/v1/merchant/pos/checkout",
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

        let connector_router_data =
            payjustnowinstore::PayjustnowinstoreRouterData::from((amount, req));
        let connector_req =
            payjustnowinstore::PayjustnowinstorePaymentsRequest::try_from(&connector_router_data)?;
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
        let response: payjustnowinstore::PayjustnowinstorePaymentsResponse = res
            .response
            .parse_struct("Payjustnowinstore PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Payjustnowinstore {
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
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_transaction_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}/v1/merchant/pos/checkout/{}",
            self.base_url(connectors),
            connector_transaction_id,
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
        let response: payjustnowinstore::PayjustnowinstoreSyncResponse = res
            .response
            .parse_struct("payjustnowinstore PaymentsSyncResponse")
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
    for Payjustnowinstore
{
    fn build_request(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Capture".to_string(),
            connector: "Payjustnowinstore",
        }
        .into())
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Payjustnowinstore {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Payjustnowinstore {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        Ok(format!(
            "{}/v2/merchant/pos/checkout/refund",
            self.base_url(connectors)
        ))
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
            payjustnowinstore::PayjustnowinstoreRouterData::from((refund_amount, req));
        let connector_req =
            payjustnowinstore::PayjustnowinstoreRefundRequest::try_from(&connector_router_data)?;
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
        let response: payjustnowinstore::PayjustnowinstoreRefundResponse = res
            .response
            .parse_struct("payjustnowinstore RefundResponse")
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Payjustnowinstore {}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Payjustnowinstore {
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let details: payjustnowinstore::PayjustnowinstoreWebhookDetails = request
            .body
            .parse_struct("PayjustnowinstoreWebhookDetails")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(details.token),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let details: payjustnowinstore::PayjustnowinstoreWebhookDetails = request
            .body
            .parse_struct("PayjustnowinstoreWebhookDetails")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        let event_type = match details.payment_status {
            payjustnowinstore::PayjustnowinstoreWebhookStatus::Paid => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess
            }
            payjustnowinstore::PayjustnowinstoreWebhookStatus::PaymentFailed => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure
            }
            payjustnowinstore::PayjustnowinstoreWebhookStatus::Pending => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing
            }
            payjustnowinstore::PayjustnowinstoreWebhookStatus::OrderCancelled => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelled
            }
        };
        Ok(event_type)
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let details: payjustnowinstore::PayjustnowinstoreWebhookDetails = request
            .body
            .parse_struct("PayjustnowinstoreWebhookDetails")
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(details))
    }

    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let signature = request
            .headers
            .get("x-signature")
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?
            .to_str()
            .map_err(|_| errors::ConnectorError::WebhookSignatureNotFound)?
            .to_string();
        let decoded_signature = base64::engine::general_purpose::STANDARD
            .decode(signature)
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;
        Ok(decoded_signature)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(request
            .body
            .iter()
            .filter(|&b| !b.is_ascii_whitespace())
            .copied()
            .collect::<Vec<u8>>())
    }

    async fn verify_webhook_source(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        connector_account_details: crypto::Encryptable<masking::Secret<serde_json::Value>>,
        _connector_name: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_auth_type: ConnectorAuthType = connector_account_details
            .parse_value("ConnectorAuthType")
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let auth = payjustnowinstore::PayjustnowinstoreAuthType::try_from(&connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let key_bytes = auth.merchant_api_key.clone().expose().into_bytes();
        let key = hmac::Key::new(hmac::HMAC_SHA256, &key_bytes);
        let webhook_secret = api_models::webhooks::ConnectorWebhookSecrets {
            secret: key_bytes.clone(),
            additional_secret: None,
        };
        let signature = self.get_webhook_source_verification_signature(request, &webhook_secret)?;

        let message =
            self.get_webhook_source_verification_message(request, merchant_id, &webhook_secret)?;
        let message_string = String::from_utf8(message)
            .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;
        let message_signature = hmac::sign(&key, message_string.as_bytes());
        let message_signature_hex = hex::encode(message_signature.as_ref());
        let decoded_message_signature = base64::engine::general_purpose::STANDARD
            .decode(message_signature_hex)
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;

        Ok(signature == decoded_message_signature)
    }
}

static PAYJUSTNOWINSTORE_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::SequentialAutomatic,
        ];

        let mut payjustnowinstore_supported_payment_methods = SupportedPaymentMethods::new();

        payjustnowinstore_supported_payment_methods.add(
            enums::PaymentMethod::PayLater,
            enums::PaymentMethodType::Payjustnow,
            PaymentMethodDetails {
                mandates: common_enums::FeatureStatus::NotSupported,
                refunds: common_enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        payjustnowinstore_supported_payment_methods
    });

static PAYJUSTNOWINSTORE_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "PayJustNow In-Store",
    description: "PayJustNow provides a BNPL solution for online and in-store payments, enabling customers to pay in three interest-free installments while merchants get paid upfront.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Sandbox,
};

static PAYJUSTNOWINSTORE_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] =
    [enums::EventClass::Payments];

impl ConnectorSpecifications for Payjustnowinstore {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&PAYJUSTNOWINSTORE_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*PAYJUSTNOWINSTORE_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&PAYJUSTNOWINSTORE_SUPPORTED_WEBHOOK_FLOWS)
    }
}
