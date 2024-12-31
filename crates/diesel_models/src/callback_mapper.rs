use common_utils::pii;
use diesel::{Identifiable, Insertable, Queryable, Selectable};

use crate::schema::callback_mapper;

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Insertable)]
#[diesel(table_name = callback_mapper,  primary_key(id, type_), check_for_backend(diesel::pg::Pg))]
pub struct CallbackMapper {
    pub id: String,
    pub type_: String,
    pub data: pii::SecretSerdeValue,
    pub created_at: time::PrimitiveDateTime,
    pub last_modified_at: time::PrimitiveDateTime,
}
