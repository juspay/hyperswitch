//! Payment related types

use std::collections::HashMap;

use common_enums::enums;
use common_utils::{
    date_time, errors, events, ext_traits::OptionExt, impl_to_sql_from_sql_json, pii,
    types::MinorUnit,
};
use diesel::{
    sql_types::{Jsonb, Text},
    AsExpression, FromSqlRow,
};
use error_stack::{Report, Result, ResultExt};
use euclid::frontend::{
    ast::Program,
    dir::{DirKeyKind, EuclidDirFilter},
};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use smithy::SmithyModel;
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::domain::{AdyenSplitData, XenditSplitSubMerchantData};
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// Fee information for Split Payments to be charged on the payment being collected
pub enum SplitPaymentsRequest {
    /// StripeSplitPayment
    #[smithy(value_type = "StripeSplitPaymentRequest")]
    StripeSplitPayment(StripeSplitPaymentRequest),
    /// AdyenSplitPayment
    #[smithy(value_type = "AdyenSplitData")]
    AdyenSplitPayment(AdyenSplitData),
    /// XenditSplitPayment
    #[smithy(value_type = "XenditSplitRequest")]
    XenditSplitPayment(XenditSplitRequest),
}
impl_to_sql_from_sql_json!(SplitPaymentsRequest);

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// Fee information for Split Payments to be charged on the payment being collected for Stripe
pub struct StripeSplitPaymentRequest {
    /// Stripe's charge type
    #[schema(value_type = PaymentChargeType, example = "direct")]
    #[smithy(value_type = "PaymentChargeType")]
    pub charge_type: enums::PaymentChargeType,

    /// Platform fees to be collected on the payment
    #[schema(value_type = i64, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub application_fees: Option<MinorUnit>,

    /// Identifier for the reseller's account where the funds were transferred
    #[smithy(value_type = "String")]
    pub transfer_account_id: String,
}
impl_to_sql_from_sql_json!(StripeSplitPaymentRequest);

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
/// Hashmap to store mca_id's with product names
pub struct AuthenticationConnectorAccountMap(
    HashMap<enums::AuthenticationProduct, common_utils::id_type::MerchantConnectorAccountId>,
);
impl_to_sql_from_sql_json!(AuthenticationConnectorAccountMap);

impl AuthenticationConnectorAccountMap {
    /// fn to get click to pay connector_account_id
    pub fn get_click_to_pay_connector_account_id(
        &self,
    ) -> Result<common_utils::id_type::MerchantConnectorAccountId, errors::ValidationError> {
        self.0
            .get(&enums::AuthenticationProduct::ClickToPay)
            .ok_or(errors::ValidationError::MissingRequiredField {
                field_name: "authentication_product_id.click_to_pay".to_string(),
            })
            .map_err(Report::from)
            .cloned()
    }
}

/// A wrapper type for merchant country codes that provides validation and conversion functionality.
///
/// This type stores a country code as a string and provides methods to validate it
/// and convert it to a `Country` enum variant.
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Text)]
#[serde(deny_unknown_fields)]
pub struct MerchantCountryCode(String);

impl MerchantCountryCode {
    /// Returns the country code as a string.
    pub fn get_country_code(&self) -> String {
        self.0.clone()
    }

    /// Validates the country code and returns a `Country` enum variant.
    ///
    /// This method attempts to parse the country code as a u32 and convert it to a `Country` enum variant.
    /// If the country code is invalid, it returns a `ValidationError` with the appropriate error message.
    pub fn validate_and_get_country_from_merchant_country_code(
        &self,
    ) -> errors::CustomResult<common_enums::Country, errors::ValidationError> {
        let country_code = self.get_country_code();
        let code = country_code
            .parse::<u32>()
            .map_err(Report::from)
            .change_context(errors::ValidationError::IncorrectValueProvided {
                field_name: "merchant_country_code",
            })
            .attach_printable_lazy(|| {
                format!("Country code {country_code} is negative or too large")
            })?;

        common_enums::Country::from_numeric(code)
            .map_err(|_| errors::ValidationError::IncorrectValueProvided {
                field_name: "merchant_country_code",
            })
            .attach_printable_lazy(|| format!("Invalid country code {code}"))
    }
    /// Creates a new `MerchantCountryCode` instance from a string.
    pub fn new(country_code: String) -> Self {
        Self(country_code)
    }
}

