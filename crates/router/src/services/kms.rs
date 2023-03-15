use crate::core::errors::{self, CustomResult};

pub struct KeyHandler;

#[cfg(feature = "kms")]
mod aws_kms {
    use aws_config::meta::region::RegionProviderChain;
    use aws_sdk_kms::{types::Blob, Client, Region};
    use base64::Engine;
    use error_stack::{report, IntoReport, ResultExt};

    use super::*;
    use crate::{consts, logger};

    impl KeyHandler {
        // Fetching KMS decrypted key
        // | Amazon KMS decryption
        // This expect a base64 encoded input but we values are set via aws cli in env than cli
        // already does that so we don't need to
        pub async fn get_kms_decrypted_key(
            aws_region: &str,
            aws_key_id: &str,
            kms_enc_key: String,
        ) -> CustomResult<String, errors::EncryptionError> {
            let region_provider =
                RegionProviderChain::first_try(Region::new(aws_region.to_owned()));
            let shared_config = aws_config::from_env().region(region_provider).load().await;
            let client = Client::new(&shared_config);
            let data = consts::BASE64_ENGINE
                .decode(kms_enc_key)
                .into_report()
                .change_context(errors::EncryptionError)
                .attach_printable("Error decoding from base64")?;
            let blob = Blob::new(data);
            let resp = client
                .decrypt()
                .key_id(aws_key_id)
                .ciphertext_blob(blob)
                .send()
                .await
                .map_err(|error| {
                    logger::error!(kms_sdk_error=?error, "Failed to KMS decrypt data");
                    error
                })
                .into_report()
                .change_context(errors::EncryptionError)
                .attach_printable("Error decrypting kms encrypted data")?;
            match resp.plaintext() {
                Some(inner) => {
                    let bytes = inner.as_ref().to_vec();
                    let res = String::from_utf8(bytes)
                        .into_report()
                        .change_context(errors::EncryptionError)
                        .attach_printable("Could not convert to UTF-8")?;
                    Ok(res)
                }
                None => Err(report!(errors::EncryptionError)
                    .attach_printable("Missing plaintext in response")),
            }
        }
    }
}

#[cfg(not(feature = "kms"))]
impl KeyHandler {
    pub async fn get_kms_decrypted_key(
        _aws_region: &str,
        _aws_key_id: &str,
        key: String,
    ) -> CustomResult<String, errors::EncryptionError> {
        Ok(key)
    }
}
