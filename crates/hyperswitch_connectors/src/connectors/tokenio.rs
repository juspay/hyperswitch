pub mod transformers;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt},
    request::{Method, Request, RequestBuilder, RequestContent},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::ResultExt;
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
    types::Response,
    webhooks,
};
use masking::{ExposeInterface, Mask};
use openssl::ec::EcKey;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::Signer;
use transformers as tokenio;

use crate::{constants::headers, types::ResponseRouterData, utils};

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Tokenio {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}

impl Tokenio {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMajorUnitForConnector,
        }
    }
    // JWT helper methods
    fn create_jwt_token(
        &self,
        auth: &tokenio::TokenioAuthType,
        method: &str,
        path: &str,
        body: &RequestContent,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        // Create JWT header
        let exp_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?
            .as_millis()
            + 600_000; // 10 minutes

        let header = serde_json::json!({
            "alg": match auth.key_algorithm {
                tokenio::CryptoAlgorithm::RS256 => "RS256",
                tokenio::CryptoAlgorithm::ES256 => "ES256",
                tokenio::CryptoAlgorithm::EDDSA => "EdDSA",
            },
            "exp": exp_time,
            "mid": auth.merchant_id.clone().expose(),
            "kid": auth.key_id.clone().expose(),
            "method": method.to_uppercase(),
            "host": connectors.tokenio.base_url.trim_start_matches("https://").trim_end_matches("/"),
            "path": path,
            "typ": "JWT",
        });

        // Create JWT payload from request body (Token.io expects the request body directly)
        let payload = match body {
            RequestContent::Json(json_body) => serde_json::to_value(json_body)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
            _ => serde_json::json!({}),
        };

        println!("=== JWT CREATION DEBUG ===");
        println!(
            "1. Header (before encoding): {}",
            serde_json::to_string(&header)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?
        );
        println!(
            "2. Payload (before encoding): {}",
            serde_json::to_string(&payload)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?
        );

        // Base64URL encode header and payload
        let encoded_header = self.base64url_encode(
            serde_json::to_string(&header)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?
                .as_bytes(),
        )?;
        let encoded_payload = self.base64url_encode(
            serde_json::to_string(&payload)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?
                .as_bytes(),
        )?;

        println!("3. Encoded header: {}", encoded_header);
        println!("4. Encoded payload: {}", encoded_payload);

        // Create signing input
        let signing_input = format!("{}.{}", encoded_header, encoded_payload);
        println!("5. Signing input: {}", signing_input);

        // Sign the JWT based on algorithm
        let signature = match auth.key_algorithm {
            tokenio::CryptoAlgorithm::RS256 => {
                self.sign_rsa(&auth.private_key.clone().expose(), &signing_input)?
            }
            tokenio::CryptoAlgorithm::ES256 => {
                self.sign_ecdsa(&auth.private_key.clone().expose(), &signing_input)?
            }
            tokenio::CryptoAlgorithm::EDDSA => {
                self.sign_eddsa(&auth.private_key.clone().expose(), &signing_input)?
            }
        };

        let encoded_signature = self.base64url_encode(&signature)?;
        println!("6. Signature: {}", encoded_signature);

        let jwt = format!(
            "{}.{}.{}",
            encoded_header, encoded_payload, encoded_signature
        );

        println!("7. Final JWT: {}", jwt);
        println!("=== END DEBUG ===\n");

        dbg!(&jwt);

        Ok(jwt)
    }
    fn base64url_encode(&self, data: &[u8]) -> CustomResult<String, errors::ConnectorError> {
        Ok(URL_SAFE_NO_PAD.encode(data))
    }

    fn sign_rsa(
        &self,
        private_key_pem: &str,
        data: &str,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let rsa = Rsa::private_key_from_pem(private_key_pem.as_bytes())
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let pkey =
            PKey::from_rsa(rsa).change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let mut signer = Signer::new(MessageDigest::sha256(), &pkey)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        signer
            .update(data.as_bytes())
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let signature = signer
            .sign_to_vec()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(signature)
    }

    fn sign_ecdsa(
        &self,
        private_key_pem: &str,
        data: &str,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let ec_key = EcKey::private_key_from_pem(private_key_pem.as_bytes())
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let pkey = PKey::from_ec_key(ec_key)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let mut signer = Signer::new(MessageDigest::sha256(), &pkey)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        signer
            .update(data.as_bytes())
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let signature = signer
            .sign_to_vec()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(signature)
    }

    fn sign_eddsa(
        &self,
        private_key_pem: &str,
        data: &str,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let pkey = PKey::private_key_from_pem(private_key_pem.as_bytes())
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let mut signer = Signer::new_without_digest(&pkey)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        signer
            .update(data.as_bytes())
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let signature = signer
            .sign_to_vec()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(signature)
    }
}