impl diesel::serialize::ToSql<Text, diesel::pg::Pg> for MerchantCountryCode {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        <String as diesel::serialize::ToSql<Text, diesel::pg::Pg>>::to_sql(&self.0, out)
    }
}

impl diesel::deserialize::FromSql<Text, diesel::pg::Pg> for MerchantCountryCode {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let s = <String as diesel::deserialize::FromSql<Text, diesel::pg::Pg>>::from_sql(bytes)?;
        Ok(Self(s))
    }
}

#[derive(
    Serialize, Default, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
/// ConditionalConfigs
pub struct ConditionalConfigs {
    /// Override 3DS
    pub override_3ds: Option<common_enums::AuthenticationType>,
}
impl EuclidDirFilter for ConditionalConfigs {
    const ALLOWED: &'static [DirKeyKind] = &[
        DirKeyKind::PaymentMethod,
        DirKeyKind::CardType,
        DirKeyKind::CardNetwork,
        DirKeyKind::MetaData,
        DirKeyKind::PaymentAmount,
        DirKeyKind::PaymentCurrency,
        DirKeyKind::CaptureMethod,
        DirKeyKind::BillingCountry,
        DirKeyKind::BusinessCountry,
    ];
}

impl_to_sql_from_sql_json!(ConditionalConfigs);

/// This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.
#[derive(
    Default,
    Eq,
    PartialEq,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    Clone,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[serde(deny_unknown_fields)]
#[diesel(sql_type = Jsonb)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CustomerAcceptance {
    /// Type of acceptance provided by the
    #[schema(example = "online")]
    #[smithy(value_type = "AcceptanceType")]
    pub acceptance_type: AcceptanceType,
    /// Specifying when the customer acceptance was provided
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    #[smithy(value_type = "Option<PrimitiveDateTime>")]
    pub accepted_at: Option<PrimitiveDateTime>,
    /// Information required for online mandate generation
    #[smithy(value_type = "Option<OnlineMandate>")]
    pub online: Option<OnlineMandate>,
}

impl_to_sql_from_sql_json!(CustomerAcceptance);

impl CustomerAcceptance {
    /// Get the IP address
    pub fn get_ip_address(&self) -> Option<String> {
        self.online
            .as_ref()
            .and_then(|data| data.ip_address.as_ref().map(|ip| ip.peek().to_owned()))
    }

    /// Get the User Agent
    pub fn get_user_agent(&self) -> Option<String> {
        self.online.as_ref().map(|data| data.user_agent.clone())
    }

    /// Get when the customer acceptance was provided
    pub fn get_accepted_at(&self) -> PrimitiveDateTime {
        self.accepted_at.unwrap_or_else(date_time::now)
    }
}

impl masking::SerializableSecret for CustomerAcceptance {}

#[derive(
    Default,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    PartialEq,
    Eq,
    Clone,
    Copy,
    ToSchema,
    SmithyModel,
)]
#[serde(rename_all = "lowercase")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// This is used to indicate if the mandate was accepted online or offline
pub enum AcceptanceType {
    /// Online
    Online,
    /// Offline
    #[default]
    Offline,
}

#[derive(
    Default,
    Eq,
    PartialEq,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    AsExpression,
    Clone,
    ToSchema,
    SmithyModel,
)]
#[serde(deny_unknown_fields)]
/// Details of online mandate
#[diesel(sql_type = Jsonb)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct OnlineMandate {
    /// Ip address of the customer machine from which the mandate was created
    #[schema(value_type = String, example = "123.32.25.123")]
    #[smithy(value_type = "String")]
    pub ip_address: Option<Secret<String, pii::IpAddress>>,
    /// The user-agent of the customer's browser
    #[smithy(value_type = "String")]
    pub user_agent: String,
}

