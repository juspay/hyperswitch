#[cfg(feature = "diesel")]
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, Queryable},
    expression::AsExpression,
    serialize::ToSql,
    sql_types,
};
use masking::Secret;

use crate::{crypto::Encryptable, pii::EncryptionStrategy};

#[cfg(feature = "diesel")]
impl<DB> FromSql<sql_types::Binary, DB> for Encryption
where
    DB: Backend,
    Secret<Vec<u8>, EncryptionStrategy>: FromSql<sql_types::Binary, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        <Secret<Vec<u8>, EncryptionStrategy>>::from_sql(bytes).map(Self::new)
    }
}

#[cfg(feature = "diesel")]
impl<DB> ToSql<sql_types::Binary, DB> for Encryption
where
    DB: Backend,
    Secret<Vec<u8>, EncryptionStrategy>: ToSql<sql_types::Binary, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.get_inner().to_sql(out)
    }
}

#[cfg(feature = "diesel")]
impl<DB> Queryable<sql_types::Binary, DB> for Encryption
where
    DB: Backend,
    Secret<Vec<u8>, EncryptionStrategy>: FromSql<sql_types::Binary, DB>,
{
    type Row = Secret<Vec<u8>, EncryptionStrategy>;
    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(Self { inner: row })
    }
}

#[cfg_attr(feature = "diesel", derive(AsExpression))]
#[cfg_attr(feature = "diesel", diesel(sql_type = sql_types::Binary))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
#[repr(transparent)]
pub struct Encryption {
    inner: Secret<Vec<u8>, EncryptionStrategy>,
}

impl<T: Clone> From<Encryptable<T>> for Encryption {
    fn from(value: Encryptable<T>) -> Self {
        Self::new(value.into_encrypted())
    }
}

impl Encryption {
    pub fn new(item: Secret<Vec<u8>, EncryptionStrategy>) -> Self {
        Self { inner: item }
    }

    #[inline]
    pub fn into_inner(self) -> Secret<Vec<u8>, EncryptionStrategy> {
        self.inner
    }

    #[inline]
    pub fn get_inner(&self) -> &Secret<Vec<u8>, EncryptionStrategy> {
        &self.inner
    }
}
