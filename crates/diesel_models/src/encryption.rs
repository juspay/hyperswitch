use common_utils::pii::EncryptionStrategy;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, Queryable},
    serialize::ToSql,
    sql_types, AsExpression,
};
use masking::Secret;

#[derive(Debug, AsExpression, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
#[diesel(sql_type = sql_types::Binary)]
#[repr(transparent)]
pub struct Encryption {
    inner: Secret<Vec<u8>, EncryptionStrategy>,
}

impl<T: Clone> From<common_utils::crypto::Encryptable<T>> for Encryption {
    fn from(value: common_utils::crypto::Encryptable<T>) -> Self {
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

impl<DB> FromSql<sql_types::Binary, DB> for Encryption
where
    DB: Backend,
    Secret<Vec<u8>, EncryptionStrategy>: FromSql<sql_types::Binary, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        <Secret<Vec<u8>, EncryptionStrategy>>::from_sql(bytes).map(Self::new)
    }
}

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
