pub mod transformers;
use std::fmt::Debug;

use common_utils::request::RequestContent;
use error_stack::{report, ResultExt};
use transformers as klarna;

use crate::{
    configs::settings,
    connector::utils as connector_utils,
    consts,
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon},
        domain,
    },
    utils::BytesExt,
};

#[derive(Debug, Clone)]
pub struct Klarna;

impl ConnectorCommon for Klarna {
    fn id(&self) -> &'static str {
        "klarna"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.klarna.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = klarna::KlarnaAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.basic_token.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: klarna::KlarnaErrorResponse = res
            .response
            .parse_struct("KlarnaErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        // KlarnaErrorResponse will either have error_messages or error_message field Ref: https://docs.klarna.com/api/errors/
        let reason = response
            .error_messages
            .map(|messages| messages.join(" & "))
            .or(response.error_message);
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: consts::NO_ERROR_MESSAGE.to_string(),
            reason,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Klarna {}

impl api::Payment for Klarna {}

impl api::PaymentAuthorize for Klarna {}
impl api::PaymentSync for Klarna {}
impl api::PaymentVoid for Klarna {}
impl api::PaymentCapture for Klarna {}
impl api::PaymentSession for Klarna {}
impl api::ConnectorAccessToken for Klarna {}
impl api::PaymentToken for Klarna {}

impl
    services::ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Klarna
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    > for Klarna
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Klarna
{
    fn get_headers(
        &self,
        req: &types::PaymentsSessionRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsSessionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "payments/v1/sessions"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSessionRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = klarna::KlarnaSessionRequest::try_from(req)?;
        // encode only for for urlencoded things.
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSessionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsSessionType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSessionType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsSessionType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSessionRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSessionRouterData, errors::ConnectorError> {
        let response: klarna::KlarnaSessionResponse = res
            .response
            .parse_struct("KlarnaSessionResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::MandateSetup for Klarna {}

impl
    services::ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Klarna
{
    // Not Implemented(R)
    fn build_request(
        &self,
        _req: &types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Klarna".to_string())
                .into(),
        )
    }
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Klarna
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Klarna
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Klarna
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let payment_method_data = &req.request.payment_method_data;
        let payment_experience = req
            .request
            .payment_experience
            .as_ref()
            .ok_or_else(connector_utils::missing_field_err("payment_experience"))?;
        let payment_method_type = req
            .request
            .payment_method_type
            .as_ref()
            .ok_or_else(connector_utils::missing_field_err("payment_method_type"))?;

        match payment_method_data {
            domain::PaymentMethodData::PayLater(domain::PayLaterData::KlarnaSdk { token }) => {
                match (payment_experience, payment_method_type) {
                    (
                        common_enums::PaymentExperience::InvokeSdkClient,
                        common_enums::PaymentMethodType::Klarna,
                    ) => Ok(format!(
                        "{}payments/v1/authorizations/{}/order",
                        self.base_url(connectors),
                        token
                    )),
                    (
                        common_enums::PaymentExperience::DisplayQrCode
                        | common_enums::PaymentExperience::DisplayWaitScreen
                        | common_enums::PaymentExperience::InvokePaymentApp
                        | common_enums::PaymentExperience::InvokeSdkClient
                        | common_enums::PaymentExperience::LinkWallet
                        | common_enums::PaymentExperience::OneClick
                        | common_enums::PaymentExperience::RedirectToUrl,
                        common_enums::PaymentMethodType::Ach
                        | common_enums::PaymentMethodType::Affirm
                        | common_enums::PaymentMethodType::AfterpayClearpay
                        | common_enums::PaymentMethodType::Alfamart
                        | common_enums::PaymentMethodType::AliPay
                        | common_enums::PaymentMethodType::AliPayHk
                        | common_enums::PaymentMethodType::Alma
                        | common_enums::PaymentMethodType::ApplePay
                        | common_enums::PaymentMethodType::Atome
                        | common_enums::PaymentMethodType::Bacs
                        | common_enums::PaymentMethodType::BancontactCard
                        | common_enums::PaymentMethodType::Becs
                        | common_enums::PaymentMethodType::Benefit
                        | common_enums::PaymentMethodType::Bizum
                        | common_enums::PaymentMethodType::Blik
                        | common_enums::PaymentMethodType::Boleto
                        | common_enums::PaymentMethodType::BcaBankTransfer
                        | common_enums::PaymentMethodType::BniVa
                        | common_enums::PaymentMethodType::BriVa
                        | common_enums::PaymentMethodType::CardRedirect
                        | common_enums::PaymentMethodType::CimbVa
                        | common_enums::PaymentMethodType::ClassicReward
                        | common_enums::PaymentMethodType::Credit
                        | common_enums::PaymentMethodType::CryptoCurrency
                        | common_enums::PaymentMethodType::Cashapp
                        | common_enums::PaymentMethodType::Dana
                        | common_enums::PaymentMethodType::DanamonVa
                        | common_enums::PaymentMethodType::Debit
                        | common_enums::PaymentMethodType::Efecty
                        | common_enums::PaymentMethodType::Eps
                        | common_enums::PaymentMethodType::Evoucher
                        | common_enums::PaymentMethodType::Giropay
                        | common_enums::PaymentMethodType::Givex
                        | common_enums::PaymentMethodType::GooglePay
                        | common_enums::PaymentMethodType::GoPay
                        | common_enums::PaymentMethodType::Gcash
                        | common_enums::PaymentMethodType::Ideal
                        | common_enums::PaymentMethodType::Interac
                        | common_enums::PaymentMethodType::Indomaret
                        | common_enums::PaymentMethodType::Klarna
                        | common_enums::PaymentMethodType::KakaoPay
                        | common_enums::PaymentMethodType::MandiriVa
                        | common_enums::PaymentMethodType::Knet
                        | common_enums::PaymentMethodType::MbWay
                        | common_enums::PaymentMethodType::MobilePay
                        | common_enums::PaymentMethodType::Momo
                        | common_enums::PaymentMethodType::MomoAtm
                        | common_enums::PaymentMethodType::Multibanco
                        | common_enums::PaymentMethodType::OnlineBankingThailand
                        | common_enums::PaymentMethodType::OnlineBankingCzechRepublic
                        | common_enums::PaymentMethodType::OnlineBankingFinland
                        | common_enums::PaymentMethodType::OnlineBankingFpx
                        | common_enums::PaymentMethodType::OnlineBankingPoland
                        | common_enums::PaymentMethodType::OnlineBankingSlovakia
                        | common_enums::PaymentMethodType::Oxxo
                        | common_enums::PaymentMethodType::PagoEfectivo
                        | common_enums::PaymentMethodType::PermataBankTransfer
                        | common_enums::PaymentMethodType::OpenBankingUk
                        | common_enums::PaymentMethodType::PayBright
                        | common_enums::PaymentMethodType::Paypal
                        | common_enums::PaymentMethodType::Pix
                        | common_enums::PaymentMethodType::PaySafeCard
                        | common_enums::PaymentMethodType::Przelewy24
                        | common_enums::PaymentMethodType::Pse
                        | common_enums::PaymentMethodType::RedCompra
                        | common_enums::PaymentMethodType::RedPagos
                        | common_enums::PaymentMethodType::SamsungPay
                        | common_enums::PaymentMethodType::Sepa
                        | common_enums::PaymentMethodType::Sofort
                        | common_enums::PaymentMethodType::Swish
                        | common_enums::PaymentMethodType::TouchNGo
                        | common_enums::PaymentMethodType::Trustly
                        | common_enums::PaymentMethodType::Twint
                        | common_enums::PaymentMethodType::UpiCollect
                        | common_enums::PaymentMethodType::Venmo
                        | common_enums::PaymentMethodType::Vipps
                        | common_enums::PaymentMethodType::Walley
                        | common_enums::PaymentMethodType::WeChatPay
                        | common_enums::PaymentMethodType::SevenEleven
                        | common_enums::PaymentMethodType::Lawson
                        | common_enums::PaymentMethodType::LocalBankTransfer
                        | common_enums::PaymentMethodType::MiniStop
                        | common_enums::PaymentMethodType::FamilyMart
                        | common_enums::PaymentMethodType::Seicomart
                        | common_enums::PaymentMethodType::PayEasy,
                    ) => Err(error_stack::report!(errors::ConnectorError::NotSupported {
                        message: payment_method_type.to_string(),
                        connector: "klarna",
                    })),
                }
            }

            domain::PaymentMethodData::Card(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::Wallet(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::CardToken(_) => Err(error_stack::report!(
                errors::ConnectorError::MismatchedPaymentData
            )),
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = klarna::KlarnaRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = klarna::KlarnaPaymentsRequest::try_from(&connector_router_data)?;
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
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: klarna::KlarnaPaymentsResponse = res
            .response
            .parse_struct("KlarnaPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Klarna
{
}

impl api::Refund for Klarna {}
impl api::RefundExecute for Klarna {}
impl api::RefundSync for Klarna {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Klarna
{
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Klarna
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Klarna {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Ok(api::IncomingWebhookEvent::EventNotSupported)
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}
