pub mod braintree_graphql_transformers;
pub mod transformers;
use std::{fmt::Debug, str::FromStr};

use api_models::webhooks::IncomingWebhookEvent;
use base64::Engine;
use common_utils::{crypto, ext_traits::XmlExt, request::RequestContent};
use diesel_models::enums;
use error_stack::{IntoReport, Report, ResultExt};
use masking::{ExposeInterface, PeekInterface};
use ring::hmac;
use sha1::{Digest, Sha1};

use self::transformers as braintree;
use super::utils::{self as connector_utils, PaymentsAuthorizeRequestData};
use crate::{
    configs::settings,
    consts,
    core::{
        errors::{self, CustomResult},
        payments,
    },
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
        domain,
        transformers::ForeignFrom,
        ErrorResponse,
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Braintree;

pub const BRAINTREE_VERSION: &str = "Braintree-Version";
pub const BRAINTREE_VERSION_VALUE: &str = "2019-01-01";
pub const BRAINTREE_API_VERSION: &str = "graphql_api";

impl Braintree {
    pub fn is_braintree_graphql_version(&self, connector_api_version: &Option<String>) -> bool {
        *connector_api_version == Some(BRAINTREE_API_VERSION.to_string())
    }
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Braintree
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                BRAINTREE_VERSION.to_string(),
                BRAINTREE_VERSION_VALUE.to_string().into(),
            ),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Braintree {
    fn id(&self) -> &'static str {
        "braintree"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.braintree.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: braintree::BraintreeAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.auth_header.into_masked(),
        )])
    }

    fn build_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: Result<braintree::ErrorResponse, Report<common_utils::errors::ParsingError>> =
            res.response.parse_struct("Braintree Error Response");

        match response {
            Ok(braintree::ErrorResponse::BraintreeApiErrorResponse(response)) => {
                event_builder.map(|i| i.set_error_response_body(&response));
                router_env::logger::info!(connector_response=?response);

                let error_object = response.api_error_response.errors;
                let error = error_object.errors.first().or(error_object
                    .transaction
                    .as_ref()
                    .and_then(|transaction_error| {
                        transaction_error.errors.first().or(transaction_error
                            .credit_card
                            .as_ref()
                            .and_then(|credit_card_error| credit_card_error.errors.first()))
                    }));
                let (code, message) = error.map_or(
                    (
                        consts::NO_ERROR_CODE.to_string(),
                        consts::NO_ERROR_MESSAGE.to_string(),
                    ),
                    |error| (error.code.clone(), error.message.clone()),
                );
                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code,
                    message,
                    reason: Some(response.api_error_response.message),
                    attempt_status: None,
                    connector_transaction_id: None,
                })
            }
            Ok(braintree::ErrorResponse::BraintreeErrorResponse(response)) => {
                event_builder.map(|i| i.set_error_response_body(&response));
                router_env::logger::info!(connector_response=?response);

                Ok(ErrorResponse {
                    status_code: res.status_code,
                    code: consts::NO_ERROR_CODE.to_string(),
                    message: consts::NO_ERROR_MESSAGE.to_string(),
                    reason: Some(response.errors),
                    attempt_status: None,
                    connector_transaction_id: None,
                })
            }
            Err(error_msg) => {
                event_builder.map(|event| event.set_error(serde_json::json!({"error": res.response.escape_ascii().to_string(), "status_code": res.status_code})));
                logger::error!(deserialization_error =? error_msg);
                utils::handle_json_response_deserialization_failure(res, "braintree".to_owned())
            }
        }
    }
}

