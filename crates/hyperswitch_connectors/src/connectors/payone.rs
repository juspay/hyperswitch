pub mod transformers;

use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use base64::Engine;
#[cfg(feature = "payouts")]
use common_utils::request::RequestContent;
use common_utils::{consts::BASE64_ENGINE, errors::CustomResult, ext_traits::BytesExt};
#[cfg(feature = "payouts")]
use common_utils::{
    request::{Method, Request, RequestBuilder},
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use error_stack::{report, ResultExt};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_data::RouterData,
    router_flow_types::PoFulfill,
    types::{PayoutsData, PayoutsResponseData, PayoutsRouterData},
};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse},
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
        ConnectorSpecifications, ConnectorValidation, CurrencyUnit, MandateSetup, Payment,
        PaymentAuthorize, PaymentCapture, PaymentSession, PaymentSync, PaymentToken, PaymentVoid,
        Refund, RefundExecute, RefundSync,
    },
    configs::Connectors,
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors::ConnectorError,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::{
    api::{PayoutFulfill, Payouts},
    types::PayoutFulfillType,
};
use masking::{ExposeInterface, Mask, Maskable, PeekInterface};
use ring::hmac;
#[cfg(feature = "payouts")]
use router_env::{instrument, tracing};

use self::transformers as payone;
#[cfg(feature = "payouts")]
use crate::constants::headers::DATE;
#[cfg(feature = "payouts")]
use crate::get_formatted_date_time;
use crate::{
    constants::headers::AUTHORIZATION,
    utils::{
        get_error_code_error_message_based_on_priority, ConnectorErrorType,
        ConnectorErrorTypeMapping,
    },
};
#[cfg(feature = "payouts")]
use crate::{types::ResponseRouterData, utils::convert_amount};
#[derive(Clone)]
pub struct Payone {
    #[cfg(feature = "payouts")]
    amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync),
}

impl Payone {
    pub fn new() -> &'static Self {
        &Self {
            #[cfg(feature = "payouts")]
            amount_converter: &MinorUnitForConnector,
        }
    }
    pub fn generate_signature(
        &self,
        auth: payone::PayoneAuthType,
        http_method: String,
        canonicalized_path: String,
        content_type: String,
        date_header: String,
    ) -> CustomResult<String, ConnectorError> {
        let payone::PayoneAuthType {
            api_key,
            api_secret,
            ..
        } = auth;
        let string_to_hash: String = format!(
            "{}\n{}\n{}\n{}\n",
            http_method,
            content_type.trim(),
            date_header.trim(),
            canonicalized_path.trim()
        );
        let key = hmac::Key::new(hmac::HMAC_SHA256, api_secret.expose().as_bytes());
        let hash_hmac = BASE64_ENGINE.encode(hmac::sign(&key, string_to_hash.as_bytes()));
        let signature_header = format!("GCS v1HMAC:{}:{}", api_key.peek(), hash_hmac);

        Ok(signature_header)
    }
}

impl Payment for Payone {}
impl PaymentSession for Payone {}
impl ConnectorAccessToken for Payone {}
impl MandateSetup for Payone {}
impl PaymentAuthorize for Payone {}
impl PaymentSync for Payone {}
impl PaymentCapture for Payone {}
impl PaymentVoid for Payone {}
impl Refund for Payone {}
impl RefundExecute for Payone {}
impl RefundSync for Payone {}
impl PaymentToken for Payone {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Payone
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Payone
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    #[cfg(feature = "payouts")]
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let auth = payone::PayoneAuthType::try_from(&req.connector_auth_type)?;
        let http_method = self.get_http_method().to_string();
        let content_type = Self::get_content_type(self);
        let base_url = self.base_url(connectors);
        let url = Self::get_url(self, req, connectors)?;
        let date_header = get_formatted_date_time!(
            "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT"
        )?;
        let path: String = url.replace(base_url, "/");
        let authorization_header: String = self.generate_signature(
            auth,
            http_method,
            path,
            content_type.to_string(),
            date_header.clone(),
        )?;
        let headers = vec![
            (DATE.to_string(), date_header.to_string().into()),
            (
                AUTHORIZATION.to_string(),
                authorization_header.to_string().into(),
            ),
        ];

        Ok(headers)
    }
}

