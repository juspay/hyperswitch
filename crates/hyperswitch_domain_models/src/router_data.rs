use std::{collections::HashMap, marker::PhantomData};

use common_types::{payments as common_payment_types, primitive_wrappers};
use common_utils::{
    errors::IntegrityCheckError,
    ext_traits::{OptionExt, ValueExt},
    id_type::{self},
    types::MinorUnit,
};
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    address::AddressDetails, network_tokenization::NetworkTokenNumber,
    payment_address::PaymentAddress, payment_method_data, payments, router_response_types,
};
#[cfg(feature = "v2")]
use crate::{
    payments::{
        payment_attempt::{ErrorDetails, PaymentAttemptUpdate},
        payment_intent::PaymentIntentUpdate,
    },
    router_flow_types, router_request_types,
};

#[derive(Debug, Clone, Serialize)]
pub struct RouterData<Flow, Request, Response> {
    pub flow: PhantomData<Flow>,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<id_type::CustomerId>,
    pub connector_customer: Option<String>,
    pub connector: String,
    // TODO: This should be a PaymentId type.
    // Make this change after all the connector dependency has been removed from connectors
    pub payment_id: String,
    pub attempt_id: String,
    pub tenant_id: id_type::TenantId,
    pub status: common_enums::enums::AttemptStatus,
    pub payment_method: common_enums::enums::PaymentMethod,
    pub payment_method_type: Option<common_enums::enums::PaymentMethodType>,
    pub connector_auth_type: ConnectorAuthType,
    pub description: Option<String>,
    pub address: PaymentAddress,
    pub auth_type: common_enums::enums::AuthenticationType,
    pub connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    pub connector_wallets_details: Option<common_utils::pii::SecretSerdeValue>,
    pub amount_captured: Option<i64>,
    pub access_token: Option<AccessToken>,
    pub session_token: Option<String>,
    pub reference_id: Option<String>,
    pub payment_method_token: Option<PaymentMethodToken>,
    pub recurring_mandate_payment_data: Option<RecurringMandatePaymentData>,
    pub preprocessing_id: Option<String>,
    /// This is the balance amount for gift cards or voucher
    pub payment_method_balance: Option<PaymentMethodBalance>,

    ///for switching between two different versions of the same connector
    pub connector_api_version: Option<String>,

    /// Contains flow-specific data required to construct a request and send it to the connector.
    pub request: Request,

    /// Contains flow-specific data that the connector responds with.
    pub response: Result<Response, ErrorResponse>,

    /// Contains a reference ID that should be sent in the connector request
    pub connector_request_reference_id: String,

    #[cfg(feature = "payouts")]
    /// Contains payout method data
    pub payout_method_data: Option<api_models::payouts::PayoutMethodData>,

    #[cfg(feature = "payouts")]
    /// Contains payout's quote ID
    pub quote_id: Option<String>,

    pub test_mode: Option<bool>,
    pub connector_http_status_code: Option<u16>,
    pub external_latency: Option<u128>,
    /// Contains apple pay flow type simplified or manual
    pub apple_pay_flow: Option<payment_method_data::ApplePayFlow>,

    pub frm_metadata: Option<common_utils::pii::SecretSerdeValue>,

    pub dispute_id: Option<String>,
    pub refund_id: Option<String>,

    /// This field is used to store various data regarding the response from connector
    pub connector_response: Option<ConnectorResponseData>,
    pub payment_method_status: Option<common_enums::PaymentMethodStatus>,

    // minor amount for amount framework
    pub minor_amount_captured: Option<MinorUnit>,
    pub minor_amount_capturable: Option<MinorUnit>,

    // stores the authorized amount in case of partial authorization
    pub authorized_amount: Option<MinorUnit>,

    pub integrity_check: Result<(), IntegrityCheckError>,

    pub additional_merchant_data: Option<api_models::admin::AdditionalMerchantData>,

    pub header_payload: Option<payments::HeaderPayload>,

    pub connector_mandate_request_reference_id: Option<String>,

    pub l2_l3_data: Option<Box<L2L3Data>>,

    pub authentication_id: Option<id_type::AuthenticationId>,
    /// Contains the type of sca exemption required for the transaction
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,

    /// Contains stringified connector raw response body
    pub raw_connector_response: Option<Secret<String>>,