impl ConnectorValidation for Braintree {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _pmt: Option<enums::PaymentMethodType>,
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
                api_models::payments::WalletData::PaypalRedirect(_) 
                | api_models::payments::WalletData::GooglePay(_)
                | api_models::payments::WalletData::ApplePay(_)
                | api_models::payments::WalletData::SamsungPay(_) => Err(
                    connector_utils::construct_mandate_not_implemented_error(pm_type, self.id()),
                ),
                 api_models::payments::WalletData::KakaoPayRedirect(_)
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
                    api_models::payments::BankTransferData::AchBankTransfer { .. }
                    | api_models::payments::BankTransferData::BacsBankTransfer { .. }
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

impl api::Payment for Braintree {}

impl api::PaymentAuthorize for Braintree {}
impl api::PaymentSync for Braintree {}
impl api::PaymentVoid for Braintree {}
impl api::PaymentCapture for Braintree {}
impl api::PaymentsCompleteAuthorize for Braintree {}
impl api::PaymentSession for Braintree {}
impl api::ConnectorAccessToken for Braintree {}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Braintree
{
    // Not Implemented (R)
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsSessionRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                types::PaymentsSessionType::get_content_type(self)
                    .to_string()
                    .into(),
            ),
            (headers::X_API_VERSION.to_string(), "6".to_string().into()),
            (
                headers::ACCEPT.to_string(),
                "application/json".to_string().into(),
            ),
        ];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        headers.append(&mut api_key);
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn get_url(
        &self,
        req: &types::PaymentsSessionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(format!(
            "{}/merchants/{}/client_token",
            self.base_url(connectors),
            auth_type.merchant_id.peek(),
        ))
    }

    fn build_request(
        &self,
        req: &types::PaymentsSessionRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => Ok(None),
            false => {
                let request = Some(
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
                );
                Ok(request)
            }
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSessionRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = braintree::BraintreeSessionRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSessionRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSessionRouterData, errors::ConnectorError> {
        let response: braintree::BraintreeSessionTokenResponse = res
            .response
            .parse_struct("braintree SessionTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
}

impl api::PaymentToken for Braintree {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Braintree
{
    fn get_headers(
        &self,
        req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = connectors
            .braintree
            .secondary_base_url
            .as_ref()
            .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
        Ok(base_url.to_string())
    }

    fn get_request_body(
        &self,
        req: &types::TokenizationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = braintree_graphql_transformers::BraintreeTokenRequest::try_from(req)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::TokenizationRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => Ok(Some(
                services::RequestBuilder::new()
                    .method(services::Method::Post)
                    .url(&types::TokenizationType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::TokenizationType::get_headers(self, req, connectors)?)
                    .set_body(types::TokenizationType::get_request_body(
                        self, req, connectors,
                    )?)
                    .build(),
            )),
            false => Ok(None),
        }
    }

    fn handle_response(
        &self,
        data: &types::TokenizationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::TokenizationRouterData, errors::ConnectorError>
    where
        types::PaymentsResponseData: Clone,
    {
        let response: braintree_graphql_transformers::BraintreeTokenResponse = res
            .response
            .parse_struct("BraintreeTokenResponse")
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
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::MandateSetup for Braintree {}

#[allow(dead_code)]
impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Braintree
{
    // Not Implemented (R)
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
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Braintree".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version.clone();
        match self.is_braintree_graphql_version(connector_api_version) {
            true => self.build_headers(req, connectors),
            false => Ok(vec![]),
        }
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_api_version = req.connector_api_version.clone();
        match self.is_braintree_graphql_version(&connector_api_version) {
            true => {
                let base_url = connectors
                    .braintree
                    .secondary_base_url
                    .as_ref()
                    .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
                Ok(base_url.to_string())
            }
            false => {
                Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
            }
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version.clone();
        let connector_router_data =
            braintree_graphql_transformers::BraintreeRouterData::try_from((
                &self.get_currency_unit(),
                req.request.currency,
                req.request.amount_to_capture,
                req,
            ))?;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let connector_req =
                    braintree_graphql_transformers::BraintreeCaptureRequest::try_from(
                        &connector_router_data,
                    )?;

                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            false => Err(errors::ConnectorError::NotImplemented(
                "get_request_body method".to_string(),
            )
            .into()),
        }
    }

    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => Ok(Some(
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
            )),
            false => Err(errors::ConnectorError::NotImplemented(
                "Capture flow not Implemented".to_string(),
            )
            .into()),
        }
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: braintree_graphql_transformers::BraintreeCaptureResponse = res
            .response
            .parse_struct("Braintree PaymentsCaptureResponse")
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
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => self.build_headers(req, connectors),
            false => {
                let mut headers = vec![
                    (
                        headers::CONTENT_TYPE.to_string(),
                        types::PaymentsSyncType::get_content_type(self)
                            .to_string()
                            .into(),
                    ),
                    (headers::X_API_VERSION.to_string(), "6".to_string().into()),
                    (
                        headers::ACCEPT.to_string(),
                        "application/json".to_string().into(),
                    ),
                ];
                let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
                headers.append(&mut api_key);
                Ok(headers)
            }
        }
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let base_url = connectors
                    .braintree
                    .secondary_base_url
                    .as_ref()
                    .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
                Ok(base_url.to_string())
            }
            false => {
                let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
                let connector_payment_id = req
                    .request
                    .connector_transaction_id
                    .get_connector_transaction_id()
                    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
                Ok(format!(
                    "{}/merchants/{}/transactions/{connector_payment_id}",
                    self.base_url(connectors),
                    auth_type.merchant_id.peek()
                ))
            }
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let connector_req =
                    braintree_graphql_transformers::BraintreePSyncRequest::try_from(req)?;

                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            false => Err(errors::ConnectorError::RequestEncodingFailed).into_report(),
        }
    }

    fn build_request(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => Ok(Some(
                services::RequestBuilder::new()
                    .method(services::Method::Post)
                    .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                    .set_body(types::PaymentsSyncType::get_request_body(
                        self, req, connectors,
                    )?)
                    .build(),
            )),
            false => Ok(Some(
                services::RequestBuilder::new()
                    .method(services::Method::Get)
                    .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                    .set_body(types::PaymentsSyncType::get_request_body(
                        self, req, connectors,
                    )?)
                    .build(),
            )),
        }
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let connector_api_version = &data.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let response: braintree_graphql_transformers::BraintreePSyncResponse = res
                    .response
                    .parse_struct("Braintree PaymentSyncResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            false => {
                let response: braintree::BraintreePaymentsResponse = res
                    .response
                    .parse_struct("Braintree PaymentsResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => self.build_headers(req, connectors),
            false => {
                let mut headers = vec![
                    (
                        headers::CONTENT_TYPE.to_string(),
                        types::PaymentsAuthorizeType::get_content_type(self)
                            .to_string()
                            .into(),
                    ),
                    (headers::X_API_VERSION.to_string(), "6".to_string().into()),
                    (
                        headers::ACCEPT.to_string(),
                        "application/json".to_string().into(),
                    ),
                ];
                let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
                headers.append(&mut api_key);
                Ok(headers)
            }
        }
    }

    fn get_url(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let base_url = connectors
                    .braintree
                    .secondary_base_url
                    .as_ref()
                    .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
                Ok(base_url.to_string())
            }
            false => {
                let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

                Ok(format!(
                    "{}merchants/{}/transactions",
                    self.base_url(connectors),
                    auth_type.merchant_id.peek()
                ))
            }
        }
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

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        let connector_router_data =
            braintree_graphql_transformers::BraintreeRouterData::try_from((
                &self.get_currency_unit(),
                req.request.currency,
                req.request.amount,
                req,
            ))?;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let connector_req =
                    braintree_graphql_transformers::BraintreePaymentsRequest::try_from(
                        &connector_router_data,
                    )?;
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            false => {
                let connector_req = braintree::BraintreePaymentsRequest::try_from(req)?;
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
        }
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let connector_api_version = &data.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => match data.request.is_auto_capture()? {
                true => {
                    let response: braintree_graphql_transformers::BraintreePaymentsResponse = res
                        .response
                        .parse_struct("Braintree PaymentsResponse")
                        .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                    event_builder.map(|i| i.set_response_body(&response));
                    router_env::logger::info!(connector_response=?response);
                    types::RouterData::try_from(types::ResponseRouterData {
                        response,
                        data: data.clone(),
                        http_code: res.status_code,
                    })
                }
                false => {
                    let response: braintree_graphql_transformers::BraintreeAuthResponse = res
                        .response
                        .parse_struct("Braintree AuthResponse")
                        .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                    event_builder.map(|i| i.set_response_body(&response));
                    router_env::logger::info!(connector_response=?response);
                    types::RouterData::try_from(types::ResponseRouterData {
                        response,
                        data: data.clone(),
                        http_code: res.status_code,
                    })
                }
            },
            false => {
                let response: braintree::BraintreePaymentsResponse = res
                    .response
                    .parse_struct("Braintree PaymentsResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => self.build_headers(req, connectors),
            false => {
                let mut headers = vec![
                    (
                        headers::CONTENT_TYPE.to_string(),
                        types::PaymentsVoidType::get_content_type(self)
                            .to_string()
                            .into(),
                    ),
                    (headers::X_API_VERSION.to_string(), "6".to_string().into()),
                    (
                        headers::ACCEPT.to_string(),
                        "application/json".to_string().into(),
                    ),
                ];
                let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
                headers.append(&mut api_key);
                Ok(headers)
            }
        }
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let base_url = connectors
                    .braintree
                    .secondary_base_url
                    .as_ref()
                    .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
                Ok(base_url.to_string())
            }
            false => {
                let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
                Ok(format!(
                    "{}merchants/{}/transactions/{}/void",
                    self.base_url(connectors),
                    auth_type.merchant_id.peek(),
                    req.request.connector_transaction_id
                ))
            }
        }
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
                .set_body(types::PaymentsVoidType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let connector_req =
                    braintree_graphql_transformers::BraintreeCancelRequest::try_from(req)?;
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            false => Err(errors::ConnectorError::RequestEncodingFailed).into_report(),
        }
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let connector_api_version = &data.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let response: braintree_graphql_transformers::BraintreeCancelResponse = res
                    .response
                    .parse_struct("Braintree VoidResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            false => {
                let response: braintree::BraintreePaymentsResponse = res
                    .response
                    .parse_struct("Braintree PaymentsVoidResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl api::Refund for Braintree {}
impl api::RefundExecute for Braintree {}
impl api::RefundSync for Braintree {}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => self.build_headers(req, connectors),
            false => {
                let mut headers = vec![
                    (
                        headers::CONTENT_TYPE.to_string(),
                        types::RefundExecuteType::get_content_type(self)
                            .to_string()
                            .into(),
                    ),
                    (headers::X_API_VERSION.to_string(), "6".to_string().into()),
                    (
                        headers::ACCEPT.to_string(),
                        "application/json".to_string().into(),
                    ),
                ];
                let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
                headers.append(&mut api_key);
                Ok(headers)
            }
        }
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let base_url = connectors
                    .braintree
                    .secondary_base_url
                    .as_ref()
                    .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
                Ok(base_url.to_string())
            }
            false => {
                let auth_type = braintree::BraintreeAuthType::try_from(&req.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
                let connector_payment_id = req.request.connector_transaction_id.clone();
                Ok(format!(
                    "{}merchants/{}/transactions/{}",
                    self.base_url(connectors),
                    auth_type.merchant_id.peek(),
                    connector_payment_id
                ))
            }
        }
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        let connector_router_data =
            braintree_graphql_transformers::BraintreeRouterData::try_from((
                &self.get_currency_unit(),
                req.request.currency,
                req.request.refund_amount,
                req,
            ))?;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let connector_req =
                    braintree_graphql_transformers::BraintreeRefundRequest::try_from(
                        connector_router_data,
                    )?;
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            false => {
                let connector_req = braintree::BraintreeRefundRequest::try_from(req)?;
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
        }
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
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let connector_api_version = &data.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let response: braintree_graphql_transformers::BraintreeRefundResponse = res
                    .response
                    .parse_struct("Braintree RefundResponse")
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            false => {
                let response: braintree::RefundResponse = res
                    .response
                    .parse_struct("Braintree RefundResponse")
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Braintree
{
    fn get_headers(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => self.build_headers(req, connectors),
            false => Ok(vec![]),
        }
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let base_url = connectors
                    .braintree
                    .secondary_base_url
                    .as_ref()
                    .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
                Ok(base_url.to_string())
            }
            false => {
                Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
            }
        }
    }

    fn get_request_body(
        &self,
        req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let connector_req =
                    braintree_graphql_transformers::BraintreeRSyncRequest::try_from(req)?;
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            false => Err(errors::ConnectorError::RequestEncodingFailed).into_report(),
        }
    }

    fn build_request(
        &self,
        req: &types::RefundSyncRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => Ok(Some(
                services::RequestBuilder::new()
                    .method(services::Method::Post)
                    .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                    .set_body(types::RefundSyncType::get_request_body(
                        self, req, connectors,
                    )?)
                    .build(),
            )),
            false => Ok(None),
        }
    }

    fn handle_response(
        &self,
        data: &types::RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<api::RSync, types::RefundsData, types::RefundsResponseData>,
        errors::ConnectorError,
    > {
        let connector_api_version = &data.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let response: braintree_graphql_transformers::BraintreeRSyncResponse = res
                    .response
                    .parse_struct("Braintree RefundResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            false => {
                let response: braintree::RefundResponse = res
                    .response
                    .parse_struct("Braintree RefundResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
        }
    }
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Braintree {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha1))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let notif_item = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature_pairs: Vec<(&str, &str)> = notif_item
            .bt_signature
            .split('&')
            .collect::<Vec<&str>>()
            .into_iter()
            .map(|pair| pair.split_once('|').unwrap_or(("", "")))
            .collect::<Vec<(_, _)>>();

        let merchant_secret = connector_webhook_secrets
            .additional_secret //public key
            .clone()
            .ok_or(errors::ConnectorError::WebhookVerificationSecretNotFound)?;

        let signature = get_matching_webhook_signature(signature_pairs, merchant_secret.expose())
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        Ok(signature.as_bytes().to_vec())
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let notify = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let message = notify.bt_payload.to_string();

        Ok(message.into_bytes())
    }

    async fn verify_webhook_source(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        merchant_account: &domain::MerchantAccount,
        merchant_connector_account: domain::MerchantConnectorAccount,
        connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_account,
                connector_label,
                merchant_connector_account,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;

        let message = self
            .get_webhook_source_verification_message(
                request,
                &merchant_account.merchant_id,
                &connector_webhook_secrets,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let sha1_hash_key = Sha1::digest(&connector_webhook_secrets.secret);

        let signing_key = hmac::Key::new(
            hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
            sha1_hash_key.as_slice(),
        );
        let signed_messaged = hmac::sign(&signing_key, &message);
        let payload_sign: String = hex::encode(signed_messaged);
        Ok(payload_sign.as_bytes().eq(&signature))
    }

    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(_request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let response = decode_webhook_payload(notif.bt_payload.replace('\n', "").as_bytes())?;

        match response.dispute {
            Some(dispute_data) => Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(
                    dispute_data.transaction.id,
                ),
            )),
            None => Err(errors::ConnectorError::WebhookReferenceIdNotFound).into_report(),
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let response = decode_webhook_payload(notif.bt_payload.replace('\n', "").as_bytes())?;

        Ok(IncomingWebhookEvent::foreign_from(response.kind.as_str()))
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let response = decode_webhook_payload(notif.bt_payload.replace('\n', "").as_bytes())?;

        Ok(Box::new(response))
    }

    fn get_webhook_api_response(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ConnectorError>
    {
        Ok(services::api::ApplicationResponse::TextPlain(
            "[accepted]".to_string(),
        ))
    }

    fn get_dispute_details(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::disputes::DisputePayload, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let response = decode_webhook_payload(notif.bt_payload.replace('\n', "").as_bytes())?;

        match response.dispute {
            Some(dispute_data) => {
                let currency = diesel_models::enums::Currency::from_str(
                    dispute_data.currency_iso_code.as_str(),
                )
                .into_report()
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
                Ok(api::disputes::DisputePayload {
                    amount: connector_utils::to_currency_lower_unit(
                        dispute_data.amount_disputed.to_string(),
                        currency,
                    )?,
                    currency: dispute_data.currency_iso_code,
                    dispute_stage: braintree_graphql_transformers::get_dispute_stage(
                        dispute_data.kind.as_str(),
                    )?,
                    connector_dispute_id: dispute_data.id,
                    connector_reason: dispute_data.reason,
                    connector_reason_code: dispute_data.reason_code,
                    challenge_required_by: dispute_data.reply_by_date,
                    connector_status: dispute_data.status,
                    created_at: dispute_data.created_at,
                    updated_at: dispute_data.updated_at,
                })
            }
            None => Err(errors::ConnectorError::WebhookResourceObjectNotFound)?,
        }
    }
}

