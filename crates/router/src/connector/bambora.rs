pub mod transformers;

use std::fmt::Debug;

use common_utils::request::RequestContent;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use transformers as bambora;

use super::utils::RefundsRequestData;
use crate::{
    configs::settings,
    connector::{
        utils as connector_utils,
        utils::{to_connector_meta, PaymentsAuthorizeRequestData, PaymentsSyncRequestData},
    },
    core::{
        errors::{self, CustomResult},
        payments,
    },
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
pub struct Bambora;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Bambora
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
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
}

impl ConnectorCommon for Bambora {
    fn id(&self) -> &'static str {
        "bambora"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.bambora.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: bambora::BamboraAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: bambora::BamboraErrorResponse = res
            .response
            .parse_struct("BamboraErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code.to_string(),
            message: response.message,
            reason: Some(serde_json::to_string(&response.details).unwrap_or_default()),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Bambora {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic | enums::CaptureMethod::Manual => Ok(()),
            enums::CaptureMethod::ManualMultiple | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }

    fn validate_mandate_payment(
        &self,
        pm_type: Option<types::storage::enums::PaymentMethodType>,
        pm_data: api_models::payments::PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        match pm_data {
            api_models::payments::PaymentMethodData::Card(_) => Err(
                connector_utils::construct_mandate_not_implemented_error(pm_type, self.id()),
            ),
            api_models::payments::PaymentMethodData::Wallet(wallet) => match wallet {
                api_models::payments::WalletData::PaypalRedirect(_) => Err(
                    connector_utils::construct_mandate_not_implemented_error(pm_type, self.id()),
                ),
                api_models::payments::WalletData::GooglePay(_)
                | api_models::payments::WalletData::ApplePay(_)
                | api_models::payments::WalletData::KakaoPayRedirect(_)
                | api_models::payments::WalletData::DanaRedirect {}
                | api_models::payments::WalletData::GcashRedirect(_)
                | api_models::payments::WalletData::TouchNGoRedirect(_)
                | api_models::payments::WalletData::AliPayRedirect(_)
                | api_models::payments::WalletData::MbWayRedirect(_)
                | api_models::payments::WalletData::AliPayHkRedirect(_)
                | api_models::payments::WalletData::WeChatPayQr(_)
                | api_models::payments::WalletData::MomoRedirect(_)
                | api_models::payments::WalletData::GoPayRedirect(_)
                | api_models::payments::WalletData::MobilePayRedirect(_)
                | api_models::payments::WalletData::SamsungPay(_)
                | api_models::payments::WalletData::TwintRedirect { .. }
                | api_models::payments::WalletData::VippsRedirect { .. }
                | api_models::payments::WalletData::CashappQr(_)
                | api_models::payments::WalletData::SwishQr(_)
                | api_models::payments::WalletData::ApplePayRedirect(_)
                | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
                | api_models::payments::WalletData::GooglePayRedirect(_)
                | api_models::payments::WalletData::GooglePayThirdPartySdk(_)
                | api_models::payments::WalletData::WeChatPayRedirect(_)
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
                api_models::payments::BankDebitData::AchBankDebit { .. } => Err(
                    connector_utils::construct_mandate_not_implemented_error(pm_type, self.id()),
                ),
                api_models::payments::BankDebitData::SepaBankDebit { .. }
                | api_models::payments::BankDebitData::BecsBankDebit { .. }
                | api_models::payments::BankDebitData::BacsBankDebit { .. } => Err(
                    connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
                ),
            },
            api_models::payments::PaymentMethodData::BankTransfer(bank_transfer) => {
                match *bank_transfer {
                    api_models::payments::BankTransferData::AchBankTransfer { .. } => {
                        Err(connector_utils::construct_mandate_not_implemented_error(
                            pm_type,
                            self.id(),
                        ))
                    }
                    api_models::payments::BankTransferData::BacsBankTransfer { .. }
                    | api_models::payments::BankTransferData::MultibancoBankTransfer { .. }
                    | api_models::payments::BankTransferData::BcaBankTransfer { .. }
                    | api_models::payments::BankTransferData::SepaBankTransfer { .. }
                    | api_models::payments::BankTransferData::PermataBankTransfer { .. }
                    | api_models::payments::BankTransferData::BniVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::BriVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::CimbVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::DanamonVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::MandiriVaBankTransfer { .. }
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
            api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::CardToken(_) => Err(
                connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
            ),
        }
    }
}

impl api::Payment for Bambora {}

impl api::PaymentToken for Bambora {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Bambora
{
    // Not Implemented (R)
}

impl api::MandateSetup for Bambora {}
impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Bambora
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
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Bambora".to_string())
                .into(),
        )
    }
}

impl api::PaymentVoid for Bambora {}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Bambora
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
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}/v1/payments/{}{}",
            self.base_url(connectors),
            connector_payment_id,
            "/completions"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = bambora::BamboraPaymentsRequest::try_from(req)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
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
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: bambora::BamboraResponse = res
            .response
            .parse_struct("bambora PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            bambora::PaymentFlow::Void,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::ConnectorAccessToken for Bambora {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Bambora
{
}

impl api::PaymentSync for Bambora {}
impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Bambora
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
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(format!(
            "{}{}{}",
            self.base_url(connectors),
            "/v1/payments/",
            connector_payment_id
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: bambora::BamboraResponse = res
            .response
            .parse_struct("bambora PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            get_payment_flow(data.request.is_auto_capture()?),
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
}

impl api::PaymentCapture for Bambora {}
impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Bambora
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
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}{}{}",
            self.base_url(connectors),
            "/v1/payments/",
            req.request.connector_transaction_id,
            "/completions"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = bambora::BamboraPaymentsCaptureRequest::try_from(req)?;
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
                .set_body(self.get_request_body(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: bambora::BamboraResponse = res
            .response
            .parse_struct("Bambora PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(bamborapayments_create_response=?response);
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            bambora::PaymentFlow::Capture,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::PaymentSession for Bambora {}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Bambora
{
    //TODO: implement sessions flow
}

impl api::PaymentAuthorize for Bambora {}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Bambora
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
        Ok(format!("{}{}", self.base_url(connectors), "/v1/payments"))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = bambora::BamboraPaymentsRequest::try_from(req)?;

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
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: bambora::BamboraResponse = res
            .response
            .parse_struct("PaymentIntentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(bamborapayments_create_response=?response);
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            get_payment_flow(data.request.is_auto_capture()?),
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Bambora {}
impl api::RefundExecute for Bambora {}
impl api::RefundSync for Bambora {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Bambora
{
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
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}{}{}{}",
            self.base_url(connectors),
            "/v1/payments/",
            connector_payment_id,
            "/returns"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = bambora::BamboraRefundRequest::try_from(req)?;
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
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: bambora::RefundResponse = res
            .response
            .parse_struct("bambora RefundResponse")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        types::RefundsRouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Bambora {
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
        let _connector_payment_id = req.request.connector_transaction_id.clone();
        let connector_refund_id = req.request.get_connector_refund_id()?;
        Ok(format!(
            "{}{}{}",
            self.base_url(connectors),
            "/v1/payments/",
            connector_refund_id
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
        res: Response,
    ) -> CustomResult<types::RefundSyncRouterData, errors::ConnectorError> {
        let response: bambora::RefundResponse = res
            .response
            .parse_struct("bambora RefundResponse")
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
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Bambora {
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

pub fn get_payment_flow(is_auto_capture: bool) -> bambora::PaymentFlow {
    if is_auto_capture {
        bambora::PaymentFlow::Capture
    } else {
        bambora::PaymentFlow::Authorize
    }
}

impl services::ConnectorRedirectResponse for Bambora {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        action: services::PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        match action {
            services::PaymentAction::PSync | services::PaymentAction::CompleteAuthorize => {
                Ok(payments::CallConnectorAction::Trigger)
            }
        }
    }
}

impl api::PaymentsCompleteAuthorize for Bambora {}

impl
    ConnectorIntegration<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for Bambora
{
    fn get_headers(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let meta: bambora::BamboraMeta = to_connector_meta(req.request.connector_meta.clone())?;
        Ok(format!(
            "{}/v1/payments/{}{}",
            self.base_url(connectors),
            meta.three_d_session_data,
            "/continue"
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = bambora::BamboraThreedsContinueRequest::try_from(&req.request)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PaymentsCompleteAuthorizeType::get_url(
                self, req, connectors,
            )?)
            .headers(types::PaymentsCompleteAuthorizeType::get_headers(
                self, req, connectors,
            )?)
            .set_body(types::PaymentsCompleteAuthorizeType::get_request_body(
                self, req, connectors,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCompleteAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCompleteAuthorizeRouterData, errors::ConnectorError> {
        let response: bambora::BamboraResponse = res
            .response
            .parse_struct("Bambora PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::debug!(bamborapayments_create_response=?response);
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            bambora::PaymentFlow::Capture,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}
