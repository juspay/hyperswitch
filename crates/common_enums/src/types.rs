use diesel::{
    backend::Backend,
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    serialize::{Output, ToSql},
    sql_types::Jsonb,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression,
)]
#[diesel(sql_type = Jsonb)]
pub struct ChargeRefunds {
    pub charge_id: String,
    pub options: Option<ChargeRefundsOptions>,
}

#[derive(Clone, Debug, ToSchema, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChargeRefundsOptions {
    Direct(DirectChargeRefund),
    Destination(DestinationChargeRefund),
}

#[derive(Clone, Debug, ToSchema, Eq, PartialEq, Deserialize, Serialize)]
pub struct DirectChargeRefund {
    pub revert_platform_fee: bool,
}

#[derive(Clone, Debug, ToSchema, Eq, PartialEq, Deserialize, Serialize)]
pub struct DestinationChargeRefund {
    pub revert_platform_fee: bool,
    pub revert_transfer: bool,
}

impl<DB: Backend> FromSql<Jsonb, DB> for ChargeRefunds
where
    serde_json::Value: FromSql<Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
        Ok(serde_json::from_value(value)?)
    }
}

impl ToSql<Jsonb, diesel::pg::Pg> for ChargeRefunds
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
