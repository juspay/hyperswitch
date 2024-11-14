use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{self, Deserialize, Serialize};
use serde_json;

use crate::schema::call_back_mapper;

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Serialize, Deserialize,
)]
#[diesel(table_name = call_back_mapper,  primary_key(id), check_for_backend(diesel::pg::Pg))]

pub struct CallBackMapper {
    pub id: String,
    #[serde(rename = "type")]
    pub id_type: String,
    pub data: serde_json::Value,
    pub created_at: time::PrimitiveDateTime,
    pub last_modified_at: time::PrimitiveDateTime,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Insertable)]
#[diesel(table_name = call_back_mapper)]
pub struct CallBackMapperNew {
    pub id: String,
    #[serde(rename = "type")]
    pub id_type: String,
    pub data: serde_json::Value,
}
