use api_models::payment_methods::{CardDetailFromLocker, NetworkTokenResponse};
use common_enums::{PaymentMethod, PaymentMethodType};
use common_utils::{id_type, pii};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::client::create::CardDetail;
#[derive(Clone, Debug)]
pub struct ModularListCustomerPaymentMethodsRequest;

/// Dummy modular service response payload.
#[derive(Debug, Deserialize)]
// TODO: replace dummy response types with real v1/modular models.
pub struct ModularListCustomerPaymentMethodsResponse {
    pub customer_payment_methods: Vec<PaymentMethodResponseItemV1>,
}

/// V1 bridge shape — deserialized from the modular service response when called from v1 router.
/// Uses v1 ID types (String, CustomerId).
#[derive(Debug, Deserialize)]
pub struct PaymentMethodResponseItemV1 {
    pub id: String,
    pub customer_id: id_type::CustomerId,
    pub payment_method_type: PaymentMethod,
    pub payment_method_subtype: PaymentMethodType,
    pub recurring_enabled: Option<bool>,
    pub payment_method_data: Option<PaymentMethodResponseData>,
    pub bank: Option<api_models::payment_methods::MaskedBankDetails>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,
    pub requires_cvv: bool,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_used_at: PrimitiveDateTime,
    pub is_default: bool,
    pub billing: Option<api_models::payments::Address>,
    pub network_tokenization: Option<NetworkTokenResponse>,
    pub psp_tokenization_enabled: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum WalletPaymentMethodData {
    ApplePay(Box<api_models::payment_methods::PaymentMethodDataWalletInfo>),
    GooglePay(Box<api_models::payment_methods::PaymentMethodDataWalletInfo>),
    PayPal(Box<api_models::payments::PaypalRedirection>),
}

/// V2 PaymentMethodResponseData enum
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodResponseData {
    Card(Box<CardDetailFromLocker>),
    BankDebit(BankDebitDetailsPaymentMethod),
    Wallet(WalletPaymentMethodData),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BankDebitDetailsPaymentMethod {
    AchBankDebit {
        account_number_last4_digits: String,
        routing_number_last4_digits: String,
        bank_account_holder_name: Option<Secret<String>>,
        bank_name: Option<common_enums::BankNames>,
        bank_type: Option<common_enums::BankType>,
        bank_holder_type: Option<common_enums::BankHolderType>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BankDebitDetail {
    Ach {
        account_number: Secret<String>,
        routing_number: Secret<String>,
        bank_account_holder_name: Option<Secret<String>>,
        bank_type: Option<common_enums::BankType>,
        bank_holder_type: Option<common_enums::BankHolderType>,
        bank_name: Option<common_enums::BankNames>,
    },
}

/// V2 modular service request payload.
#[derive(Clone, Debug)]
pub struct ModularPMRetrieveRequest;

/// V2 PaymentMethodResponse as returned by the V2 API.
/// This is a copy of the V2 PaymentMethodResponse struct from api_models for use in V1-only builds.
#[derive(Clone, Debug, Deserialize)]
pub struct ModularPMRetrieveResponse {
    pub id: String,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<id_type::CustomerId>,
    pub payment_method_type: PaymentMethod,
    pub payment_method_subtype: Option<PaymentMethodType>,
    pub recurring_enabled: Option<bool>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_used_at: Option<PrimitiveDateTime>,
    pub payment_method_data: Option<PaymentMethodResponseData>,
    pub connector_tokens: Option<Vec<ConnectorTokenDetails>>,
    pub network_token: Option<NetworkTokenResponse>,
    pub storage_type: Option<common_enums::StorageType>,
    pub card_cvc_token_storage: Option<CardCVCTokenStorageDetails>,
    pub raw_payment_method_data: Option<RawPaymentMethodData>,
    pub billing: Option<hyperswitch_domain_models::address::Address>,
    pub network_transaction_id: Option<String>,
}
/// V2 RawPaymentMethodData enum
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawPaymentMethodData {
    Card(CardDetail),
    CardWithNT(RawCardWithNTDetails),
    BankDebit(BankDebitDetail),
}

#[derive(Clone, Debug, Deserialize)]
pub struct RawCardWithNTDetails {
    pub card_details: CardDetail,
    pub network_token_details: CardDetail,
}

/// V2 ConnectorTokenDetails (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConnectorTokenDetails {
    pub connector_id: id_type::MerchantConnectorAccountId,
    pub token_type: common_enums::TokenizationType,
    pub status: common_enums::ConnectorTokenStatus,
    pub connector_token_request_reference_id: Option<String>,
    pub original_payment_authorized_amount: Option<common_utils::types::MinorUnit>,
    pub original_payment_authorized_currency: Option<common_enums::Currency>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer_id: Option<String>,
    pub token: Secret<String>,
}

/// V2 CardCVCTokenStorageDetails (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct CardCVCTokenStorageDetails {
    pub is_stored: bool,

    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub expires_at: Option<PrimitiveDateTime>,
}

// ---------------------------------------------------------------------------
// From conversions → api_models client-facing types
// ---------------------------------------------------------------------------

impl From<WalletPaymentMethodData>
    for api_models::payment_methods::WalletPaymentMethodDataForClient
{
    fn from(wallet_info: WalletPaymentMethodData) -> Self {
        match wallet_info {
            WalletPaymentMethodData::ApplePay(apple_pay_info) => Self::ApplePay(apple_pay_info),
            WalletPaymentMethodData::GooglePay(google_pay_info) => Self::GooglePay(google_pay_info),
            WalletPaymentMethodData::PayPal(paypal_info) => Self::PayPal(paypal_info),
        }
    }
}

impl From<BankDebitDetailsPaymentMethod> for api_models::payment_methods::BankDebitDataForClient {
    fn from(bank_debit_info: BankDebitDetailsPaymentMethod) -> Self {
        match bank_debit_info {
            BankDebitDetailsPaymentMethod::AchBankDebit {
                account_number_last4_digits,
                routing_number_last4_digits,
                bank_account_holder_name,
                bank_name,
                bank_type,
                bank_holder_type,
            } => Self::AchBankDebit {
                account_number_last4_digits,
                routing_number_last4_digits,
                bank_account_holder_name,
                bank_name,
                bank_type,
                bank_holder_type,
            },
        }
    }
}

impl From<PaymentMethodResponseData>
    for Option<api_models::payment_methods::CustomerPaymentMethodDataForClient>
{
    fn from(payment_method_response_data: PaymentMethodResponseData) -> Self {
        match payment_method_response_data {
            PaymentMethodResponseData::Card(card_info) => Some(
                api_models::payment_methods::CustomerPaymentMethodDataForClient::Card(card_info),
            ),
            PaymentMethodResponseData::Wallet(wallet_info) => Some(
                api_models::payment_methods::CustomerPaymentMethodDataForClient::Wallet(
                    wallet_info.into(),
                ),
            ),
            PaymentMethodResponseData::BankDebit(bank_debit_info) => Some(
                api_models::payment_methods::CustomerPaymentMethodDataForClient::BankDebit(
                    bank_debit_info.into(),
                ),
            ),
        }
    }
}
