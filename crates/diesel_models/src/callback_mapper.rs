use common_utils::pii;
use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{self, Deserialize, Serialize};

use crate::schema::callback_mapper;

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Identifiable,
    Queryable,
    Selectable,
    Serialize,
    Deserialize,
    Insertable,
)]
#[diesel(table_name = callback_mapper,  primary_key(id, type_), check_for_backend(diesel::pg::Pg))]
pub struct CallbackMapper {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub data: pii::SecretSerdeValue,
    pub created_at: time::PrimitiveDateTime,
    pub last_modified_at: time::PrimitiveDateTime,
}
