use std::primitive::i64;

use common_enums::enums;
use diesel::{
    backend::Backend,
    deserialize,
    deserialize::FromSql,
    serialize::{Output, ToSql},
    sql_types,
    sql_types::Jsonb,
    AsExpression, FromSqlRow,
};
use error_stack::{report, ResultExt};
use masking::Secret;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{errors::ParsingError, ext_traits::Encode, id_type, types::MinorUnit};

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::VariantNames,
    FromSqlRow,
    AsExpression,
)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
#[diesel(sql_type = sql_types::Text)]
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

impl<DB: Backend> FromSql<sql_types::Text, DB> for GenericLinkStatus
where
    String: FromSql<sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<sql_types::Text, DB>>::from_sql(bytes)?;
        // let a = serde_json::from_str(&value)?;
        Ok(serde_json::from_str(&value)?)
    }
}

impl ToSql<sql_types::Text, diesel::pg::Pg> for GenericLinkStatus
where
    String: ToSql<sql_types::Text, diesel::pg::Pg>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        let value = self.encode_to_string_of_json()?;

        // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
        // please refer to the diesel migration blog:
        // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
        <String as ToSql<sql_types::Text, diesel::pg::Pg>>::to_sql(&value, &mut out.reborrow())
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    strum::Display,
    serde::Serialize,
    strum::VariantNames,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
/// Status variants for payment method collect links
pub enum PaymentMethodCollectStatus {
    /// Link was initialized
    Initiated,
    /// Link was expired or invalidated
    Invalidated,
    /// Payment method details were submitted
    Submitted,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    strum::Display,
    serde::Serialize,
    strum::VariantNames,
    FromSqlRow,
    AsExpression,
)]
#[serde(rename_all = "snake_case")]
#[diesel(sql_type = sql_types::Text)]
/// Status variants for payout links
pub enum PayoutLinkStatus {
    /// Link was initialized
    Initiated,
    /// Link was expired or invalidated
    Invalidated,
    /// Payout details were submitted
    Submitted,
}

impl<DB: Backend> FromSql<sql_types::Text, DB> for PayoutLinkStatus
where
    String: FromSql<sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<sql_types::Text, DB>>::from_sql(bytes)?;
        let generic_status: GenericLinkStatus = serde_json::from_str(&value)?;
        match generic_status {
            GenericLinkStatus::PayoutLink(status) => Ok(status),
            GenericLinkStatus::PaymentMethodCollect(_) => {
                Err(report!(ParsingError::EnumParseFailure("PayoutLinkStatus")))
                    .attach_printable("Invalid status for PayoutLink")?
            }
        }
    }
}

impl ToSql<sql_types::Text, diesel::pg::Pg> for PayoutLinkStatus
where
    String: ToSql<sql_types::Text, diesel::pg::Pg>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        let value = GenericLinkStatus::PayoutLink(*self).encode_to_string_of_json()?;

        // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
        // please refer to the diesel migration blog:
        // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
        <String as ToSql<sql_types::Text, diesel::pg::Pg>>::to_sql(&value, &mut out.reborrow())
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
    pub ui_config: enums::GenericLinkUIConfig,
    /// List of enabled payment methods
    pub enabled_payment_methods: Option<Vec<enums::EnabledPaymentMethod>>,
    /// Payout amount
    pub amount: MinorUnit,
    /// Payout currency
    pub currency: enums::Currency,
}

impl<DB: Backend> FromSql<Jsonb, DB> for PayoutLinkData
where
    serde_json::Value: FromSql<Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
        Ok(serde_json::from_value(value)?)
    }
}

impl ToSql<Jsonb, diesel::pg::Pg> for PayoutLinkData
where
    serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        let value = serde_json::to_value(self)?;

        // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
        // please refer to the diesel migration blog:
        // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
        <serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(&value, &mut out.reborrow())
    }
}
