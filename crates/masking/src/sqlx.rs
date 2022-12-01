use crate::{Secret, Strategy, StrongSecret, ZeroizableSecret};

impl<'q, DB: sqlx::Database, T, S> sqlx::encode::Encode<'q, DB> for StrongSecret<T, S>
where
    T: sqlx::encode::Encode<'q, DB> + ZeroizableSecret,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        T::encode_by_ref(&self.inner_secret, buf)
    }
    fn produces(&self) -> Option<DB::TypeInfo> {
        T::produces(&self.inner_secret)
    }
    fn size_hint(&self) -> usize {
        T::size_hint(&self.inner_secret)
    }

    fn encode(
        self,
        buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull
    where
        Self: Sized,
    {
        self.encode_by_ref(buf)
    }
}

impl<'r, DB: sqlx::Database, T, S> sqlx::decode::Decode<'r, DB> for StrongSecret<T, S>
where
    T: sqlx::Decode<'r, DB> + ZeroizableSecret,
{
    fn decode(
        value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        T::decode(value).map(Self::new)
    }
}

impl<DB: sqlx::Database, T, S> sqlx::Type<DB> for StrongSecret<T, S>
where
    T: sqlx::Type<DB> + ZeroizableSecret,
{
    fn type_info() -> DB::TypeInfo {
        T::type_info()
    }
    fn compatible(ty: &DB::TypeInfo) -> bool {
        T::compatible(ty)
    }
}

impl<T, S> sqlx::postgres::PgHasArrayType for StrongSecret<T, S>
where
    T: sqlx::postgres::PgHasArrayType + ZeroizableSecret,
{
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        T::array_type_info()
    }
}

impl<'q, DB: sqlx::Database, T, S: Strategy<T>> sqlx::encode::Encode<'q, DB> for Secret<T, S>
where
    T: sqlx::encode::Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        T::encode_by_ref(&self.inner_secret, buf)
    }

    fn produces(&self) -> Option<DB::TypeInfo> {
        T::produces(&self.inner_secret)
    }

    fn size_hint(&self) -> usize {
        T::size_hint(&self.inner_secret)
    }

    fn encode(
        self,
        buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull
    where
        Self: Sized,
    {
        self.encode_by_ref(buf)
    }
}

impl<'r, DB: sqlx::Database, T, S: Strategy<T>> sqlx::decode::Decode<'r, DB> for Secret<T, S>
where
    T: sqlx::Decode<'r, DB>,
{
    fn decode(
        value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        T::decode(value).map(Self::new)
    }
}

impl<DB: sqlx::Database, T, S: Strategy<T>> sqlx::Type<DB> for Secret<T, S>
where
    T: sqlx::Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        T::type_info()
    }
    fn compatible(ty: &DB::TypeInfo) -> bool {
        T::compatible(ty)
    }
}

impl<T> sqlx::postgres::PgHasArrayType for Secret<T>
where
    T: sqlx::postgres::PgHasArrayType,
{
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        T::array_type_info()
    }
}
