use diesel_models::callback_mapper::{
    CallBackMapper as DieselCallBackMapper, CallBackMapperNew as DieselCallBackMapperNew,
};

use hyperswitch_domain_models::callback_mapper::{CallBackMapper, CallBackMapperNew};

use crate::DataModelExt;

impl DataModelExt for CallBackMapper {
    type StorageModel = DieselCallBackMapper;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselCallBackMapper {
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

impl DataModelExt for CallBackMapperNew {
    type StorageModel = DieselCallBackMapperNew;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselCallBackMapperNew {
            id: self.id,
            type_: self.type_,
            data: self.data,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            id: storage_model.id,
            type_: storage_model.type_,
            data: storage_model.data,
        }
    }
}
