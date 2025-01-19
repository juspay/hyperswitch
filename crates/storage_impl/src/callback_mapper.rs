use diesel_models::callback_mapper::CallbackMapper as DieselCallbackMapper;
use hyperswitch_domain_models::callback_mapper::CallbackMapper;

use crate::DataModelExt;

impl DataModelExt for CallbackMapper {
    type StorageModel = DieselCallbackMapper;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselCallbackMapper {
            id: self.id,
            type_: self.type_,
            data: self.data,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            id: storage_model.id,
            type_: storage_model.type_,
            data: storage_model.data,
            created_at: storage_model.created_at,
            last_modified_at: storage_model.last_modified_at,
        }
    }
}