impl_to_sql_from_sql_json!(OnlineMandate);

#[derive(Serialize, Deserialize, Debug, Clone, FromSqlRow, AsExpression, ToSchema)]
#[diesel(sql_type = Jsonb)]
/// DecisionManagerRecord
pub struct DecisionManagerRecord {
    /// Name of the Decision Manager
    pub name: String,
    /// Program to be executed
    pub program: Program<ConditionalConfigs>,
    /// Created at timestamp
    pub created_at: i64,
}

impl events::ApiEventMetric for DecisionManagerRecord {
    fn get_api_event_type(&self) -> Option<events::ApiEventsType> {
        Some(events::ApiEventsType::Routing)
    }
}
impl_to_sql_from_sql_json!(DecisionManagerRecord);

/// DecisionManagerResponse
pub type DecisionManagerResponse = DecisionManagerRecord;

/// Fee information to be charged on the payment being collected via Stripe
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct StripeChargeResponseData {
    /// Identifier for charge created for the payment
    #[smithy(value_type = "Option<String>")]
    pub charge_id: Option<String>,

    /// Type of charge (connector specific)
    #[schema(value_type = PaymentChargeType, example = "direct")]
    #[smithy(value_type = "PaymentChargeType")]
    pub charge_type: enums::PaymentChargeType,

    /// Platform fees collected on the payment
    #[schema(value_type = i64, example = 6540)]
    #[smithy(value_type = "Option<i64>")]
    pub application_fees: Option<MinorUnit>,

    /// Identifier for the reseller's account where the funds were transferred
    #[smithy(value_type = "String")]
    pub transfer_account_id: String,
}
impl_to_sql_from_sql_json!(StripeChargeResponseData);

/// Charge Information
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum ConnectorChargeResponseData {
    /// StripeChargeResponseData
    #[smithy(value_type = "StripeChargeResponseData")]
    StripeSplitPayment(StripeChargeResponseData),
    /// AdyenChargeResponseData
    #[smithy(value_type = "AdyenSplitData")]
    AdyenSplitPayment(AdyenSplitData),
    /// XenditChargeResponseData
    #[smithy(value_type = "XenditChargeResponseData")]
    XenditSplitPayment(XenditChargeResponseData),
}

impl_to_sql_from_sql_json!(ConnectorChargeResponseData);

/// Fee information to be charged on the payment being collected via xendit
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct XenditSplitRoute {
    /// Amount of payments to be split
    #[smithy(value_type = "Option<i64>")]
    pub flat_amount: Option<MinorUnit>,
    /// Amount of payments to be split, using a percent rate as unit
    #[smithy(value_type = "Option<i64>")]
    pub percent_amount: Option<i64>,
    /// Currency code
    #[schema(value_type = Currency, example = "USD")]
    #[smithy(value_type = "Currency")]
    pub currency: enums::Currency,
    ///  ID of the destination account where the amount will be routed to
    #[smithy(value_type = "String")]
    pub destination_account_id: String,
    /// Reference ID which acts as an identifier of the route itself
    #[smithy(value_type = "String")]
    pub reference_id: String,
}
impl_to_sql_from_sql_json!(XenditSplitRoute);

/// Fee information to be charged on the payment being collected via xendit
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct XenditMultipleSplitRequest {
    /// Name to identify split rule. Not required to be unique. Typically based on transaction and/or sub-merchant types.
    #[smithy(value_type = "String")]
    pub name: String,
    /// Description to identify fee rule
    #[smithy(value_type = "String")]
    pub description: String,
    /// The sub-account user-id that you want to make this transaction for.
    #[smithy(value_type = "Option<String>")]
    pub for_user_id: Option<String>,
    /// Array of objects that define how the platform wants to route the fees and to which accounts.
    #[smithy(value_type = "Vec<XenditSplitRoute>")]
    pub routes: Vec<XenditSplitRoute>,
}
impl_to_sql_from_sql_json!(XenditMultipleSplitRequest);

