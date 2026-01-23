//! Retrieve payment method flow types and models.

use api_models::payment_methods::{CardDetailFromLocker, PaymentMethodId};
use cards::CardNumber;
use common_enums::{PaymentMethod, PaymentMethodType};
use common_utils::{id_type, pii, request::Method};
use hyperswitch_domain_models::payment_method_data::NetworkTokenDetailsPaymentMethod;
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use serde::Deserialize;
use time;
use api_models::payment_methods::PaymentMethodResponse as RetrievePaymentMethodResponse;

/// V1-facing retrieve flow type.
#[derive(Debug)]
pub struct RetrievePaymentMethod;

/// V1-facing retrieve request payload.
#[derive(Debug)]
pub struct RetrievePaymentMethodV1Request {
    pub payment_method_id: PaymentMethodId,
}

/// V2 modular service request payload.
#[derive(Clone, Debug)]
pub struct ModularPMRetrieveResquest {
    pub payment_method_id: PaymentMethodId,
}

/// V2 PaymentMethodResponse as returned by the V2 API.
/// This is a copy of the V2 PaymentMethodResponse struct from api_models for use in V1-only builds.
#[derive(Clone, Debug, Deserialize)]
pub struct ModularPMRetrieveResponse {
    pub id: String,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<id_type::CustomerId>,
    pub payment_method_type: Option<PaymentMethod>,
    pub payment_method_subtype: Option<PaymentMethodType>,
    pub recurring_enabled: Option<bool>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_used_at: Option<time::PrimitiveDateTime>,
    pub payment_method_data: Option<PaymentMethodResponseData>,
    pub connector_tokens: Option<Vec<ConnectorTokenDetails>>,
    pub network_token: Option<NetworkTokenResponse>,
    pub storage_type: Option<common_enums::StorageType>,
    pub card_cvc_token_storage: Option<CardCVCTokenStorageDetails>,
}

/// V2 ConnectorTokenDetails (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct ConnectorTokenDetails {
    pub connector_id: id_type::MerchantConnectorAccountId,
    pub token_type: common_enums::TokenizationType,
    pub status: common_enums::ConnectorTokenStatus,
    pub connector_token_request_reference_id: Option<String>,
    pub original_payment_authorized_amount: Option<common_utils::types::MinorUnit>,
    pub original_payment_authorized_currency: Option<common_enums::Currency>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub token: masking::Secret<String>,
}

/// V2 NetworkTokenResponse (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct NetworkTokenResponse {
    pub payment_method_data: NetworkTokenDetailsPaymentMethod,
}

/// V2 CardCVCTokenStorageDetails (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct CardCVCTokenStorageDetails {
    pub is_stored: bool,

    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub expires_at: Option<time::PrimitiveDateTime>,
}

/// V2 PaymentMethodResponseData enum
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "payment_method_data")]
pub enum PaymentMethodResponseData {
    Card(CardDetailFromLockerV2),
}

/// V2 CardDetailFromLocker for deserialization
#[derive(Clone, Debug, Deserialize)]
pub struct CardDetailFromLockerV2 {
    pub issuer_country: Option<common_enums::CountryAlpha2>,
    pub last4_digits: Option<String>,
    #[serde(skip)]
    pub card_number: Option<CardNumber>,
    pub expiry_month: Option<masking::Secret<String>>,
    pub expiry_year: Option<masking::Secret<String>>,
    pub card_holder_name: Option<masking::Secret<String>>,
    pub card_fingerprint: Option<masking::Secret<String>>,
    pub nick_name: Option<masking::Secret<String>>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_isin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_type: Option<String>,
    pub saved_to_locker: bool,
}

impl TryFrom<&RetrievePaymentMethodV1Request> for ModularPMRetrieveResquest {
    type Error = MicroserviceClientError;

    fn try_from(value: &RetrievePaymentMethodV1Request) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: value.payment_method_id.clone(),
        })
    }
}

impl TryFrom<ModularPMRetrieveResponse> for RetrievePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(v2_resp: ModularPMRetrieveResponse) -> Result<Self, Self::Error> {
        // Extract payment_method_id from GlobalPaymentMethodId
        let payment_method_id = v2_resp.id.clone();

        // Convert GlobalCustomerId to CustomerId
        let customer_id = v2_resp
            .customer_id;

        // Convert card details from V2 to V1 format
        let card = v2_resp.payment_method_data.map(|pmd| match pmd {
            PaymentMethodResponseData::Card(v2_card) => CardDetailFromLocker {
                scheme: None,
                issuer_country: v2_card.issuer_country.map(|c| c.to_string()),
                issuer_country_code: None,
                last4_digits: v2_card.last4_digits,
                card_number: v2_card.card_number,
                expiry_month: v2_card.expiry_month,
                expiry_year: v2_card.expiry_year,
                card_token: None,
                card_holder_name: v2_card.card_holder_name,
                card_fingerprint: v2_card.card_fingerprint,
                nick_name: v2_card.nick_name,
                card_network: v2_card.card_network,
                card_isin: v2_card.card_isin,
                card_issuer: v2_card.card_issuer,
                card_type: v2_card.card_type,
                saved_to_locker: v2_card.saved_to_locker,
            },
        });

        Ok(Self {
            payment_method_id,
            merchant_id: v2_resp.merchant_id,
            customer_id,
            payment_method: v2_resp.payment_method_type,
            payment_method_type: v2_resp.payment_method_subtype,
            card,
            recurring_enabled: v2_resp.recurring_enabled,
            created: v2_resp.created,
            last_used_at: v2_resp.last_used_at,
            installment_payment_enabled: None,
            payment_experience: None,
            metadata: None,
            bank_transfer: None,
            client_secret: None,
        })
    }
}

impl RetrievePaymentMethod {
    fn validate_request(
        &self,
        request: &RetrievePaymentMethodV1Request,
    ) -> Result<(), MicroserviceClientError> {
        if request
            .payment_method_id
            .payment_method_id
            .trim()
            .is_empty()
        {
            return Err(MicroserviceClientError {
                operation: std::any::type_name::<Self>().to_string(),
                kind: MicroserviceClientErrorKind::InvalidRequest(
                    "Payment method ID cannot be empty".to_string(),
                ),
            });
        }
        Ok(())
    }

    fn build_path_params(
        &self,
        request: &ModularPMRetrieveResquest,
    ) -> Vec<(&'static str, String)> {
        vec![("id", request.payment_method_id.payment_method_id.clone())]
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    RetrievePaymentMethod,
    method = Method::Get,
    path = "/v2/payment-methods/{id}",
    v1_request = RetrievePaymentMethodV1Request,
    v2_request = ModularPMRetrieveResquest,
    v2_response = ModularPMRetrieveResponse,
    v1_response = RetrievePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    path_params = RetrievePaymentMethod::build_path_params,
    validate = RetrievePaymentMethod::validate_request
);
