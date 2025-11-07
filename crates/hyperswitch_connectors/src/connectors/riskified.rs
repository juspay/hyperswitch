pub mod transformers;
#[cfg(feature = "frm")]
use api_models::webhooks::{ConnectorWebhookSecrets, IncomingWebhookEvent, ObjectReferenceId};
#[cfg(feature = "frm")]
use base64::Engine;
#[cfg(feature = "frm")]
use common_utils::types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector};
#[cfg(feature = "frm")]
use common_utils::{
    consts::BASE64_ENGINE,
    request::{Method, RequestBuilder},
    types::MinorUnit,
};
#[cfg(feature = "frm")]
use common_utils::{crypto, ext_traits::ByteSliceExt, request::RequestContent};
use common_utils::{errors::CustomResult, request::Request};
#[cfg(feature = "frm")]
use error_stack::ResultExt;
#[cfg(feature = "frm")]
use hyperswitch_domain_models::{
    router_data::ErrorResponse,
    router_flow_types::{Checkout, Fulfillment, RecordReturn, Sale, Transaction},
    router_request_types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
        FraudCheckSaleData, FraudCheckTransactionData,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};
use hyperswitch_domain_models::{
    router_data::{AccessToken, RouterData},
    router_flow_types::{
        AccessTokenAuth, Authorize, Capture, Execute, PSync, PaymentMethodToken, RSync, Session,
        SetupMandate, Void,
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentsResponseData, RefundsResponseData, SupportedPaymentMethods,
    },
};
use hyperswitch_interfaces::{
    api::{
        ConnectorAccessToken, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration,
        ConnectorSpecifications, ConnectorValidation, MandateSetup, Payment, PaymentAuthorize,
        PaymentCapture, PaymentSession, PaymentSync, PaymentToken, PaymentVoid, Refund,
        RefundExecute, RefundSync,
    },
    configs::Connectors,
    errors::ConnectorError,
};
#[cfg(feature = "frm")]
use hyperswitch_interfaces::{
    api::{
        FraudCheck, FraudCheckCheckout, FraudCheckFulfillment, FraudCheckRecordReturn,
        FraudCheckSale, FraudCheckTransaction,
    },
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
#[cfg(feature = "frm")]
use masking::Maskable;
#[cfg(feature = "frm")]
use masking::{ExposeInterface, Mask, PeekInterface, Secret};
#[cfg(feature = "frm")]
use ring::hmac;
#[cfg(feature = "frm")]
use transformers as riskified;

#[cfg(feature = "frm")]
use crate::constants::headers;
#[cfg(feature = "frm")]
use crate::utils::convert_amount;
#[cfg(feature = "frm")]
use crate::{
    types::{
        FrmCheckoutRouterData, FrmCheckoutType, FrmFulfillmentRouterData, FrmFulfillmentType,
        FrmTransactionRouterData, FrmTransactionType, ResponseRouterData,
    },
    utils::FrmTransactionRouterDataRequest,
};

#[derive(Clone)]
pub struct Riskified {
    #[cfg(feature = "frm")]
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Riskified {
    pub fn new() -> &'static Self {
        &Self {
            #[cfg(feature = "frm")]
            amount_converter: &StringMajorUnitForConnector,
        }
    }

    #[cfg(feature = "frm")]
    pub fn generate_authorization_signature(
        &self,
        auth: &riskified::RiskifiedAuthType,
        payload: &str,
    ) -> CustomResult<String, ConnectorError> {
        let key = hmac::Key::new(
            hmac::HMAC_SHA256,
            auth.secret_token.clone().expose().as_bytes(),
        );

        let signature_value = hmac::sign(&key, payload.as_bytes());

        let digest = signature_value.as_ref();

        Ok(hex::encode(digest))
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Riskified
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    #[cfg(feature = "frm")]
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let auth: riskified::RiskifiedAuthType =
            riskified::RiskifiedAuthType::try_from(&req.connector_auth_type)?;

        let riskified_req = self.get_request_body(req, connectors)?;

        let binding = riskified_req.get_inner_value();
        let payload = binding.peek();

        let digest = self
            .generate_authorization_signature(&auth, payload)
            .change_context(ConnectorError::RequestEncodingFailed)?;

        let header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                "X-RISKIFIED-SHOP-DOMAIN".to_string(),
                auth.domain_name.clone().into(),
            ),
            (
                "X-RISKIFIED-HMAC-SHA256".to_string(),
                Mask::into_masked(digest),
            ),
            (
                "Accept".to_string(),
                "application/vnd.riskified.com; version=2".into(),
            ),
        ];

        Ok(header)
    }
}

