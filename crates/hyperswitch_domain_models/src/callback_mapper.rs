use common_utils::errors;
use common_enums::enums as common_enums;
use common_types::callback_mapper::CallbackMapperData;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallbackMapper {
    pub id: String,
    pub callback_mapper_id_type: common_enums::CallbackMapperIdType,
    pub data: CallbackMapperData,
    pub created_at: time::PrimitiveDateTime,
    pub last_modified_at: time::PrimitiveDateTime,
}

impl CallbackMapper {
    pub fn new(
        id: String,
        callback_mapper_id_type: common_enums::CallbackMapperIdType,
        data: CallbackMapperData,
        created_at: time::PrimitiveDateTime,
        last_modified_at: time::PrimitiveDateTime,
    ) -> Self {
        Self {
            id,
            callback_mapper_id_type,
            data,
            created_at,
            last_modified_at,
        }
    }
}

#[async_trait::async_trait]
pub trait CallbackMapperInterface {
    type Error;
    async fn insert_call_back_mapper(
        &self,
        call_back_mapper: CallbackMapper,
    ) -> errors::CustomResult<CallbackMapper, Self::Error>;

    async fn find_call_back_mapper_by_id(
        &self,
        id: &str,
    ) -> errors::CustomResult<CallbackMapper, Self::Error>;
}
