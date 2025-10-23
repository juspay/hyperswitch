pub mod transformers;

use std::sync::LazyLock;

use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use common_enums::enums;
use common_utils::{
    crypto::{RsaPssSha256, SignMessage},
    errors::CustomResult,
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hex;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData as WalletDataPaymentMethod},
    router_data::{AccessToken, ErrorResponse, RouterData},
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
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};
use masking::{ExposeInterface, Mask, Maskable, PeekInterface, Secret};
use sha2::{Digest, Sha256};
use transformers as amazonpay;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils as connector_utils,
    utils::{self, PaymentsSyncRequestData},
};

const SIGNING_ALGO: &str = "AMZN-PAY-RSASSA-PSS-V2";
const HEADER_ACCEPT: &str = "accept";
const HEADER_CONTENT_TYPE: &str = "content-type";
const HEADER_DATE: &str = "x-amz-pay-date";
const HEADER_HOST: &str = "x-amz-pay-host";
const HEADER_IDEMPOTENCY_KEY: &str = "x-amz-pay-idempotency-key";
const HEADER_REGION: &str = "x-amz-pay-region";
const FINALIZE_SEGMENT: &str = "finalize";
const AMAZON_PAY_API_BASE_URL: &str = "https://pay-api.amazon.com";
const AMAZON_PAY_HOST: &str = "pay-api.amazon.com";

#[derive(Clone)]
pub struct Amazonpay {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Amazonpay {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }

    fn get_last_segment(canonical_uri: &str) -> String {
        canonical_uri
            .chars()
            .rev()
            .take_while(|&c| c != '/')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub fn create_authorization_header(
        &self,
        auth: amazonpay::AmazonpayAuthType,
        canonical_uri: &str,
        http_method: &Method,
        hashed_payload: &str,
        header: &[(String, Maskable<String>)],
    ) -> String {
        let amazonpay::AmazonpayAuthType {
            public_key,
            private_key,
        } = auth;

        let mut signed_headers =
            format!("{HEADER_ACCEPT};{HEADER_CONTENT_TYPE};{HEADER_DATE};{HEADER_HOST};",);
        if *http_method == Method::Post
            && Self::get_last_segment(canonical_uri) != *FINALIZE_SEGMENT.to_string()
        {
            signed_headers.push_str(HEADER_IDEMPOTENCY_KEY);
            signed_headers.push(';');
        }
        signed_headers.push_str(HEADER_REGION);

        format!(
            "{} PublicKeyId={}, SignedHeaders={}, Signature={}",
            SIGNING_ALGO,
            public_key.expose().clone(),
            signed_headers,
            Self::create_signature(
                &private_key,
                *http_method,
                canonical_uri,
                &signed_headers,
                hashed_payload,
                header
            )
            .unwrap_or_else(|_| "Invalid signature".to_string())
        )
    }

    fn create_signature(
        private_key: &Secret<String>,
        http_method: Method,
        canonical_uri: &str,
        signed_headers: &str,
        hashed_payload: &str,
        header: &[(String, Maskable<String>)],
    ) -> Result<String, String> {
        let mut canonical_request = http_method.to_string() + "\n" + canonical_uri + "\n\n";

        let mut lowercase_sorted_header_keys: Vec<String> =
            header.iter().map(|(key, _)| key.to_lowercase()).collect();

        lowercase_sorted_header_keys.sort();

        for key in lowercase_sorted_header_keys {
            if let Some((_, maskable_value)) = header.iter().find(|(k, _)| k.to_lowercase() == key)
            {
                let value: String = match maskable_value {
                    Maskable::Normal(v) => v.clone(),
                    Maskable::Masked(secret) => secret.clone().expose(),
                };
                canonical_request.push_str(&format!("{key}:{value}\n"));
            }
        }

        canonical_request.push_str(&("\n".to_owned() + signed_headers + "\n" + hashed_payload));

        let string_to_sign = format!(
            "{}\n{}",
            SIGNING_ALGO,
            hex::encode(Sha256::digest(canonical_request.as_bytes()))
        );

        Self::sign(private_key, &string_to_sign)
            .map_err(|e| format!("Failed to create signature: {e}"))
    }

    fn sign(
        private_key_pem_str: &Secret<String>,
        string_to_sign: &String,
    ) -> Result<String, String> {
        let rsa_pss_sha256_signer = RsaPssSha256;
        let signature_bytes = rsa_pss_sha256_signer
            .sign_message(
                private_key_pem_str.peek().as_bytes(),
                string_to_sign.as_bytes(),
            )
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .map_err(|e| format!("Crypto operation failed: {e:?}"))?;

        Ok(STANDARD.encode(signature_bytes))
    }
}

impl api::Payment for Amazonpay {}
impl api::PaymentSession for Amazonpay {}
impl api::ConnectorAccessToken for Amazonpay {}
impl api::MandateSetup for Amazonpay {}
impl api::PaymentAuthorize for Amazonpay {}
impl api::PaymentSync for Amazonpay {}
impl api::PaymentCapture for Amazonpay {}
impl api::PaymentVoid for Amazonpay {}
impl api::Refund for Amazonpay {}
impl api::RefundExecute for Amazonpay {}
impl api::RefundSync for Amazonpay {}
impl api::PaymentToken for Amazonpay {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Amazonpay
{
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Amazonpay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let http_method = self.get_http_method();

