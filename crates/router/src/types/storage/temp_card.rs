use diesel::{Identifiable, Insertable, Queryable};
use serde_json::Value;
use time::PrimitiveDateTime;

use crate::schema::temp_card;

#[derive(Clone, Debug, router_derive::DebugAsDisplay, Queryable, Identifiable, Insertable)]
#[diesel(table_name = temp_card)]
pub struct TempCard {
    pub id: i32,
    pub date_created: PrimitiveDateTime,
    pub txn_id: Option<String>,
    pub card_info: Option<Value>,
}

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = temp_card)]
pub struct TempCardNew {
    pub id: Option<i32>,
    pub card_info: Option<Value>,
    pub date_created: PrimitiveDateTime,
    pub txn_id: Option<String>,
}
