use base64::Engine;
use error_stack::{IntoReport, ResultExt};

use crate::{consts, errors};

pub fn create_identity_from_certificate_and_key(
    encoded_certificate: String,
    encoded_certificate_key: String,
) -> Result<reqwest::Identity, error_stack::Report<errors::ApiClientError>> {
    let decoded_certificate = consts::BASE64_ENGINE
        .decode(encoded_certificate)
        .into_report()
        .change_context(errors::ApiClientError::CertificateDecodeFailed)?;

    let decoded_certificate_key = consts::BASE64_ENGINE
        .decode(encoded_certificate_key)
        .into_report()
        .change_context(errors::ApiClientError::CertificateDecodeFailed)?;

    let certificate = String::from_utf8(decoded_certificate)
        .into_report()
        .change_context(errors::ApiClientError::CertificateDecodeFailed)?;

    let certificate_key = String::from_utf8(decoded_certificate_key)
        .into_report()
        .change_context(errors::ApiClientError::CertificateDecodeFailed)?;

    reqwest::Identity::from_pkcs8_pem(certificate.as_bytes(), certificate_key.as_bytes())
        .into_report()
        .change_context(errors::ApiClientError::CertificateDecodeFailed)
}