use common_utils::{
    errors::{CustomResult, ValidationError},
    types::keymanager::KeyManagerState,
};
use masking::Secret;

/// Trait for converting domain types to storage models
#[async_trait::async_trait]
pub trait Conversion {
    type DstType;
    type NewDstType;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError>;

    async fn convert_back(
        state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_store_ref_id: String,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized;

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError>;
}

#[async_trait::async_trait]
pub trait ReverseConversion<SrcType: Conversion> {
    async fn convert(
        self,
        state: &KeyManagerState,
        key: &Secret<Vec<u8>>,
        key_store_ref_id: String,
    ) -> CustomResult<SrcType, ValidationError>;
}

#[async_trait::async_trait]
impl<T: Send, U: Conversion<DstType = T>> ReverseConversion<U> for T {
    async fn convert(
        self,
        state: &KeyManagerState,
        key: &Secret<Vec<u8>>,
        key_store_ref_id: String,
    ) -> CustomResult<U, ValidationError> {
        U::convert_back(state, self, key, key_store_ref_id).await
    }
}
