use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, Queryable},
    expression::AsExpression,
    serialize::ToSql,
    sql_types,
};
use error_stack::{report, ResultExt};
use josekit::{jwe, jws};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{crypto::Encryptable, errors, fp_utils, pii::EncryptionStrategy};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwsBody {
    pub header: String,
    pub payload: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JweBody {
    pub header: String,
    pub iv: String,
    pub encrypted_payload: String,
    pub tag: String,
    pub encrypted_key: String,
}

#[allow(missing_debug_implementations)]
pub enum KeyIdCheck<'a> {
    RequestResponseKeyId((&'a str, &'a str)),
    SkipKeyIdCheck,
}

pub async fn encrypt_jwe(
    payload: &[u8],
    public_key: impl AsRef<[u8]>,
    algorithm: EncryptionAlgorithm,
    key_id: Option<&str>,
) -> errors::CustomResult<String, errors::EncryptionError> {
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
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting JweEncryptor")?;

    jwe::serialize_compact(payload, &src_header, &encrypter)
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting jwt string")
}

pub async fn decrypt_jwe(
    jwt: &str,
    key_ids: KeyIdCheck<'_>,
    private_key: impl AsRef<[u8]>,
    alg: jwe::alg::rsaes::RsaesJweAlgorithm,
) -> errors::CustomResult<String, errors::EncryptionError> {
    if let KeyIdCheck::RequestResponseKeyId((req_key_id, resp_key_id)) = key_ids {
        fp_utils::when(req_key_id.ne(resp_key_id), || {
            Err(report!(errors::EncryptionError)
                .attach_printable("key_id mismatch, Error authenticating response"))
        })?;
    }

    let decrypter = alg
        .decrypter_from_pem(private_key)
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting JweDecryptor")?;

    let (dst_payload, _dst_header) = jwe::deserialize_compact(jwt, &decrypter)
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting Decrypted jwe")?;

    String::from_utf8(dst_payload)
        .change_context(errors::EncryptionError)
        .attach_printable("Could not decode JWE payload from UTF-8")
}

pub async fn jws_sign_payload(
    payload: &[u8],
    kid: &str,
    private_key: impl AsRef<[u8]>,
) -> errors::CustomResult<String, errors::EncryptionError> {
    let alg = jws::RS256;
    let mut src_header = jws::JwsHeader::new();
    src_header.set_key_id(kid);
    let signer = alg
        .signer_from_pem(private_key)
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting signer")?;
    let jwt = jws::serialize_compact(payload, &src_header, &signer)
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting signed jwt string")?;
    Ok(jwt)
}

pub fn verify_sign(
    jws_body: String,
    key: impl AsRef<[u8]>,
) -> errors::CustomResult<String, errors::EncryptionError> {
    let alg = jws::RS256;
    let input = jws_body.as_bytes();
    let verifier = alg
        .verifier_from_pem(key)
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting verifier")?;
    let (dst_payload, _dst_header) = jws::deserialize_compact(input, &verifier)
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting Decrypted jws")?;
    let resp = String::from_utf8(dst_payload)
        .change_context(errors::EncryptionError)
        .attach_printable("Could not convert to UTF-8")?;
    Ok(resp)
}

pub fn get_dotted_jwe(jwe: JweBody) -> String {
    let header = jwe.header;
    let encryption_key = jwe.encrypted_key;
    let iv = jwe.iv;
    let encryption_payload = jwe.encrypted_payload;
    let tag = jwe.tag;
    format!("{header}.{encryption_key}.{iv}.{encryption_payload}.{tag}")
}

pub fn get_dotted_jws(jws: JwsBody) -> String {
    let header = jws.header;
    let payload = jws.payload;
    let signature = jws.signature;
    format!("{header}.{payload}.{signature}")
}
