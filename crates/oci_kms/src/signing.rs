//! OCI Signature v1 request signing (RSA-SHA256 over a canonical header string).
//!
//! Pure and network-free: given credentials and request parts, produces the header
//! values a caller attaches itself. Implements Oracle's POST-signing scheme
//! (<https://docs.oracle.com/en-us/iaas/Content/API/Concepts/signingrequests.htm>),
//! the only one this backend needs since Encrypt/Decrypt are both POSTs.

use base64::Engine;
use rsa::{
    pkcs1v15::SigningKey,
    sha2::{Digest, Sha256},
    signature::{RandomizedSigner, SignatureEncoding},
    RsaPrivateKey,
};

use crate::{environment::Environment, error::OciKmsError};

const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// Header order signed in both the canonical string and the `Authorization` header's
/// `headers` field — matches Oracle's POST-signing example verbatim.
const SIGNED_HEADERS: &str =
    "date (request-target) host content-length content-type x-content-sha256";

/// RFC 7231 IMF-fixdate, the form OCI requires in the signed `date` header.
const HTTP_DATE_FORMAT: &[time::format_description::FormatItem<'_>] = time::macros::format_description!(
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
    environment: &dyn Environment,
    key_id: &str,
    private_key: &RsaPrivateKey,
    host: &str,
    path: &str,
    body: &[u8],
) -> Result<SignedHeaders, OciKmsError> {
    let date = http_date(environment.now_unix_timestamp())?;

    Ok(sign_post_request_at(
        date,
        key_id,
        private_key,
        host,
        path,
        body,
    ))
}

fn http_date(unix_timestamp: i64) -> Result<String, OciKmsError> {
    time::OffsetDateTime::from_unix_timestamp(unix_timestamp)
        .map_err(|error| OciKmsError::SigningFailed(format!("invalid clock reading: {error}")))?
        .format(HTTP_DATE_FORMAT)
        .map_err(|error| OciKmsError::SigningFailed(format!("failed to format the date: {error}")))
}

/// [`sign_post_request`] with the `date` header value supplied, so output is reproducible.
fn sign_post_request_at(
    date: String,
    key_id: &str,
    private_key: &RsaPrivateKey,
    host: &str,
    path: &str,
    body: &[u8],
) -> SignedHeaders {
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

    SignedHeaders {
        date,
        authorization,
        x_content_sha256: body_hash,
    }
}

#[cfg(test)]
mod tests {
    use rsa::{
        pkcs1v15::{Signature, VerifyingKey},
        signature::Verifier,
    };

    use super::*;

    const DATE: &str = "Thu, 05 Jan 2014 21:31:40 GMT";
    const HOST: &str = "iaas.us-phoenix-1.oraclecloud.com";
    const PATH: &str = "/20160918/volumeAttachments";
    const BODY: &[u8] = br#"{"compartmentId":"ocid1.compartment.oc1..aaaa"}"#;
    const KEY_ID: &str = "ocid1.tenancy.oc1..test/ocid1.user.oc1..test/fingerprint";

    /// Generated per run rather than checked in, so no private key lives in the repo.
    fn test_key() -> RsaPrivateKey {
        RsaPrivateKey::new(&mut rand::rngs::OsRng, 1024).expect("key generation succeeds")
    }

    /// Pulls the base64 `signature="..."` value out of an `Authorization` header.
    fn signature_of(authorization: &str) -> Vec<u8> {
        let encoded = authorization
            .split("signature=\"")
            .nth(1)
            .and_then(|rest| rest.strip_suffix('"'))
            .expect("authorization header carries a signature");
        BASE64_ENGINE.decode(encoded).expect("signature is base64")
    }

    #[test]
    fn x_content_sha256_is_the_base64_sha256_of_the_body() {
        let signed = sign_post_request_at(DATE.to_owned(), KEY_ID, &test_key(), HOST, PATH, b"");
        // SHA-256 of the empty string.
        assert_eq!(
            signed.x_content_sha256,
            "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU="
        );
    }

    #[test]
    fn authorization_header_has_the_oci_signature_v1_shape() {
        let signed = sign_post_request_at(DATE.to_owned(), KEY_ID, &test_key(), HOST, PATH, BODY);

        assert_eq!(signed.date, DATE);
        assert!(signed.authorization.starts_with(
            "Signature version=\"1\",\
             headers=\"date (request-target) host content-length content-type x-content-sha256\",\
             keyId=\"ocid1.tenancy.oc1..test/ocid1.user.oc1..test/fingerprint\",\
             algorithm=\"rsa-sha256\",\
             signature=\""
        ));
    }

    /// The signing string is written out literally here, in the exact form Oracle's docs
    /// specify for a POST, rather than rebuilt from the code under test. The signature only
    /// verifies if the code signed exactly this string, with RSA PKCS#1 v1.5 over SHA-256.
    #[test]
    fn signature_verifies_over_oracles_post_signing_string() {
        let private_key = test_key();
        let signed = sign_post_request_at(DATE.to_owned(), KEY_ID, &private_key, HOST, PATH, BODY);

        let expected_signing_string = "date: Thu, 05 Jan 2014 21:31:40 GMT\n\
             (request-target): post /20160918/volumeAttachments\n\
             host: iaas.us-phoenix-1.oraclecloud.com\n\
             content-length: 47\n\
             content-type: application/json\n\
             x-content-sha256: JL3n1o6nGMwsBAd+52/KQ24uJCKaE4r2X8cuX1MVPRw=";

        let signature = Signature::try_from(signature_of(&signed.authorization).as_slice())
            .expect("well-formed PKCS#1 v1.5 signature");
        VerifyingKey::<Sha256>::new(private_key.to_public_key())
            .verify(expected_signing_string.as_bytes(), &signature)
            .expect("signature verifies over Oracle's signing string");
    }

    #[test]
    fn signature_does_not_verify_over_a_different_signing_string() {
        let private_key = test_key();
        let signed = sign_post_request_at(DATE.to_owned(), KEY_ID, &private_key, HOST, PATH, BODY);

        let signature = Signature::try_from(signature_of(&signed.authorization).as_slice())
            .expect("well-formed PKCS#1 v1.5 signature");
        let verified = VerifyingKey::<Sha256>::new(private_key.to_public_key()).verify(
            b"date: Thu, 05 Jan 2014 21:31:41 GMT\n(request-target): post /20160918/volumeAttachments",
            &signature,
        );
        assert!(verified.is_err());
    }

    #[test]
    fn http_date_is_rfc_7231_in_gmt() {
        // 2014-01-02 09:05:07 UTC, a Thursday.
        assert_eq!(
            http_date(1_388_653_507).expect("valid timestamp"),
            "Thu, 02 Jan 2014 09:05:07 GMT"
        );
    }
}
