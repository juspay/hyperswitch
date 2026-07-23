use api_models::unreferenced_refund::{RecipientCardData, RecipientPaymentMethodData};
use base64::Engine;
use bytes::Bytes;
use common_utils::{
    consts::BASE64_ENGINE,
    crypto::encrypt_rsa_oaep_sha256,
    request::{Method, Request, RequestContent},
    types::FloatMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{connector_endpoints::Connectors, router_data::ConnectorAuthType};
use hyperswitch_interfaces::{
    errors::ConnectorError,
    relay::{ConnectorRelayIntegration, UnreferencedRefundResponse, UnreferencedRefundRouterData},
};
use hyperswitch_masking::{ExposeInterface, Mask, Maskable, PeekInterface, Secret};
use ring::hmac;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct Fiservcommercehub;

struct FiservcommercehubAuthType {
    api_key: Secret<String>,
    api_secret: Secret<String>,
    merchant_id: Secret<String>,
    terminal_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for FiservcommercehubAuthType {
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(auth: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth {
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Ok(Self {
                api_key: api_key.clone(),
                api_secret: api_secret.clone(),
                merchant_id: key1.clone(),
                terminal_id: key2.clone(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl FiservcommercehubAuthType {
    fn generate_hmac_signature(
        &self,
        client_request_id: &str,
        timestamp_str: &str,
        payload: &str,
    ) -> String {
        let raw = format!(
            "{}{}{}{}",
            self.api_key.peek(),
            client_request_id,
            timestamp_str,
            payload
        );
        let key = hmac::Key::new(
            hmac::HMAC_SHA256,
            self.api_secret.clone().expose().as_bytes(),
        );
        BASE64_ENGINE.encode(hmac::sign(&key, raw.as_bytes()).as_ref())
    }
}

// ── Status Mapping ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum FiservcommercehubRefundState {
    Approved,
    Captured,
    Authorized,
    Pending,
    Declined,
    Rejected,
    Failed,
    Cancelled,
}

impl From<FiservcommercehubRefundState> for common_enums::RefundStatus {
    fn from(state: FiservcommercehubRefundState) -> Self {
        match state {
            FiservcommercehubRefundState::Approved | FiservcommercehubRefundState::Captured => {
                Self::Success
            }
            FiservcommercehubRefundState::Authorized | FiservcommercehubRefundState::Pending => {
                Self::Pending
            }
            FiservcommercehubRefundState::Declined
            | FiservcommercehubRefundState::Rejected
            | FiservcommercehubRefundState::Failed
            | FiservcommercehubRefundState::Cancelled => Self::Failure,
        }
    }
}

// ── Request Structs ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct Amount {
    currency: String,
    total: FloatMajorUnit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionData {
    key_id: String,
    encryption_type: String,
    encryption_block: Secret<String>,
    encryption_block_fields: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EncryptedSource {
    source_type: String,
    encryption_data: EncryptionData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MerchantDetails {
    merchant_id: Secret<String>,
    terminal_id: Secret<String>,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum TransactionInteractionOrigin {
    #[default]
    Ecom,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum TransactionInteractionEciIndicator {
    #[default]
    ChannelEncrypted,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum TransactionInteractionPosConditionCode {
    #[default]
    CardNotPresentEcom,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionInteraction {
    origin: TransactionInteractionOrigin,
    eci_indicator: TransactionInteractionEciIndicator,
    pos_condition_code: TransactionInteractionPosConditionCode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_transaction_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FiservcommercehubCreditRequestEncrypted {
    amount: Amount,
    source: EncryptedSource,
    merchant_details: MerchantDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    transaction_details: Option<TransactionDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transaction_interaction: Option<TransactionInteraction>,
}

// ── Response Structs ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionProcessingDetails {
    transaction_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GatewayResponse {
    transaction_state: FiservcommercehubRefundState,
    transaction_processing_details: TransactionProcessingDetails,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProcessorResponseDetails {
    response_code: String,
    response_message: String,
    approval_code: Option<String>,
    network_routed: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaymentReceipt {
    processor_response_details: ProcessorResponseDetails,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NetworkDetails {
    network_response_code: Option<String>,
    transaction_identifier: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FiservcommercehubCreditResponse {
    gateway_response: GatewayResponse,
    payment_receipt: Option<PaymentReceipt>,
    network_details: Option<NetworkDetails>,
}

impl TryFrom<&FiservcommercehubCreditResponse>
    for hyperswitch_domain_models::relay::RelayResponseData
{
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(response: &FiservcommercehubCreditResponse) -> Result<Self, Self::Error> {
        let processor_response_details = response
            .payment_receipt
            .as_ref()
            .map(|receipt| &receipt.processor_response_details);
        let network_details = response.network_details.as_ref();

        Ok(Self {
            approval_code: processor_response_details
                .and_then(|processor| processor.approval_code.clone()),
            network: processor_response_details
                .and_then(|processor| processor.network_routed.as_deref())
                .and_then(|network_routed| {
                    serde_json::from_value::<common_enums::CardNetwork>(serde_json::Value::String(
                        network_routed.to_owned(),
                    ))
                    .ok()
                }),
            network_transaction_id: network_details
                .and_then(|details| details.transaction_identifier.clone()),
            network_response_code: network_details
                .and_then(|details| details.network_response_code.clone()),
        })
    }
}

#[derive(Debug, Deserialize)]
struct ErrorEntry {
    #[serde(rename = "type")]
    error_type: Option<String>,
    code: Option<String>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct FiservcommercehubErrorResponse {
    error: Vec<ErrorEntry>,
}

const ACCESS_TOKEN_SEPARATOR: &str = "|||";

// ── Helper Methods ────────────────────────────────────────────────────────────

impl Fiservcommercehub {
    /// Build the 7 HMAC auth headers. Must be called with the already-serialised
    /// body string so that the signature covers the exact bytes that will be sent.
    fn build_auth_headers(
        auth: &FiservcommercehubAuthType,
        body_str: &str,
    ) -> Vec<(String, Maskable<String>)> {
        let timestamp_ms = OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
        let timestamp_str = timestamp_ms.to_string();
        let client_request_id = Uuid::new_v4().to_string();
        let signature = auth.generate_hmac_signature(&client_request_id, &timestamp_str, body_str);

        vec![
            (
                "Content-Type".to_string(),
                "application/json".to_string().into(),
            ),
            (
                "Api-Key".to_string(),
                auth.api_key.peek().to_string().into_masked(),
            ),
            ("Timestamp".to_string(), timestamp_str.into()),
            ("Client-Request-Id".to_string(), client_request_id.into()),
            ("Authorization".to_string(), signature.into_masked()),
            ("Auth-Token-Type".to_string(), "HMAC".to_string().into()),
            ("Accept-Language".to_string(), "en".to_string().into()),
        ]
    }

    /// Encrypt card fields via RSA-OAEP-SHA256 using the public key embedded in
    /// `access_token` ("keyId|||Base64DERPublicKey").
    fn encrypt_card(
        card: &RecipientCardData,
        access_token: &str,
    ) -> error_stack::Result<EncryptionData, ConnectorError> {
        let parts: Vec<&str> = access_token.split(ACCESS_TOKEN_SEPARATOR).collect();

        let key_id = parts
            .first()
            .ok_or(ConnectorError::RequestEncodingFailed)
            .attach_printable("access_token missing key_id")?
            .to_string();

        let encoded_public_key = parts
            .get(1)
            .ok_or(ConnectorError::RequestEncodingFailed)
            .attach_printable("access_token missing encoded_public_key")?;

        let public_key_der = BASE64_ENGINE
            .decode(encoded_public_key)
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to decode Base64 RSA public key")?;

        let card_data = card.card_number.peek().to_string();
        let name_on_card = card.card_holder_name.as_ref().map(|n| n.peek().to_string());
        let expiration_month = card.card_exp_month.peek().to_string();
        let expiration_year = card.get_expiry_year_4_digit().peek().to_string();

        let plain_block = format!(
            "{}{}{}{}",
            card_data,
            expiration_month,
            expiration_year,
            name_on_card.as_deref().unwrap_or(""),
        );

        let ciphertext = encrypt_rsa_oaep_sha256(&public_key_der, plain_block.as_bytes())
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("RSA-OAEP card encryption failed")?;

        let encryption_block = Secret::new(BASE64_ENGINE.encode(&ciphertext));

        let mut encryption_block_fields = format!(
            "card.cardData:{},card.expirationMonth:{},card.expirationYear:{}",
            card_data.len(),
            expiration_month.len(),
            expiration_year.len(),
        );
        name_on_card.as_ref().map(|name| {
            encryption_block_fields.push_str(&format!(",card.nameOnCard:{}", name.len()));
        });

        Ok(EncryptionData {
            key_id,
            encryption_type: "RSA".to_string(),
            encryption_block,
            encryption_block_fields,
        })
    }
}

impl ConnectorRelayIntegration for Fiservcommercehub {
    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.fiservcommercehub.base_url.as_ref()
    }

    fn supports_access_token(&self) -> bool {
        true
    }

    fn build_relay_request(
        &self,
        router_data: &UnreferencedRefundRouterData<'_>,
    ) -> error_stack::Result<Request, ConnectorError> {
        let auth = FiservcommercehubAuthType::try_from(router_data.auth_type)
            .change_context(ConnectorError::FailedToObtainAuthType)?;

        let request = router_data.request;

        let total = request
            .amount
            .to_major_unit_as_f64(request.currency)
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to convert minor unit amount to major unit float")?;

        let amount = Amount {
            currency: request.currency.to_string(),
            total,
        };
        let merchant_details = MerchantDetails {
            merchant_id: auth.merchant_id.clone(),
            terminal_id: auth.terminal_id.clone(),
        };
        let transaction_interaction = Some(TransactionInteraction::default());

        let access_token = router_data
            .access_token
            .as_ref()
            .map(|t| t.token.peek().as_str())
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "access_token",
            })
            .attach_printable(
                "FiservCommerceHub requires an RSA public key via access_token for card encryption",
            )?;

        let RecipientPaymentMethodData::Card(ref card) = request.recipient_payment_method_data;

        let url = format!("{}payments/v1/refunds", router_data.base_url);

        let encryption_data = Self::encrypt_card(card, access_token)?;

        let transaction_details = router_data
            .connector_resource_id
            .map(|connector_resource_id| TransactionDetails {
                merchant_transaction_id: Some(connector_resource_id.to_owned()),
            });

        let body = FiservcommercehubCreditRequestEncrypted {
            amount,
            source: EncryptedSource {
                source_type: "PaymentCard".to_string(),
                encryption_data,
            },
            merchant_details,
            transaction_details,
            transaction_interaction,
        };

        // Serialize body first, then build HMAC over that exact string.
        let body_str = serde_json::to_string(&body)
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to serialize encrypted credit request")?;

        let mut req = Request::new(Method::Post, &url);
        req.set_body(RequestContent::Json(Box::new(body)));

        for (name, value) in Self::build_auth_headers(&auth, &body_str) {
            req.add_header(&name, value);
        }

        Ok(req)
    }

    fn handle_relay_success_response(
        &self,
        response: Bytes,
    ) -> error_stack::Result<UnreferencedRefundResponse, ConnectorError> {
        let raw_response = serde_json::from_slice::<serde_json::Value>(&response)
            .ok()
            .map(Secret::new);

        let parsed: FiservcommercehubCreditResponse = serde_json::from_slice(&response)
            .change_context(ConnectorError::ResponseDeserializationFailed)
            .attach_printable("Failed to parse success response from Commerce Hub")?;

        let refund_status =
            common_enums::RefundStatus::from(parsed.gateway_response.transaction_state.clone());

        let connector_refund_id = Some(
            parsed
                .gateway_response
                .transaction_processing_details
                .transaction_id
                .clone(),
        );

        let (error_code, error_message) = parsed
            .payment_receipt
            .as_ref()
            .filter(|_| refund_status == common_enums::RefundStatus::Failure)
            .map(|receipt| {
                let processor_response = &receipt.processor_response_details;
                (
                    Some(processor_response.response_code.clone()),
                    Some(processor_response.response_message.clone()),
                )
            })
            .unwrap_or((None, None));

        let response_data =
            hyperswitch_domain_models::relay::RelayResponseData::try_from(&parsed).ok();

        Ok(UnreferencedRefundResponse {
            connector_refund_id,
            refund_status,
            error_code,
            error_message,
            raw_response,
            response_data,
        })
    }

    fn get_relay_error_response(
        &self,
        response: Bytes,
        _status_code: u16,
    ) -> error_stack::Result<UnreferencedRefundResponse, ConnectorError> {
        let raw_response = serde_json::from_slice::<serde_json::Value>(&response)
            .ok()
            .map(Secret::new);

        let err_resp: FiservcommercehubErrorResponse = serde_json::from_slice(&response)
            .change_context(ConnectorError::ResponseDeserializationFailed)
            .attach_printable("Failed to parse error response from Commerce Hub")?;
        let entry = err_resp.error.into_iter().next();
        let code = entry
            .as_ref()
            .and_then(|e| e.code.as_deref().or(e.error_type.as_deref()))
            .unwrap_or("UNKNOWN_ERROR")
            .to_string();
        let message = entry
            .map(|e| e.message)
            .unwrap_or_else(|| "Unknown error from Commerce Hub".to_string());

        Ok(UnreferencedRefundResponse {
            connector_refund_id: None,
            refund_status: common_enums::RefundStatus::Failure,
            error_code: Some(code),
            error_message: Some(message),
            raw_response,
            response_data: None,
        })
    }
}
