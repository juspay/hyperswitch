use common_utils::errors::CustomResult;
use external_services::kms::{decrypt::KmsDecrypt, KmsClient, KmsError};
use masking::ExposeInterface;

use crate::configs::settings;

#[async_trait::async_trait]
impl KmsDecrypt for settings::Jwekey {
    type Output = Self;

    async fn decrypt_inner(
        mut self,
        kms_client: &KmsClient,
    ) -> CustomResult<Self::Output, KmsError> {
        (
            self.vault_encryption_key,
            self.rust_locker_encryption_key,
            self.vault_private_key,
            self.tunnel_private_key,
        ) = tokio::try_join!(
            kms_client.decrypt(self.vault_encryption_key),
            kms_client.decrypt(self.rust_locker_encryption_key),
            kms_client.decrypt(self.vault_private_key),
            kms_client.decrypt(self.tunnel_private_key),
        )?;
        Ok(self)
    }
}

#[async_trait::async_trait]
impl KmsDecrypt for settings::ActiveKmsSecrets {
    type Output = Self;
    async fn decrypt_inner(
        mut self,
        kms_client: &KmsClient,
    ) -> CustomResult<Self::Output, KmsError> {
        self.jwekey = self.jwekey.expose().decrypt_inner(kms_client).await?.into();
        Ok(self)
    }
}

#[async_trait::async_trait]
impl KmsDecrypt for settings::Database {
    type Output = storage_impl::config::Database;

    async fn decrypt_inner(
        mut self,
        kms_client: &KmsClient,
    ) -> CustomResult<Self::Output, KmsError> {
        Ok(storage_impl::config::Database {
            host: self.host,
            port: self.port,
            dbname: self.dbname,
            username: self.username,
            password: self.password.decrypt_inner(kms_client).await?.into(),
            pool_size: self.pool_size,
            connection_timeout: self.connection_timeout,
            queue_strategy: self.queue_strategy,
            min_idle: self.min_idle,
            max_lifetime: self.max_lifetime,
        })
    }
}

#[cfg(feature = "olap")]
#[async_trait::async_trait]
impl KmsDecrypt for settings::PayPalOnboarding {
    type Output = Self;

    async fn decrypt_inner(
        mut self,
        kms_client: &KmsClient,
    ) -> CustomResult<Self::Output, KmsError> {
        self.client_id = kms_client.decrypt(self.client_id.expose()).await?.into();
        self.client_secret = kms_client
            .decrypt(self.client_secret.expose())
            .await?
            .into();
        self.partner_id = kms_client.decrypt(self.partner_id.expose()).await?.into();
        Ok(self)
    }
}

#[cfg(feature = "olap")]
#[async_trait::async_trait]
impl KmsDecrypt for settings::ConnectorOnboarding {
    type Output = Self;

    async fn decrypt_inner(
        mut self,
        kms_client: &KmsClient,
    ) -> CustomResult<Self::Output, KmsError> {
        self.paypal = self.paypal.decrypt_inner(kms_client).await?;
        Ok(self)
    }
}
