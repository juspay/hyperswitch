/// Trait for converting domain types to storage models
#[async_trait::async_trait]
pub trait Conversion {
    type DstType;
    async fn convert(self) -> Self::DstType;

    async fn convert_back(item: Self::DstType) -> Self;
}

#[async_trait::async_trait]
pub trait ReverseConversion<SrcType: Conversion> {
    async fn convert(self) -> SrcType;
}

#[async_trait::async_trait]
impl<T: Send, U: Conversion<DstType = T>> ReverseConversion<U> for T {
    async fn convert(self) -> U {
        U::convert_back(self).await
    }
}
