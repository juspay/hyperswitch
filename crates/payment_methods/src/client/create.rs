//! Create payment method flow types and dummy models.

use api_models::payments;
use cards::CardNumber;
use common_utils::{
    id_type, pii,
    request::{Method, RequestContent},
    types::MinorUnit,
};
use hyperswitch_domain_models::payment_method_data::PaymentMethodData;
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
/// V1-facing create flow type.
#[derive(Debug)]
pub struct CreatePaymentMethod;

#[derive(Debug)]
pub struct CreatePaymentMethodV1Request {
    pub merchant_id: id_type::MerchantId,
    pub payment_method: common_enums::PaymentMethod,
    pub payment_method_type: common_enums::PaymentMethodType,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub customer_id: id_type::CustomerId, // Payment method data will be saved when customer acceptance is given, hence customer id will always be present
    pub payment_method_data: PaymentMethodData,
    pub billing: Option<hyperswitch_domain_models::address::Address>,
    pub network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
    pub storage_type: Option<common_enums::StorageType>,
    pub modular_service_prefix: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModularPMCreateRequest {
    pub payment_method_type: common_enums::PaymentMethod,
    pub payment_method_subtype: common_enums::PaymentMethodType,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub customer_id: id_type::CustomerId, // Payment method data will be saved when customer acceptance is given, hence customer id will always be present
    pub payment_method_data: PaymentMethodCreateData,
    pub billing: Option<payments::Address>,
    pub psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
    pub network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
    pub storage_type: Option<common_enums::StorageType>,
}

//This struct will be deprecated when we fully migrate to Modular PMs
//cannot reuse CardDetail since CardDetail under v2 does not have card_issuing_country code.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardDetail {
    pub card_number: CardNumber,
    pub card_exp_month: Secret<String>,
    pub card_exp_year: Secret<String>,
    pub card_holder_name: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
    pub card_issuing_country: Option<String>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_issuer: Option<String>,
    pub card_type: Option<common_enums::CardType>,
    pub card_cvc: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodCreateData {
    Card(CardDetail),
}

#[derive(Clone, Debug, Deserialize)]
pub struct ModularPaymentMethodResponse {
    //payment method id
    pub id: String,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<id_type::CustomerId>,
    pub payment_method_type: Option<common_enums::PaymentMethod>,
    pub payment_method_subtype: Option<common_enums::PaymentMethodType>,
    pub recurring_enabled: Option<bool>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_used_at: Option<PrimitiveDateTime>,
    pub payment_method_data: Option<PaymentMethodResponseData>,
    pub connector_tokens: Option<Vec<ConnectorTokenDetails>>,
    pub network_token: Option<api_models::payment_methods::NetworkTokenResponse>,
    pub storage_type: Option<common_enums::StorageType>,
    pub billing: Option<hyperswitch_domain_models::address::Address>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodResponseData {
    Card(api_models::payment_methods::CardDetailFromLocker),
}

#[derive(Clone, Debug)]
pub struct CreatePaymentMethodResponse {
    //payment method id
    pub payment_method_id: String,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<id_type::CustomerId>,
    pub payment_method: Option<common_enums::PaymentMethod>,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub recurring_enabled: Option<bool>,
    pub created: Option<PrimitiveDateTime>,
    pub last_used_at: Option<PrimitiveDateTime>,
    pub payment_method_data: Option<PaymentMethodResponseData>,
    pub connector_tokens: Option<Vec<ConnectorTokenDetails>>,
    pub network_token: Option<api_models::payment_methods::NetworkTokenResponse>,
    pub billing: Option<hyperswitch_domain_models::address::Address>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorTokenDetails {
    pub connector_id: id_type::MerchantConnectorAccountId,
    pub token_type: common_enums::TokenizationType,
    pub status: common_enums::ConnectorTokenStatus,
    pub connector_token_request_reference_id: Option<String>,
    pub original_payment_authorized_amount: Option<MinorUnit>,
    pub original_payment_authorized_currency: Option<common_enums::Currency>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub token: Secret<String>,
}

impl TryFrom<PaymentMethodData> for PaymentMethodCreateData {
    type Error = MicroserviceClientError;

    fn try_from(value: PaymentMethodData) -> Result<Self, Self::Error> {
        match value {
            PaymentMethodData::Card(card) => {
                let card_detail = CardDetail {
                    card_number: card.card_number,
                    card_exp_month: card.card_exp_month,
                    card_exp_year: card.card_exp_year,
                    card_holder_name: card.card_holder_name,
                    nick_name: card.nick_name,
                    card_issuing_country: card.card_issuing_country,
                    card_network: card.card_network,
                    card_issuer: card.card_issuer,
                    card_type: None,
                    card_cvc: Some(card.card_cvc),
                };
                Ok(Self::Card(card_detail))
            }
            _ => Err(MicroserviceClientError {
                operation: "CreatePaymentMethodV1Request to ModularPMCreateRequest".to_string(),
                kind: MicroserviceClientErrorKind::InvalidRequest(
                    "Unsupported payment method type for modular PM creation".to_string(),
                ),
            }),
        }
    }
}

impl TryFrom<&CreatePaymentMethodV1Request> for ModularPMCreateRequest {
    type Error = MicroserviceClientError;

    fn try_from(request: &CreatePaymentMethodV1Request) -> Result<Self, Self::Error> {
        let payment_method_data =
            PaymentMethodCreateData::try_from(request.payment_method_data.clone())?;
        Ok(Self {
            payment_method_type: request.payment_method,
            payment_method_subtype: request.payment_method_type,
            metadata: request.metadata.clone(),
            customer_id: request.customer_id.clone(),
            payment_method_data,
            billing: request
                .billing
                .as_ref()
                .map(|billing| billing.clone().into()),
            psp_tokenization: None,
            network_tokenization: request.network_tokenization.clone(),
            storage_type: request.storage_type,
        })
    }
}

impl TryFrom<ModularPaymentMethodResponse> for CreatePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(response: ModularPaymentMethodResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: response.id,
            merchant_id: response.merchant_id,
            customer_id: response.customer_id,
            payment_method: response.payment_method_type,
            payment_method_type: response.payment_method_subtype,
            recurring_enabled: response.recurring_enabled,
            created: response.created,
            last_used_at: response.last_used_at,
            payment_method_data: response.payment_method_data,
            connector_tokens: response.connector_tokens,
            network_token: response.network_token,
            billing: response.billing,
        })
    }
}

impl CreatePaymentMethod {
    fn build_body(&self, request: ModularPMCreateRequest) -> Option<RequestContent> {
        Some(RequestContent::Json(Box::new(request)))
    }

    fn build_path_params(
        &self,
        request: &CreatePaymentMethodV1Request,
    ) -> Vec<(&'static str, String)> {
        vec![("prefix", request.modular_service_prefix.clone())]
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    CreatePaymentMethod,
    method = Method::Post,
    path = "/{prefix}/payment-methods",
    v1_request = CreatePaymentMethodV1Request,
    v2_request = ModularPMCreateRequest,
    v2_response = ModularPaymentMethodResponse,
    v1_response = CreatePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    body = CreatePaymentMethod::build_body,
    path_params = CreatePaymentMethod::build_path_params
);
