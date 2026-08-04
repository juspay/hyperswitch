pub mod transformers;

use std::sync::LazyLock;

use common_enums::{enums, CaptureMethod, PaymentMethod, PaymentMethodType};
use common_utils::{
    crypto::{self, GenerateDigest},
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt, ValueExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
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
        RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    consts, errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use hyperswitch_masking::{ExposeInterface, Mask, PeekInterface};
use transformers as tdaypay;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self, get_header_key_value},
};

#[derive(Clone)]
pub struct Tdaypay {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Tdaypay {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }

    /// Build TDayPay signed headers for a given method + raw JSON body.
    fn build_signed_headers(
        &self,
        auth: &tdaypay::TdaypayAuthType,
        method: &str,
        raw_body: &str,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        let timestamp = tdaypay::current_timestamp();
        let sign = tdaypay::sign_request(
            auth.mch_id.peek(),
            method,
            &timestamp,
            raw_body,
            auth.merchant_key.peek(),
        )?;
        Ok(vec![
            (
                headers::CONTENT_TYPE.to_string(),
                "application/json".to_string().into(),
            ),
            ("Accept".to_string(), "application/json".to_string().into()),
            (
                "serviceName".to_string(),
                tdaypay::SERVICE_NAME.to_string().into(),
            ),
            ("method".to_string(), method.to_string().into()),
            (
                "mchId".to_string(),
                auth.mch_id.peek().to_string().into_masked(),
            ),
            (
                "signType".to_string(),
                tdaypay::SIGN_TYPE.to_string().into(),
            ),
            ("timestamp".to_string(), timestamp.into()),
            ("sign".to_string(), sign.into_masked()),
        ])
    }
}

impl api::Payment for Tdaypay {}
impl api::PaymentSession for Tdaypay {}
impl api::ConnectorAccessToken for Tdaypay {}
impl api::MandateSetup for Tdaypay {}
impl api::PaymentAuthorize for Tdaypay {}
impl api::PaymentSync for Tdaypay {}
impl api::PaymentCapture for Tdaypay {}
impl api::PaymentVoid for Tdaypay {}
impl api::Refund for Tdaypay {}
impl api::RefundExecute for Tdaypay {}
impl api::RefundSync for Tdaypay {}
impl api::PaymentToken for Tdaypay {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Tdaypay
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Tdaypay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        Ok(vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )])
    }
}