/// Xendit Charge Request
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum XenditSplitRequest {
    /// Split Between Multiple Accounts
    #[smithy(value_type = "XenditMultipleSplitRequest")]
    MultipleSplits(XenditMultipleSplitRequest),
    /// Collect Fee for Single Account
    #[smithy(value_type = "XenditSplitSubMerchantData")]
    SingleSplit(XenditSplitSubMerchantData),
}

impl_to_sql_from_sql_json!(XenditSplitRequest);

/// Charge Information
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub enum XenditChargeResponseData {
    /// Split Between Multiple Accounts
    #[smithy(value_type = "XenditMultipleSplitResponse")]
    MultipleSplits(XenditMultipleSplitResponse),
    /// Collect Fee for Single Account
    #[smithy(value_type = "XenditSplitSubMerchantData")]
    SingleSplit(XenditSplitSubMerchantData),
}

impl_to_sql_from_sql_json!(XenditChargeResponseData);

/// Fee information charged on the payment being collected via xendit
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    SmithyModel,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct XenditMultipleSplitResponse {
    /// Identifier for split rule created for the payment
    #[smithy(value_type = "String")]
    pub split_rule_id: String,
    /// The sub-account user-id that you want to make this transaction for.
    #[smithy(value_type = "Option<String>")]
    pub for_user_id: Option<String>,
    /// Name to identify split rule. Not required to be unique. Typically based on transaction and/or sub-merchant types.
    #[smithy(value_type = "String")]
    pub name: String,
    /// Description to identify fee rule
    #[smithy(value_type = "String")]
    pub description: String,
    /// Array of objects that define how the platform wants to route the fees and to which accounts.
    #[smithy(value_type = "Vec<XenditSplitRoute>")]
    pub routes: Vec<XenditSplitRoute>,
}
impl_to_sql_from_sql_json!(XenditMultipleSplitResponse);

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// This enum is used to represent the Gpay payment data, which can either be encrypted or decrypted.
pub enum GpayTokenizationData {
    /// This variant contains the decrypted Gpay payment data as a structured object.
    #[smithy(value_type = "GPayPredecryptData")]
    Decrypted(GPayPredecryptData),
    /// This variant contains the encrypted Gpay payment data as a string.
    #[smithy(value_type = "GpayEcryptedTokenizationData")]
    Encrypted(GpayEcryptedTokenizationData),
}

#[derive(
    Eq, PartialEq, Clone, Debug, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// This struct represents the encrypted Gpay payment data
pub struct GpayEcryptedTokenizationData {
    /// The type of the token
    #[serde(rename = "type")]
    #[smithy(value_type = "String")]
    pub token_type: String,
    /// Token generated for the wallet
    #[smithy(value_type = "String")]
    pub token: String,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// This struct represents the decrypted Google Pay payment data
pub struct GPayPredecryptData {
    /// The card's expiry month
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub card_exp_month: Secret<String>,

    /// The card's expiry year
    #[schema(value_type = String)]
    #[smithy(value_type = "String")]
    pub card_exp_year: Secret<String>,

    /// The Primary Account Number (PAN) of the card
    #[schema(value_type = String, example = "4242424242424242")]
    #[smithy(value_type = "String")]
    pub application_primary_account_number: cards::CardNumber,

    /// Cryptogram generated by the Network
    #[schema(value_type = String, example = "AgAAAAAAAIR8CQrXcIhbQAAAAAA")]
    #[smithy(value_type = "Option<String>")]
    pub cryptogram: Option<Secret<String>>,

