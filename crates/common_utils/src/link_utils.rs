//! Common

use std::primitive::i64;

use common_enums::enums;
use diesel::{
    backend::Backend,
    deserialize,
    deserialize::FromSql,
    serialize::{Output, ToSql},
    sql_types::Jsonb,
    AsExpression, FromSqlRow,
};
use error_stack::{report, ResultExt};
use masking::Secret;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{errors::ParsingError, id_type, types::MinorUnit};

#[derive(
    Serialize, serde::Deserialize, Debug, Clone, Eq, PartialEq, FromSqlRow, AsExpression, ToSchema,
)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
#[diesel(sql_type = Jsonb)]
/// Link status enum
pub enum GenericLinkStatus {
    /// Status variants for payment method collect link
    PaymentMethodCollect(PaymentMethodCollectStatus),
    /// Status variants for payout link
    PayoutLink(PayoutLinkStatus),
}

impl Default for GenericLinkStatus {
    fn default() -> Self {
        Self::PaymentMethodCollect(PaymentMethodCollectStatus::Initiated)
    }
}

crate::impl_to_sql_from_sql_json!(GenericLinkStatus);

#[derive(
    Serialize, serde::Deserialize, Debug, Clone, Eq, PartialEq, FromSqlRow, AsExpression, ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[diesel(sql_type = Jsonb)]
/// Status variants for payment method collect links
pub enum PaymentMethodCollectStatus {
    /// Link was initialized
    Initiated,
    /// Link was expired or invalidated
    Invalidated,
    /// Payment method details were submitted
    Submitted,
}

impl<DB: Backend> FromSql<Jsonb, DB> for PaymentMethodCollectStatus
where
    serde_json::Value: FromSql<Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
        let generic_status: GenericLinkStatus = serde_json::from_value(value)?;
        match generic_status {
            GenericLinkStatus::PaymentMethodCollect(status) => Ok(status),
            GenericLinkStatus::PayoutLink(_) => Err(report!(ParsingError::EnumParseFailure(
                "PaymentMethodCollectStatus"
            )))
            .attach_printable("Invalid status for PaymentMethodCollect")?,
        }
    }
}

impl ToSql<Jsonb, diesel::pg::Pg> for PaymentMethodCollectStatus
where
    serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>,
{
    // This wraps PaymentMethodCollectStatus with GenericLinkStatus
    // Required for storing the status in required format in DB (GenericLinkStatus)
    // This type is used in PaymentMethodCollectLink (a variant of GenericLink, used in the application for avoiding conversion of data and status)
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        let value = serde_json::to_value(GenericLinkStatus::PaymentMethodCollect(self.clone()))?;

        // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
        // please refer to the diesel migration blog:
        // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
        <serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(&value, &mut out.reborrow())
    }
}

#[derive(
    Serialize, serde::Deserialize, Debug, Clone, Eq, PartialEq, FromSqlRow, AsExpression, ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[diesel(sql_type = Jsonb)]
/// Status variants for payout links
pub enum PayoutLinkStatus {
    /// Link was initialized
    Initiated,
    /// Link was expired or invalidated
    Invalidated,
    /// Payout details were submitted
    Submitted,
}

impl<DB: Backend> FromSql<Jsonb, DB> for PayoutLinkStatus
where
    serde_json::Value: FromSql<Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
        let generic_status: GenericLinkStatus = serde_json::from_value(value)?;
        match generic_status {
            GenericLinkStatus::PayoutLink(status) => Ok(status),
            GenericLinkStatus::PaymentMethodCollect(_) => {
                Err(report!(ParsingError::EnumParseFailure("PayoutLinkStatus")))
                    .attach_printable("Invalid status for PayoutLink")?
            }
        }
    }
}

impl ToSql<Jsonb, diesel::pg::Pg> for PayoutLinkStatus
where
    serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>,
{
    // This wraps PayoutLinkStatus with GenericLinkStatus
    // Required for storing the status in required format in DB (GenericLinkStatus)
    // This type is used in PayoutLink (a variant of GenericLink, used in the application for avoiding conversion of data and status)
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        let value = serde_json::to_value(GenericLinkStatus::PayoutLink(self.clone()))?;

        // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
        // please refer to the diesel migration blog:
        // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
        <serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(&value, &mut out.reborrow())
    }
}

#[derive(Serialize, serde::Deserialize, Debug, Clone, FromSqlRow, AsExpression, ToSchema)]
#[diesel(sql_type = Jsonb)]
/// Payout link object
pub struct PayoutLinkData {
    /// Identifier for the payout link
    pub payout_link_id: String,
    /// Identifier for the customer
    pub customer_id: id_type::CustomerId,
    /// Identifier for the payouts resource
    pub payout_id: String,
    /// Link to render the payout link
    pub link: Secret<String>,
    /// Client secret generated for authenticating frontend APIs
    pub client_secret: Secret<String>,
    /// Expiry in seconds from the time it was created
    pub session_expiry: u32,
    #[serde(flatten)]
    /// Payout link's UI configurations
    pub ui_config: GenericLinkUiConfig,
    /// List of enabled payment methods
    pub enabled_payment_methods: Option<Vec<EnabledPaymentMethod>>,
    /// Payout amount
    pub amount: MinorUnit,
    /// Payout currency
    pub currency: enums::Currency,
}

crate::impl_to_sql_from_sql_json!(PayoutLinkData);

/// Object for GenericLinkUiConfig
#[derive(Clone, Debug, Default, serde::Deserialize, Serialize, ToSchema)]
pub struct GenericLinkUiConfig {
    /// Merchant's display logo
    #[schema(value_type = Option<String>, max_length = 255, example = "https://hyperswitch.io/favicon.ico")]
    pub logo: Option<String>,

    /// Custom merchant name for the link
    #[schema(value_type = Option<String>, max_length = 255, example = "Hyperswitch")]
    pub merchant_name: Option<Secret<String>>,

    /// Primary color to be used in the form represented in hex format
    #[schema(value_type = Option<String>, max_length = 255, example = "#4285F4")]
    pub theme: Option<String>,
}

/// Object for GenericLinkUIConfigFormData
#[derive(Clone, Debug, Default, serde::Deserialize, Serialize, ToSchema)]
pub struct GenericLinkUIConfigFormData {
    /// Merchant's display logo
    #[schema(value_type = String, max_length = 255, example = "https://hyperswitch.io/favicon.ico")]
    pub logo: String,

    /// Custom merchant name for the link
    #[schema(value_type = String, max_length = 255, example = "Hyperswitch")]
    pub merchant_name: Secret<String>,

    /// Primary color to be used in the form represented in hex format
    #[schema(value_type = String, max_length = 255, example = "#4285F4")]
    pub theme: String,
}

/// Object for EnabledPaymentMethod
#[derive(Clone, Debug, Serialize, serde::Deserialize, ToSchema)]
pub struct EnabledPaymentMethod {
    /// Payment method (banks, cards, wallets) enabled for the operation
    #[schema(value_type = PaymentMethod)]
    pub payment_method: enums::PaymentMethod,

    /// An array of associated payment method types
    #[schema(value_type = Vec<PaymentMethodType>)]
    pub payment_method_types: Vec<enums::PaymentMethodType>,
}
