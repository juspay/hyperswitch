pub mod transformers;

use std::fmt::Debug;

use common_utils::{
    crypto::{self, SignMessage},
    date_time,
    request::RequestContent,
};
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use hex::encode;
use masking::PeekInterface;
use transformers as dlocal;

use crate::{
    configs::settings,
    connector::utils as connector_utils,
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers, logger,
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
    utils::BytesExt,
};

#[derive(Debug, Clone)]
pub struct Dlocal;

impl api::Payment for Dlocal {}
impl api::PaymentToken for Dlocal {}
impl api::PaymentSession for Dlocal {}
impl api::ConnectorAccessToken for Dlocal {}
impl api::MandateSetup for Dlocal {}
impl api::PaymentAuthorize for Dlocal {}
impl api::PaymentSync for Dlocal {}
impl api::PaymentCapture for Dlocal {}
impl api::PaymentVoid for Dlocal {}
impl api::Refund for Dlocal {}
impl api::RefundExecute for Dlocal {}
impl api::RefundSync for Dlocal {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Dlocal
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, services::request::Maskable<String>)>, errors::ConnectorError>
    {
        let dlocal_req = self.get_request_body(req, connectors)?;

        let date = date_time::date_as_yyyymmddthhmmssmmmz()
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let auth = dlocal::DlocalAuthType::try_from(&req.connector_auth_type)?;
        let sign_req: String = format!(
            "{}{}{}",
            auth.x_login.peek(),
            date,
            types::RequestBody::get_inner_value(dlocal_req)
                .peek()
                .to_owned()
        );
        let authz = crypto::HmacSha256::sign_message(
            &crypto::HmacSha256,
            auth.secret.peek().as_bytes(),
            sign_req.as_bytes(),
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to sign the message")?;
        let auth_string: String = format!("V2-HMAC-SHA256, Signature: {}", encode(authz));
        let headers = vec![
            (
                headers::AUTHORIZATION.to_string(),
                auth_string.into_masked(),
            ),
            (headers::X_LOGIN.to_string(), auth.x_login.into_masked()),
            (
                headers::X_TRANS_KEY.to_string(),
                auth.x_trans_key.into_masked(),
            ),
            (headers::X_VERSION.to_string(), "2.1".to_string().into()),
            (headers::X_DATE.to_string(), date.into()),
            (
                headers::CONTENT_TYPE.to_string(),
                Self.get_content_type().to_string().into(),
            ),
        ];
        Ok(headers)
    }
}

