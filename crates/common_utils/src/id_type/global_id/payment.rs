/// A global id that can be used to identify a payment
#[derive(
    Debug,
    Clone,
    Hash,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    diesel::expression::AsExpression,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub struct PaymentGlobalId(super::GlobalId);

// Database related implementations so that this field can be used directly in the database tables
crate::impl_queryable_id_type!(PaymentGlobalId);

impl<DB> diesel::serialize::ToSql<diesel::sql_types::Text, DB> for PaymentGlobalId
where
    DB: diesel::backend::Backend,
    super::GlobalId: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}
impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for PaymentGlobalId
where
    DB: diesel::backend::Backend,
    super::GlobalId: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        super::GlobalId::from_sql(value).map(Self)
    }
}