impl api::Payment for Tokenio {}
impl api::PaymentSession for Tokenio {}
impl api::ConnectorAccessToken for Tokenio {}
impl api::MandateSetup for Tokenio {}
impl api::PaymentAuthorize for Tokenio {}
impl api::PaymentSync for Tokenio {}
impl api::PaymentCapture for Tokenio {}
impl api::PaymentVoid for Tokenio {}
impl api::Refund for Tokenio {}
impl api::RefundExecute for Tokenio {}
impl api::RefundSync for Tokenio {}
impl api::PaymentToken for Tokenio {}

impl ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
    for Tokenio
{
    // Not Implemented (R)
}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Tokenio
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &RouterData<Flow, Request, Response>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        // Basic headers - JWT will be added in individual build_request methods
        let header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];

        Ok(header)
    }
}

impl ConnectorCommon for Tokenio {
    fn id(&self) -> &'static str {
        "tokenio"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.tokenio.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        // JWT auth is handled in build_request methods
        Ok(vec![])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: tokenio::TokenioErrorResponse = res
            .response
            .parse_struct("TokenioErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.get_error_code(),
            message: response.get_message(),
            reason: Some(response.get_message()),
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        })
    }
}

impl ConnectorValidation for Tokenio {
    //TODO: implement functions when support enabled
}

impl ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData> for Tokenio {
    //TODO: implement sessions flow
}

impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for Tokenio {}

impl ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for Tokenio {}

impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Tokenio {
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
        let base_url = self.base_url(connectors);
        Ok(format!("{}/v2/payments", base_url))
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

        let connector_router_data = tokenio::TokenioRouterData::from((amount, req));
        let connector_req = tokenio::TokenioPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let auth = tokenio::TokenioAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let url = self.get_url(req, connectors)?;
        let body = self.get_request_body(req, connectors)?;

        // Create JWT for authentication
        let jwt = self.create_jwt_token(&auth, "POST", "/v2/payments", &body, connectors)?;

        // Build headers with JWT authorization
        let headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                "application/json".to_string().into(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", jwt).into_masked(),
            ),
        ];

        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&url)
                .attach_default_headers()
                .headers(headers)
                .set_body(body)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        dbg!(res.response.clone());

        let response: tokenio::TokenioPaymentsResponse = res
            .response
            .parse_struct("Tokenio PaymentsAuthorizeResponse")
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

impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for Tokenio {
    fn get_headers(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        // For GET requests, we need JWT with no body
        let auth = tokenio::TokenioAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let empty_body = RequestContent::Json(Box::new(serde_json::json!({})));
        let jwt = self.create_jwt_token(
            &auth,
            "GET",
            &format!(
                "/v2/payments/{}",
                req.request
                    .connector_transaction_id
                    .get_connector_transaction_id()
                    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
            ),
            &empty_body,
            connectors,
        )?;
        let headers = vec![
            (
                headers::CONTENT_TYPE.to_string(),
                "application/json".to_string().into(),
            ),
            (
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", jwt).into_masked(),
            ),
        ];
        Ok(headers)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        let base_url = self.base_url(connectors);
        Ok(format!("{}/v2/payments/{}", base_url, connector_payment_id))
    }

    fn build_request(
        &self,
        req: &PaymentsSyncRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Get)
                .url(&self.get_url(req, connectors)?)
                .attach_default_headers()
                .headers(self.get_headers(req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
        dbg!(res.response.clone());
        let response: tokenio::TokenioPaymentsResponse = res
            .response
            .parse_struct("tokenio TokenioPaymentsResponse")
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

impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData> for Tokenio {
    fn build_request(
        &self,
        _req: &PaymentsCaptureRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Capture".to_string(),
            connector: "Tokenio".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData> for Tokenio {
    fn build_request(
        &self,
        _req: &PaymentsCancelRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Refunds".to_string(),
            connector: "Tokenio".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for Tokenio {
    fn build_request(
        &self,
        _req: &RefundsRouterData<Execute>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Refunds".to_string(),
            connector: "Tokenio".to_string(),
        }
        .into())
    }
}

impl ConnectorIntegration<RSync, RefundsData, RefundsResponseData> for Tokenio {
    fn build_request(
        &self,
        _req: &RefundSyncRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::FlowNotSupported {
            flow: "Refund Sync".to_string(),
            connector: "Tokenio".to_string(),
        }
        .into())
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Tokenio {
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook_payload: tokenio::TokenioWebhookPayload = request
            .body
            .parse_struct("TokenioWebhookPayload")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        match &webhook_payload.event_data {
            tokenio::TokenioWebhookEventData::PaymentV2 { payment } => {
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    api_models::payments::PaymentIdType::ConnectorTransactionId(payment.id.clone()),
                ))
            }
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        // Check token-event header first
        let event_type = if let Some(header_value) = request.headers.get("token-event") {
            header_value
                .to_str()
                .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?
                .to_string()
        } else {
            // Fallback to parsing body for eventType field
            let webhook_payload: tokenio::TokenioWebhookPayload = request
                .body
                .parse_struct("TokenioWebhookPayload")
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

            webhook_payload
                .event_type
                .ok_or(errors::ConnectorError::WebhookEventTypeNotFound)?
        };
        match event_type.as_str() {
            "PAYMENT_STATUS_CHANGED" => {
                let webhook_payload: tokenio::TokenioWebhookPayload = request
                    .body
                    .parse_struct("TokenioWebhookPayload")
                    .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

                let tokenio::TokenioWebhookEventData::PaymentV2 { payment } =
                    &webhook_payload.event_data;

                match payment.status.as_str() {
                    "INITIATION_COMPLETED" => {
                        Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing)
                    }
                    "PAYMENT_COMPLETED" => {
                        Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess)
                    }
                    "PAYMENT_FAILED" => {
                        Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure)
                    }
                    "PAYMENT_CANCELLED" => {
                        Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelled)
                    }
                    "INITIATION_REJECTED" => {
                        Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure)
                    }
                    "INITIATION_PROCESSING" => {
                        Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing)
                    }
                    _ => Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported),
                }
            }
            _ => Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported),
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook_payload: tokenio::TokenioWebhookPayload = request
            .body
            .parse_struct("TokenioWebhookPayload")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(Box::new(webhook_payload))
    }

    // // Override source verification to handle Token.io ED25519 signature verification
    // fn get_webhook_source_verification_algorithm(
    //     &self,
    //     _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    // ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
    //     // Token.io uses ED25519 signature verification
    //     Ok(Box::new(crypto::Ed25519))
    // }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        // Extract token-signature header
        let signature = request
            .headers
            .get("token-signature")
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?
            .to_str()
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;

        // Decode base64url signature (Token.io uses base64url encoding)
        let decoded_signature = URL_SAFE_NO_PAD
            .decode(signature)
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;

        Ok(decoded_signature)
    }
}

impl ConnectorSpecifications for Tokenio {}