impl ConnectorCommon for Payone {
    fn id(&self) -> &'static str {
        "payone"
    }

    fn get_currency_unit(&self) -> CurrencyUnit {
        CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.payone.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        let auth = payone::PayoneAuthType::try_from(auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            AUTHORIZATION.to_string(),
            auth.api_key.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        let response: payone::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        let errors_list = response.errors.clone().unwrap_or_default();
        let option_error_code_message = get_error_code_error_message_based_on_priority(
            self.clone(),
            errors_list
                .into_iter()
                .map(|errors| errors.into())
                .collect(),
        );
        match response.errors {
            Some(errors) => Ok(ErrorResponse {
                status_code: res.status_code,
                code: option_error_code_message
                    .clone()
                    .map(|error_code_message| error_code_message.error_code)
                    .unwrap_or(NO_ERROR_CODE.to_string()),
                message: option_error_code_message
                    .clone()
                    .map(|error_code_message| error_code_message.error_code)
                    .unwrap_or(NO_ERROR_CODE.to_string()),
                reason: Some(
                    errors
                        .iter()
                        .map(|error| format!("{} : {}", error.code, error.message))
                        .collect::<Vec<String>>()
                        .join(", "),
                ),
                attempt_status: None,
                connector_transaction_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            }),
            None => Ok(ErrorResponse {
                status_code: res.status_code,
                code: NO_ERROR_CODE.to_string(),
                message: NO_ERROR_MESSAGE.to_string(),
                reason: None,
                attempt_status: None,
                connector_transaction_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            }),
        }
    }
}
impl ConnectorValidation for Payone {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Payone {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Payone {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Payone {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Payone {}

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Payone {}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Payone {}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Payone {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Payone {}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Payone {}
#[cfg(feature = "payouts")]
impl Payouts for Payone {}

#[cfg(feature = "payouts")]
impl PayoutFulfill for Payone {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData> for Payone {
    fn get_url(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        _connectors: &Connectors,
    ) -> CustomResult<String, ConnectorError> {
        let auth = payone::PayoneAuthType::try_from(&req.connector_auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;
        Ok(format!(
            "{}v2/{}/payouts",
            self.base_url(_connectors),
            auth.merchant_account.peek()
        ))
    }

    fn get_headers(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, ConnectorError> {
        let amount = convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.destination_currency,
        )?;
        let connector_router_data = payone::PayoneRouterData::from((amount, req));
        let connector_req = payone::PayonePayoutFulfillRequest::try_from(connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PayoutsRouterData<PoFulfill>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&PayoutFulfillType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(PayoutFulfillType::get_headers(self, req, connectors)?)
            .set_body(PayoutFulfillType::get_request_body(self, req, connectors)?)
            .build();
        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &PayoutsRouterData<PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PayoutsRouterData<PoFulfill>, ConnectorError> {
        let response: payone::PayonePayoutFulfillResponse = res
            .response
            .parse_struct("PayonePayoutFulfillResponse")
            .change_context(ConnectorError::ResponseDeserializationFailed)?;

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
    ) -> CustomResult<ErrorResponse, ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl IncomingWebhook for Payone {
    fn get_webhook_object_reference_id(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, ConnectorError> {
        Err(report!(ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        Err(report!(ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, ConnectorError> {
        Err(report!(ConnectorError::WebhooksNotImplemented))
    }
}

impl ConnectorErrorTypeMapping for Payone {
    fn get_connector_error_type(
        &self,
        error_code: String,
        _error_message: String,
    ) -> ConnectorErrorType {
        match error_code.as_str() {
            "30001101" => ConnectorErrorType::BusinessError,
            "30001100" => ConnectorErrorType::BusinessError,
            "30001102" => ConnectorErrorType::BusinessError,
            "30001104" => ConnectorErrorType::BusinessError,
            "30001105" => ConnectorErrorType::BusinessError,
            "30001106" => ConnectorErrorType::TechnicalError,
            "30001120" => ConnectorErrorType::BusinessError,
            "30001130" => ConnectorErrorType::BusinessError,
            "30001140" => ConnectorErrorType::BusinessError,
            "30001141" => ConnectorErrorType::BusinessError,
            "30001142" => ConnectorErrorType::BusinessError,
            "30001143" => ConnectorErrorType::BusinessError,
            "30001158" => ConnectorErrorType::UserError,
            "30001180" => ConnectorErrorType::TechnicalError,
            "30031001" => ConnectorErrorType::UserError,
            "30041001" => ConnectorErrorType::UserError,
            "30051001" => ConnectorErrorType::BusinessError,
            "30141001" => ConnectorErrorType::UserError,
            "30431001" => ConnectorErrorType::UserError,
            "30511001" => ConnectorErrorType::UserError,
            "30581001" => ConnectorErrorType::UserError,
            "30591001" => ConnectorErrorType::BusinessError,
            "30621001" => ConnectorErrorType::BusinessError,
            "30921001" => ConnectorErrorType::TechnicalError,
            "40001134" => ConnectorErrorType::BusinessError,
            "40001135" => ConnectorErrorType::BusinessError,
            "50001081" => ConnectorErrorType::TechnicalError,
            "40001137" => ConnectorErrorType::TechnicalError,
            "40001138" => ConnectorErrorType::TechnicalError,
            "40001139" => ConnectorErrorType::UserError,
            "50001054" => ConnectorErrorType::TechnicalError,
            "50001087" => ConnectorErrorType::TechnicalError,
            _ => ConnectorErrorType::UnknownError,
        }
    }
}

static PAYONE_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Payone",
    description: "Payone payout connector for European market disbursements and automated fund distribution with comprehensive compliance support",
    connector_type: common_enums::HyperswitchConnectorCategory::PayoutProcessor,
    integration_status: common_enums::ConnectorIntegrationStatus::Sandbox,
};

impl ConnectorSpecifications for Payone {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&PAYONE_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        None
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::enums::EventClass]> {
        None
    }
}
