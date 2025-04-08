use std::ops::Deref;

use crate::events::ApiEventMetric;

crate::id_type!(
    RoutingId,
    " A type for routing_id that can be used for routing ids"
);

crate::impl_id_type_methods!(RoutingId, "routing_id");

// This is to display the `RoutingId` as RoutingId(abcd)
crate::impl_debug_id_type!(RoutingId);
crate::impl_try_from_cow_str_id_type!(RoutingId, "routing_id");

crate::impl_generate_id_id_type!(RoutingId, "routing");
crate::impl_serializable_secret_id_type!(RoutingId);
crate::impl_queryable_id_type!(RoutingId);
crate::impl_to_sql_from_sql_id_type!(RoutingId);

impl ApiEventMetric for RoutingId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::Routing)
    }
}

#[derive(
    Clone,
    Hash,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    diesel::expression::AsExpression,
    utoipa::ToSchema,
)]
#[diesel(sql_type = diesel::sql_types::Text,)]
#[schema(value_type = String)]
/// A wrapper type for `RoutingId` that can be used for surcharge routing ids
pub struct SurchargeRoutingId(pub RoutingId);

impl Deref for SurchargeRoutingId {
    type Target = RoutingId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ApiEventMetric for SurchargeRoutingId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::Routing)
    }
}
crate::impl_serializable_secret_id_type!(SurchargeRoutingId);
crate::impl_queryable_id_type!(SurchargeRoutingId);

impl<DB> diesel::serialize::ToSql<diesel::sql_types::Text, DB> for SurchargeRoutingId
where
    DB: diesel::backend::Backend,
    RoutingId: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for SurchargeRoutingId
where
    DB: diesel::backend::Backend,
    RoutingId: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let val = RoutingId::from_sql(value)?;
        Ok(Self(val))
    }
}