    /// Electronic Commerce Indicator
    #[schema(value_type = String, example = "07")]
    #[smithy(value_type = "Option<String>")]
    pub eci_indicator: Option<String>,
}
impl GpayTokenizationData {
    /// Get the encrypted Google Pay payment data, returning an error if it does not exist
    pub fn get_encrypted_google_pay_payment_data_mandatory(
        &self,
    ) -> Result<&GpayEcryptedTokenizationData, errors::ValidationError> {
        match self {
            Self::Encrypted(encrypted_data) => Ok(encrypted_data),
            Self::Decrypted(_) => Err(errors::ValidationError::InvalidValue {
                message: "Encrypted Google Pay payment data is mandatory".to_string(),
            }
            .into()),
        }
    }
    /// Get the optional decrypted Google Pay payment data
    pub fn get_decrypted_google_pay_payment_data_optional(&self) -> Option<&GPayPredecryptData> {
        match self {
            Self::Decrypted(token) => Some(token),
            Self::Encrypted(_) => None,
        }
    }
    /// Get the token from Google Pay tokenization data
    /// Returns the token string if encrypted data exists, otherwise returns an error
    pub fn get_encrypted_google_pay_token(&self) -> Result<String, errors::ValidationError> {
        Ok(self
            .get_encrypted_google_pay_payment_data_mandatory()?
            .token
            .clone())
    }

    /// Get the token type from Google Pay tokenization data
    /// Returns the token_type string if encrypted data exists, otherwise returns an error
    pub fn get_encrypted_token_type(&self) -> Result<String, errors::ValidationError> {
        Ok(self
            .get_encrypted_google_pay_payment_data_mandatory()?
            .token_type
            .clone())
    }
}
impl GPayPredecryptData {
    /// Get the four-digit expiration year from the Google Pay pre-decrypt data
    pub fn get_four_digit_expiry_year(&self) -> Result<Secret<String>, errors::ValidationError> {
        let mut year = self.card_exp_year.peek().clone();

        // If it's a 2-digit year, convert to 4-digit
        if year.len() == 2 {
            year = format!("20{year}");
        } else if year.len() != 4 {
            return Err(errors::ValidationError::InvalidValue {
                message: format!(
                    "Invalid expiry year length: {}. Must be 2 or 4 digits",
                    year.len()
                ),
            }
            .into());
        }
        Ok(Secret::new(year))
    }
    /// Get the 2-digit expiration year from the Google Pay pre-decrypt data
    pub fn get_two_digit_expiry_year(&self) -> Result<Secret<String>, errors::ValidationError> {
        let binding = self.card_exp_year.clone();
        let year = binding.peek();
        Ok(Secret::new(
            year.get(year.len() - 2..)
                .ok_or(errors::ValidationError::InvalidValue {
                    message: "Invalid two-digit year".to_string(),
                })?
                .to_string(),
        ))
    }
    /// Get the expiry date in MMYY format from the Google Pay pre-decrypt data
    pub fn get_expiry_date_as_mmyy(&self) -> Result<Secret<String>, errors::ValidationError> {
        let year = self.get_two_digit_expiry_year()?.expose();
        let month = self.get_expiry_month()?.clone().expose();
        Ok(Secret::new(format!("{month}{year}")))
    }

