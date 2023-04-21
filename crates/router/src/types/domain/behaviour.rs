use common_utils::errors::{CustomResult, ValidationError};

use crate::db::StorageInterface;

/// Trait for converting domain types to storage models
#[async_trait::async_trait]
pub trait Conversion {
    type DstType;
    type NewDstType;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError>;

    async fn convert_back(
        item: Self::DstType,
        db: &dyn StorageInterface,
        merchant_id: &str,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized;

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError>;
}

#[async_trait::async_trait]
pub trait ReverseConversion<SrcType: Conversion> {
    async fn convert(
        self,
        db: &dyn StorageInterface,
        merchant_id: &str,
    ) -> CustomResult<SrcType, ValidationError>;
}

#[async_trait::async_trait]
impl<T: Send, U: Conversion<DstType = T>> ReverseConversion<U> for T {
    async fn convert(
        self,
        db: &dyn StorageInterface,
        merchant_id: &str,
    ) -> CustomResult<U, ValidationError> {
        U::convert_back(self, db, merchant_id).await
    }
}
