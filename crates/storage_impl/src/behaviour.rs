use common_utils::{
    errors::{CustomResult, ValidationError},
    types::keymanager::{Identifier, KeyManagerState},
};
use hyperswitch_masking::Secret;

#[async_trait::async_trait]
pub trait Conversion {
    type DstType;
    type NewDstType;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError>;

    async fn convert_back(
        state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: Identifier,
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
        key_manager_identifier: Identifier,
    ) -> CustomResult<SrcType, ValidationError>;
}

#[async_trait::async_trait]
impl<T: Send, U: Conversion<DstType = T>> ReverseConversion<U> for T {
    async fn convert(
        self,
        state: &KeyManagerState,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: Identifier,
    ) -> CustomResult<U, ValidationError> {
        U::convert_back(state, self, key, key_manager_identifier).await
    }
}

pub trait ForeignInto<T> {
    fn foreign_into(self) -> T;
}

pub trait ForeignTryInto<T> {
    type Error;

    fn foreign_try_into(self) -> Result<T, Self::Error>;
}

pub trait ForeignFrom<F> {
    fn foreign_from(from: F) -> Self;
}

pub trait ForeignTryFrom<F>: Sized {
    type Error;

    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

impl<F, T> ForeignInto<T> for F
where
    T: ForeignFrom<F>,
{
    fn foreign_into(self) -> T {
        T::foreign_from(self)
    }
}

impl<F, T> ForeignTryInto<T> for F
where
    T: ForeignTryFrom<F>,
{
    type Error = <T as ForeignTryFrom<F>>::Error;

    fn foreign_try_into(self) -> Result<T, Self::Error> {
        T::foreign_try_from(self)
    }
}
