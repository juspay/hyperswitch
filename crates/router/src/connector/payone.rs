pub mod transformers;

use base64::Engine;
#[cfg(feature = "payouts")]
use common_utils::request::RequestContent;
#[cfg(feature = "payouts")]
use common_utils::types::{AmountConvertor, MinorUnit, MinorUnitForConnector};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, PeekInterface};
use ring::hmac;
#[cfg(feature = "payouts")]
use router_env::{instrument, tracing};

use self::transformers as payone;
#[cfg(feature = "payouts")]
use crate::connector::utils::convert_amount;
#[cfg(feature = "payouts")]
use crate::get_formatted_date_time;
#[cfg(feature = "payouts")]
use crate::services;
use crate::{
    configs::settings,
    connector::{
        utils as connector_utils,
        utils::{ConnectorErrorType, ConnectorErrorTypeMapping},
    },
    consts,
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers,
    services::{
        request::{self, Mask},
        ConnectorIntegration, ConnectorSpecifications, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::BytesExt,
};
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
    ) -> CustomResult<String, errors::ConnectorError> {
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
        let hash_hmac = consts::BASE64_ENGINE.encode(hmac::sign(&key, string_to_hash.as_bytes()));
        let signature_header = format!("GCS v1HMAC:{}:{}", api_key.peek(), hash_hmac);

        Ok(signature_header)
    }
}

impl api::Payment for Payone {}
impl api::PaymentSession for Payone {}
impl api::ConnectorAccessToken for Payone {}
impl api::MandateSetup for Payone {}
impl api::PaymentAuthorize for Payone {}
impl api::PaymentSync for Payone {}
impl api::PaymentCapture for Payone {}
impl api::PaymentVoid for Payone {}
impl api::Refund for Payone {}
impl api::RefundExecute for Payone {}
impl api::RefundSync for Payone {}
impl api::PaymentToken for Payone {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Payone
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Payone
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    #[cfg(feature = "payouts")]
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
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
            (headers::DATE.to_string(), date_header.to_string().into()),
            (
                headers::AUTHORIZATION.to_string(),
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

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.payone.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = payone::PayoneAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.expose().into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: payone::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        let errors_list = response.errors.clone().unwrap_or_default();
        let option_error_code_message =
            connector_utils::get_error_code_error_message_based_on_priority(
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
                    .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                message: option_error_code_message
                    .clone()
                    .map(|error_code_message| error_code_message.error_code)
                    .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                reason: Some(
                    errors
                        .iter()
                        .map(|error| format!("{} : {}", error.code, error.message))
                        .collect::<Vec<String>>()
                        .join(", "),
                ),
                attempt_status: None,
                connector_transaction_id: None,
                issuer_error_code: None,
                issuer_error_message: None,
            }),
            None => Ok(ErrorResponse {
                status_code: res.status_code,
                code: consts::NO_ERROR_CODE.to_string(),
                message: consts::NO_ERROR_MESSAGE.to_string(),
                reason: None,
                attempt_status: None,
                connector_transaction_id: None,
                issuer_error_code: None,
                issuer_error_message: None,
            }),
        }
    }
}
impl ConnectorValidation for Payone {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Payone
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Payone
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Payone
{
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Payone
{
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Payone
{
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Payone
{
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Payone
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Payone {}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Payone {}
impl api::Payouts for Payone {}

#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Payone {}

#[cfg(feature = "payouts")]
impl ConnectorIntegration<api::PoFulfill, types::PayoutsData, types::PayoutsResponseData>
    for Payone
{
    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth = payone::PayoneAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(format!(
            "{}v2/{}/payouts",
            self.base_url(_connectors),
            auth.merchant_account.peek()
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
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
        req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutFulfillType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutFulfillType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PayoutFulfillType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoFulfill>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoFulfill>, errors::ConnectorError> {
        let response: payone::PayonePayoutFulfillResponse = res
            .response
            .parse_struct("PayonePayoutFulfillResponse")
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

#[async_trait::async_trait]
impl api::IncomingWebhook for Payone {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
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

impl ConnectorSpecifications for Payone {}