    /// Get the expiration month from the Google Pay pre-decrypt data
    pub fn get_expiry_month(&self) -> Result<Secret<String>, errors::ValidationError> {
        let month_str = self.card_exp_month.peek();
        let month = month_str
            .parse::<u8>()
            .map_err(|_| errors::ValidationError::InvalidValue {
                message: format!("Failed to parse expiry month: {month_str}"),
            })?;

        if !(1..=12).contains(&month) {
            return Err(errors::ValidationError::InvalidValue {
                message: format!("Invalid expiry month: {month}. Must be between 1 and 12"),
            }
            .into());
        }
        Ok(self.card_exp_month.clone())
    }
}
#[derive(
    Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// This enum is used to represent the Apple Pay payment data, which can either be encrypted or decrypted.
pub enum ApplePayPaymentData {
    /// This variant contains the decrypted Apple Pay payment data as a structured object.
    #[smithy(value_type = "ApplePayPredecryptData")]
    Decrypted(ApplePayPredecryptData),
    /// This variant contains the encrypted Apple Pay payment data as a string.
    #[smithy(value_type = "String")]
    Encrypted(String),
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// This struct represents the decrypted Apple Pay payment data
pub struct ApplePayPredecryptData {
    /// The primary account number
    #[schema(value_type = String, example = "4242424242424242")]
    #[smithy(value_type = "String")]
    pub application_primary_account_number: cards::CardNumber,
    /// The application expiration date (PAN expiry month)
    #[schema(value_type = String, example = "12")]
    #[smithy(value_type = "String")]
    pub application_expiration_month: Secret<String>,
    /// The application expiration date (PAN expiry year)
    #[schema(value_type = String, example = "24")]
    #[smithy(value_type = "String")]
    pub application_expiration_year: Secret<String>,
    /// The payment data, which contains the cryptogram and ECI indicator
    #[schema(value_type = ApplePayCryptogramData)]
    #[smithy(value_type = "ApplePayCryptogramData")]
    pub payment_data: ApplePayCryptogramData,
}

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, ToSchema, SmithyModel,
)]
#[serde(rename_all = "snake_case")]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
/// This struct represents the cryptogram data for Apple Pay transactions
pub struct ApplePayCryptogramData {
    /// The online payment cryptogram
    #[schema(value_type = String, example = "A1B2C3D4E5F6G7H8")]
    #[smithy(value_type = "String")]
    pub online_payment_cryptogram: Secret<String>,
    /// The ECI (Electronic Commerce Indicator) value
    #[schema(value_type = String, example = "05")]
    #[smithy(value_type = "Option<String>")]
    pub eci_indicator: Option<String>,
}

impl ApplePayPaymentData {
    /// Get the encrypted Apple Pay payment data if it exists
    pub fn get_encrypted_apple_pay_payment_data_optional(&self) -> Option<&String> {
        match self {
            Self::Encrypted(encrypted_data) => Some(encrypted_data),
            Self::Decrypted(_) => None,
        }
    }

    /// Get the decrypted Apple Pay payment data if it exists
    pub fn get_decrypted_apple_pay_payment_data_optional(&self) -> Option<&ApplePayPredecryptData> {
        match self {
            Self::Encrypted(_) => None,
            Self::Decrypted(decrypted_data) => Some(decrypted_data),
        }
    }

    /// Get the encrypted Apple Pay payment data, returning an error if it does not exist
    pub fn get_encrypted_apple_pay_payment_data_mandatory(
        &self,
    ) -> Result<&String, errors::ValidationError> {
        self.get_encrypted_apple_pay_payment_data_optional()
            .get_required_value("Encrypted Apple Pay payment data")
            .attach_printable("Encrypted Apple Pay payment data is mandatory")
    }

    /// Get the decrypted Apple Pay payment data, returning an error if it does not exist
    pub fn get_decrypted_apple_pay_payment_data_mandatory(
        &self,
    ) -> Result<&ApplePayPredecryptData, errors::ValidationError> {
        self.get_decrypted_apple_pay_payment_data_optional()
            .get_required_value("Decrypted Apple Pay payment data")
            .attach_printable("Decrypted Apple Pay payment data is mandatory")
    }
}

impl ApplePayPredecryptData {
    /// Get the four-digit expiration year from the Apple Pay pre-decrypt data
    pub fn get_two_digit_expiry_year(&self) -> Result<Secret<String>, errors::ValidationError> {
        let binding = self.application_expiration_year.clone();
        let year = binding.peek();
        Ok(Secret::new(
            year.get(year.len() - 2..)
                .ok_or(errors::ValidationError::InvalidValue {
                    message: "Invalid two-digit year".to_string(),
                })?
                .to_string(),
        ))
    }

    /// Get the four-digit expiration year from the Apple Pay pre-decrypt data
    pub fn get_four_digit_expiry_year(&self) -> Secret<String> {
        let mut year = self.application_expiration_year.peek().clone();
        if year.len() == 2 {
            year = format!("20{year}");
        }
        Secret::new(year)
    }