        let canonical_uri: String =
            self.get_url(req, connectors)?
                .replacen(AMAZON_PAY_API_BASE_URL, "", 1);

        let mut header = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            ),
            (
                headers::ACCEPT.to_string(),
                "application/json".to_string().into(),
            ),
            (
                HEADER_DATE.to_string(),
                Utc::now()
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string()
                    .into_masked(),
            ),
            (
                HEADER_HOST.to_string(),
                AMAZON_PAY_HOST.to_string().into_masked(),
            ),
            (HEADER_REGION.to_string(), "na".to_string().into_masked()),
        ];

        if http_method == Method::Post
            && Self::get_last_segment(&canonical_uri) != *FINALIZE_SEGMENT.to_string()
        {
            header.push((
                HEADER_IDEMPOTENCY_KEY.to_string(),
                req.connector_request_reference_id.clone().into_masked(),
            ));
        }

        let hashed_payload = if http_method == Method::Get {
            hex::encode(Sha256::digest("".as_bytes()))
        } else {
            hex::encode(Sha256::digest(
                self.get_request_body(req, connectors)?
                    .get_inner_value()
                    .expose()
                    .as_bytes(),
            ))
        };

        let authorization = self.create_authorization_header(
            amazonpay::AmazonpayAuthType::try_from(&req.connector_auth_type)?,
            &canonical_uri,
            &http_method,
            &hashed_payload,
            &header,
        );

        header.push((
            headers::AUTHORIZATION.to_string(),
            authorization.clone().into_masked(),
        ));

        Ok(header)
    }
}

