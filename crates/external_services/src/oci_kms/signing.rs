//! OCI Signature v1 request signing (RSA-SHA256 over a canonical header string).
//!
//! Pure and network-free: given credentials and request parts, produces the header
//! values a caller attaches itself. Implements Oracle's POST-signing scheme
//! (<https://docs.oracle.com/en-us/iaas/Content/API/Concepts/signingrequests.htm>),
//! the only one this backend needs since Encrypt/Decrypt are both POSTs.

use base64::Engine;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use rsa::{
    pkcs1v15::SigningKey,
    sha2::{Digest, Sha256},
    signature::{RandomizedSigner, SignatureEncoding},
    RsaPrivateKey,
};
use time::macros::format_description;

use super::core::OciKmsError;

const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// Header order signed in both the canonical string and the `Authorization` header's
/// `headers` field — matches Oracle's POST-signing example verbatim.
const SIGNED_HEADERS: &str =
    "date (request-target) host content-length content-type x-content-sha256";

const HTTP_DATE_FORMAT: &[time::format_description::FormatItem<'_>] = format_description!(
    "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT"
);

/// Header values to attach to a signed request. `x_content_sha256` must be sent as a
/// literal header too, not just folded into the signature — OCI independently checks
/// it against the request body it received.
pub(crate) struct SignedHeaders {
    pub(crate) date: String,
    pub(crate) authorization: String,
    pub(crate) x_content_sha256: String,
}

/// Signs a POST request per OCI's Signature v1 scheme. `key_id` is the `keyId` value
/// (`ST$<session-token>` for Workload Identity). `path` includes any query string;
/// `host` is the request's Host header value (no scheme).
pub(crate) fn sign_post_request(
    key_id: &str,
    private_key: &RsaPrivateKey,
    host: &str,
    path: &str,
    body: &[u8],
) -> CustomResult<SignedHeaders, OciKmsError> {
    let date = time::OffsetDateTime::now_utc()
        .format(HTTP_DATE_FORMAT)
        .change_context(OciKmsError::SigningFailed)
        .attach_printable("Failed to format the request date")?;
    let content_length = body.len().to_string();
    let content_type = "application/json";
    let body_hash = BASE64_ENGINE.encode(Sha256::digest(body));

    let signing_string = format!(
        "date: {date}\n\
         (request-target): post {path}\n\
         host: {host}\n\
         content-length: {content_length}\n\
         content-type: {content_type}\n\
         x-content-sha256: {body_hash}"
    );

    // `sign_with_rng` (not the deterministic `sign`) blinds the RSA operation against
    // timing side channels — PKCS#1 v1.5 output is still deterministic either way.
    let signing_key = SigningKey::<Sha256>::new(private_key.clone());
    let mut rng = rand::rngs::OsRng;
    let signature = signing_key.sign_with_rng(&mut rng, signing_string.as_bytes());
    let encoded_signature = BASE64_ENGINE.encode(signature.to_bytes());

    let authorization = format!(
        "Signature version=\"1\",headers=\"{SIGNED_HEADERS}\",keyId=\"{key_id}\",\
         algorithm=\"rsa-sha256\",signature=\"{encoded_signature}\""
    );

    Ok(SignedHeaders {
        date,
        authorization,
        x_content_sha256: body_hash,
    })
}
