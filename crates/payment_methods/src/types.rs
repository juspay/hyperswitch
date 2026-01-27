use api_models::payment_methods::{
    CardDetailFromLocker,
};
use hyperswitch_domain_models::payment_method_data::NetworkTokenDetailsPaymentMethod;
use common_enums::{PaymentMethod, PaymentMethodType};
use common_utils::{id_type};
use serde::Deserialize;
use time::PrimitiveDateTime;
#[derive(Clone, Debug)]
pub struct ModularListCustomerPaymentMethodsRequest;

/// Dummy modular service response payload.
#[derive(Debug, Deserialize)]
// TODO: replace dummy response types with real v1/modular models.
pub struct ModularListCustomerPaymentMethodsResponse {
    pub customer_payment_methods: Vec<PaymentMethodResponseItem>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentMethodResponseItem {
    pub id: String,
    pub customer_id: id_type::CustomerId,
    pub payment_method_type: PaymentMethod,
    pub payment_method_subtype: PaymentMethodType,
    pub recurring_enabled: Option<bool>,
    pub payment_method_data: Option<PaymentMethodResponseData>,
    pub bank: Option<api_models::payment_methods::MaskedBankDetails>,
    pub created: PrimitiveDateTime,
    pub requires_cvv: bool,
    pub last_used_at: PrimitiveDateTime,
    pub is_default: bool,
    pub billing: Option<api_models::payments::Address>,
    pub network_tokenization: Option<NetworkTokenResponse>,
    pub psp_tokenization_enabled: bool,
}
/// V2 PaymentMethodResponseData enum
#[derive(Clone, Debug, Deserialize)]
pub enum PaymentMethodResponseData {
    Card(CardDetailFromLocker),
}

/// V2 NetworkTokenResponse (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct NetworkTokenResponse {
    pub payment_method_data: NetworkTokenDetailsPaymentMethod,
}