impl ConnectorCommon for Amazonpay {
    fn id(&self) -> &'static str {
        "amazonpay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.amazonpay.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: amazonpay::AmazonpayErrorResponse = res
            .response
            .parse_struct("AmazonpayErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.reason_code.clone(),
            message: response.message.clone(),
            attempt_status: None,
            connector_transaction_id: None,
            reason: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Amazonpay {}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Amazonpay {}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Amazonpay {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for Amazonpay
{
}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Amazonpay {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        match req.request.payment_method_data.clone() {
            PaymentMethodData::Wallet(ref wallet_data) => match wallet_data {
                WalletDataPaymentMethod::AmazonPay(ref req_wallet) => Ok(format!(
                    "{}/checkoutSessions/{}/finalize",
                    self.base_url(connectors),
                    req_wallet.checkout_session_id.clone()
                )),
                WalletDataPaymentMethod::AliPayQr(_)
                | WalletDataPaymentMethod::AliPayRedirect(_)
                | WalletDataPaymentMethod::AliPayHkRedirect(_)
                | WalletDataPaymentMethod::AmazonPayRedirect(_)
                | WalletDataPaymentMethod::MomoRedirect(_)
                | WalletDataPaymentMethod::KakaoPayRedirect(_)
                | WalletDataPaymentMethod::GoPayRedirect(_)
                | WalletDataPaymentMethod::GcashRedirect(_)
                | WalletDataPaymentMethod::ApplePay(_)
                | WalletDataPaymentMethod::ApplePayRedirect(_)
                | WalletDataPaymentMethod::ApplePayThirdPartySdk(_)
                | WalletDataPaymentMethod::DanaRedirect {}
                | WalletDataPaymentMethod::GooglePay(_)
                | WalletDataPaymentMethod::GooglePayRedirect(_)
                | WalletDataPaymentMethod::GooglePayThirdPartySdk(_)
                | WalletDataPaymentMethod::MbWayRedirect(_)
                | WalletDataPaymentMethod::MobilePayRedirect(_)
                | WalletDataPaymentMethod::PaypalRedirect(_)
                | WalletDataPaymentMethod::PaypalSdk(_)
                | WalletDataPaymentMethod::Paze(_)
                | WalletDataPaymentMethod::SamsungPay(_)
                | WalletDataPaymentMethod::TwintRedirect {}
                | WalletDataPaymentMethod::VippsRedirect {}
                | WalletDataPaymentMethod::BluecodeRedirect {}
                | WalletDataPaymentMethod::TouchNGoRedirect(_)
                | WalletDataPaymentMethod::WeChatPayRedirect(_)
                | WalletDataPaymentMethod::WeChatPayQr(_)
                | WalletDataPaymentMethod::CashappQr(_)
                | WalletDataPaymentMethod::SwishQr(_)
                | WalletDataPaymentMethod::RevolutPay(_)
                | WalletDataPaymentMethod::Paysera(_)
                | WalletDataPaymentMethod::Skrill(_)
                | WalletDataPaymentMethod::Mifinity(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("amazonpay"),
                    )
                    .into())
                }
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }

    fn get_request_body(
        &self,
        req: &PaymentsAuthorizeRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data = amazonpay::AmazonpayRouterData::from((amount, req));
        let connector_req = amazonpay::AmazonpayFinalizeRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
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
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: amazonpay::AmazonpayFinalizeResponse = res
            .response
            .parse_struct("Amazonpay PaymentsAuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        
        let response_integrity_object = connector_utils::get_authorise_integrity_object(
            self.amount_converter,
            response.amount,
            response.currency.to_string().clone(),
        )?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed);

        new_router_data.map(|mut router_data| {
            router_data.request.integrity_object = Some(response_integrity_object);
            router_data
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Amazonpay {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/charges/{}",
            self.base_url(connectors),
            req.request.get_connector_transaction_id()?
        ))
    }

    fn get_http_method(&self) -> Method {
        Method::Get
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        let response: amazonpay::AmazonpayPaymentsResponse = res
            .response
            .clone()
            .parse_struct("xendit XenditResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let response_integrity_object = match response.clone() {
            amazonpay::AmazonpayPaymentsResponse::Payment(p) => connector_utils::get_sync_integrity_object(
                self.amount_converter,
                p.amount,
                p.currency.to_string().clone(),
            ),
            amazonpay::AmazonpayPaymentsResponse::Webhook(p) => connector_utils::get_sync_integrity_object(
                self.amount_converter,
                p.data.amount,
                p.data.currency.to_string().clone(),
            ),
        };

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        });

        new_router_data.and_then(|mut router_data| {
            let integrity_object = response_integrity_object?;
            router_data.request.integrity_object = Some(integrity_object);
            Ok(router_data)
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Amazonpay {
    fn build_request(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Capture".to_string()).into())
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Amazonpay {
    fn build_request(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Void".to_string()).into())
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Amazonpay {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/refunds", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let refund_amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_refund_amount,
            req.request.currency,
        )?;

        let connector_router_data = amazonpay::AmazonpayRouterData::from((refund_amount, req));
        let connector_req = amazonpay::AmazonpayRefundRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Post)
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
        data: &RefundsRouterData<Execute>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundsRouterData<Execute>, errors::ConnectorError> {
        let response: amazonpay::RefundResponse = res
            .response
            .parse_struct("amazonpay RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let response_integrity_object = connector_utils::get_refund_integrity_object(
            self.amount_converter,
            response.amount,
            response.currency.to_string().clone(),
        )?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        });

        new_router_data
            .map(|mut router_data| {
                router_data.request.integrity_object = Some(response_integrity_object);
                router_data
            })
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

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Amazonpay {
    fn get_headers(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/refunds/{}",
            self.base_url(connectors),
            req.request.connector_refund_id.clone().unwrap_or_default()
        ))
    }

    fn get_http_method(&self) -> Method {
        Method::Get
    }

    fn build_request(
        &self,
        req: &RefundSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&types::RefundSyncType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundSyncType::get_headers(self, req, connectors)?)
                .set_body(types::RefundSyncType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &RefundSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RefundSyncRouterData, errors::ConnectorError> {
        let response: amazonpay::RefundResponse = res
            .response
            .clone()
            .parse_struct("amazonpay RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let response_integrity_object = connector_utils::get_refund_integrity_object(
            self.amount_converter,
            response.amount,
            response.currency.to_string().clone(),
        )?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        let new_router_data = RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        });

        new_router_data
            .map(|mut router_data| {
                router_data.request.integrity_object = Some(response_integrity_object);
                router_data
            })
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
impl webhooks::IncomingWebhook for Amazonpay {
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }
}

static AMAZONPAY_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> =
    LazyLock::new(|| {
        let supported_capture_methods = vec![enums::CaptureMethod::Automatic];

        let mut amazonpay_supported_payment_methods = SupportedPaymentMethods::new();

        amazonpay_supported_payment_methods.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::AmazonPay,
            PaymentMethodDetails {
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            },
        );

        amazonpay_supported_payment_methods
    });

static AMAZONPAY_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Amazon Pay",
    description: "Amazon Pay is an Alternative Payment Method (APM) connector that allows merchants to accept payments using customers' stored Amazon account details, providing a seamless checkout experience.",
    connector_type: enums::HyperswitchConnectorCategory::AlternativePaymentMethod,
    integration_status: enums::ConnectorIntegrationStatus::Alpha,
};

static AMAZONPAY_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 0] = [];

impl ConnectorSpecifications for Amazonpay {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&AMAZONPAY_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&AMAZONPAY_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&AMAZONPAY_SUPPORTED_WEBHOOK_FLOWS)
    }
}
