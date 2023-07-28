use common_utils::errors::CustomResult;
use external_services::kms;
use masking::ExposeInterface;

use crate::configs::settings;

#[async_trait::async_trait]
// This trait performs in place decryption of the structure on which this is implemented
pub trait KmsDecrypt {
    pub type Output;
    async fn decrypt_inner(self, kms_config: &kms::KmsConfig) -> CustomResult<Self::Output, kms::KmsError>
    where
        Self: Sized;
}

#[async_trait::async_trait]
impl KmsDecrypt for settings::Jwekey {
    pub type Output = Self;

    async fn decrypt_inner(self, kms_config: &kms::KmsConfig) -> CustomResult<Self::Output, kms::KmsError> {
        let client = kms::get_kms_client(kms_config).await;

        // If this pattern required repetition, a macro approach needs to be devised
        let (
            locker_encryption_key1,
            locker_encryption_key2,
            locker_decryption_key1,
            locker_decryption_key2,
            vault_encryption_key,
            vault_private_key,
            tunnel_private_key,
        ) = tokio::try_join!(
            client.decrypt(self.locker_encryption_key1),
            client.decrypt(self.locker_encryption_key2),
            client.decrypt(self.locker_decryption_key1),
            client.decrypt(self.locker_decryption_key2),
            client.decrypt(self.vault_encryption_key),
            client.decrypt(self.vault_private_key),
            client.decrypt(self.tunnel_private_key),
        )?;

        self.locker_encryption_key1 = locker_encryption_key1;
        self.locker_encryption_key2 = locker_encryption_key2;
        self.locker_decryption_key1 = locker_decryption_key1;
        self.locker_decryption_key2 = locker_decryption_key2;
        self.vault_encryption_key = vault_encryption_key;
        self.tunnel_private_key = tunnel_private_key;
        Ok(())
    }
}

#[async_trait::async_trait]
impl KmsDecrypt for settings::ActiveKmsSecrets {
    pub type Output = Self;
    async fn decrypt_inner(&mut self, kms_config: &kms::KmsConfig) -> CustomResult<Self::Output, kms::KmsError> {
        self.jwekey = self.jwekey.expose().decrypt_inner(kms_config).await?.into();
        Ok(())
    }
}

impl KmsDecrypt for KMSValue {
    type Output = String;
    fn decrypt(self, kms_client: &KmsClient) -> CustomResult<Self::Output, KmsError> {
        kms_client.decrypt(self.0).attach_printable("Failed to decrypt KMS value")
    }
}