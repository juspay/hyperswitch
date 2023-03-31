use common_utils::errors::{CustomResult, ValidationError};

/// Trait for converting domain types to storage models
#[async_trait::async_trait]
pub trait Conversion {
    type DstType;
    type NewDstType;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError>;

    async fn convert_back(item: Self::DstType) -> CustomResult<Self, ValidationError>
    where
        Self: Sized;

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError>;
}

#[async_trait::async_trait]
pub trait ReverseConversion<SrcType: Conversion> {
    async fn convert(self) -> CustomResult<SrcType, ValidationError>;
}

#[async_trait::async_trait]
impl<T: Send, U: Conversion<DstType = T>> ReverseConversion<U> for T {
    async fn convert(self) -> CustomResult<U, ValidationError> {
        U::convert_back(self).await
    }
}