impl ConnectorCommon for Dlocal {
    fn id(&self) -> &'static str {
        "dlocal"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.dlocal.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: dlocal::DlocalErrorResponse = res
            .response
            .parse_struct("Dlocal ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i: &mut ConnectorEvent| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code.to_string(),
            message: response.message,
            reason: response.param,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Dlocal {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_supported_error_report(capture_method, self.id()),
            ),
        }
    }

    fn validate_mandate_payment(
        &self,
        pm_type: Option<types::storage::enums::PaymentMethodType>,
        pm_data: api_models::payments::PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        match pm_data {
            api_models::payments::PaymentMethodData::Card(_) => Ok(()),
            api_models::payments::PaymentMethodData::Wallet(wallet) => match wallet {
                api_models::payments::WalletData::GooglePay(_) 
                | api_models::payments::WalletData::DanaRedirect {}
                => Err(
                    connector_utils::construct_mandate_not_implemented_error(pm_type, self.id()),
                ),

                api_models::payments::WalletData::AliPayRedirect(_)
                | api_models::payments::WalletData::PaypalRedirect(_) 
                | api_models::payments::WalletData::ApplePay(_) 
                | api_models::payments::WalletData::WeChatPayRedirect(_)
                | api_models::payments::WalletData::SamsungPay(_)
                | api_models::payments::WalletData::KakaoPayRedirect(_)
                | api_models::payments::WalletData::GcashRedirect(_)
                | api_models::payments::WalletData::TouchNGoRedirect(_)
                | api_models::payments::WalletData::MbWayRedirect(_)
                | api_models::payments::WalletData::AliPayHkRedirect(_)
                | api_models::payments::WalletData::WeChatPayQr(_)
                | api_models::payments::WalletData::MomoRedirect(_)
                | api_models::payments::WalletData::GoPayRedirect(_)
                | api_models::payments::WalletData::MobilePayRedirect(_)
                | api_models::payments::WalletData::TwintRedirect { .. }
                | api_models::payments::WalletData::VippsRedirect { .. }
                | api_models::payments::WalletData::CashappQr(_)
                | api_models::payments::WalletData::SwishQr(_)
                | api_models::payments::WalletData::ApplePayRedirect(_)
                | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
                | api_models::payments::WalletData::GooglePayRedirect(_)
                | api_models::payments::WalletData::GooglePayThirdPartySdk(_)
                | api_models::payments::WalletData::AliPayQr(_)
                | api_models::payments::WalletData::PaypalSdk(_) => Err(
                    connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
                ),
            },
            api_models::payments::PaymentMethodData::CardRedirect(card_redirect) => {
                match card_redirect {
                    api_models::payments::CardRedirectData::Knet {}
                    | api_models::payments::CardRedirectData::Benefit {}
                    | api_models::payments::CardRedirectData::MomoAtm {}
                    | api_models::payments::CardRedirectData::CardRedirect {} => Err(
                        connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
                    ),
                }
            }
            api_models::payments::PaymentMethodData::PayLater(pay_later) => match pay_later {
                api_models::payments::PayLaterData::AffirmRedirect {} 
                | api_models::payments::PayLaterData::PayBrightRedirect {}
                | api_models::payments::PayLaterData::WalleyRedirect {}
                | api_models::payments::PayLaterData::KlarnaRedirect { .. }
                | api_models::payments::PayLaterData::KlarnaSdk { .. }
                | api_models::payments::PayLaterData::AlmaRedirect {}
                | api_models::payments::PayLaterData::AtomeRedirect {}
                | api_models::payments::PayLaterData::AfterpayClearpayRedirect { .. } => Err(
                    connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
                ),
            },

            api_models::payments::PaymentMethodData::BankRedirect(bank_redirect) => {
                match bank_redirect {
                    api_models::payments::BankRedirectData::Sofort { .. }
                    | api_models::payments::BankRedirectData::Ideal { .. }
                    | api_models::payments::BankRedirectData::OnlineBankingCzechRepublic {
                        ..
                    }
                    | api_models::payments::BankRedirectData::OpenBankingUk { .. }
                    | api_models::payments::BankRedirectData::OnlineBankingFinland { .. }
                    | api_models::payments::BankRedirectData::OnlineBankingPoland { .. }
                    | api_models::payments::BankRedirectData::OnlineBankingSlovakia { .. }
                    | api_models::payments::BankRedirectData::OnlineBankingFpx { .. }
                    | api_models::payments::BankRedirectData::Bizum {}
                    | api_models::payments::BankRedirectData::Blik { .. }
                    | api_models::payments::BankRedirectData::Eps { .. }
                    | api_models::payments::BankRedirectData::Giropay { .. }
                    | api_models::payments::BankRedirectData::Przelewy24 { .. }
                    | api_models::payments::BankRedirectData::Interac { .. }
                    | api_models::payments::BankRedirectData::Trustly { .. }
                    | api_models::payments::BankRedirectData::OnlineBankingThailand { .. }
                    | api_models::payments::BankRedirectData::BancontactCard { .. } => Err(
                        connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
                    ),
                }
            }
            api_models::payments::PaymentMethodData::BankDebit(bank_debit) => match bank_debit {
                api_models::payments::BankDebitData::SepaBankDebit { .. }
                | api_models::payments::BankDebitData::AchBankDebit { .. }
                | api_models::payments::BankDebitData::BacsBankDebit { .. } => Err(
                    connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
                ),

                api_models::payments::BankDebitData::BecsBankDebit { .. } => Err(
                    connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
                ),
            },
            api_models::payments::PaymentMethodData::BankTransfer(bank_transfer) => {
                match *bank_transfer {
                    api_models::payments::BankTransferData::AchBankTransfer { .. }
                    | api_models::payments::BankTransferData::BcaBankTransfer { .. }
                    | api_models::payments::BankTransferData::PermataBankTransfer { .. }
                    | api_models::payments::BankTransferData::BacsBankTransfer { .. }
                    | api_models::payments::BankTransferData::BniVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::CimbVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::DanamonVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::MandiriVaBankTransfer { .. }
                    => Err(connector_utils::construct_mandate_not_implemented_error(pm_type, self.id())),
                     api_models::payments::BankTransferData::MultibancoBankTransfer { .. }
                    | api_models::payments::BankTransferData::SepaBankTransfer { .. }
                    | api_models::payments::BankTransferData::BriVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::Pix {}
                    | api_models::payments::BankTransferData::Pse {} => Err(
                        connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
                    ),
                }
            }
            api_models::payments::PaymentMethodData::MandatePayment => Ok(()),
            api_models::payments::PaymentMethodData::Voucher(voucher) => match voucher {
                api_models::payments::VoucherData::Boleto(_)
                | api_models::payments::VoucherData::Efecty
                | api_models::payments::VoucherData::PagoEfectivo
                | api_models::payments::VoucherData::RedCompra
                | api_models::payments::VoucherData::RedPagos
                | api_models::payments::VoucherData::Alfamart(_)
                | api_models::payments::VoucherData::Indomaret(_)
                | api_models::payments::VoucherData::Oxxo
                | api_models::payments::VoucherData::Lawson(_)
                | api_models::payments::VoucherData::MiniStop(_)
                | api_models::payments::VoucherData::FamilyMart(_)
                | api_models::payments::VoucherData::Seicomart(_)
                | api_models::payments::VoucherData::PayEasy(_)
                | api_models::payments::VoucherData::SevenEleven(_) => Err(
                    connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
                ),
            },
            api_models::payments::PaymentMethodData::GiftCard(gift_card) => match *gift_card {
                api_models::payments::GiftCardData::Givex(_)
                | api_models::payments::GiftCardData::PaySafeCard {} => Err(
                    connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
                ),
            },
            | api_models::payments::PaymentMethodData::Upi(_) => Err(
                connector_utils::construct_mandate_not_implemented_error(pm_type, self.id()),
            ),
            api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::CardToken(_) => Err(
                connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
            ),
        }
    }
}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Dlocal
{
    // Not Implemented (R)
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Dlocal
{
    //TODO: implement sessions flow
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Dlocal
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Dlocal
{
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
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Dlocal".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Dlocal
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
        Ok(format!("{}secure_payments", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = dlocal::DlocalRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = dlocal::DlocalPaymentsRequest::try_from(&connector_router_data)?;
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
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        logger::debug!(dlocal_payments_authorize_response=?res);
        let response: dlocal::DlocalPaymentsResponse = res
            .response
            .parse_struct("Dlocal PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Dlocal
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
        let sync_data = dlocal::DlocalPaymentsSyncRequest::try_from(req)?;
        Ok(format!(
            "{}payments/{}/status",
            self.base_url(connectors),
            sync_data.authz_id,
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

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        logger::debug!(dlocal_payment_sync_response=?res);
        let response: dlocal::DlocalPaymentsResponse = res
            .response
            .parse_struct("Dlocal PaymentsSyncResponse")
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
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Dlocal
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}payments", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = dlocal::DlocalPaymentsCaptureRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsCaptureType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        logger::debug!(dlocal_payments_capture_response=?res);
        let response: dlocal::DlocalPaymentsResponse = res
            .response
            .parse_struct("Dlocal PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Dlocal
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let cancel_data = dlocal::DlocalPaymentsCancelRequest::try_from(req)?;
        Ok(format!(
            "{}payments/{}/cancel",
            self.base_url(connectors),
            cancel_data.cancel_id
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        logger::debug!(dlocal_payments_cancel_response=?res);
        let response: dlocal::DlocalPaymentsResponse = res
            .response
            .parse_struct("Dlocal PaymentsCancelResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Dlocal {
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
        Ok(format!("{}refunds", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = dlocal::DlocalRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let connector_req = dlocal::DlocalRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
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
            .set_body(types::RefundExecuteType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        logger::debug!(dlocal_refund_response=?res);
        let response: dlocal::RefundResponse =
            res.response
                .parse_struct("Dlocal RefundResponse")
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Dlocal {
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let sync_data = dlocal::DlocalRefundsSyncRequest::try_from(req)?;
        Ok(format!(
            "{}refunds/{}/status",
            self.base_url(connectors),
            sync_data.refund_id,
        ))
    }

    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        logger::debug!(dlocal_refund_sync_response=?res);
        let response: dlocal::RefundResponse = res
            .response
            .parse_struct("Dlocal RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
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
impl api::IncomingWebhook for Dlocal {
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
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
