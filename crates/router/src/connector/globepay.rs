pub mod transformers;

use std::fmt::Debug;

use common_utils::{
    crypto::{self, GenerateDigest},
    request::RequestContent,
};
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
use hex::encode;
use masking::ExposeInterface;
use rand::distributions::DistString;
use time::OffsetDateTime;
use transformers as globepay;

use crate::{
    configs::settings,
    connector::utils as connector_utils,
    consts,
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers,
    services::{self, request, ConnectorIntegration, ConnectorValidation},
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        ErrorResponse, Response,
    },
    utils::BytesExt,
};

#[derive(Debug, Clone)]
pub struct Globepay;

impl api::Payment for Globepay {}
impl api::PaymentSession for Globepay {}
impl api::ConnectorAccessToken for Globepay {}
impl api::MandateSetup for Globepay {}
impl api::PaymentAuthorize for Globepay {}
impl api::PaymentSync for Globepay {}
impl api::PaymentCapture for Globepay {}
impl api::PaymentVoid for Globepay {}
impl api::Refund for Globepay {}
impl api::RefundExecute for Globepay {}
impl api::RefundSync for Globepay {}
impl api::PaymentToken for Globepay {}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Globepay
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Globepay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        Ok(header)
    }
}

fn get_globlepay_query_params(
    connector_auth_type: &types::ConnectorAuthType,
) -> CustomResult<String, errors::ConnectorError> {
    let auth_type = globepay::GlobepayAuthType::try_from(connector_auth_type)?;
    let time = (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000).to_string();
    let nonce_str = rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 12);
    let valid_string = format!(
        "{}&{time}&{nonce_str}&{}",
        auth_type.partner_code.expose(),
        auth_type.credential_code.expose()
    );
    let digest = crypto::Sha256
        .generate_digest(valid_string.as_bytes())
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("error encoding the query params")?;
    let sign = encode(digest).to_lowercase();
    let param = format!("?sign={sign}&time={time}&nonce_str={nonce_str}");
    Ok(param)
}

fn get_partner_code(
    connector_auth_type: &types::ConnectorAuthType,
) -> CustomResult<String, errors::ConnectorError> {
    let auth_type = globepay::GlobepayAuthType::try_from(connector_auth_type)?;
    Ok(auth_type.partner_code.expose())
}

