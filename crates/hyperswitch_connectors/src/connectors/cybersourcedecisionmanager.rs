pub mod transformers;

use std::sync::LazyLock;

use base64::Engine;
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit,StringMajorUnitForConnector, MinorUnit},
};

use url::Url;
#[cfg(feature = "frm")]
use hyperswitch_interfaces::{api::{FraudCheck,FraudCheckCheckout, FraudCheckTransaction },errors::ConnectorError,};
use time::OffsetDateTime;
use common_utils::consts;
use error_stack::{report, Report, ResultExt};
use ring::{digest, hmac};
use masking::{ExposeInterface, Mask, Maskable, PeekInterface};

use crate::utils::convert_amount;
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
        ConnectorInfo, PaymentsResponseData, RefundsResponseData, SupportedPaymentMethods,
    },
};
#[cfg(feature = "frm")]
use hyperswitch_domain_models::{
    router_flow_types::{Checkout, Transaction},
    router_request_types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckTransactionData,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};

#[cfg(feature = "frm")]
use crate::types::{
        FrmCheckoutRouterData, FrmCheckoutType,
        FrmTransactionRouterData, FrmTransactionType, ResponseRouterData,
    };
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks,
};
use transformers as cybersourcedecisionmanager;

use crate::{constants::{self, headers}, utils};

#[derive(Clone)]
pub struct Cybersourcedecisionmanager {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Cybersourcedecisionmanager {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }

    pub fn generate_digest(&self, payload: &[u8]) -> String {
        let payload_digest = digest::digest(&digest::SHA256, payload);
        consts::BASE64_ENGINE.encode(payload_digest)
    }

    pub fn generate_signature(
        &self,
        auth: cybersourcedecisionmanager::CybersourcedecisionmanagerAuthType,
        host: String,
        resource: &str,
        payload: &String,
        date: OffsetDateTime,
        http_method: Method,
    ) -> CustomResult<String, ConnectorError> {
        let cybersourcedecisionmanager::CybersourcedecisionmanagerAuthType {
            api_key,
            merchant_account,
            api_secret,
        } = auth;
        let is_post_method = matches!(http_method, Method::Post);
        let is_patch_method = matches!(http_method, Method::Patch);
        let is_delete_method = matches!(http_method, Method::Delete);
        let digest_str = if is_post_method || is_patch_method {
            "digest "
        } else {
            ""
        };
        let headers = format!("host date (request-target) {digest_str}v-c-merchant-id");
        let request_target = if is_post_method {
            format!("(request-target): post {resource}\ndigest: SHA-256={payload}\n")
        } else if is_patch_method {
            format!("(request-target): patch {resource}\ndigest: SHA-256={payload}\n")
        } else if is_delete_method {
            format!("(request-target): delete {resource}\n")
        } else {
            format!("(request-target): get {resource}\n")
        };
        let signature_string = format!(
            "host: {host}\ndate: {date}\n{request_target}v-c-merchant-id: {}",
            merchant_account.peek()
        );
        let key_value = consts::BASE64_ENGINE
            .decode(api_secret.expose())
            .change_context(ConnectorError::InvalidConnectorConfig {
                config: "connector_account_details.api_secret",
            })?;
        let key = hmac::Key::new(hmac::HMAC_SHA256, &key_value);
        let signature_value =
            consts::BASE64_ENGINE.encode(hmac::sign(&key, signature_string.as_bytes()).as_ref());
        let signature_header = format!(
            r#"keyid="{}", algorithm="HmacSHA256", headers="{headers}", signature="{signature_value}""#,
            api_key.peek()
        );

        Ok(signature_header)
    }
}