fn get_matching_webhook_signature(
    signature_pairs: Vec<(&str, &str)>,
    secret: String,
) -> Option<String> {
    for (public_key, signature) in signature_pairs {
        if *public_key == secret {
            return Some(signature.to_string());
        }
    }
    None
}

fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<braintree_graphql_transformers::BraintreeWebhookResponse, errors::ParsingError> {
    serde_urlencoded::from_bytes::<braintree_graphql_transformers::BraintreeWebhookResponse>(body)
        .into_report()
        .change_context(errors::ParsingError::StructParseFailure(
            "BraintreeWebhookResponse",
        ))
}

fn decode_webhook_payload(
    payload: &[u8],
) -> CustomResult<braintree_graphql_transformers::Notification, errors::ConnectorError> {
    let decoded_response = consts::BASE64_ENGINE
        .decode(payload)
        .into_report()
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

    let xml_response = String::from_utf8(decoded_response)
        .into_report()
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

    xml_response
        .parse_xml::<braintree_graphql_transformers::Notification>()
        .into_report()
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
}

impl services::ConnectorRedirectResponse for Braintree {
    fn get_flow_type(
        &self,
        _query_params: &str,
        json_payload: Option<serde_json::Value>,
        action: services::PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        match action {
            services::PaymentAction::PSync => match json_payload {
                Some(payload) => {
                    let redirection_response:braintree_graphql_transformers::BraintreeRedirectionResponse = serde_json::from_value(payload)
                            .into_report()
                            .change_context(
                                errors::ConnectorError::MissingConnectorRedirectionPayload {
                                    field_name: "redirection_response",
                                },
                            )?;
                    let braintree_payload =
                        serde_json::from_str::<
                            braintree_graphql_transformers::BraintreeThreeDsErrorResponse,
                        >(&redirection_response.authentication_response);
                    let (error_code, error_message) = match braintree_payload {
                        Ok(braintree_response_payload) => (
                            braintree_response_payload.code,
                            braintree_response_payload.message,
                        ),
                        Err(_) => (
                            consts::NO_ERROR_CODE.to_string(),
                            redirection_response.authentication_response,
                        ),
                    };
                    Ok(payments::CallConnectorAction::StatusUpdate {
                        status: enums::AttemptStatus::AuthenticationFailed,
                        error_code: Some(error_code),
                        error_message: Some(error_message),
                    })
                }
                None => Ok(payments::CallConnectorAction::Avoid),
            },
            services::PaymentAction::CompleteAuthorize => {
                Ok(payments::CallConnectorAction::Trigger)
            }
        }
    }
}