impl ConnectorCommon for Riskified {
    fn id(&self) -> &'static str {
        "riskified"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }
    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.riskified.base_url.as_ref()
    }

    #[cfg(feature = "frm")]
    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        use hyperswitch_interfaces::consts::NO_ERROR_CODE;

        let response: riskified::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            attempt_status: None,
            code: NO_ERROR_CODE.to_string(),
            message: response.error.message.clone(),
            reason: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Checkout, FraudCheckCheckoutData, FraudCheckResponseData> for Riskified {
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
        Ok(format!("{}{}", self.base_url(connectors), "/decide"))
    }

    fn get_request_body(
        &self,
        req: &FrmCheckoutRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            MinorUnit::new(req.request.amount),
            req.request
                .currency
                .ok_or(ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?,
        )?;
        let req_data = riskified::RiskifiedRouterData::from((amount, req));
        let req_obj = riskified::RiskifiedPaymentsCheckoutRequest::try_from(&req_data)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
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
        let response: riskified::RiskifiedPaymentsResponse = res
            .response
            .parse_struct("RiskifiedPaymentsResponse Checkout")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        <FrmCheckoutRouterData>::try_from(ResponseRouterData {
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

impl Payment for Riskified {}
impl PaymentAuthorize for Riskified {}
impl PaymentSync for Riskified {}
impl PaymentVoid for Riskified {}
impl PaymentCapture for Riskified {}
impl MandateSetup for Riskified {}
impl ConnectorAccessToken for Riskified {}
impl PaymentToken for Riskified {}
impl Refund for Riskified {}
impl RefundExecute for Riskified {}
impl RefundSync for Riskified {}
impl ConnectorValidation for Riskified {}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Sale, FraudCheckSaleData, FraudCheckResponseData> for Riskified {}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Transaction, FraudCheckTransactionData, FraudCheckResponseData>
    for Riskified
{
    fn get_headers(
        &self,
        req: &FrmTransactionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &FrmTransactionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        match req.is_payment_successful() {
            Some(false) => Ok(format!(
                "{}{}",
                self.base_url(connectors),
                "/checkout_denied"
            )),
            _ => Ok(format!("{}{}", self.base_url(connectors), "/decision")),
        }
    }

    fn get_request_body(
        &self,
        req: &FrmTransactionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        match req.is_payment_successful() {
            Some(false) => {
                let req_obj = riskified::TransactionFailedRequest::try_from(req)?;
                Ok(RequestContent::Json(Box::new(req_obj)))
            }
            _ => {
                let amount = convert_amount(
                    self.amount_converter,
                    MinorUnit::new(req.request.amount),
                    req.request
                        .currency
                        .ok_or(ConnectorError::MissingRequiredField {
                            field_name: "currency",
                        })?,
                )?;
                let req_data = riskified::RiskifiedRouterData::from((amount, req));
                let req_obj = riskified::TransactionSuccessRequest::try_from(&req_data)?;
                Ok(RequestContent::Json(Box::new(req_obj)))
            }
        }
    }

    fn build_request(
        &self,
        req: &FrmTransactionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&FrmTransactionType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(FrmTransactionType::get_headers(self, req, connectors)?)
                .set_body(FrmTransactionType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &FrmTransactionRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<FrmTransactionRouterData, ConnectorError> {
        let response: riskified::RiskifiedTransactionResponse = res
            .response
            .parse_struct("RiskifiedPaymentsResponse Transaction")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        match response {
            riskified::RiskifiedTransactionResponse::FailedResponse(response_data) => {
                <FrmTransactionRouterData>::try_from(ResponseRouterData {
                    response: response_data,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            riskified::RiskifiedTransactionResponse::SuccessResponse(response_data) => {
                <FrmTransactionRouterData>::try_from(ResponseRouterData {
                    response: response_data,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
        }
    }
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(feature = "frm")]
impl ConnectorIntegration<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>
    for Riskified
{
    fn get_headers(
        &self,
        req: &FrmFulfillmentRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &FrmFulfillmentRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "/fulfill"))
    }

    fn get_request_body(
        &self,
        req: &FrmFulfillmentRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let req_obj = riskified::RiskifiedFulfillmentRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

    fn build_request(
        &self,
        req: &FrmFulfillmentRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&FrmFulfillmentType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(FrmFulfillmentType::get_headers(self, req, connectors)?)
                .set_body(FrmFulfillmentType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &FrmFulfillmentRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<FrmFulfillmentRouterData, ConnectorError> {
        let response: riskified::RiskifiedFulfilmentResponse = res
            .response
            .parse_struct("RiskifiedFulfilmentResponse fulfilment")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        FrmFulfillmentRouterData::try_from(ResponseRouterData {
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

#[cfg(feature = "frm")]
impl ConnectorIntegration<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>
    for Riskified
{
}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Riskified
{
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Riskified {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Riskified
{
    fn build_request(
        &self,
        _req: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        Err(ConnectorError::NotImplemented("Setup Mandate flow for Riskified".to_string()).into())
    }
}

impl PaymentSession for Riskified {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Riskified {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Riskified {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Riskified {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Riskified {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Riskified {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Riskified {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Riskified {}

#[cfg(feature = "frm")]
impl FraudCheck for Riskified {}
#[cfg(feature = "frm")]
impl FraudCheckSale for Riskified {}
#[cfg(feature = "frm")]
impl FraudCheckCheckout for Riskified {}
#[cfg(feature = "frm")]
impl FraudCheckTransaction for Riskified {}
#[cfg(feature = "frm")]
impl FraudCheckFulfillment for Riskified {}
#[cfg(feature = "frm")]
impl FraudCheckRecordReturn for Riskified {}

#[cfg(feature = "frm")]
#[async_trait::async_trait]
impl IncomingWebhook for Riskified {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        let header_value =
            crate::utils::get_header_key_value("x-riskified-hmac-sha256", request.headers)?;
        Ok(header_value.as_bytes().to_vec())
    }

    async fn verify_webhook_source(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: crypto::Encryptable<Secret<serde_json::Value>>,
        connector_label: &str,
    ) -> CustomResult<bool, ConnectorError> {
        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_id,
                connector_label,
                connector_webhook_details,
            )
            .await
            .change_context(ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(ConnectorError::WebhookSourceVerificationFailed)?;

        let message = self
            .get_webhook_source_verification_message(
                request,
                merchant_id,
                &connector_webhook_secrets,
            )
            .change_context(ConnectorError::WebhookSourceVerificationFailed)?;

        let signing_key = hmac::Key::new(hmac::HMAC_SHA256, &connector_webhook_secrets.secret);
        let signed_message = hmac::sign(&signing_key, &message);
        let payload_sign = BASE64_ENGINE.encode(signed_message.as_ref());
        Ok(payload_sign.as_bytes().eq(&signature))
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, ConnectorError> {
        Ok(request.body.to_vec())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, ConnectorError> {
        let resource: riskified::RiskifiedWebhookBody = request
            .body
            .parse_struct("RiskifiedWebhookBody")
            .change_context(ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::PaymentAttemptId(resource.id),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        let resource: riskified::RiskifiedWebhookBody = request
            .body
            .parse_struct("RiskifiedWebhookBody")
            .change_context(ConnectorError::WebhookEventTypeNotFound)?;
        Ok(IncomingWebhookEvent::from(resource.status))
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, ConnectorError> {
        let resource: riskified::RiskifiedWebhookBody = request
            .body
            .parse_struct("RiskifiedWebhookBody")
            .change_context(ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(resource))
    }
}

static RISKIFIED_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Riskified",
    description: "Riskified fraud and risk management provider with guaranteed real-time decisions and machine learning-powered ecommerce fraud prevention",
    connector_type: common_enums::HyperswitchConnectorCategory::FraudAndRiskManagementProvider,
    integration_status: common_enums::ConnectorIntegrationStatus::Sandbox,
};

impl ConnectorSpecifications for Riskified {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&RISKIFIED_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        None
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::enums::EventClass]> {
        None
    }
}