impl api::Payment for Cybersourcedecisionmanager {}
impl api::PaymentSession for Cybersourcedecisionmanager {}
impl api::ConnectorAccessToken for Cybersourcedecisionmanager {}
impl api::MandateSetup for Cybersourcedecisionmanager {}
impl api::PaymentAuthorize for Cybersourcedecisionmanager {}
impl api::PaymentSync for Cybersourcedecisionmanager {}
impl api::PaymentCapture for Cybersourcedecisionmanager {}
impl api::PaymentVoid for Cybersourcedecisionmanager {}
impl api::Refund for Cybersourcedecisionmanager {}
impl api::RefundExecute for Cybersourcedecisionmanager {}
impl api::RefundSync for Cybersourcedecisionmanager {}
impl api::PaymentToken for Cybersourcedecisionmanager {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Cybersourcedecisionmanager
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response>
    for Cybersourcedecisionmanager
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let date = OffsetDateTime::now_utc();
        let cybersource_req = self.get_request_body(req, connectors)?;
        let auth =  cybersourcedecisionmanager::CybersourcedecisionmanagerAuthType::try_from(&req.connector_auth_type)?;
        let merchant_account = auth.merchant_account.clone();
        let base_url = connectors.cybersource.base_url.as_str();
        let cybersource_host =
            Url::parse(base_url).change_context(ConnectorError::RequestEncodingFailed)?;
        let host = cybersource_host
            .host_str()
            .ok_or(ConnectorError::RequestEncodingFailed)?;
        let path: String = self
            .get_url(req, connectors)?
            .chars()
            .skip(base_url.len() - 1)
            .collect();
        let sha256 = self.generate_digest(cybersource_req.get_inner_value().expose().as_bytes());
        let http_method = self.get_http_method();
        let signature = self.generate_signature(
            auth,
            host.to_string(),
            path.as_str(),
            &sha256,
            date,
            http_method,
        )?;

        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::ACCEPT.to_string(),
                "application/hal+json;charset=utf-8".to_string().into(),
            ),
            (
                "v-c-merchant-id".to_string(),
                merchant_account.into_masked(),
            ),
            ("Date".to_string(), date.to_string().into()),
            ("Host".to_string(), host.to_string().into()),
            ("Signature".to_string(), signature.into_masked()),
        ];
        if matches!(http_method, Method::Post | Method::Put | Method::Patch) {
            headers.push((
                "Digest".to_string(),
                format!("SHA-256={sha256}").into_masked(),
            ));
        }
        Ok(headers)
    }
}

