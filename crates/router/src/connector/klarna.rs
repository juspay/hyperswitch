pub mod transformers;
use std::fmt::Debug;

use api_models::payments as api_payments;
use error_stack::{IntoReport, ResultExt};
use transformers as klarna;

use crate::{
    configs::settings,
    connector::utils as connector_utils,
    consts,
    core::errors::{self, CustomResult},
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon},
        storage::enums as storage_enums,
    },
    utils::{self, BytesExt},
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
        let auth: klarna::KlarnaAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.basic_token.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: klarna::KlarnaErrorResponse = res
            .response
            .parse_struct("KlarnaErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = klarna::KlarnaSessionRequest::try_from(req)?;
        // encode only for for urlencoded things.
        let klarna_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<klarna::KlarnaSessionRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(klarna_req))
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
                .body(types::PaymentsSessionType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSessionRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSessionRouterData, errors::ConnectorError> {
        let response: klarna::KlarnaSessionResponse = res
            .response
            .parse_struct("KlarnaSessionResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
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
            api_payments::PaymentMethodData::PayLater(api_payments::PayLaterData::KlarnaSdk {
                token,
            }) => match (payment_experience, payment_method_type) {
                (
                    storage_enums::PaymentExperience::InvokeSdkClient,
                    storage_enums::PaymentMethodType::Klarna,
                ) => Ok(format!(
                    "{}payments/v1/authorizations/{}/order",
                    self.base_url(connectors),
                    token
                )),
                (
                    storage_enums::PaymentExperience::DisplayQrCode
                    | storage_enums::PaymentExperience::DisplayWaitScreen
                    | storage_enums::PaymentExperience::InvokePaymentApp
                    | storage_enums::PaymentExperience::InvokeSdkClient
                    | storage_enums::PaymentExperience::LinkWallet
                    | storage_enums::PaymentExperience::OneClick
                    | storage_enums::PaymentExperience::RedirectToUrl,
                    api_models::enums::PaymentMethodType::Ach
                    | api_models::enums::PaymentMethodType::Affirm
                    | api_models::enums::PaymentMethodType::AfterpayClearpay
                    | api_models::enums::PaymentMethodType::Alfamart
                    | api_models::enums::PaymentMethodType::AliPay
                    | api_models::enums::PaymentMethodType::AliPayHk
                    | api_models::enums::PaymentMethodType::Alma
                    | api_models::enums::PaymentMethodType::ApplePay
                    | api_models::enums::PaymentMethodType::Atome
                    | api_models::enums::PaymentMethodType::Bacs
                    | api_models::enums::PaymentMethodType::BancontactCard
                    | api_models::enums::PaymentMethodType::Becs
                    | api_models::enums::PaymentMethodType::Benefit
                    | api_models::enums::PaymentMethodType::Bizum
                    | api_models::enums::PaymentMethodType::Blik
                    | api_models::enums::PaymentMethodType::Boleto
                    | api_models::enums::PaymentMethodType::BcaBankTransfer
                    | api_models::enums::PaymentMethodType::BniVa
                    | api_models::enums::PaymentMethodType::BriVa
                    | api_models::enums::PaymentMethodType::CimbVa
                    | api_models::enums::PaymentMethodType::ClassicReward
                    | api_models::enums::PaymentMethodType::Credit
                    | api_models::enums::PaymentMethodType::CryptoCurrency
                    | api_models::enums::PaymentMethodType::Cashapp
                    | api_models::enums::PaymentMethodType::Dana
                    | api_models::enums::PaymentMethodType::DanamonVa
                    | api_models::enums::PaymentMethodType::Debit
                    | api_models::enums::PaymentMethodType::Efecty
                    | api_models::enums::PaymentMethodType::Eps
                    | api_models::enums::PaymentMethodType::Evoucher
                    | api_models::enums::PaymentMethodType::Giropay
                    | api_models::enums::PaymentMethodType::Givex
                    | api_models::enums::PaymentMethodType::GooglePay
                    | api_models::enums::PaymentMethodType::GoPay
                    | api_models::enums::PaymentMethodType::Gcash
                    | api_models::enums::PaymentMethodType::Ideal
                    | api_models::enums::PaymentMethodType::Interac
                    | api_models::enums::PaymentMethodType::Indomaret
                    | api_models::enums::PaymentMethodType::Klarna
                    | api_models::enums::PaymentMethodType::KakaoPay
                    | api_models::enums::PaymentMethodType::MandiriVa
                    | api_models::enums::PaymentMethodType::Knet
                    | api_models::enums::PaymentMethodType::MbWay
                    | api_models::enums::PaymentMethodType::MobilePay
                    | api_models::enums::PaymentMethodType::Momo
                    | api_models::enums::PaymentMethodType::MomoAtm
                    | api_models::enums::PaymentMethodType::Multibanco
                    | api_models::enums::PaymentMethodType::OnlineBankingThailand
                    | api_models::enums::PaymentMethodType::OnlineBankingCzechRepublic
                    | api_models::enums::PaymentMethodType::OnlineBankingFinland
                    | api_models::enums::PaymentMethodType::OnlineBankingFpx
                    | api_models::enums::PaymentMethodType::OnlineBankingPoland
                    | api_models::enums::PaymentMethodType::OnlineBankingSlovakia
                    | api_models::enums::PaymentMethodType::Oxxo
                    | api_models::enums::PaymentMethodType::PagoEfectivo
                    | api_models::enums::PaymentMethodType::PermataBankTransfer
                    | api_models::enums::PaymentMethodType::OpenBankingUk
                    | api_models::enums::PaymentMethodType::PayBright
                    | api_models::enums::PaymentMethodType::Paypal
                    | api_models::enums::PaymentMethodType::Pix
                    | api_models::enums::PaymentMethodType::PaySafeCard
                    | api_models::enums::PaymentMethodType::Przelewy24
                    | api_models::enums::PaymentMethodType::Pse
                    | api_models::enums::PaymentMethodType::RedCompra
                    | api_models::enums::PaymentMethodType::RedPagos
                    | api_models::enums::PaymentMethodType::SamsungPay
                    | api_models::enums::PaymentMethodType::Sepa
                    | api_models::enums::PaymentMethodType::Sofort
                    | api_models::enums::PaymentMethodType::Swish
                    | api_models::enums::PaymentMethodType::TouchNGo
                    | api_models::enums::PaymentMethodType::Trustly
                    | api_models::enums::PaymentMethodType::Twint
                    | api_models::enums::PaymentMethodType::UpiCollect
                    | api_models::enums::PaymentMethodType::Vipps
                    | api_models::enums::PaymentMethodType::Walley
                    | api_models::enums::PaymentMethodType::WeChatPay
                    | api_models::enums::PaymentMethodType::SevenEleven
                    | api_models::enums::PaymentMethodType::Lawson
                    | api_models::enums::PaymentMethodType::MiniStop
                    | api_models::enums::PaymentMethodType::FamilyMart
                    | api_models::enums::PaymentMethodType::Seicomart
                    | api_models::enums::PaymentMethodType::PayEasy,
                ) => Err(error_stack::report!(errors::ConnectorError::NotSupported {
                    message: payment_method_type.to_string(),
                    connector: "klarna",
                })),
            },

            api_payments::PaymentMethodData::Card(_)
            | api_payments::PaymentMethodData::CardRedirect(_)
            | api_payments::PaymentMethodData::Wallet(_)
            | api_payments::PaymentMethodData::PayLater(_)
            | api_payments::PaymentMethodData::BankRedirect(_)
            | api_payments::PaymentMethodData::BankDebit(_)
            | api_payments::PaymentMethodData::BankTransfer(_)
            | api_payments::PaymentMethodData::Crypto(_)
            | api_payments::PaymentMethodData::MandatePayment
            | api_payments::PaymentMethodData::Reward
            | api_payments::PaymentMethodData::Upi(_)
            | api_payments::PaymentMethodData::Voucher(_)
            | api_payments::PaymentMethodData::GiftCard(_) => Err(error_stack::report!(
                errors::ConnectorError::MismatchedPaymentData
            )),
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = klarna::KlarnaRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = klarna::KlarnaPaymentsRequest::try_from(&connector_router_data)?;
        let klarna_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<klarna::KlarnaPaymentsRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(klarna_req))
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
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: klarna::KlarnaPaymentsResponse = res
            .response
            .parse_struct("KlarnaPaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
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
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
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
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