    /// Get the expiration month from the Apple Pay pre-decrypt data
    pub fn get_expiry_month(&self) -> Result<Secret<String>, errors::ValidationError> {
        let month_str = self.application_expiration_month.peek();
        let month = month_str
            .parse::<u8>()
            .map_err(|_| errors::ValidationError::InvalidValue {
                message: format!("Failed to parse expiry month: {month_str}"),
            })?;

        if !(1..=12).contains(&month) {
            return Err(errors::ValidationError::InvalidValue {
                message: format!("Invalid expiry month: {month}. Must be between 1 and 12"),
            }
            .into());
        }
        Ok(self.application_expiration_month.clone())
    }

    /// Get the expiry date in MMYY format from the Apple Pay pre-decrypt data
    pub fn get_expiry_date_as_mmyy(&self) -> Result<Secret<String>, errors::ValidationError> {
        let year = self.get_two_digit_expiry_year()?.expose();
        let month = self.get_expiry_month()?.expose();
        Ok(Secret::new(format!("{month}{year}")))
    }

    /// Get the expiry date in YYMM format from the Apple Pay pre-decrypt data
    pub fn get_expiry_date_as_yymm(&self) -> Result<Secret<String>, errors::ValidationError> {
        let year = self.get_two_digit_expiry_year()?.expose();
        let month = self.get_expiry_month()?.expose();
        Ok(Secret::new(format!("{year}{month}")))
    }
}

/// type of action that needs to taken after consuming recovery payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    /// Stops the process tracker and update the payment intent.
    CancelInvoice,
    /// Records the external transaction against payment intent.
    ScheduleFailedPayment,
    /// Records the external payment and stops the internal process tracker.
    SuccessPaymentExternal,
    /// Pending payments from billing processor.
    PendingPayment,
    /// No action required.
    NoAction,
    /// Invalid event has been received.
    InvalidAction,
}

/// Billing Descriptor information to be sent to the payment gateway
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, AsExpression, FromSqlRow, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct BillingDescriptor {
    /// name to be put in billing description
    #[schema(value_type = Option<String>, example = "The Online Retailer")]
    pub name: Option<Secret<String>>,
    /// city to be put in billing description
    #[schema(value_type = Option<String>, example = "San Francisco")]
    pub city: Option<Secret<String>>,
    /// phone to be put in billing description
    #[schema(value_type = Option<String>, example = "9123456789")]
    pub phone: Option<Secret<String>>,
    /// a short description for the payment
    pub statement_descriptor: Option<String>,
    /// Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor.
    pub statement_descriptor_suffix: Option<String>,
    /// A reference to be shown on billing description
    pub reference: Option<String>,
}

impl_to_sql_from_sql_json!(BillingDescriptor);

///  Information identifying partner / external platform details
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, AsExpression, FromSqlRow, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct PartnerApplicationDetails {
    /// Name of the partner/external platform
    #[schema(value_type = Option<String>)]
    pub name: Option<String>,
    /// Version of the partner/external platform
    #[schema(value_type = Option<String>, example = "1.0.0")]
    pub version: Option<String>,
    /// Integrator
    #[schema(value_type = Option<String>)]
    pub integrator: Option<String>,
}
impl_to_sql_from_sql_json!(PartnerApplicationDetails);

///  Information identifying merchant details
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, AsExpression, FromSqlRow, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct MerchantApplicationDetails {
    /// Name of the the merchant application
    #[schema(value_type = Option<String>)]
    pub name: Option<String>,
    /// Version of the merchant application
    #[schema(value_type = Option<String>)]
    pub version: Option<String>,
}
impl_to_sql_from_sql_json!(MerchantApplicationDetails);

/// Information identifying partner and merchant application initiating the request
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, AsExpression, FromSqlRow, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct PartnerMerchantIdentifierDetails {
    ///  Information identifying partner/external platform details
    #[schema(value_type = Option<PartnerApplicationDetails>)]
    pub partner_details: Option<PartnerApplicationDetails>,
    ///  Information identifying merchant details
    #[schema(value_type = Option<MerchantApplicationDetails>)]
    pub merchant_details: Option<MerchantApplicationDetails>,
}

impl_to_sql_from_sql_json!(PartnerMerchantIdentifierDetails);
