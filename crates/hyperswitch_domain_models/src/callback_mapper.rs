use serde::{self, Deserialize, Serialize};
use common_types::callback_mapper::CallbackMapperData;
use common_enums::enums as common_enums;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallbackMapper {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: common_enums::CallbackMapperIdType,
    pub data: CallbackMapperData,
    pub created_at: time::PrimitiveDateTime,
    pub last_modified_at: time::PrimitiveDateTime,
}


impl CallbackMapper {
    pub fn new(
        id: String,
        type_: common_enums::CallbackMapperIdType,
        data: CallbackMapperData,
        created_at: time::PrimitiveDateTime,
        last_modified_at: time::PrimitiveDateTime,
    ) -> Self {
        Self {
            id,
            type_,
            data,
            created_at,
            last_modified_at,
        }
    }
}