impl ConnectorCommon for Cybersourcedecisionmanager {
    fn id(&self) -> &'static str {
        "cybersourcedecisionmanager"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json;charset=utf-8"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.cybersourcedecisionmanager.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let auth =
            cybersourcedecisionmanager::CybersourcedecisionmanagerAuthType::try_from(auth_type)
                .change_context(ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: Result<
            cybersourcedecisionmanager::CybersourceDecisionManagerErrorResponse,
            Report<common_utils::errors::ParsingError>,
        > = res.response.parse_struct("CybersourceDecisionManagerErrorResponse");

        let error_message = if res.status_code == 401 {
            constants::CONNECTOR_UNAUTHORIZED_ERROR
        } else {
            hyperswitch_interfaces::consts::NO_ERROR_MESSAGE
        };
        match response {
            Ok(cybersourcedecisionmanager::CybersourceDecisionManagerErrorResponse::StandardError(response)) => {
                event_builder.map(|i| i.set_error_response_body(&response));
                router_env::logger::info!(connector_response=?response);

                let (code, message, reason) = match response.error_information {
                    Some(ref error_info) => {
                        let detailed_error_info = error_info.details.as_ref().map(|details| {
                            details
                                .iter()
                                .map(|det| format!("{} : {}", det.field, det.reason))
                                .collect::<Vec<_>>()
                                .join(", ")
                        });
                        (
                            error_info.reason.clone(),
                            error_info.message.clone(),
                            cybersourcedecisionmanager::get_error_reason(
                                Some(error_info.message.clone()),
                                detailed_error_info,
                                None,
                            ),
                        )
                    }
                    None => {
                        let detailed_error_info = response.details.map(|details| {
                            details
                                .iter()
                                .map(|det| format!("{} : {}", det.field, det.reason))
                                .collect::<Vec<_>>()
                                .join(", ")
                        });
                        (
                            response.reason.clone().map_or(
                                hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(),
                                |reason| reason.to_string(),
                            ),
                            response
                                .message
                                .clone()
                                .map_or(error_message.to_string(), |msg| msg.to_string()),
                            transformers::get_error_reason(
                                response.message,
                                detailed_error_info,
                                None,
                            ),
                        )
                    }
                };

                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code,
                    message,
                    reason,
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            Ok(cybersourcedecisionmanager::CybersourceDecisionManagerErrorResponse::AuthenticationError(response)) => {
                event_builder.map(|i| i.set_error_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(),
                    message: response.response.rmsg.clone(),
                    reason: Some(response.response.rmsg),
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            Ok(cybersourcedecisionmanager::CybersourceDecisionManagerErrorResponse::NotAvailableError(response)) => {
                event_builder.map(|i| i.set_error_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                let error_response = response
                    .errors
                    .iter()
                    .map(|error_info| {
                        format!(
                            "{}: {}",
                            error_info.error_type.clone().unwrap_or("".to_string()),
                            error_info.message.clone().unwrap_or("".to_string())
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(" & ");
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string(),
                    message: error_response.clone(),
                    reason: Some(error_response),
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            }
            Err(error_msg) => {
                event_builder.map(|event| event.set_error(serde_json::json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
                router_env::logger::error!(deserialization_error =? error_msg);
                utils::handle_json_response_deserialization_failure(res, "cybersource")
            }
        }
    }
}

impl ConnectorValidation for Cybersourcedecisionmanager {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData>
    for Cybersourcedecisionmanager
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for Cybersourcedecisionmanager
{
}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Cybersourcedecisionmanager
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Cybersourcedecisionmanager {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData>
    for Cybersourcedecisionmanager {}
impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData>
    for Cybersourcedecisionmanager {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData>
    for Cybersourcedecisionmanager {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData>
    for Cybersourcedecisionmanager {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Cybersourcedecisionmanager {}

#[cfg(feature = "frm")]
impl FraudCheck for Cybersourcedecisionmanager {}
#[cfg(feature = "frm")]
impl FraudCheckCheckout for Cybersourcedecisionmanager {}
#[cfg(feature = "frm")]
impl FraudCheckTransaction for Cybersourcedecisionmanager {}


#[cfg(feature = "frm")]
impl ConnectorIntegration<Checkout, FraudCheckCheckoutData, FraudCheckResponseData> for Cybersourcedecisionmanager {
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
        Ok(format!("{}{}", self.base_url(connectors), "risk/v1/decisions"))
    }

    fn get_request_body(
        &self,
        req: &FrmCheckoutRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let currency =
            req.request
                .currency
                .ok_or(ConnectorError::MissingRequiredField {
                    field_name: "Currency",
                })?;
        let amount = convert_amount(self.amount_converter, MinorUnit::new(req.request.amount), currency)?;

        let connector_router_data = cybersourcedecisionmanager::CybersourcedecisionmanagerRouterData::from((amount, req));
        let req_obj = cybersourcedecisionmanager::CybersourcedecisionmanagerCheckoutRequest::try_from(&connector_router_data)?;
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
        let response: cybersourcedecisionmanager::CybersourcedecisionmanagerResponse = res
            .response
            .parse_struct("CybersourcedecisionmanagerPaymentsResponse")
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

impl ConnectorIntegration<Transaction, FraudCheckTransactionData, FraudCheckResponseData> for Cybersourcedecisionmanager {
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
        let id = req.request.connector_transaction_id.clone().ok_or(ConnectorError::MissingRequiredField {
            field_name: "connector_transaction_id",
        })?;
        Ok(format!("{}risk/v1/decisions/{}/actions", self.base_url(connectors), id))
    }

    fn get_request_body(
        &self,
        req: &FrmTransactionRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let req_obj = cybersourcedecisionmanager::CybersourcedecisionmanagerTransactionRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
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
        let response: cybersourcedecisionmanager::CybersourcedecisionmanagerResponse = res
            .response
            .parse_struct("CybersourcedecisionmanagerPaymentsResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        <FrmTransactionRouterData>::try_from(ResponseRouterData {
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


#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Cybersourcedecisionmanager {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, ConnectorError> {
        Err(report!(ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, ConnectorError> {
        Err(report!(ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, ConnectorError> {
        Err(report!(ConnectorError::WebhooksNotImplemented))
    }
}

static CYBERSOURCEDECISIONMANAGER_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(SupportedPaymentMethods::new);

static CYBERSOURCEDECISIONMANAGER_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Cybersourcedecisionmanager",
    description: "Cybersourcedecisionmanager connector",
    connector_type: enums::HyperswitchConnectorCategory::PaymentGateway,
    integration_status: enums::ConnectorIntegrationStatus::Live,
};

static CYBERSOURCEDECISIONMANAGER_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Cybersourcedecisionmanager {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&CYBERSOURCEDECISIONMANAGER_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*CYBERSOURCEDECISIONMANAGER_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&CYBERSOURCEDECISIONMANAGER_SUPPORTED_WEBHOOK_FLOWS)
    }
}
