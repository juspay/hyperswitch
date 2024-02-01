use common_utils::errors::{CustomResult, ValidationError};

use crate::pii::Secret;

/// Trait for converting domain types to storage models
#[async_trait::async_trait]
pub trait Conversion {
    type DstType;
    type NewDstType;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError>;

    async fn convert_back(
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized;

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError>;
}

#[async_trait::async_trait]
pub trait ReverseConversion<SrcType: Conversion> {
    async fn convert(self, key: &Secret<Vec<u8>>) -> CustomResult<SrcType, ValidationError>;
}

#[async_trait::async_trait]
impl<T: Send, U: Conversion<DstType = T>> ReverseConversion<U> for T {
        /// Asynchronously converts the given value using the provided encryption key, and returns a CustomResult containing the converted value or a ValidationError.
    async fn convert(self, key: &Secret<Vec<u8>>) -> CustomResult<U, ValidationError> {
        U::convert_back(self, key).await
    }
}
