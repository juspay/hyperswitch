pub mod transformers;

use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use common_enums::enums;
use common_utils::{
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
        payments::{
            Authorize, Capture, CompleteAuthorize, PSync, PaymentMethodToken, Session,
            SetupMandate, Void,
        },
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, CompleteAuthorizeData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, SetupMandateRequestData,
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
use lazy_static::lazy_static;
use masking::{ExposeInterface, Mask, Maskable, PeekInterface, Secret};
use openssl::{
    hash::MessageDigest,
    pkey::PKey,
    rsa::Padding,
    sign::{RsaPssSaltlen, Signer},
};
use sha2::{Digest, Sha256};
use transformers as amazonpay;

use crate::{
    constants::headers,
    types::ResponseRouterData,
    utils::{self, PaymentsSyncRequestData},
};

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

        let mut signed_headers = "accept;content-type;x-amz-pay-date;x-amz-pay-host;".to_string();
        if *http_method == Method::Post
            && Self::get_last_segment(canonical_uri) != *"finalize".to_string()
        {
            signed_headers.push_str("x-amz-pay-idempotency-key;");
        }
        signed_headers.push_str("x-amz-pay-region");

        format!(
            "AMZN-PAY-RSASSA-PSS-V2 PublicKeyId={}, SignedHeaders={}, Signature={}",
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
                canonical_request.push_str(&format!("{}:{}\n", key, value));
            }
        }

        canonical_request.push_str(&("\n".to_owned() + signed_headers + "\n" + hashed_payload));

        let string_to_sign = "AMZN-PAY-RSASSA-PSS-V2\n".to_string()
            + &hex::encode(Sha256::digest(canonical_request.as_bytes()));

        Self::sign(private_key, &string_to_sign)
            .map_err(|e| format!("Failed to create signature: {}", e))
    }

    fn sign(private_key_pem: &Secret<String>, string_to_sign: &String) -> Result<String, String> {
        let pkey = PKey::private_key_from_pem(private_key_pem.peek().as_bytes())
            .map_err(|e| format!("Failed to parse PKCS8 private key: {}", e))?;

        let mut signer = Signer::new(MessageDigest::sha256(), &pkey)
            .map_err(|e| format!("Failed to create signer: {}", e))?;

        signer
            .set_rsa_padding(Padding::PKCS1_PSS)
            .map_err(|e| format!("Failed to set RSA padding: {}", e))?;

        signer
            .set_rsa_pss_saltlen(RsaPssSaltlen::custom(32))
            .map_err(|e| format!("Failed to set RSA PSS salt length: {}", e))?;

        signer
            .update(string_to_sign.as_bytes())
            .map_err(|e| format!("Failed to update signer with string: {}", e))?;

        Ok(STANDARD.encode(
            signer
                .sign_to_vec()
                .map_err(|e| format!("Failed to sign data: {}", e))?,
        ))
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
impl api::PaymentsCompleteAuthorize for Amazonpay {}

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

        let mut canonical_uri = "/sandbox/v2".to_string(); // TODO: change to "/live/v2" for production

        let trimmed_url: String = self
            .get_url(req, connectors)?
            .chars()
            .skip(connectors.amazonpay.base_url.as_str().len())
            .collect();

        canonical_uri.push_str(&trimmed_url);

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
                "x-amz-pay-date".to_string(),
                Utc::now()
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string()
                    .into_masked(),
            ),
            (
                "x-amz-pay-host".to_string(),
                "pay-api.amazon.com".to_string().into_masked(),
            ),
            (
                "x-amz-pay-region".to_string(),
                "na".to_string().into_masked(),
            ),
        ];

        if http_method == Method::Post
            && Self::get_last_segment(&canonical_uri) != *"finalize".to_string()
        {
            header.push((
                "x-amz-pay-idempotency-key".to_string(),
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
                | WalletDataPaymentMethod::TouchNGoRedirect(_)
                | WalletDataPaymentMethod::WeChatPayRedirect(_)
                | WalletDataPaymentMethod::WeChatPayQr(_)
                | WalletDataPaymentMethod::CashappQr(_)
                | WalletDataPaymentMethod::SwishQr(_)
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>
    for Amazonpay
{
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
            .parse_struct("Amazonpay PaymentsSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Amazonpay {
    fn get_headers(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Capture".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Capture".to_string()).into())
    }

    fn build_request(
        &self,
        req: &PaymentsCaptureRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
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
        data: &PaymentsCaptureRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: amazonpay::AmazonpayPaymentsResponse = res
            .response
            .parse_struct("Amazonpay PaymentsCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Amazonpay {
    fn get_headers(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Void".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("Void".to_string()).into())
    }

    fn get_http_method(&self) -> Method {
        Method::Delete
    }

    fn build_request(
        &self,
        req: &PaymentsCancelRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Delete)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .set_body(types::PaymentsVoidType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsCancelRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsCancelRouterData, errors::ConnectorError> {
        let response: amazonpay::AmazonpayPaymentsResponse = res
            .response
            .parse_struct("Amazonpay PaymentsVoidResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
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
            .parse_struct("amazonpay RefundSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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

lazy_static! {
    static ref AMAZONPAY_SUPPORTED_PAYMENT_METHODS: SupportedPaymentMethods = {
        let supported_capture_methods = vec![
            enums::CaptureMethod::Automatic,
        ];

        let mut amazonpay_supported_payment_methods = SupportedPaymentMethods::new();

        amazonpay_supported_payment_methods.add(
            enums::PaymentMethod::Wallet,
            enums::PaymentMethodType::AmazonPay,
            PaymentMethodDetails{
                mandates: enums::FeatureStatus::NotSupported,
                refunds: enums::FeatureStatus::Supported,
                supported_capture_methods: supported_capture_methods.clone(),
                specific_features: None,
            }
        );

        amazonpay_supported_payment_methods
    };

    static ref AMAZONPAY_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
        description: "Amazon Pay is an Alternative Payment Method (APM) connector that allows merchants to accept payments using customers' stored Amazon account details, providing a seamless checkout experience.".to_string(),
        connector_type: enums::PaymentConnectorCategory::AlternativePaymentMethod,
    };

    static ref AMAZONPAY_SUPPORTED_WEBHOOK_FLOWS: Vec<enums::EventClass> = Vec::new();
}

impl ConnectorSpecifications for Amazonpay {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&*AMAZONPAY_CONNECTOR_INFO)
    }

    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*AMAZONPAY_SUPPORTED_PAYMENT_METHODS)
    }

    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&*AMAZONPAY_SUPPORTED_WEBHOOK_FLOWS)
    }
}
