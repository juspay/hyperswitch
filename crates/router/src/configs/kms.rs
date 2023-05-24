use common_utils::errors::CustomResult;
use external_services::kms;
use masking::ExposeInterface;

use crate::configs::settings;

#[async_trait::async_trait]
// This trait performs inplace decryption of the structure on which this is implemented
pub(crate) trait KmsDecrypt {
    async fn decrypt_inner(self, kms_config: &kms::KmsConfig) -> CustomResult<Self, kms::KmsError>
    where
        Self: Sized;
}

#[async_trait::async_trait]
impl KmsDecrypt for settings::Jwekey {
    async fn decrypt_inner(self, kms_config: &kms::KmsConfig) -> CustomResult<Self, kms::KmsError> {
        let client = kms::get_kms_client(kms_config).await;

        // If this pattern required repetition, a macro approach needs to be deviced
        let (
            locker_key_identifier1,
            locker_key_identifier2,
            locker_encryption_key1,
            locker_encryption_key2,
            locker_decryption_key1,
            locker_decryption_key2,
            vault_encryption_key,
            vault_private_key,
            tunnel_private_key,
        ) = tokio::try_join!(
            client.decrypt(self.locker_key_identifier1),
            client.decrypt(self.locker_key_identifier2),
            client.decrypt(self.locker_encryption_key1),
            client.decrypt(self.locker_encryption_key2),
            client.decrypt(self.locker_decryption_key1),
            client.decrypt(self.locker_decryption_key2),
            client.decrypt(self.vault_encryption_key),
            client.decrypt(self.vault_private_key),
            client.decrypt(self.tunnel_private_key),
        )?;

        Ok(Self {
            locker_key_identifier1,
            locker_key_identifier2,
            locker_encryption_key1,
            locker_encryption_key2,
            locker_decryption_key1,
            locker_decryption_key2,
            vault_encryption_key,
            vault_private_key,
            tunnel_private_key,
        })
    }
}

#[async_trait::async_trait]
impl KmsDecrypt for settings::ActiveKmsSecrets {
    async fn decrypt_inner(self, kms_config: &kms::KmsConfig) -> CustomResult<Self, kms::KmsError> {
        Ok(Self {
            jwekey: self.jwekey.expose().decrypt_inner(kms_config).await?.into(),
        })
    }
}
