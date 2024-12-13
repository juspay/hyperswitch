pub mod transformers;

use common_utils::{
    errors::CustomResult,
    ext_traits::{BytesExt, Encode},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
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
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData, PaymentsSessionRouterData
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
use masking::{ExposeInterface, Mask, PeekInterface};
use transformers as amazonpay;

use crate::{constants::headers, types::ResponseRouterData, utils};
use chrono::Utc;
use masking::Secret;
use sha2::{Sha256, Digest};
use hex;
use openssl::rsa::{Rsa, Padding};
use openssl::sign::{Signer, RsaPssSaltlen};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use base64::encode;

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

    pub fn create_authorization_header (
        &self,
        auth: amazonpay::AmazonpayAuthType,
        http_method: &Method,
        hashed_payload: &String,
        header: &Vec<(String, masking::Maskable<String>)>
    ) -> String {
        let amazonpay::AmazonpayAuthType {
            public_key,
            private_key
        } = auth;

        let algorithm = "AMZN-PAY-RSASSA-PSS".to_string();

        let public_key = public_key.expose().clone();

        let canonical_uri = "/sandbox/v2/charges".to_string();

        let mut signed_headers = "accept;content-type;x-amz-pay-date;x-amz-pay-host;".to_string();
        if *http_method == Method::Post {
            signed_headers.push_str("x-amz-pay-idempotency-key");
        }
        signed_headers.push_str("x-amz-pay-region");

        let signature = Self::create_signature(&private_key, http_method, &canonical_uri, &signed_headers, hashed_payload, header);

        format!(
            "{:?} PublicKeyId={:?}, SignedHeaders={:?}, Signature={:?}",
            algorithm,
            public_key,
            signed_headers,
            signature
        )
    }

    fn create_signature(
        private_key: &Secret<String>,
        http_method: &Method,
        canonical_uri: &String,
        signed_headers: &String,
        hashed_payload: &String,
        header: &Vec<(String, masking::Maskable<String>)>
    ) -> String {
        let mut canonical_request = http_method.to_string() + "\n" + canonical_uri + "\n\n";

        let mut lowercase_sorted_header_keys: Vec<String> = header.iter()
        .map(|(key, _)| key.to_lowercase())
        .collect();

        lowercase_sorted_header_keys.sort();

        for key in lowercase_sorted_header_keys {
            if let Some(value) = header.iter().find(|(k, _)| k.to_lowercase() == key) {
                canonical_request.push_str(&format!("{:?}:{:?}\n", key, value));
            }
        }

        canonical_request.push_str(&("\n".to_owned() + signed_headers + "\n" + hashed_payload));

        let hashed_canonical_request = hex::encode(Sha256::digest(canonical_request.as_bytes()));

        let string_to_sign = "AMZN-PAY-RSASSA-PSS\n".to_string() + &hashed_canonical_request;

        let signature = Self::sign(private_key, &string_to_sign);

        signature
    }

    fn get_timestamp() -> String {
        let date = Utc::now();
        date.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    fn sign(
        private_key_pem: &Secret<String>,
        string_to_sign: &String
    ) -> String {
        let salt_length: i32 = 32;

        let rsa = Rsa::private_key_from_pem(private_key_pem.peek().as_bytes()).expect("Failed to parse RSA private key");
        let pkey = PKey::from_rsa(rsa).expect("Failed to create PKey from RSA private key");

        let mut signer = Signer::new(MessageDigest::sha256(), &pkey).unwrap();
        signer.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
        let _ = signer.set_rsa_pss_saltlen(RsaPssSaltlen::custom(salt_length));
        signer.update(string_to_sign.as_bytes()).unwrap();

        let signature = signer.sign_to_vec().unwrap();

        encode(signature)
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
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Amazonpay
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let timestamp = Amazonpay::get_timestamp();
        let http_method = self.get_http_method();
        let payload = self.get_request_body(req, connectors)?;
        let hashed_payload = hex::encode(Sha256::digest(payload.get_inner_value().expose().as_bytes()));

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
                timestamp.into_masked(),
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

        if http_method == Method::Post {
            header.push((
                "x-amz-pay-idempotency-key".to_string(),
                req.connector_request_reference_id.clone().into_masked(),
            ));
        }

        let auth = amazonpay::AmazonpayAuthType::try_from(&req.connector_auth_type)?;
        let authorization = self.create_authorization_header(auth, &http_method, &hashed_payload, &header);
        header.push((
            headers::AUTHORIZATION.to_string(),
            authorization.into_masked(),
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
        // todo!()
        //    TODO! Check connector documentation, on which unit they are processing the currency.
        //    If the connector accepts amount in lower unit ( i.e cents for USD) then return api::CurrencyUnit::Minor,
        //    if connector accepts amount in base unit (i.e dollars for USD) then return api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.amazonpay.base_url.as_ref()
    }

    // fn get_auth_header(
    //     &self,
    //     auth_type: &ConnectorAuthType,
    // ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
    //     let auth = amazonpay::AmazonpayAuthType::try_from(auth_type)
    //         .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
    //     Ok(vec![(
    //         headers::AUTHORIZATION.to_string(),
    //         auth.api_key.expose().into_masked(),
    //     )])
    // }

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
            code: response.code,
            message: response.message,
            reason: response.reason,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Amazonpay {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Amazonpay {
    //TODO: implement sessions flow
    fn get_headers(
        &self,
        req: &PaymentsSessionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        // idempotency_key is not there for finalize checkout session but as it is a POST request, build_headers will push idempotency_key into the header vec, so need to handle this case
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsSessionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
        // Ok(format!("{}/checkoutSessions/{}/finalize", self.base_url(connectors)))  // add checkoutSessionId variable
    }

    // uncomment this after implementing try_from()

    /*
    fn get_request_body(
        &self,
        req: &PaymentsSessionRouterData,
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
    */

    fn build_request(
        &self,
        req: &PaymentsSessionRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsSessionType::get_url(
                    self, req, connectors,
                )?)
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
        data: &PaymentsSessionRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSessionRouterData, errors::ConnectorError> {
        let response: amazonpay::AmazonpayPaymentsResponse = res
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
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/charges", self.base_url(connectors)))
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
        let connector_req = amazonpay::AmazonpayPaymentsRequest::try_from(&connector_router_data)?;
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
        let response: amazonpay::AmazonpayPaymentsResponse = res
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Amazonpay {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &PaymentsSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
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
            .parse_struct("amazonpay PaymentsSyncResponse")
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
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
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
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_request_body method".to_string()).into())
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

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Amazonpay {}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Amazonpay {
    fn get_headers(
        &self,
        req: &RefundsRouterData<Execute>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
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
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
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

impl ConnectorSpecifications for Amazonpay {}
