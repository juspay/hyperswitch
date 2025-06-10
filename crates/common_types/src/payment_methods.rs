//! Common types to be used in payment methods

use diesel::{
    backend::Backend,
    deserialize,
    deserialize::FromSql,
    serialize::ToSql,
    sql_types::{Json, Jsonb},
    AsExpression, Queryable,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Details of all the payment methods enabled for the connector for the given merchant account

// sql_type for this can be json instead of jsonb. This is because validation at database is not required since it will always be written by the application.
// This is a performance optimization to avoid json validation at database level.
// jsonb enables faster querying on json columns, but it doesn't justify here since we are not querying on this column.
// https://docs.rs/diesel/latest/diesel/sql_types/struct.Jsonb.html
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, AsExpression)]
#[serde(deny_unknown_fields)]
#[diesel(sql_type = Json)]
pub struct PaymentMethodsEnabled {
    /// Type of payment method.
    #[schema(value_type = PaymentMethod,example = "card")]
    pub payment_method_type: common_enums::PaymentMethod,

    /// Payment method configuration, this includes all the filters associated with the payment method
    pub payment_method_subtypes: Option<Vec<RequestPaymentMethodTypes>>,
}

// Custom FromSql implementation to handle deserialization of v1 data format
impl FromSql<Json, diesel::pg::Pg> for PaymentMethodsEnabled {
    fn from_sql(bytes: <diesel::pg::Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let helper: PaymentMethodsEnabledHelper = serde_json::from_slice(bytes.as_bytes())
            .map_err(|e| Box::new(diesel::result::Error::DeserializationError(Box::new(e))))?;
        Ok(helper.into())
    }
}

// In this ToSql implementation, we are directly serializing the PaymentMethodsEnabled struct to JSON
impl ToSql<Json, diesel::pg::Pg> for PaymentMethodsEnabled {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        let value = serde_json::to_value(self)?;
        // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
        // please refer to the diesel migration blog:
        // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
        <serde_json::Value as ToSql<Json, diesel::pg::Pg>>::to_sql(&value, &mut out.reborrow())
    }
}

// Intermediate type to handle deserialization of v1 data format of PaymentMethodsEnabled
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum PaymentMethodsEnabledHelper {
    V2 {
        payment_method_type: common_enums::PaymentMethod,
        payment_method_subtypes: Option<Vec<RequestPaymentMethodTypes>>,
    },
    V1 {
        payment_method: common_enums::PaymentMethod,
        payment_method_types: Option<Vec<RequestPaymentMethodTypesV1>>,
    },
}

impl From<PaymentMethodsEnabledHelper> for PaymentMethodsEnabled {
    fn from(helper: PaymentMethodsEnabledHelper) -> Self {
        match helper {
            PaymentMethodsEnabledHelper::V2 {
                payment_method_type,
                payment_method_subtypes,
            } => Self {
                payment_method_type,
                payment_method_subtypes,
            },
            PaymentMethodsEnabledHelper::V1 {
                payment_method,
                payment_method_types,
            } => Self {
                payment_method_type: payment_method,
                payment_method_subtypes: payment_method_types.map(|subtypes| {
                    subtypes
                        .into_iter()
                        .map(RequestPaymentMethodTypes::from)
                        .collect()
                }),
            },
        }
    }
}

impl PaymentMethodsEnabled {
    /// Get payment_method_type
    #[cfg(feature = "v2")]
    pub fn get_payment_method(&self) -> Option<common_enums::PaymentMethod> {
        Some(self.payment_method_type)
    }

    /// Get payment_method_subtypes
    #[cfg(feature = "v2")]
    pub fn get_payment_method_type(&self) -> Option<&Vec<RequestPaymentMethodTypes>> {
        self.payment_method_subtypes.as_ref()
    }
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

    /// Indicates whether the payment method supports recurring payments. Optional.
    #[schema(example = true)]
    pub recurring_enabled: Option<bool>,

    /// Indicates whether the payment method is eligible for installment payments (e.g., EMI, BNPL). Optional.
    #[schema(example = true)]
    pub installment_payment_enabled: Option<bool>,
}

impl From<RequestPaymentMethodTypesV1> for RequestPaymentMethodTypes {
    fn from(value: RequestPaymentMethodTypesV1) -> Self {
        Self {
            payment_method_subtype: value.payment_method_type,
            payment_experience: value.payment_experience,
            card_networks: value.card_networks,
            accepted_currencies: value.accepted_currencies,
            accepted_countries: value.accepted_countries,
            minimum_amount: value.minimum_amount,
            maximum_amount: value.maximum_amount,
            recurring_enabled: value.recurring_enabled,
            installment_payment_enabled: value.installment_payment_enabled,
        }
    }
}

#[derive(serde::Deserialize)]
struct RequestPaymentMethodTypesV1 {
    pub payment_method_type: common_enums::PaymentMethodType,
    pub payment_experience: Option<common_enums::PaymentExperience>,
    pub card_networks: Option<Vec<common_enums::CardNetwork>>,
    pub accepted_currencies: Option<AcceptedCurrencies>,
    pub accepted_countries: Option<AcceptedCountries>,
    pub minimum_amount: Option<common_utils::types::MinorUnit>,
    pub maximum_amount: Option<common_utils::types::MinorUnit>,
    pub recurring_enabled: Option<bool>,
    pub installment_payment_enabled: Option<bool>,
}

impl RequestPaymentMethodTypes {
    ///Get payment_method_subtype
    pub fn get_payment_method_type(&self) -> Option<common_enums::PaymentMethodType> {
        Some(self.payment_method_subtype)
    }
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
