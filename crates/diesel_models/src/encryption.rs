use common_utils::pii::EncryptionStratergy;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, Queryable},
    serialize::ToSql,
    sql_types, AsExpression,
};
use masking::Secret;

#[derive(Debug, AsExpression, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
#[diesel(sql_type = diesel::sql_types::Binary)]
#[repr(transparent)]
pub struct Encryption {
    inner: Secret<Vec<u8>, EncryptionStratergy>,
}

impl<T: Clone> From<common_utils::crypto::Encryptable<T>> for Encryption {
        /// Converts an `Encryptable` value into the current type by creating a new instance with the value encrypted.
    fn from(value: common_utils::crypto::Encryptable<T>) -> Self {
        Self::new(value.into_encrypted())
    }
}

impl Encryption {
        /// Creates a new instance of Self with the provided Secret containing a vector of bytes and encryption strategy.
    pub fn new(item: Secret<Vec<u8>, EncryptionStratergy>) -> Self {
        Self { inner: item }
    }

    #[inline]
        /// Consumes the wrapper and returns the inner `Secret` value containing a vector of u8 bytes and an encryption strategy.
    pub fn into_inner(self) -> Secret<Vec<u8>, EncryptionStratergy> {
        self.inner
    }

    #[inline]
        /// This method returns a reference to the inner `Secret` value, which contains a vector of unsigned bytes
    /// and an encryption strategy.
    pub fn get_inner(&self) -> &Secret<Vec<u8>, EncryptionStratergy> {
        &self.inner
    }
}

impl<DB> FromSql<sql_types::Binary, DB> for Encryption
where
    DB: Backend,
    Secret<Vec<u8>, EncryptionStratergy>: FromSql<sql_types::Binary, DB>,
{
        /// Converts a raw value from the database into a Result<Self> by first converting it into a Secret<Vec<u8>, EncryptionStrategy> and then mapping it to a new instance of Self.
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        <Secret<Vec<u8>, EncryptionStratergy>>::from_sql(bytes).map(Self::new)
    }
}

impl<DB> ToSql<sql_types::Binary, DB> for Encryption
where
    DB: Backend,
    Secret<Vec<u8>, EncryptionStratergy>: ToSql<sql_types::Binary, DB>,
{
        /// Converts the value to its SQL representation and writes it to the provided `Output`.
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
    Secret<Vec<u8>, EncryptionStratergy>: FromSql<sql_types::Binary, DB>,
{
    type Row = Secret<Vec<u8>, EncryptionStratergy>;
        /// Builds a new instance of the struct using the provided row data.
    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(Self { inner: row })
    }
}