    /// Indicates whether the payment ID was provided by the merchant (true),
    /// or generated internally by Hyperswitch (false)
    pub is_payment_id_from_merchant: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct L2L3Data {
    pub order_info: Option<OrderInfo>,
    pub tax_info: Option<TaxInfo>,
    pub customer_info: Option<CustomerInfo>,
    pub shipping_details: Option<AddressDetails>,
    pub billing_details: Option<BillingDetails>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInfo {
    pub order_date: Option<time::PrimitiveDateTime>,
    pub order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
    pub merchant_order_reference_id: Option<String>,
    pub discount_amount: Option<MinorUnit>,
    pub shipping_cost: Option<MinorUnit>,
    pub duty_amount: Option<MinorUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxInfo {
    pub tax_status: Option<common_enums::TaxStatus>,
    pub customer_tax_registration_id: Option<Secret<String>>,
    pub merchant_tax_registration_id: Option<Secret<String>>,
    pub shipping_amount_tax: Option<MinorUnit>,
    pub order_tax_amount: Option<MinorUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerInfo {
    pub customer_id: Option<id_type::CustomerId>,
    pub customer_email: Option<common_utils::pii::Email>,
    pub customer_name: Option<Secret<String>>,
    pub customer_phone_number: Option<Secret<String>>,
    pub customer_phone_country_code: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingDetails {
    pub address_city: Option<String>,
}
impl L2L3Data {
    pub fn get_shipping_country(&self) -> Option<common_enums::enums::CountryAlpha2> {
        self.shipping_details
            .as_ref()
            .and_then(|address| address.country)
    }

    pub fn get_shipping_city(&self) -> Option<String> {
        self.shipping_details
            .as_ref()
            .and_then(|address| address.city.clone())
    }

    pub fn get_shipping_state(&self) -> Option<Secret<String>> {
        self.shipping_details
            .as_ref()
            .and_then(|address| address.state.clone())
    }

    pub fn get_shipping_origin_zip(&self) -> Option<Secret<String>> {
        self.shipping_details
            .as_ref()
            .and_then(|address| address.origin_zip.clone())
    }

    pub fn get_shipping_zip(&self) -> Option<Secret<String>> {
        self.shipping_details
            .as_ref()
            .and_then(|address| address.zip.clone())
    }

    pub fn get_shipping_address_line1(&self) -> Option<Secret<String>> {
        self.shipping_details
            .as_ref()
            .and_then(|address| address.line1.clone())
    }

    pub fn get_shipping_address_line2(&self) -> Option<Secret<String>> {
        self.shipping_details
            .as_ref()
            .and_then(|address| address.line2.clone())
    }

    pub fn get_order_date(&self) -> Option<time::PrimitiveDateTime> {
        self.order_info.as_ref().and_then(|order| order.order_date)
    }

    pub fn get_order_details(&self) -> Option<Vec<api_models::payments::OrderDetailsWithAmount>> {
        self.order_info
            .as_ref()
            .and_then(|order| order.order_details.clone())
    }

    pub fn get_merchant_order_reference_id(&self) -> Option<String> {
        self.order_info
            .as_ref()
            .and_then(|order| order.merchant_order_reference_id.clone())
    }

    pub fn get_discount_amount(&self) -> Option<MinorUnit> {
        self.order_info
            .as_ref()
            .and_then(|order| order.discount_amount)
    }

    pub fn get_shipping_cost(&self) -> Option<MinorUnit> {
        self.order_info
            .as_ref()
            .and_then(|order| order.shipping_cost)
    }

    pub fn get_duty_amount(&self) -> Option<MinorUnit> {
        self.order_info.as_ref().and_then(|order| order.duty_amount)
    }

    pub fn get_tax_status(&self) -> Option<common_enums::TaxStatus> {
        self.tax_info.as_ref().and_then(|tax| tax.tax_status)
    }

    pub fn get_customer_tax_registration_id(&self) -> Option<Secret<String>> {
        self.tax_info
            .as_ref()
            .and_then(|tax| tax.customer_tax_registration_id.clone())
    }

    pub fn get_merchant_tax_registration_id(&self) -> Option<Secret<String>> {
        self.tax_info
            .as_ref()
            .and_then(|tax| tax.merchant_tax_registration_id.clone())
    }

    pub fn get_shipping_amount_tax(&self) -> Option<MinorUnit> {
        self.tax_info
            .as_ref()
            .and_then(|tax| tax.shipping_amount_tax)
    }

    pub fn get_order_tax_amount(&self) -> Option<MinorUnit> {
        self.tax_info.as_ref().and_then(|tax| tax.order_tax_amount)
    }

    pub fn get_customer_id(&self) -> Option<id_type::CustomerId> {
        self.customer_info
            .as_ref()
            .and_then(|customer| customer.customer_id.clone())
    }

    pub fn get_customer_email(&self) -> Option<common_utils::pii::Email> {
        self.customer_info
            .as_ref()
            .and_then(|customer| customer.customer_email.clone())
    }

    pub fn get_customer_name(&self) -> Option<Secret<String>> {
        self.customer_info
            .as_ref()
            .and_then(|customer| customer.customer_name.clone())
    }

    pub fn get_customer_phone_number(&self) -> Option<Secret<String>> {
        self.customer_info
            .as_ref()
            .and_then(|customer| customer.customer_phone_number.clone())
    }

    pub fn get_customer_phone_country_code(&self) -> Option<String> {
        self.customer_info
            .as_ref()
            .and_then(|customer| customer.customer_phone_country_code.clone())
    }
    pub fn get_billing_city(&self) -> Option<String> {
        self.billing_details
            .as_ref()
            .and_then(|billing| billing.address_city.clone())
    }
}
// Different patterns of authentication.
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "auth_type")]
pub enum ConnectorAuthType {
    TemporaryAuth,
    HeaderKey {
        api_key: Secret<String>,
    },
    BodyKey {
        api_key: Secret<String>,
        key1: Secret<String>,
    },
    SignatureKey {
        api_key: Secret<String>,
        key1: Secret<String>,
        api_secret: Secret<String>,
    },
    MultiAuthKey {
        api_key: Secret<String>,
        key1: Secret<String>,
        api_secret: Secret<String>,
        key2: Secret<String>,
    },
    CurrencyAuthKey {
        auth_key_map: HashMap<common_enums::enums::Currency, common_utils::pii::SecretSerdeValue>,
    },
    CertificateAuth {
        certificate: Secret<String>,
        private_key: Secret<String>,
    },
    #[default]
    NoKey,
}

impl ConnectorAuthType {
    pub fn from_option_secret_value(
        value: Option<common_utils::pii::SecretSerdeValue>,
    ) -> common_utils::errors::CustomResult<Self, common_utils::errors::ParsingError> {
        value
            .parse_value::<Self>("ConnectorAuthType")
            .change_context(common_utils::errors::ParsingError::StructParseFailure(
                "ConnectorAuthType",
            ))
    }

    pub fn from_secret_value(
        value: common_utils::pii::SecretSerdeValue,
    ) -> common_utils::errors::CustomResult<Self, common_utils::errors::ParsingError> {
        value
            .parse_value::<Self>("ConnectorAuthType")
            .change_context(common_utils::errors::ParsingError::StructParseFailure(
                "ConnectorAuthType",
            ))
    }

    // show only first and last two digits of the key and mask others with *
    // mask the entire key if it's length is less than or equal to 4
    fn mask_key(&self, key: String) -> Secret<String> {
        let key_len = key.len();
        let masked_key = if key_len <= 4 {
            "*".repeat(key_len)
        } else {
            // Show the first two and last two characters, mask the rest with '*'
            let mut masked_key = String::new();
            let key_len = key.len();
            // Iterate through characters by their index
            for (index, character) in key.chars().enumerate() {
                if index < 2 || index >= key_len - 2 {
                    masked_key.push(character); // Keep the first two and last two characters
                } else {
                    masked_key.push('*'); // Mask the middle characters
                }
            }
            masked_key
        };
        Secret::new(masked_key)
    }

    // Mask the keys in the auth_type
    pub fn get_masked_keys(&self) -> Self {
        match self {
            Self::TemporaryAuth => Self::TemporaryAuth,
            Self::NoKey => Self::NoKey,
            Self::HeaderKey { api_key } => Self::HeaderKey {
                api_key: self.mask_key(api_key.clone().expose()),
            },
            Self::BodyKey { api_key, key1 } => Self::BodyKey {
                api_key: self.mask_key(api_key.clone().expose()),
                key1: self.mask_key(key1.clone().expose()),
            },
            Self::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Self::SignatureKey {
                api_key: self.mask_key(api_key.clone().expose()),
                key1: self.mask_key(key1.clone().expose()),
                api_secret: self.mask_key(api_secret.clone().expose()),
            },
            Self::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Self::MultiAuthKey {
                api_key: self.mask_key(api_key.clone().expose()),
                key1: self.mask_key(key1.clone().expose()),
                api_secret: self.mask_key(api_secret.clone().expose()),
                key2: self.mask_key(key2.clone().expose()),
            },
            Self::CurrencyAuthKey { auth_key_map } => Self::CurrencyAuthKey {
                auth_key_map: auth_key_map.clone(),
            },
            Self::CertificateAuth {
                certificate,
                private_key,
            } => Self::CertificateAuth {
                certificate: self.mask_key(certificate.clone().expose()),
                private_key: self.mask_key(private_key.clone().expose()),
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccessTokenAuthenticationResponse {
    pub code: Secret<String>,
    pub expires: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccessToken {
    pub token: Secret<String>,
    pub expires: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PaymentMethodToken {
    Token(Secret<String>),
    ApplePayDecrypt(Box<common_payment_types::ApplePayPredecryptData>),
    GooglePayDecrypt(Box<common_payment_types::GPayPredecryptData>),
    PazeDecrypt(Box<PazeDecryptedData>),
}

impl PaymentMethodToken {
    pub fn get_payment_method_token(&self) -> Option<Secret<String>> {
        match self {
            Self::Token(secret_token) => Some(secret_token.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayPredecryptDataInternal {
    pub application_primary_account_number: cards::CardNumber,
    pub application_expiration_date: String,
    pub currency_code: String,
    pub transaction_amount: i64,
    pub device_manufacturer_identifier: Secret<String>,
    pub payment_data_type: Secret<String>,
    pub payment_data: ApplePayCryptogramDataInternal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayCryptogramDataInternal {
    pub online_payment_cryptogram: Secret<String>,
    pub eci_indicator: Option<String>,
}

impl TryFrom<ApplePayPredecryptDataInternal> for common_payment_types::ApplePayPredecryptData {
    type Error = common_utils::errors::ValidationError;
    fn try_from(data: ApplePayPredecryptDataInternal) -> Result<Self, Self::Error> {
        let application_expiration_month = data.clone().get_expiry_month()?;
        let application_expiration_year = data.clone().get_four_digit_expiry_year()?;

        Ok(Self {
            application_primary_account_number: data.application_primary_account_number.clone(),
            application_expiration_month,
            application_expiration_year,
            payment_data: data.payment_data.into(),
        })
    }
}

impl From<GooglePayPredecryptDataInternal> for common_payment_types::GPayPredecryptData {
    fn from(data: GooglePayPredecryptDataInternal) -> Self {
        Self {
            card_exp_month: Secret::new(data.payment_method_details.expiration_month.two_digits()),
            card_exp_year: Secret::new(data.payment_method_details.expiration_year.four_digits()),
            application_primary_account_number: data.payment_method_details.pan.clone(),
            cryptogram: data.payment_method_details.cryptogram.clone(),
            eci_indicator: data.payment_method_details.eci_indicator.clone(),
        }
    }
}

impl From<ApplePayCryptogramDataInternal> for common_payment_types::ApplePayCryptogramData {
    fn from(payment_data: ApplePayCryptogramDataInternal) -> Self {
        Self {
            online_payment_cryptogram: payment_data.online_payment_cryptogram,
            eci_indicator: payment_data.eci_indicator,
        }
    }
}

impl ApplePayPredecryptDataInternal {
    /// This logic being applied as apple pay provides application_expiration_date in the YYMMDD format
    fn get_four_digit_expiry_year(
        &self,
    ) -> Result<Secret<String>, common_utils::errors::ValidationError> {
        Ok(Secret::new(format!(
            "20{}",
            self.application_expiration_date.get(0..2).ok_or(
                common_utils::errors::ValidationError::InvalidValue {
                    message: "Invalid two-digit year".to_string(),
                }
            )?
        )))
    }

    fn get_expiry_month(&self) -> Result<Secret<String>, common_utils::errors::ValidationError> {
        Ok(Secret::new(
            self.application_expiration_date
                .get(2..4)
                .ok_or(common_utils::errors::ValidationError::InvalidValue {
                    message: "Invalid two-digit month".to_string(),
                })?
                .to_owned(),
        ))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayPredecryptDataInternal {
    pub message_expiration: String,
    pub message_id: String,
    #[serde(rename = "paymentMethod")]
    pub payment_method_type: String,
    pub payment_method_details: GooglePayPaymentMethodDetails,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayPaymentMethodDetails {
    pub auth_method: common_enums::enums::GooglePayAuthMethod,
    pub expiration_month: cards::CardExpirationMonth,
    pub expiration_year: cards::CardExpirationYear,
    pub pan: cards::CardNumber,
    pub cryptogram: Option<Secret<String>>,
    pub eci_indicator: Option<String>,
    pub card_network: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PazeDecryptedData {
    pub client_id: Secret<String>,
    pub profile_id: String,
    pub token: PazeToken,
    pub payment_card_network: common_enums::enums::CardNetwork,
    pub dynamic_data: Vec<PazeDynamicData>,
    pub billing_address: PazeAddress,
    pub consumer: PazeConsumer,
    pub eci: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PazeToken {
    pub payment_token: NetworkTokenNumber,
    pub token_expiration_month: Secret<String>,
    pub token_expiration_year: Secret<String>,
    pub payment_account_reference: Secret<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PazeDynamicData {
    pub dynamic_data_value: Option<Secret<String>>,
    pub dynamic_data_type: Option<String>,
    pub dynamic_data_expiration: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PazeAddress {
    pub name: Option<Secret<String>>,
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub line3: Option<Secret<String>>,
    pub city: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
    pub country_code: Option<common_enums::enums::CountryAlpha2>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PazeConsumer {
    // This is consumer data not customer data.
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub full_name: Secret<String>,
    pub email_address: common_utils::pii::Email,
    pub mobile_number: Option<PazePhoneNumber>,
    pub country_code: Option<common_enums::enums::CountryAlpha2>,
    pub language_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PazePhoneNumber {
    pub country_code: Secret<String>,
    pub phone_number: Secret<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct RecurringMandatePaymentData {
    pub payment_method_type: Option<common_enums::enums::PaymentMethodType>, //required for making recurring payment using saved payment method through stripe
    pub original_payment_authorized_amount: Option<i64>,
    pub original_payment_authorized_currency: Option<common_enums::enums::Currency>,
    pub mandate_metadata: Option<common_utils::pii::SecretSerdeValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentMethodBalance {
    pub amount: MinorUnit,
    pub currency: common_enums::enums::Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorResponseData {
    pub additional_payment_method_data: Option<AdditionalPaymentMethodConnectorResponse>,
    extended_authorization_response_data: Option<ExtendedAuthorizationResponseData>,
    is_overcapture_enabled: Option<primitive_wrappers::OvercaptureEnabledBool>,
    pub mandate_reference: Option<router_response_types::MandateReference>,
}

impl ConnectorResponseData {
    pub fn with_additional_payment_method_data(
        additional_payment_method_data: AdditionalPaymentMethodConnectorResponse,
    ) -> Self {
        Self {
            additional_payment_method_data: Some(additional_payment_method_data),
            extended_authorization_response_data: None,
            is_overcapture_enabled: None,
            mandate_reference: None,
        }
    }
    pub fn new(
        additional_payment_method_data: Option<AdditionalPaymentMethodConnectorResponse>,
        is_overcapture_enabled: Option<primitive_wrappers::OvercaptureEnabledBool>,
        extended_authorization_response_data: Option<ExtendedAuthorizationResponseData>,
        mandate_reference: Option<router_response_types::MandateReference>,
    ) -> Self {
        Self {
            additional_payment_method_data,
            extended_authorization_response_data,
            is_overcapture_enabled,
            mandate_reference,
        }
    }

    pub fn get_extended_authorization_response_data(
        &self,
    ) -> Option<&ExtendedAuthorizationResponseData> {
        self.extended_authorization_response_data.as_ref()
    }

    pub fn is_overcapture_enabled(&self) -> Option<primitive_wrappers::OvercaptureEnabledBool> {
        self.is_overcapture_enabled
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdditionalPaymentMethodConnectorResponse {
    Card {
        /// Details regarding the authentication details of the connector, if this is a 3ds payment.
        authentication_data: Option<serde_json::Value>,
        /// Various payment checks that are done for a payment
        payment_checks: Option<serde_json::Value>,
        /// Card Network returned by the processor
        card_network: Option<String>,
        /// Domestic(Co-Branded) Card network returned by the processor
        domestic_network: Option<String>,
    },
    PayLater {
        klarna_sdk: Option<KlarnaSdkResponse>,
    },
    BankRedirect {
        interac: Option<InteracCustomerInfo>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedAuthorizationResponseData {
    pub extended_authentication_applied:
        Option<primitive_wrappers::ExtendedAuthorizationAppliedBool>,
    pub extended_authorization_last_applied_at: Option<time::PrimitiveDateTime>,
    pub capture_before: Option<time::PrimitiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlarnaSdkResponse {
    pub payment_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteracCustomerInfo {
    pub customer_info: Option<common_utils::pii::SecretSerdeValue>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub status_code: u16,
    pub attempt_status: Option<common_enums::enums::AttemptStatus>,
    pub connector_transaction_id: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_error_message: Option<String>,
    pub connector_metadata: Option<Secret<serde_json::Value>>,
}

impl Default for ErrorResponse {
    fn default() -> Self {
        Self {
            code: "HE_00".to_string(),
            message: "Something went wrong".to_string(),
            reason: None,
            status_code: http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
            connector_metadata: None,
        }
    }
}

impl ErrorResponse {
    pub fn get_not_implemented() -> Self {
        Self {
            code: "IR_00".to_string(),
            message: "This API is under development and will be made available soon.".to_string(),
            reason: None,
            status_code: http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
            connector_metadata: None,
        }
    }
}

/// Get updatable trakcer objects of payment intent and payment attempt
#[cfg(feature = "v2")]
pub trait TrackerPostUpdateObjects<Flow, FlowRequest, D> {
    fn get_payment_intent_update(
        &self,
        payment_data: &D,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentIntentUpdate;

    fn get_payment_attempt_update(
        &self,
        payment_data: &D,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentAttemptUpdate;

    /// Get the amount that can be captured for the payment
    fn get_amount_capturable(&self, payment_data: &D) -> Option<MinorUnit>;

    /// Get the amount that has been captured for the payment
    fn get_captured_amount(&self, payment_data: &D) -> Option<MinorUnit>;

    /// Get the attempt status based on parameters like captured amount and amount capturable
    fn get_attempt_status_for_db_update(
        &self,
        payment_data: &D,
    ) -> common_enums::enums::AttemptStatus;
}

#[cfg(feature = "v2")]
impl
    TrackerPostUpdateObjects<
        router_flow_types::Authorize,
        router_request_types::PaymentsAuthorizeData,
        payments::PaymentConfirmData<router_flow_types::Authorize>,
    >
    for RouterData<
        router_flow_types::Authorize,
        router_request_types::PaymentsAuthorizeData,
        router_response_types::PaymentsResponseData,
    >
{
    fn get_payment_intent_update(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::Authorize>,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentIntentUpdate {
        let amount_captured = self.get_captured_amount(payment_data);
        let updated_feature_metadata =
            payment_data
                .payment_intent
                .feature_metadata
                .clone()
                .map(|mut feature_metadata| {
                    if let Some(ref mut payment_revenue_recovery_metadata) =
                        feature_metadata.payment_revenue_recovery_metadata
                    {
                        payment_revenue_recovery_metadata.payment_connector_transmission = if self
                            .response
                            .is_ok()
                        {
                            common_enums::PaymentConnectorTransmission::ConnectorCallSucceeded
                        } else {
                            common_enums::PaymentConnectorTransmission::ConnectorCallUnsuccessful
                        };
                    }
                    Box::new(feature_metadata)
                });

        match self.response {
            Ok(ref _response) => PaymentIntentUpdate::ConfirmIntentPostUpdate {
                status: common_enums::IntentStatus::from(
                    self.get_attempt_status_for_db_update(payment_data),
                ),
                amount_captured,
                updated_by: storage_scheme.to_string(),
                feature_metadata: updated_feature_metadata,
            },
            Err(ref error) => PaymentIntentUpdate::ConfirmIntentPostUpdate {
                status: {
                    let attempt_status = match error.attempt_status {
                        // Use the status sent by connector in error_response if it's present
                        Some(status) => status,
                        None => match error.status_code {
                            500..=511 => common_enums::enums::AttemptStatus::Pending,
                            _ => common_enums::enums::AttemptStatus::Failure,
                        },
                    };
                    common_enums::IntentStatus::from(attempt_status)
                },
                amount_captured,
                updated_by: storage_scheme.to_string(),
                feature_metadata: updated_feature_metadata,
            },
        }
    }

    fn get_payment_attempt_update(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::Authorize>,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentAttemptUpdate {
        let amount_capturable = self.get_amount_capturable(payment_data);

        match self.response {
            Ok(ref response_router_data) => match response_router_data {
                router_response_types::PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data,
                    connector_metadata,
                    connector_response_reference_id,
                    ..
                } => {
                    let attempt_status = self.get_attempt_status_for_db_update(payment_data);

                    let connector_payment_id = match resource_id {
                        router_request_types::ResponseId::NoResponseId => None,
                        router_request_types::ResponseId::ConnectorTransactionId(id)
                        | router_request_types::ResponseId::EncodedData(id) => Some(id.to_owned()),
                    };

                    PaymentAttemptUpdate::ConfirmIntentResponse(Box::new(
                        payments::payment_attempt::ConfirmIntentResponseUpdate {
                            status: attempt_status,
                            connector_payment_id,
                            updated_by: storage_scheme.to_string(),
                            redirection_data: *redirection_data.clone(),
                            amount_capturable,
                            connector_metadata: connector_metadata.clone().map(Secret::new),
                            connector_token_details: response_router_data
                                .get_updated_connector_token_details(
                                    payment_data
                                        .payment_attempt
                                        .connector_token_details
                                        .as_ref()
                                        .and_then(|token_details| {
                                            token_details.get_connector_token_request_reference_id()
                                        }),
                                ),
                            connector_response_reference_id: connector_response_reference_id
                                .clone(),
                        },
                    ))
                }
                router_response_types::PaymentsResponseData::MultipleCaptureResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::SessionResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::SessionTokenResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::TransactionUnresolvedResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::TokenizationResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::ConnectorCustomerResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PreProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::IncrementalAuthorizationResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PostProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::PaymentResourceUpdateResponse {
                    ..
                } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::PaymentsCreateOrderResponse {
                    ..
                } => todo!(),
            },
            Err(ref error_response) => {
                let ErrorResponse {
                    code,
                    message,
                    reason,
                    status_code: _,
                    attempt_status: _,
                    connector_transaction_id,
                    network_decline_code,
                    network_advice_code,
                    network_error_message,
                    connector_metadata,
                } = error_response.clone();

                let attempt_status = match error_response.attempt_status {
                    // Use the status sent by connector in error_response if it's present
                    Some(status) => status,
                    None => match error_response.status_code {
                        500..=511 => common_enums::enums::AttemptStatus::Pending,
                        _ => common_enums::enums::AttemptStatus::Failure,
                    },
                };
                let error_details = ErrorDetails {
                    code,
                    message,
                    reason,
                    unified_code: None,
                    unified_message: None,
                    network_advice_code,
                    network_decline_code,
                    network_error_message,
                };

                PaymentAttemptUpdate::ErrorUpdate {
                    status: attempt_status,
                    error: error_details,
                    amount_capturable,
                    connector_payment_id: connector_transaction_id,
                    updated_by: storage_scheme.to_string(),
                }
            }
        }
    }

    fn get_attempt_status_for_db_update(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::Authorize>,
    ) -> common_enums::AttemptStatus {
        match self.status {
            common_enums::AttemptStatus::Charged => {
                let amount_captured = self
                    .get_captured_amount(payment_data)
                    .unwrap_or(MinorUnit::zero());
                let total_amount = payment_data.payment_attempt.amount_details.get_net_amount();

                if amount_captured == total_amount {
                    common_enums::AttemptStatus::Charged
                } else {
                    common_enums::AttemptStatus::PartialCharged
                }
            }
            _ => self.status,
        }
    }

    fn get_amount_capturable(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::Authorize>,
    ) -> Option<MinorUnit> {
        // Based on the status of the response, we can determine the amount capturable
        let intent_status = common_enums::IntentStatus::from(self.status);
        let amount_capturable_from_intent_status = match intent_status {
            // If the status is already succeeded / failed we cannot capture any more amount
            // So set the amount capturable to zero
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Conflicted
            | common_enums::IntentStatus::Expired => Some(MinorUnit::zero()),
            // For these statuses, update the capturable amount when it reaches terminal / capturable state
            common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::Processing => None,
            // Invalid states for this flow
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture => None,
            // If status is requires capture, get the total amount that can be captured
            // This is in cases where the capture method will be `manual` or `manual_multiple`
            // We do not need to handle the case where amount_to_capture is provided here as it cannot be passed in authroize flow
            common_enums::IntentStatus::RequiresCapture => {
                let total_amount = payment_data.payment_attempt.amount_details.get_net_amount();
                Some(total_amount)
            }
            // Invalid statues for this flow, after doing authorization this state is invalid
            common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
        };
        self.minor_amount_capturable
            .or(amount_capturable_from_intent_status)
            .or(Some(payment_data.payment_attempt.get_total_amount()))
    }

    fn get_captured_amount(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::Authorize>,
    ) -> Option<MinorUnit> {
        // Based on the status of the response, we can determine the amount that was captured
        let intent_status = common_enums::IntentStatus::from(self.status);
        let amount_captured_from_intent_status = match intent_status {
            // If the status is succeeded then we have captured the whole amount
            // we need not check for `amount_to_capture` here because passing `amount_to_capture` when authorizing is not supported
            common_enums::IntentStatus::Succeeded | common_enums::IntentStatus::Conflicted => {
                let total_amount = payment_data.payment_attempt.amount_details.get_net_amount();
                Some(total_amount)
            }
            // No amount is captured
            common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Expired => Some(MinorUnit::zero()),
            // For these statuses, update the amount captured when it reaches terminal state
            common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::Processing => None,
            // Invalid states for this flow
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation => None,
            // No amount has been captured yet
            common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture => {
                Some(MinorUnit::zero())
            }
            // Invalid statues for this flow
            common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
        };
        self.minor_amount_captured
            .or(amount_captured_from_intent_status)
            .or(Some(payment_data.payment_attempt.get_total_amount()))
    }
}
#[cfg(feature = "v2")]
impl
    TrackerPostUpdateObjects<
        router_flow_types::Capture,
        router_request_types::PaymentsCaptureData,
        payments::PaymentCaptureData<router_flow_types::Capture>,
    >
    for RouterData<
        router_flow_types::Capture,
        router_request_types::PaymentsCaptureData,
        router_response_types::PaymentsResponseData,
    >
{
    fn get_payment_intent_update(
        &self,
        payment_data: &payments::PaymentCaptureData<router_flow_types::Capture>,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentIntentUpdate {
        let amount_captured = self.get_captured_amount(payment_data);
        match self.response {
            Ok(ref _response) => PaymentIntentUpdate::CaptureUpdate {
                status: common_enums::IntentStatus::from(
                    self.get_attempt_status_for_db_update(payment_data),
                ),
                amount_captured,
                updated_by: storage_scheme.to_string(),
            },
            Err(ref error) => PaymentIntentUpdate::CaptureUpdate {
                status: error
                    .attempt_status
                    .map(common_enums::IntentStatus::from)
                    .unwrap_or(common_enums::IntentStatus::Failed),
                amount_captured,
                updated_by: storage_scheme.to_string(),
            },
        }
    }

    fn get_payment_attempt_update(
        &self,
        payment_data: &payments::PaymentCaptureData<router_flow_types::Capture>,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentAttemptUpdate {
        let amount_capturable = self.get_amount_capturable(payment_data);

        match self.response {
            Ok(ref response_router_data) => match response_router_data {
                router_response_types::PaymentsResponseData::TransactionResponse { .. } => {
                    let attempt_status = self.status;

                    PaymentAttemptUpdate::CaptureUpdate {
                        status: attempt_status,
                        amount_capturable,
                        updated_by: storage_scheme.to_string(),
                    }
                }
                router_response_types::PaymentsResponseData::MultipleCaptureResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::SessionResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::SessionTokenResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::TransactionUnresolvedResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::TokenizationResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::ConnectorCustomerResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PreProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::IncrementalAuthorizationResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PostProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::PaymentResourceUpdateResponse {
                    ..
                } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::PaymentsCreateOrderResponse {
                    ..
                } => todo!(),
            },
            Err(ref error_response) => {
                let ErrorResponse {
                    code,
                    message,
                    reason,
                    status_code: _,
                    attempt_status,
                    connector_transaction_id,
                    network_advice_code,
                    network_decline_code,
                    network_error_message,
                    connector_metadata: _,
                } = error_response.clone();
                let attempt_status = attempt_status.unwrap_or(self.status);

                let error_details = ErrorDetails {
                    code,
                    message,
                    reason,
                    unified_code: None,
                    unified_message: None,
                    network_advice_code,
                    network_decline_code,
                    network_error_message,
                };

                PaymentAttemptUpdate::ErrorUpdate {
                    status: attempt_status,
                    error: error_details,
                    amount_capturable,
                    connector_payment_id: connector_transaction_id,
                    updated_by: storage_scheme.to_string(),
                }
            }
        }
    }

    fn get_attempt_status_for_db_update(
        &self,
        payment_data: &payments::PaymentCaptureData<router_flow_types::Capture>,
    ) -> common_enums::AttemptStatus {
        match self.status {
            common_enums::AttemptStatus::Charged => {
                let amount_captured = self
                    .get_captured_amount(payment_data)
                    .unwrap_or(MinorUnit::zero());
                let total_amount = payment_data.payment_attempt.amount_details.get_net_amount();

                if amount_captured == total_amount {
                    common_enums::AttemptStatus::Charged
                } else {
                    common_enums::AttemptStatus::PartialCharged
                }
            }
            _ => self.status,
        }
    }

    fn get_amount_capturable(
        &self,
        payment_data: &payments::PaymentCaptureData<router_flow_types::Capture>,
    ) -> Option<MinorUnit> {
        // Based on the status of the response, we can determine the amount capturable
        let intent_status = common_enums::IntentStatus::from(self.status);
        match intent_status {
            // If the status is already succeeded / failed we cannot capture any more amount
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Conflicted
            | common_enums::IntentStatus::Expired => Some(MinorUnit::zero()),
            // For these statuses, update the capturable amount when it reaches terminal / capturable state
            common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::Processing => None,
            // Invalid states for this flow
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation => None,
            common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture => {
                let total_amount = payment_data.payment_attempt.amount_details.get_net_amount();
                Some(total_amount)
            }
            // Invalid statues for this flow
            common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
        }
    }

    fn get_captured_amount(
        &self,
        payment_data: &payments::PaymentCaptureData<router_flow_types::Capture>,
    ) -> Option<MinorUnit> {
        // Based on the status of the response, we can determine the amount capturable
        let intent_status = common_enums::IntentStatus::from(self.status);
        match intent_status {
            // If the status is succeeded then we have captured the whole amount
            common_enums::IntentStatus::Succeeded | common_enums::IntentStatus::Conflicted => {
                let amount_to_capture = payment_data
                    .payment_attempt
                    .amount_details
                    .get_amount_to_capture();

                let amount_captured = amount_to_capture
                    .unwrap_or(payment_data.payment_attempt.amount_details.get_net_amount());

                Some(amount_captured)
            }
            // No amount is captured
            common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Expired => Some(MinorUnit::zero()),
            common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture => {
                let total_amount = payment_data.payment_attempt.amount_details.get_net_amount();
                Some(total_amount)
            }
            // For these statuses, update the amount captured when it reaches terminal state
            common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::Processing => None,
            // Invalid states for this flow
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation => None,
            // Invalid statues for this flow
            common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => {
                todo!()
            }
        }
    }
}

#[cfg(feature = "v2")]
impl
    TrackerPostUpdateObjects<
        router_flow_types::PSync,
        router_request_types::PaymentsSyncData,
        payments::PaymentStatusData<router_flow_types::PSync>,
    >
    for RouterData<
        router_flow_types::PSync,
        router_request_types::PaymentsSyncData,
        router_response_types::PaymentsResponseData,
    >
{
    fn get_payment_intent_update(
        &self,
        payment_data: &payments::PaymentStatusData<router_flow_types::PSync>,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentIntentUpdate {
        let amount_captured = self.get_captured_amount(payment_data);
        match self.response {
            Ok(ref _response) => PaymentIntentUpdate::SyncUpdate {
                status: common_enums::IntentStatus::from(
                    self.get_attempt_status_for_db_update(payment_data),
                ),
                amount_captured,
                updated_by: storage_scheme.to_string(),
            },
            Err(ref error) => PaymentIntentUpdate::SyncUpdate {
                status: {
                    let attempt_status = match error.attempt_status {
                        // Use the status sent by connector in error_response if it's present
                        Some(status) => status,
                        None => match error.status_code {
                            200..=299 => common_enums::enums::AttemptStatus::Failure,
                            _ => self.status,
                        },
                    };
                    common_enums::IntentStatus::from(attempt_status)
                },
                amount_captured,
                updated_by: storage_scheme.to_string(),
            },
        }
    }

    fn get_payment_attempt_update(
        &self,
        payment_data: &payments::PaymentStatusData<router_flow_types::PSync>,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentAttemptUpdate {
        let amount_capturable = self.get_amount_capturable(payment_data);

        match self.response {
            Ok(ref response_router_data) => match response_router_data {
                router_response_types::PaymentsResponseData::TransactionResponse { .. } => {
                    let attempt_status = self.get_attempt_status_for_db_update(payment_data);

                    PaymentAttemptUpdate::SyncUpdate {
                        status: attempt_status,
                        amount_capturable,
                        updated_by: storage_scheme.to_string(),
                    }
                }
                router_response_types::PaymentsResponseData::MultipleCaptureResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::SessionResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::SessionTokenResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::TransactionUnresolvedResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::TokenizationResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::ConnectorCustomerResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PreProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::IncrementalAuthorizationResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PostProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::PaymentResourceUpdateResponse {
                    ..
                } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::PaymentsCreateOrderResponse {
                    ..
                } => todo!(),
            },
            Err(ref error_response) => {
                let ErrorResponse {
                    code,
                    message,
                    reason,
                    status_code: _,
                    attempt_status: _,
                    connector_transaction_id,
                    network_advice_code,
                    network_decline_code,
                    network_error_message,
                    connector_metadata: _,
                } = error_response.clone();

                let attempt_status = match error_response.attempt_status {
                    // Use the status sent by connector in error_response if it's present
                    Some(status) => status,
                    None => match error_response.status_code {
                        200..=299 => common_enums::enums::AttemptStatus::Failure,
                        _ => self.status,
                    },
                };

                let error_details = ErrorDetails {
                    code,
                    message,
                    reason,
                    unified_code: None,
                    unified_message: None,
                    network_advice_code,
                    network_decline_code,
                    network_error_message,
                };

                PaymentAttemptUpdate::ErrorUpdate {
                    status: attempt_status,
                    error: error_details,
                    amount_capturable,
                    connector_payment_id: connector_transaction_id,
                    updated_by: storage_scheme.to_string(),
                }
            }
        }
    }

    fn get_attempt_status_for_db_update(
        &self,
        payment_data: &payments::PaymentStatusData<router_flow_types::PSync>,
    ) -> common_enums::AttemptStatus {
        match self.status {
            common_enums::AttemptStatus::Charged => {
                let amount_captured = self
                    .get_captured_amount(payment_data)
                    .unwrap_or(MinorUnit::zero());

                let total_amount = payment_data.payment_attempt.amount_details.get_net_amount();

                if amount_captured == total_amount {
                    common_enums::AttemptStatus::Charged
                } else {
                    common_enums::AttemptStatus::PartialCharged
                }
            }
            _ => self.status,
        }
    }

    fn get_amount_capturable(
        &self,
        payment_data: &payments::PaymentStatusData<router_flow_types::PSync>,
    ) -> Option<MinorUnit> {
        let payment_attempt = &payment_data.payment_attempt;

        // Based on the status of the response, we can determine the amount capturable
        let intent_status = common_enums::IntentStatus::from(self.status);
        let amount_capturable_from_intent_status = match intent_status {
            // If the status is already succeeded / failed we cannot capture any more amount
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Conflicted
            | common_enums::IntentStatus::Expired => Some(MinorUnit::zero()),
            // For these statuses, update the capturable amount when it reaches terminal / capturable state
            common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::Processing => None,
            // Invalid states for this flow
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation => None,
            common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
            | common_enums::IntentStatus::PartiallyCaptured => {
                let total_amount = payment_attempt.amount_details.get_net_amount();
                Some(total_amount)
            }
            // Invalid statues for this flow
            common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
        };
        self.minor_amount_capturable
            .or(amount_capturable_from_intent_status)
    }

    fn get_captured_amount(
        &self,
        payment_data: &payments::PaymentStatusData<router_flow_types::PSync>,
    ) -> Option<MinorUnit> {
        let payment_attempt = &payment_data.payment_attempt;

        // Based on the status of the response, we can determine the amount capturable
        let intent_status = common_enums::IntentStatus::from(self.status);
        let amount_captured_from_intent_status = match intent_status {
            // If the status is succeeded then we have captured the whole amount or amount_to_capture
            common_enums::IntentStatus::Succeeded | common_enums::IntentStatus::Conflicted => {
                let amount_to_capture = payment_attempt.amount_details.get_amount_to_capture();

                let amount_captured =
                    amount_to_capture.unwrap_or(payment_attempt.amount_details.get_net_amount());

                Some(amount_captured)
            }
            // No amount is captured
            common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Expired => Some(MinorUnit::zero()),
            // For these statuses, update the amount captured when it reaches terminal state
            common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::Processing => None,
            // Invalid states for this flow
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation => None,
            common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture => {
                let total_amount = payment_attempt.amount_details.get_net_amount();
                Some(total_amount)
            }
            // Invalid statues for this flow
            common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
        };
        self.minor_amount_captured
            .or(amount_captured_from_intent_status)
            .or(Some(payment_data.payment_attempt.get_total_amount()))
    }
}

#[cfg(feature = "v2")]
impl
    TrackerPostUpdateObjects<
        router_flow_types::ExternalVaultProxy,
        router_request_types::ExternalVaultProxyPaymentsData,
        payments::PaymentConfirmData<router_flow_types::ExternalVaultProxy>,
    >
    for RouterData<
        router_flow_types::ExternalVaultProxy,
        router_request_types::ExternalVaultProxyPaymentsData,
        router_response_types::PaymentsResponseData,
    >
{
    fn get_payment_intent_update(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::ExternalVaultProxy>,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentIntentUpdate {
        let amount_captured = self.get_captured_amount(payment_data);
        match self.response {
            Ok(ref _response) => PaymentIntentUpdate::ConfirmIntentPostUpdate {
                status: common_enums::IntentStatus::from(
                    self.get_attempt_status_for_db_update(payment_data),
                ),
                amount_captured,
                updated_by: storage_scheme.to_string(),
                feature_metadata: None,
            },
            Err(ref error) => PaymentIntentUpdate::ConfirmIntentPostUpdate {
                status: {
                    let attempt_status = match error.attempt_status {
                        // Use the status sent by connector in error_response if it's present
                        Some(status) => status,
                        None => match error.status_code {
                            500..=511 => common_enums::enums::AttemptStatus::Pending,
                            _ => common_enums::enums::AttemptStatus::Failure,
                        },
                    };
                    common_enums::IntentStatus::from(attempt_status)
                },
                amount_captured,
                updated_by: storage_scheme.to_string(),
                feature_metadata: None,
            },
        }
    }

    fn get_payment_attempt_update(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::ExternalVaultProxy>,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentAttemptUpdate {
        let amount_capturable = self.get_amount_capturable(payment_data);

        match self.response {
            Ok(ref response_router_data) => match response_router_data {
                router_response_types::PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data,
                    connector_metadata,
                    connector_response_reference_id,
                    ..
                } => {
                    let attempt_status = self.get_attempt_status_for_db_update(payment_data);

                    let connector_payment_id = match resource_id {
                        router_request_types::ResponseId::NoResponseId => None,
                        router_request_types::ResponseId::ConnectorTransactionId(id)
                        | router_request_types::ResponseId::EncodedData(id) => Some(id.to_owned()),
                    };

                    PaymentAttemptUpdate::ConfirmIntentResponse(Box::new(
                        payments::payment_attempt::ConfirmIntentResponseUpdate {
                            status: attempt_status,
                            connector_payment_id,
                            updated_by: storage_scheme.to_string(),
                            redirection_data: *redirection_data.clone(),
                            amount_capturable,
                            connector_metadata: connector_metadata.clone().map(Secret::new),
                            connector_token_details: response_router_data
                                .get_updated_connector_token_details(
                                    payment_data
                                        .payment_attempt
                                        .connector_token_details
                                        .as_ref()
                                        .and_then(|token_details| {
                                            token_details.get_connector_token_request_reference_id()
                                        }),
                                ),
                            connector_response_reference_id: connector_response_reference_id
                                .clone(),
                        },
                    ))
                }
                router_response_types::PaymentsResponseData::MultipleCaptureResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::SessionResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::SessionTokenResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::TransactionUnresolvedResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::TokenizationResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::ConnectorCustomerResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PreProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::IncrementalAuthorizationResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PostProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::PaymentResourceUpdateResponse {
                    ..
                } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::PaymentsCreateOrderResponse {
                    ..
                } => todo!(),
            },
            Err(ref error_response) => {
                let ErrorResponse {
                    code,
                    message,
                    reason,
                    status_code: _,
                    attempt_status: _,
                    connector_transaction_id,
                    network_decline_code,
                    network_advice_code,
                    network_error_message,
                    connector_metadata,
                } = error_response.clone();

                let attempt_status = match error_response.attempt_status {
                    // Use the status sent by connector in error_response if it's present
                    Some(status) => status,
                    None => match error_response.status_code {
                        500..=511 => common_enums::enums::AttemptStatus::Pending,
                        _ => common_enums::enums::AttemptStatus::Failure,
                    },
                };
                let error_details = ErrorDetails {
                    code,
                    message,
                    reason,
                    unified_code: None,
                    unified_message: None,
                    network_advice_code,
                    network_decline_code,
                    network_error_message,
                };

                PaymentAttemptUpdate::ErrorUpdate {
                    status: attempt_status,
                    error: error_details,
                    amount_capturable,
                    connector_payment_id: connector_transaction_id,
                    updated_by: storage_scheme.to_string(),
                }
            }
        }
    }

    fn get_attempt_status_for_db_update(
        &self,
        _payment_data: &payments::PaymentConfirmData<router_flow_types::ExternalVaultProxy>,
    ) -> common_enums::AttemptStatus {
        // For this step, consider whatever status was given by the connector module
        // We do not need to check for amount captured or amount capturable here because we are authorizing the whole amount
        self.status
    }

    fn get_amount_capturable(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::ExternalVaultProxy>,
    ) -> Option<MinorUnit> {
        // Based on the status of the response, we can determine the amount capturable
        let intent_status = common_enums::IntentStatus::from(self.status);
        match intent_status {
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Conflicted
            | common_enums::IntentStatus::Expired => Some(MinorUnit::zero()),
            common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::Processing => None,
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation => None,
            common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture => {
                let total_amount = payment_data.payment_attempt.amount_details.get_net_amount();
                Some(total_amount)
            }
            common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
        }
    }

    fn get_captured_amount(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::ExternalVaultProxy>,
    ) -> Option<MinorUnit> {
        // Based on the status of the response, we can determine the amount that was captured
        let intent_status = common_enums::IntentStatus::from(self.status);
        match intent_status {
            common_enums::IntentStatus::Succeeded | common_enums::IntentStatus::Conflicted => {
                let total_amount = payment_data.payment_attempt.amount_details.get_net_amount();
                Some(total_amount)
            }
            common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
            | common_enums::IntentStatus::Expired => Some(MinorUnit::zero()),
            common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::Processing => None,
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation => None,
            common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::CancelledPostCapture => Some(MinorUnit::zero()),
            common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
        }
    }
}

#[cfg(feature = "v2")]
impl
    TrackerPostUpdateObjects<
        router_flow_types::SetupMandate,
        router_request_types::SetupMandateRequestData,
        payments::PaymentConfirmData<router_flow_types::SetupMandate>,
    >
    for RouterData<
        router_flow_types::SetupMandate,
        router_request_types::SetupMandateRequestData,
        router_response_types::PaymentsResponseData,
    >
{
    fn get_payment_intent_update(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::SetupMandate>,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentIntentUpdate {
        let amount_captured = self.get_captured_amount(payment_data);
        match self.response {
            Ok(ref _response) => PaymentIntentUpdate::ConfirmIntentPostUpdate {
                status: common_enums::IntentStatus::from(
                    self.get_attempt_status_for_db_update(payment_data),
                ),
                amount_captured,
                updated_by: storage_scheme.to_string(),
                feature_metadata: None,
            },
            Err(ref error) => PaymentIntentUpdate::ConfirmIntentPostUpdate {
                status: error
                    .attempt_status
                    .map(common_enums::IntentStatus::from)
                    .unwrap_or(common_enums::IntentStatus::Failed),
                amount_captured,
                updated_by: storage_scheme.to_string(),
                feature_metadata: None,
            },
        }
    }

    fn get_payment_attempt_update(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::SetupMandate>,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentAttemptUpdate {
        let amount_capturable = self.get_amount_capturable(payment_data);

        match self.response {
            Ok(ref response_router_data) => match response_router_data {
                router_response_types::PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data,
                    connector_metadata,
                    ..
                } => {
                    let attempt_status = self.get_attempt_status_for_db_update(payment_data);

                    let connector_payment_id = match resource_id {
                        router_request_types::ResponseId::NoResponseId => None,
                        router_request_types::ResponseId::ConnectorTransactionId(id)
                        | router_request_types::ResponseId::EncodedData(id) => Some(id.to_owned()),
                    };

                    PaymentAttemptUpdate::ConfirmIntentResponse(Box::new(
                        payments::payment_attempt::ConfirmIntentResponseUpdate {
                            status: attempt_status,
                            connector_payment_id,
                            updated_by: storage_scheme.to_string(),
                            redirection_data: *redirection_data.clone(),
                            amount_capturable,
                            connector_metadata: connector_metadata.clone().map(Secret::new),
                            connector_token_details: response_router_data
                                .get_updated_connector_token_details(
                                    payment_data
                                        .payment_attempt
                                        .connector_token_details
                                        .as_ref()
                                        .and_then(|token_details| {
                                            token_details.get_connector_token_request_reference_id()
                                        }),
                                ),
                            connector_response_reference_id: None,
                        },
                    ))
                }
                router_response_types::PaymentsResponseData::MultipleCaptureResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::SessionResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::SessionTokenResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::TransactionUnresolvedResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::TokenizationResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::ConnectorCustomerResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PreProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::IncrementalAuthorizationResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PostProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::PaymentResourceUpdateResponse {
                    ..
                } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::PaymentsCreateOrderResponse {
                    ..
                } => todo!(),
            },
            Err(ref error_response) => {
                let ErrorResponse {
                    code,
                    message,
                    reason,
                    status_code: _,
                    attempt_status,
                    connector_transaction_id,
                    network_advice_code,
                    network_decline_code,
                    network_error_message,
                    connector_metadata: _,
                } = error_response.clone();
                let attempt_status = attempt_status.unwrap_or(self.status);

                let error_details = ErrorDetails {
                    code,
                    message,
                    reason,
                    unified_code: None,
                    unified_message: None,
                    network_advice_code,
                    network_decline_code,
                    network_error_message,
                };

                PaymentAttemptUpdate::ErrorUpdate {
                    status: attempt_status,
                    error: error_details,
                    amount_capturable,
                    connector_payment_id: connector_transaction_id,
                    updated_by: storage_scheme.to_string(),
                }
            }
        }
    }

    fn get_attempt_status_for_db_update(
        &self,
        _payment_data: &payments::PaymentConfirmData<router_flow_types::SetupMandate>,
    ) -> common_enums::AttemptStatus {
        // For this step, consider whatever status was given by the connector module
        // We do not need to check for amount captured or amount capturable here because we are authorizing the whole amount
        self.status
    }

    fn get_amount_capturable(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::SetupMandate>,
    ) -> Option<MinorUnit> {
        // Based on the status of the response, we can determine the amount capturable
        let intent_status = common_enums::IntentStatus::from(self.status);
        match intent_status {
            // If the status is already succeeded / failed we cannot capture any more amount
            // So set the amount capturable to zero
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Conflicted
            | common_enums::IntentStatus::Expired => Some(MinorUnit::zero()),
            // For these statuses, update the capturable amount when it reaches terminal / capturable state
            common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::Processing => None,
            // Invalid states for this flow
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation => None,
            // If status is requires capture, get the total amount that can be captured
            // This is in cases where the capture method will be `manual` or `manual_multiple`
            // We do not need to handle the case where amount_to_capture is provided here as it cannot be passed in authroize flow
            common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture => {
                let total_amount = payment_data.payment_attempt.amount_details.get_net_amount();
                Some(total_amount)
            }
            // Invalid statues for this flow, after doing authorization this state is invalid
            common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
        }
    }

    fn get_captured_amount(
        &self,
        payment_data: &payments::PaymentConfirmData<router_flow_types::SetupMandate>,
    ) -> Option<MinorUnit> {
        // Based on the status of the response, we can determine the amount that was captured
        let intent_status = common_enums::IntentStatus::from(self.status);
        match intent_status {
            // If the status is succeeded then we have captured the whole amount
            // we need not check for `amount_to_capture` here because passing `amount_to_capture` when authorizing is not supported
            common_enums::IntentStatus::Succeeded | common_enums::IntentStatus::Conflicted => {
                let total_amount = payment_data.payment_attempt.amount_details.get_net_amount();
                Some(total_amount)
            }
            // No amount is captured
            common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Expired => Some(MinorUnit::zero()),
            // For these statuses, update the amount captured when it reaches terminal state
            common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::Processing => None,
            // Invalid states for this flow
            common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresConfirmation => None,
            // No amount has been captured yet
            common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture => {
                Some(MinorUnit::zero())
            }
            // Invalid statues for this flow
            common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => None,
        }
    }
}

#[cfg(feature = "v2")]
impl
    TrackerPostUpdateObjects<
        router_flow_types::Void,
        router_request_types::PaymentsCancelData,
        payments::PaymentCancelData<router_flow_types::Void>,
    >
    for RouterData<
        router_flow_types::Void,
        router_request_types::PaymentsCancelData,
        router_response_types::PaymentsResponseData,
    >
{
    fn get_payment_intent_update(
        &self,
        payment_data: &payments::PaymentCancelData<router_flow_types::Void>,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentIntentUpdate {
        let intent_status =
            common_enums::IntentStatus::from(self.get_attempt_status_for_db_update(payment_data));
        PaymentIntentUpdate::VoidUpdate {
            status: intent_status,
            updated_by: storage_scheme.to_string(),
        }
    }

    fn get_payment_attempt_update(
        &self,
        payment_data: &payments::PaymentCancelData<router_flow_types::Void>,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentAttemptUpdate {
        match &self.response {
            Err(ref error_response) => {
                let ErrorResponse {
                    code,
                    message,
                    reason,
                    status_code: _,
                    attempt_status: _,
                    connector_transaction_id,
                    network_decline_code,
                    network_advice_code,
                    network_error_message,
                    connector_metadata: _,
                } = error_response.clone();

                // Handle errors exactly
                let status = match error_response.attempt_status {
                    // Use the status sent by connector in error_response if it's present
                    Some(status) => status,
                    None => match error_response.status_code {
                        500..=511 => common_enums::AttemptStatus::Pending,
                        _ => common_enums::AttemptStatus::VoidFailed,
                    },
                };

                let error_details = ErrorDetails {
                    code,
                    message,
                    reason,
                    unified_code: None,
                    unified_message: None,
                    network_advice_code,
                    network_decline_code,
                    network_error_message,
                };

                PaymentAttemptUpdate::ErrorUpdate {
                    status,
                    amount_capturable: Some(MinorUnit::zero()),
                    error: error_details,
                    updated_by: storage_scheme.to_string(),
                    connector_payment_id: connector_transaction_id,
                }
            }
            Ok(ref _response) => PaymentAttemptUpdate::VoidUpdate {
                status: self.status,
                cancellation_reason: payment_data.payment_attempt.cancellation_reason.clone(),
                updated_by: storage_scheme.to_string(),
            },
        }
    }

    fn get_amount_capturable(
        &self,
        _payment_data: &payments::PaymentCancelData<router_flow_types::Void>,
    ) -> Option<MinorUnit> {
        // For void operations, no amount is capturable
        Some(MinorUnit::zero())
    }

    fn get_captured_amount(
        &self,
        _payment_data: &payments::PaymentCancelData<router_flow_types::Void>,
    ) -> Option<MinorUnit> {
        // For void operations, no amount is captured
        Some(MinorUnit::zero())
    }

    fn get_attempt_status_for_db_update(
        &self,
        _payment_data: &payments::PaymentCancelData<router_flow_types::Void>,
    ) -> common_enums::AttemptStatus {
        // For void operations, determine status based on response
        match &self.response {
            Err(ref error_response) => match error_response.attempt_status {
                Some(status) => status,
                None => match error_response.status_code {
                    500..=511 => common_enums::AttemptStatus::Pending,
                    _ => common_enums::AttemptStatus::VoidFailed,
                },
            },
            Ok(ref _response) => self.status,
        }
    }
}
