use common_utils::errors::CustomResult;
use external_services::aws_kms::{
    core::{AwsKmsClient, AwsKmsError},
    decrypt::AwsKmsDecrypt,
};
use masking::ExposeInterface;

use crate::configs::settings;

#[async_trait::async_trait]
impl AwsKmsDecrypt for settings::Jwekey {
    type Output = Self;

    async fn decrypt_inner(
        mut self,
        aws_kms_client: &AwsKmsClient,
    ) -> CustomResult<Self::Output, AwsKmsError> {
        (
            self.vault_encryption_key,
            self.rust_locker_encryption_key,
            self.vault_private_key,
            self.tunnel_private_key,
        ) = tokio::try_join!(
            aws_kms_client.decrypt(self.vault_encryption_key),
            aws_kms_client.decrypt(self.rust_locker_encryption_key),
            aws_kms_client.decrypt(self.vault_private_key),
            aws_kms_client.decrypt(self.tunnel_private_key),
        )?;
        Ok(self)
    }
}

#[async_trait::async_trait]
impl AwsKmsDecrypt for settings::ActiveKmsSecrets {
    type Output = Self;
    async fn decrypt_inner(
        mut self,
        aws_kms_client: &AwsKmsClient,
    ) -> CustomResult<Self::Output, AwsKmsError> {
        self.jwekey = self
            .jwekey
            .expose()
            .decrypt_inner(aws_kms_client)
            .await?
            .into();
        Ok(self)
    }
}

#[async_trait::async_trait]
impl AwsKmsDecrypt for settings::Database {
    type Output = storage_impl::config::Database;

    async fn decrypt_inner(
        mut self,
        aws_kms_client: &AwsKmsClient,
    ) -> CustomResult<Self::Output, AwsKmsError> {
        Ok(storage_impl::config::Database {
            host: self.host,
            port: self.port,
            dbname: self.dbname,
            username: self.username,
            password: self.password.decrypt_inner(aws_kms_client).await?.into(),
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
impl AwsKmsDecrypt for settings::PayPalOnboarding {
    type Output = Self;

    async fn decrypt_inner(
        mut self,
        aws_kms_client: &AwsKmsClient,
    ) -> CustomResult<Self::Output, AwsKmsError> {
        self.client_id = aws_kms_client
            .decrypt(self.client_id.expose())
            .await?
            .into();
        self.client_secret = aws_kms_client
            .decrypt(self.client_secret.expose())
            .await?
            .into();
        self.partner_id = aws_kms_client
            .decrypt(self.partner_id.expose())
            .await?
            .into();
        Ok(self)
    }
}

#[cfg(feature = "olap")]
#[async_trait::async_trait]
impl AwsKmsDecrypt for settings::ConnectorOnboarding {
    type Output = Self;

    async fn decrypt_inner(
        mut self,
        aws_kms_client: &AwsKmsClient,
    ) -> CustomResult<Self::Output, AwsKmsError> {
        self.paypal = self.paypal.decrypt_inner(aws_kms_client).await?;
        Ok(self)
    }
}
