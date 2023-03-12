use aws_config::meta::region::RegionProviderChain;
use aws_sdk_kms::{types::Blob, Client, Region};
use base64::Engine;
use error_stack::{report, IntoReport, ResultExt};

use crate::{
    consts,
    core::errors::{self, CustomResult},
    logger,
    routes::metrics,
};

pub struct KeyHandler;

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
        let region_provider = RegionProviderChain::first_try(Region::new(aws_region.to_owned()));
        let sdk_config = aws_config::from_env().region(region_provider).load().await;
        let client = Client::new(&sdk_config);
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
                metrics::AWS_KMS_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                error
            })
            .into_report()
            .change_context(errors::EncryptionError)
            .attach_printable("Error decrypting kms encrypted data")?;
        match resp.plaintext {
            Some(blob) => {
                let res = String::from_utf8(blob.into_inner())
                    .into_report()
                    .change_context(errors::EncryptionError)
                    .attach_printable("Could not convert to UTF-8")?;
                Ok(res)
            }
            None => {
                Err(report!(errors::EncryptionError)
                    .attach_printable("Missing plaintext in response"))
            }
        }
    }
}