impl ConnectorCommon for Tdaypay {
    fn id(&self) -> &'static str {
        "tdaypay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        // TDayPay amounts are major units (e.g. "10.00" BRL, "1000" CLP).
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.tdaypay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, hyperswitch_masking::Maskable<String>)>, errors::ConnectorError>
    {
        // Auth is body-signature based; headers are built in build_request.
        Ok(vec![])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: tdaypay::TdaypayErrorResponse = res
            .response
            .parse_struct("TdaypayErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error_code
                .or(response.result_code)
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .error_msg
                .clone()
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: response.error_msg,
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

impl ConnectorValidation for Tdaypay {
    fn validate_psync_reference_id(
        &self,
        _data: &PaymentsSyncData,
        _is_three_ds: bool,
        _status: enums::AttemptStatus,
        _connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), errors::ConnectorError> {
        Ok(())
    }
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Tdaypay {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Tdaypay {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Tdaypay {
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Tdaypay".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Tdaypay {
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
        Ok(self.base_url(connectors).to_string())
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
        let connector_router_data = tdaypay::TdaypayRouterData::from((amount, req));
        let connector_req = tdaypay::TdaypayPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth = tdaypay::TdaypayAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let body = types::PaymentsAuthorizeType::get_request_body(self, req, connectors)?;
        let raw_body = body.get_inner_value().expose();
        let mut headers = self.build_signed_headers(&auth, "pay", &raw_body)?;
        let mut content_headers = types::PaymentsAuthorizeType::get_headers(self, req, connectors)?;
        headers.append(&mut content_headers);

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(headers)
                .set_body(body)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: tdaypay::TdaypayResponse = res
            .response
            .parse_struct("TdaypayPaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Tdaypay {
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
        _req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(self.base_url(connectors).to_string())
    }

    fn get_request_body(
        &self,
        req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = tdaypay::TdaypaySyncRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth = tdaypay::TdaypayAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let body = types::PaymentsSyncType::get_request_body(self, req, connectors)?;
        let raw_body = body.get_inner_value().expose();
        let mut headers = self.build_signed_headers(&auth, "verifyStatus", &raw_body)?;
        let mut content_headers = types::PaymentsSyncType::get_headers(self, req, connectors)?;
        headers.append(&mut content_headers);

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(headers)
                .set_body(body)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: tdaypay::TdaypayResponse = res
            .response
            .parse_struct("TdaypayPaymentsSyncResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Tdaypay {
    fn build_request(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Capture".to_string(),
            connector: "Tdaypay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Tdaypay {
    fn build_request(
        &self,
        _req: &RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Void".to_string(),
            connector: "Tdaypay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Tdaypay {
    fn build_request(
        &self,
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Refunds".to_string(),
            connector: "Tdaypay".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Tdaypay {
    fn build_request(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Refund Sync".to_string(),
            connector: "Tdaypay".to_string(),
        }
        .into())
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Tdaypay {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::Sha512))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let signature = get_header_key_value("sign", request.headers)
            .or_else(|_| get_header_key_value("Sign", request.headers))?;
        Ok(signature.as_bytes().to_vec())
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        // Webhook sign = SHA512(rawBody + merchantKey)
        let mut message = request.body.to_vec();
        message.extend_from_slice(&connector_webhook_secrets.secret);
        Ok(message)
    }

    async fn verify_webhook_source(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        connector_account_details: crypto::Encryptable<
            hyperswitch_masking::Secret<serde_json::Value>,
        >,
        _connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let merchant_key = {
            let connector_account_details: ConnectorAuthType = connector_account_details
                .parse_value::<ConnectorAuthType>("ConnectorAuthType")
                .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
            if let Ok(auth) = tdaypay::TdaypayAuthType::try_from(&connector_account_details) {
                auth.merchant_key.expose()
            } else if let Some(details) = connector_webhook_details {
                details
                    .expose()
                    .get("merchant_secret")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?
            } else {
                return Err(errors::ConnectorError::WebhookSourceVerificationFailed.into());
            }
        };

        let signature = get_header_key_value("sign", request.headers)
            .or_else(|_| get_header_key_value("Sign", request.headers))
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;

        let mut message = request.body.to_vec();
        message.extend_from_slice(merchant_key.as_bytes());

        let digest = crypto::Sha512
            .generate_digest(&message)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let expected = hex::encode(digest);

        Ok(expected.eq_ignore_ascii_case(signature.trim()))
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let notif: tdaypay::TdaypayWebhookBody = request
            .body
            .parse_struct("TdaypayWebhookBody")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;

        if let Some(order_id) = notif.order_id {
            Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(order_id),
            ))
        } else if let Some(mch_order_id) = notif.mch_order_id {
            Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::PaymentAttemptId(mch_order_id),
            ))
        } else {
            Err(errors::ConnectorError::WebhookReferenceIdNotFound.into())
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _context: Option<&webhooks::WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let notif: tdaypay::TdaypayWebhookBody = request
            .body
            .parse_struct("TdaypayWebhookBody")
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(notif
            .order_status
            .map(api_models::webhooks::IncomingWebhookEvent::from)
            .unwrap_or(api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing))
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn hyperswitch_masking::ErasedMaskSerialize>, errors::ConnectorError>
    {
        let notif: tdaypay::TdaypayWebhookBody = request
            .body
            .parse_struct("TdaypayWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(Box::new(notif))
    }
}

static TDAYPAY_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let supported_capture_methods =
        vec![CaptureMethod::Automatic, CaptureMethod::SequentialAutomatic];
    let mut methods = SupportedPaymentMethods::new();
    // BRL — PIX
    methods.add(
        PaymentMethod::BankTransfer,
        PaymentMethodType::Pix,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    // MXN SPEI / CLP CASH / ARS·PEN·ECS TRANSFER
    methods.add(
        PaymentMethod::BankTransfer,
        PaymentMethodType::LocalBankTransfer,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods: supported_capture_methods.clone(),
            specific_features: None,
        },
    );
    // COP — PSE
    methods.add(
        PaymentMethod::BankTransfer,
        PaymentMethodType::Pse,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::NotSupported,
            supported_capture_methods,
            specific_features: None,
        },
    );
    methods
});

static TDAYPAY_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "TDayPay",
    description: "TDayPay / TodayPay / Vamospago — generic LATAM local payments; currencies enabled via MCA/profile.",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Sandbox,
};

static TDAYPAY_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] = [enums::EventClass::Payments];

impl ConnectorSpecifications for Tdaypay {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&TDAYPAY_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*TDAYPAY_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&TDAYPAY_SUPPORTED_WEBHOOK_FLOWS)
    }
}