impl ConnectorCommon for Globepay {
    fn id(&self) -> &'static str {
        "globepay"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.globepay.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: globepay::GlobepayErrorResponse = res
            .response
            .parse_struct("GlobepayErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_error_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.return_code.to_string(),
            message: consts::NO_ERROR_MESSAGE.to_string(),
            reason: Some(response.return_msg),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Globepay {
    fn validate_mandate_payment(
        &self,
        pm_type: Option<types::storage::enums::PaymentMethodType>,
        pm_data: api_models::payments::PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        match pm_data {
            api_models::payments::PaymentMethodData::Wallet(wallet) => match wallet {
                api_models::payments::WalletData::GooglePay(_)
                | api_models::payments::WalletData::PaypalRedirect(_) => Ok(()),
                api_models::payments::WalletData::SamsungPay(_)
                | api_models::payments::WalletData::AliPayRedirect(_)
                | api_models::payments::WalletData::WeChatPayRedirect(_)
                | api_models::payments::WalletData::ApplePay(_)
                | api_models::payments::WalletData::DanaRedirect {}
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
            api_models::payments::PaymentMethodData::BankRedirect(bank_redirect) => {
                match bank_redirect {
                    api_models::payments::BankRedirectData::Sofort { .. }
                    | api_models::payments::BankRedirectData::Ideal { .. }
                    | api_models::payments::BankRedirectData::Eps { .. }
                    | api_models::payments::BankRedirectData::Giropay { .. } => Ok(()),
                    api_models::payments::BankRedirectData::OnlineBankingCzechRepublic {
                        ..
                    }
                    | api_models::payments::BankRedirectData::OpenBankingUk { .. }
                    | api_models::payments::BankRedirectData::OnlineBankingFinland { .. }
                    | api_models::payments::BankRedirectData::OnlineBankingPoland { .. }
                    | api_models::payments::BankRedirectData::OnlineBankingSlovakia { .. }
                    | api_models::payments::BankRedirectData::OnlineBankingFpx { .. }
                    | api_models::payments::BankRedirectData::Bizum {}
                    | api_models::payments::BankRedirectData::Blik { .. }
                    | api_models::payments::BankRedirectData::Przelewy24 { .. }
                    | api_models::payments::BankRedirectData::Interac { .. }
                    | api_models::payments::BankRedirectData::Trustly { .. }
                    | api_models::payments::BankRedirectData::OnlineBankingThailand { .. }
                    | api_models::payments::BankRedirectData::BancontactCard { .. } => Err(
                        connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
                    ),
                }
            }
            api_models::payments::PaymentMethodData::MandatePayment => Ok(()),
            api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::Card(_)
            | api_models::payments::PaymentMethodData::BankDebit(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::GiftCard(_)
            | api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::CardToken(_) => Err(
                connector_utils::construct_mandate_not_supported_error(pm_type, self.id()),
            ),
        }
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Globepay
{
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Globepay
{
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Globepay
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
            errors::ConnectorError::NotImplemented("Setup Mandate flow for Globepay".to_string())
                .into(),
        )
    }
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Globepay
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
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let query_params = get_globlepay_query_params(&req.connector_auth_type)?;
        if req.request.capture_method == Some(enums::CaptureMethod::Automatic) {
            Ok(format!(
                "{}api/v1.0/gateway/partners/{}/orders/{}{query_params}",
                self.base_url(connectors),
                get_partner_code(&req.connector_auth_type)?,
                req.payment_id
            ))
        } else {
            Err(errors::ConnectorError::FlowNotSupported {
                flow: "Manual Capture".to_owned(),
                connector: "Globepay".to_owned(),
            }
            .into())
        }
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = globepay::GlobepayPaymentsRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Put)
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
        let response: globepay::GlobepayPaymentsResponse = res
            .response
            .parse_struct("Globepay PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Globepay
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
        let query_params = get_globlepay_query_params(&req.connector_auth_type)?;
        Ok(format!(
            "{}api/v1.0/gateway/partners/{}/orders/{}{query_params}",
            self.base_url(connectors),
            get_partner_code(&req.connector_auth_type)?,
            req.payment_id
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

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: globepay::GlobepaySyncResponse = res
            .response
            .parse_struct("globepay PaymentsSyncResponse")
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

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Globepay
{
    fn build_request(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Manual Capture".to_owned(),
            connector: "Globepay".to_owned(),
        }
        .into())
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Globepay
{
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Globepay
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
        let query_params = get_globlepay_query_params(&req.connector_auth_type)?;
        Ok(format!(
            "{}api/v1.0/gateway/partners/{}/orders/{}/refunds/{}{query_params}",
            self.base_url(connectors),
            get_partner_code(&req.connector_auth_type)?,
            req.payment_id,
            req.request.refund_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = globepay::GlobepayRefundRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Put)
                .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundExecuteType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::RefundExecuteType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: globepay::GlobepayRefundResponse = res
            .response
            .parse_struct("Globalpay RefundResponse")
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

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Globepay {
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
        let query_params = get_globlepay_query_params(&req.connector_auth_type)?;
        Ok(format!(
            "{}api/v1.0/gateway/partners/{}/orders/{}/refunds/{}{query_params}",
            self.base_url(connectors),
            get_partner_code(&req.connector_auth_type)?,
            req.payment_id,
            req.request.refund_id
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
        let response: globepay::GlobepayRefundResponse = res
            .response
            .parse_struct("Globalpay RefundResponse")
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
impl api::IncomingWebhook for Globepay {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
