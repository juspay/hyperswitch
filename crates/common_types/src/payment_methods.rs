//! Common types to be used in payment methods

use diesel::{
    backend::Backend, deserialize, deserialize::FromSql, sql_types::Jsonb, AsExpression, Queryable,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Details of all the payment methods enabled for the connector for the given merchant account
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, AsExpression)]
#[serde(deny_unknown_fields)]
#[diesel(sql_type = Jsonb)]
pub struct PaymentMethodsEnabled {
    /// Type of payment method.
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method_type: common_enums::PaymentMethod,

    /// Payment method configuration, this includes all the filters associated with the payment method
    pub payment_method_subtypes: Option<Vec<RequestPaymentMethodTypes>>,
}

/// Details of a specific payment method subtype enabled for the connector for the given merchant account
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema, PartialEq, Eq, Hash)]
pub struct RequestPaymentMethodTypes {
    /// The payment method subtype
    #[schema(value_type = PaymentMethodType)]
    pub payment_method_subtype: common_enums::PaymentMethodType,

    /// The payment experience for the payment method
    #[schema(value_type = Option<PaymentExperience>)]
    pub payment_experience: Option<common_enums::PaymentExperience>,

    /// List of cards networks that are enabled for this payment method, applicable for credit and debit payment method subtypes only
    #[schema(value_type = Option<Vec<CardNetwork>>)]
    pub card_networks: Option<Vec<common_enums::CardNetwork>>,
    /// List of currencies accepted or has the processing capabilities of the processor
    #[schema(example = json!(
        {
            "type": "enable_only",
            "list": ["USD", "INR"]
        }
    ), value_type = Option<AcceptedCurrencies>)]
    pub accepted_currencies: Option<AcceptedCurrencies>,

    ///  List of Countries accepted or has the processing capabilities of the processor
    #[schema(example = json!(
        {
            "type": "enable_only",
            "list": ["UK", "AU"]
        }
    ), value_type = Option<AcceptedCountries>)]
    pub accepted_countries: Option<AcceptedCountries>,

    /// Minimum amount supported by the processor. To be represented in the lowest denomination of the target currency (For example, for USD it should be in cents)
    #[schema(example = 1)]
    pub minimum_amount: Option<common_utils::types::MinorUnit>,

    /// Maximum amount supported by the processor. To be represented in the lowest denomination of
    /// the target currency (For example, for USD it should be in cents)
    #[schema(example = 1313)]
    pub maximum_amount: Option<common_utils::types::MinorUnit>,

    /// Boolean to enable recurring payments / mandates. Default is true.
    #[schema(default = true, example = false)]
    pub recurring_enabled: bool,

    /// Boolean to enable installment / EMI / BNPL payments. Default is true.
    #[schema(default = true, example = false)]
    pub installment_payment_enabled: bool,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, serde::Serialize, Deserialize, ToSchema)]
#[serde(
    deny_unknown_fields,
    tag = "type",
    content = "list",
    rename_all = "snake_case"
)]
/// Object to filter the countries for which the payment method subtype is enabled
pub enum AcceptedCountries {
    /// Only enable the payment method subtype for specific countries
    #[schema(value_type = Vec<CountryAlpha2>)]
    EnableOnly(Vec<common_enums::CountryAlpha2>),

    /// Only disable the payment method subtype for specific countries
    #[schema(value_type = Vec<CountryAlpha2>)]
    DisableOnly(Vec<common_enums::CountryAlpha2>),

    /// Enable the payment method subtype for all countries, in which the processor has the processing capabilities
    AllAccepted,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, serde::Serialize, Deserialize, ToSchema)]
#[serde(
    deny_unknown_fields,
    tag = "type",
    content = "list",
    rename_all = "snake_case"
)]
/// Object to filter the countries for which the payment method subtype is enabled
pub enum AcceptedCurrencies {
    /// Only enable the payment method subtype for specific currencies
    #[schema(value_type = Vec<Currency>)]
    EnableOnly(Vec<common_enums::Currency>),

    /// Only disable the payment method subtype for specific currencies
    #[schema(value_type = Vec<Currency>)]
    DisableOnly(Vec<common_enums::Currency>),

    /// Enable the payment method subtype for all currencies, in which the processor has the processing capabilities
    AllAccepted,
}

impl<DB> Queryable<Jsonb, DB> for PaymentMethodsEnabled
where
    DB: Backend,
    Self: FromSql<Jsonb, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

common_utils::impl_to_sql_from_sql_json!(PaymentMethodsEnabled);

/// The network tokenization configuration for creating the payment method session
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct NetworkTokenization {
    /// Enable the network tokenization for payment methods that are created using the payment method session
    #[schema(value_type = NetworkTokenizationToggle)]
    pub enable: common_enums::NetworkTokenizationToggle,
}

/// The Payment Service Provider Configuration for payment methods that are created using the payment method session
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct PspTokenization {
    /// The tokenization type to be applied for the payment method
    #[schema(value_type = TokenizationType)]
    pub tokenization_type: common_enums::TokenizationType,

    /// The merchant connector id to be used for tokenization
    #[schema(value_type = String)]
    pub connector_id: common_utils::id_type::MerchantConnectorAccountId,
}