impl
    ConnectorIntegration<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for Braintree
{
    fn get_headers(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => self.build_headers(req, connectors),
            false => Err(errors::ConnectorError::NotImplemented(
                "get_headers method".to_string(),
            ))?,
        }
    }
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }
    fn get_url(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let base_url = connectors
                    .braintree
                    .secondary_base_url
                    .as_ref()
                    .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
                Ok(base_url.to_string())
            }
            false => Err(errors::ConnectorError::NotImplemented(
                "get_url method".to_string(),
            ))?,
        }
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data =
            braintree_graphql_transformers::BraintreeRouterData::try_from((
                &self.get_currency_unit(),
                req.request.currency,
                req.request.amount,
                req,
            ))?;
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => {
                let connector_req =
                    braintree_graphql_transformers::BraintreePaymentsRequest::try_from(
                        &connector_router_data,
                    )?;
                Ok(RequestContent::Json(Box::new(connector_req)))
            }
            false => Err(errors::ConnectorError::NotImplemented(
                "get_request_body method".to_string(),
            ))?,
        }
    }

    fn build_request(
        &self,
        req: &types::PaymentsCompleteAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let connector_api_version = &req.connector_api_version;
        match self.is_braintree_graphql_version(connector_api_version) {
            true => Ok(Some(
                services::RequestBuilder::new()
                    .method(services::Method::Post)
                    .url(&types::PaymentsCompleteAuthorizeType::get_url(
                        self, req, connectors,
                    )?)
                    .attach_default_headers()
                    .headers(types::PaymentsCompleteAuthorizeType::get_headers(
                        self, req, connectors,
                    )?)
                    .set_body(types::PaymentsCompleteAuthorizeType::get_request_body(
                        self, req, connectors,
                    )?)
                    .build(),
            )),
            false => Err(errors::ConnectorError::NotImplemented(
                "payment method".to_string(),
            ))?,
        }
    }
    fn handle_response(
        &self,
        data: &types::PaymentsCompleteAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCompleteAuthorizeRouterData, errors::ConnectorError> {
        match connector_utils::PaymentsCompleteAuthorizeRequestData::is_auto_capture(&data.request)?
        {
            true => {
                let response: braintree_graphql_transformers::BraintreeCompleteChargeResponse = res
                    .response
                    .parse_struct("Braintree PaymentsResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
            false => {
                let response: braintree_graphql_transformers::BraintreeCompleteAuthResponse = res
                    .response
                    .parse_struct("Braintree AuthResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                event_builder.map(|i| i.set_response_body(&response));
                router_env::logger::info!(connector_response=?response);
                types::RouterData::try_from(types::ResponseRouterData {
                    response,
                    data: data.clone(),
                    http_code: res.status_code,
                })
            }
        }
    }

    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}
