//! Create payment method flow types and dummy models.

use common_utils::request::{Method, RequestContent};
use hyperswitch_interfaces::micro_service::MicroserviceClientError;
use serde::Deserialize;
use serde_json::Value;

/// V1-facing create flow type.
#[derive(Debug)]
pub struct CreatePaymentMethod;

#[derive(Debug)]
pub struct CreatePaymentMethodV1Request {
    pub merchant_id: id_type::MerchantId,
    pub payment_method: common_enums::PaymentMethod,
    pub payment_method_type: common_enums::PaymentMethodType,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub customer_id: id_types::CustomerId, // Payment method data will be saved when customer acceptance is given, hence customer id will always be present
    pub payment_method_data: api_models::payments::PaymentMethodData,
    pub billing: Option<payments::Address>,
    pub network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
    pub storage_type: Option<common_enums::StorageType>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModularPMCreateRequest {
    pub payment_method_type: common_enums::PaymentMethod,
    pub payment_method_subtype: common_enums::PaymentMethodType,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub customer_id: id_types::CustomerId, // Payment method data will be saved when customer acceptance is given, hence customer id will always be present
    pub payment_method_data: PaymentMethodCreateData,
    pub billing: Option<payments::Address>,
    pub psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
    pub network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
    pub storage_type: Option<common_enums::StorageType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentMethodCreateData {
    Card(pm_api_models::CardDetail),
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreatePaymentMethodV2Response {
    //payment method id
    pub id: String,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<CustomerId>,
    pub payment_method_type: Option<common_enums::PaymentMethod>,
    pub payment_method_subtype: Option<common_enums::PaymentMethodType>,
    pub recurring_enabled: Option<bool>,
    pub created: Option<time::PrimitiveDateTime>,
    pub last_used_at: Option<time::PrimitiveDateTime>,
    pub payment_method_data: Option<PaymentMethodResponseData>,
    pub connector_tokens: Option<Vec<ConnectorTokenDetails>>,
    pub network_token: Option<pm_api_models::NetworkTokenResponse>,
    pub storage_type: Option<common_enums::StorageType>,
    pub card_cvc_token_storage: Option<CardCVCTokenStorageDetails>,
}

#[derive(Clone, Debug)]
pub struct CreatePaymentMethodResponse {
    //payment method id
    pub payment_method_id: String,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<CustomerId>,
    pub payment_method_type: Option<common_enums::PaymentMethod>,
    pub payment_method_subtype: Option<common_enums::PaymentMethodType>,
    pub recurring_enabled: Option<bool>,
    pub created: Option<time::PrimitiveDateTime>,
    pub last_used_at: Option<time::PrimitiveDateTime>,
    pub payment_method_data: Option<PaymentMethodResponseData>,
    pub connector_tokens: Option<Vec<ConnectorTokenDetails>>,
    pub network_token: Option<pm_api_models::NetworkTokenResponse>
}


impl TryFrom<api_models::payments::PaymentMethodData> for PaymentMethodCreateData {
    type Error = errors::ApiErrorResponse;

    fn try_from(value: api_models::payments::PaymentMethodData) -> Result<Self, Self::Error> {
        match value {
            api_models::payments::PaymentMethodData::Card(card) => {
                let card_detail = pm_api_models::CardDetail {
                    card_number: card.card_number,
                    card_exp_month: card.card_exp_month,
                    card_exp_year: card.card_exp_year,
                    card_holder_name: card.card_holder_name,
                    nick_name: card.nick_name,
                    card_issuing_country: card.card_issuing_country,
                    card_network: None,
                    card_issuer: card.card_issuer,
                    card_type: None,
                    card_cvc: None,
                    card_issuing_country_code: card.card_issuing_country_code,
                };
                Ok(PaymentMethodCreateData::Card(card_detail))
            }
            _ => Err(errors::ApiErrorResponse::NotSupported {
                message: "Unsupported payment method type for modular PM creation".to_string(),
            }),
        }
    }
}

impl TryFrom<&CreatePaymentMethodV1Request> for ModularPMCreateRequest {
    type Error = MicroserviceClientError;

    fn try_from(request: &CreatePaymentMethodV1Request) -> Result<Self, Self::Error> {
        let payment_method_data = PaymentMethodCreateData::try_from(payment_method_data)?;
        Ok(Self {
            payment_method: request.payment_method,
            payment_method_type: request.payment_method_type,
            metadata: request.metadata.clone(),
            customer_id: request.customer_id.clone(),
            payment_method_data,
            billing: request.billing.clone(),
            psp_tokenization: None,
            network_tokenization: request.network_tokenization.clone(),
            storage_type: request.storage_type,
        })
    }
}

impl TryFrom<CreatePaymentMethodV2Response> for CreatePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(response: CreatePaymentMethodV2Response) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: response.id,
            merchant_id: response.merchant_id,
            customer_id: response.customer_id,
            payment_method_type: response.payment_method_type,
            payment_method_subtype: response.payment_method_subtype,
            recurring_enabled: response.recurring_enabled,
            created: response.created,
            last_used_at: response.last_used_at,
            payment_method_data: response.payment_method_data,
            connector_tokens: response.connector_tokens,
            network_token: response.network_token,
        })
    }
}

impl CreatePaymentMethod {
    fn build_body(&self, request: ModularPMCreateRequest) -> Option<RequestContent> {
        Some(RequestContent::Json(Box::new(request.payload)))
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    CreatePaymentMethod,
    method = Method::Post,
    path = "/v2/payment-methods",
    v1_request = CreatePaymentMethodV1Request,
    v2_request = ModularPMCreateRequest,
    v2_response = CreatePaymentMethodV2Response,
    v1_response = CreatePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    body = CreatePaymentMethod::build_body
);
