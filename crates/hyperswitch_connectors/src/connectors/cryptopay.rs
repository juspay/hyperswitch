pub mod transformers;
use std::sync::LazyLock;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    crypto::{self, GenerateDigest, SignMessage},
    date_time,
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::ResultExt;
use hex::encode;
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
    types::{PaymentsAuthorizeRouterData, PaymentsSyncRouterData},
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{PaymentsAuthorizeType, PaymentsSyncType, Response},
    webhooks,
};
use masking::{Mask, PeekInterface};
use transformers as cryptopay;

use self::cryptopay::CryptopayWebhookDetails;
use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self, ForeignTryFrom},
};

#[derive(Clone)]
pub struct Cryptopay {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Cryptopay {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }
}

impl api::Payment for Cryptopay {}
impl api::PaymentSession for Cryptopay {}
impl api::ConnectorAccessToken for Cryptopay {}
impl api::MandateSetup for Cryptopay {}
impl api::PaymentAuthorize for Cryptopay {}
impl api::PaymentSync for Cryptopay {}
impl api::PaymentCapture for Cryptopay {}
impl api::PaymentVoid for Cryptopay {}
impl api::Refund for Cryptopay {}
impl api::RefundExecute for Cryptopay {}
impl api::RefundSync for Cryptopay {}
impl api::PaymentToken for Cryptopay {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Cryptopay
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Cryptopay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let method = self.get_http_method();
        let payload = match method {
            Method::Get => String::default(),
            Method::Post | Method::Put | Method::Delete | Method::Patch => {
                let body = self
                    .get_request_body(req, connectors)?
                    .get_inner_value()
                    .peek()
                    .to_owned();
                let md5_payload = crypto::Md5
                    .generate_digest(body.as_bytes())
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                encode(md5_payload)
            }
        };
        let api_method = method.to_string();

        let now = date_time::date_as_yyyymmddthhmmssmmmz()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let date = format!("{}+00:00", now.split_at(now.len() - 5).0);

        let content_type = self.get_content_type().to_string();

        let api = (self.get_url(req, connectors)?).replace(self.base_url(connectors), "");

        let auth = cryptopay::CryptopayAuthType::try_from(&req.connector_auth_type)?;

        let sign_req: String = format!("{api_method}\n{payload}\n{content_type}\n{date}\n{api}");
        let authz = crypto::HmacSha1::sign_message(
            &crypto::HmacSha1,
            auth.api_secret.peek().as_bytes(),
            sign_req.as_bytes(),
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to sign the message")?;
        let authz = common_utils::consts::BASE64_ENGINE.encode(authz);
        let auth_string: String = format!("HMAC {}:{}", auth.api_key.peek(), authz);

        let headers = vec![
            (
                headers::AUTHORIZATION.to_string(),
                auth_string.into_masked(),
            ),
            (headers::DATE.to_string(), date.into()),
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
        ];
        Ok(headers)
    }
}

impl ConnectorCommon for Cryptopay {
    fn id(&self) -> &'static str {
        "cryptopay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.cryptopay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = cryptopay::CryptopayAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.peek().to_owned().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: cryptopay::CryptopayErrorResponse = res
            .response
            .parse_struct("CryptopayErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.error.code,
            message: response.error.message,
            reason: response.error.reason,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Cryptopay {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Cryptopay {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Cryptopay
{
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Cryptopay".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Cryptopay {
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
        Ok(format!("{}/api/invoices", self.base_url(connectors)))
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
        let connector_router_data = cryptopay::CryptopayRouterData::from((amount, req));
        let connector_req = cryptopay::CryptopayPaymentsRequest::try_from(&connector_router_data)?;
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
                .url(&PaymentsAuthorizeType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(PaymentsAuthorizeType::get_headers(self, req, connectors)?)
                .set_body(PaymentsAuthorizeType::get_request_body(
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
        let response: cryptopay::CryptopayPaymentsResponse = res
            .response
            .parse_struct("Cryptopay PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        let capture_amount_in_minor_units = match response.data.price_amount {
            Some(ref amount) => Some(utils::convert_back_amount_to_minor_units(
                self.amount_converter,
                amount.clone(),
                data.request.currency,
            )?),
            None => None,
        };
        RouterData::foreign_try_from((
            ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            capture_amount_in_minor_units,
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorValidation for Cryptopay {
    fn validate_psync_reference_id(
        &self,
        _data: &PaymentsSyncData,
        _is_three_ds: bool,
        _status: enums::AttemptStatus,
        _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), errors::ConnectorError> {
        // since we can make psync call with our reference_id, having connector_transaction_id is not an mandatory criteria
        Ok(())
    }
}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Cryptopay {
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

    fn get_http_method(&self) -> Method {
        Method::Get
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let custom_id = req.connector_request_reference_id.clone();
        Ok(format!(
            "{}/api/invoices/custom_id/{custom_id}",
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
        let response: cryptopay::CryptopayPaymentsResponse = res
            .response
            .parse_struct("cryptopay PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        let capture_amount_in_minor_units = match response.data.price_amount {
            Some(ref amount) => Some(utils::convert_back_amount_to_minor_units(
                self.amount_converter,
                amount.clone(),
                data.request.currency,
            )?),
            None => None,
        };
        RouterData::foreign_try_from((
            ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            capture_amount_in_minor_units,
        ))
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Cryptopay {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Cryptopay {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Cryptopay {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Cryptopay {}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Cryptopay {
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
        let base64_signature =
            utils::get_header_key_value("X-Cryptopay-Signature", request.headers)?;
        hex::decode(base64_signature)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let message = std::str::from_utf8(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        Ok(message.to_string().into_bytes())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let notif: CryptopayWebhookDetails =
            request
                .body
                .parse_struct("CryptopayWebhookDetails")
                .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        match notif.data.custom_id {
            Some(custom_id) => Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::PaymentAttemptId(custom_id),
            )),
            None => Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(notif.data.id),
            )),
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let notif: CryptopayWebhookDetails =
            request
                .body
                .parse_struct("CryptopayWebhookDetails")
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        match notif.data.status {
            cryptopay::CryptopayPaymentStatus::Completed => {
                Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess)
            }
            cryptopay::CryptopayPaymentStatus::Unresolved => {
                Ok(api_models::webhooks::IncomingWebhookEvent::PaymentActionRequired)
            }
            cryptopay::CryptopayPaymentStatus::Cancelled => {
                Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure)
            }
            _ => Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported),
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let notif: CryptopayWebhookDetails =
            request
                .body
                .parse_struct("CryptopayWebhookDetails")
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(Box::new(notif))
    }
}

static CRYPTOPAY_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
            enums::CaptureMethod::SequentialAutomatic,
        ];

        let mut cryptopay_supported_payment_methods = SupportedPaymentMethods::new();

        cryptopay_supported_payment_methods.add(
            enums::PaymentMethod::Crypto,
            enums::PaymentMethodType::CryptoCurrency,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::NotSupported,
                supported_capture_methods,
                specific_features: None,
            },
        );

        cryptopay_supported_payment_methods
    });

static CRYPTOPAY_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Cryptopay",
    description: "Simple and secure solution to buy and manage crypto. Make quick international transfers, spend your BTC, ETH and other crypto assets.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Live,
};

static CRYPTOPAY_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] = [enums::EventClass::Payments];

impl ConnectorSpecifications for Cryptopay {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&CRYPTOPAY_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*CRYPTOPAY_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&CRYPTOPAY_SUPPORTED_WEBHOOK_FLOWS)
    }
}
