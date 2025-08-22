use common_enums::enums as common_enums;
use common_types::callback_mapper::CallbackMapperData;
use diesel::{Identifiable, Insertable, Queryable, Selectable};

use crate::schema::callback_mapper;

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Insertable)]
#[diesel(table_name = callback_mapper,  primary_key(id, type_), check_for_backend(diesel::pg::Pg))]
pub struct CallbackMapper {
    pub id: String,
    pub type_: common_enums::CallbackMapperIdType,
    pub data: CallbackMapperData,
    pub created_at: time::PrimitiveDateTime,
    pub last_modified_at: time::PrimitiveDateTime,
}
