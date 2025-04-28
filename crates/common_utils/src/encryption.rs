use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, Queryable},
    expression::AsExpression,
    serialize::ToSql,
    sql_types,
};
use error_stack::{report, ResultExt};
use josekit::jwe;
use masking::Secret;

use crate::{crypto::Encryptable, errors::CustomResult, fp_utils, pii::EncryptionStrategy};

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

#[derive(Debug, AsExpression, Clone, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
#[diesel(sql_type = sql_types::Binary)]
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

#[derive(Debug, Eq, PartialEq, Copy, Clone, strum::AsRefStr, strum::Display)]
pub enum EncryptionAlgorithm {
    A128GCM,
    A256GCM,
}

pub async fn encrypt_jwe(
    payload: &[u8],
    public_key: impl AsRef<[u8]>,
    algorithm: EncryptionAlgorithm,
    key_id: Option<&str>,
) -> CustomResult<String, std::fmt::Error> {
    let alg = jwe::RSA_OAEP_256;
    let mut src_header = jwe::JweHeader::new();
    let enc_str = algorithm.as_ref();
    src_header.set_content_encryption(enc_str);
    src_header.set_token_type("JWT");
    if let Some(key_id) = key_id {
        src_header.set_key_id(key_id);
    }
    let encrypter = alg
        .encrypter_from_pem(public_key)
        .change_context(std::fmt::Error)
        .attach_printable("Error getting JweEncryptor")?;

    jwe::serialize_compact(payload, &src_header, &encrypter)
        .change_context(std::fmt::Error)
        .attach_printable("Error getting jwt string")
}

#[derive(Debug, Clone)]
pub enum KeyIdCheck<'a> {
    RequestResponseKeyId((&'a str, &'a str)),
    SkipKeyIdCheck,
}

pub async fn decrypt_jwe(
    jwt: &str,
    key_ids: KeyIdCheck<'_>,
    private_key: impl AsRef<[u8]>,
    alg: jwe::alg::rsaes::RsaesJweAlgorithm,
) -> CustomResult<String, std::fmt::Error> {
    if let KeyIdCheck::RequestResponseKeyId((req_key_id, resp_key_id)) = key_ids {
        fp_utils::when(req_key_id.ne(resp_key_id), || {
            Err(report!(std::fmt::Error)
                .attach_printable("key_id mismatch, Error authenticating response"))
        })?;
    }

    let decrypter = alg
        .decrypter_from_pem(private_key)
        .change_context(std::fmt::Error)
        .attach_printable("Error getting JweDecryptor")?;

    let (dst_payload, _dst_header) = jwe::deserialize_compact(jwt, &decrypter)
        .change_context(std::fmt::Error)
        .attach_printable("Error getting Decrypted jwe")?;

    String::from_utf8(dst_payload)
        .change_context(std::fmt::Error)
        .attach_printable("Could not decode JWE payload from UTF-8")
}
