use common_utils::errors::CustomResult;
use external_services::kms::{decrypt::KmsDecrypt, KmsClient, KmsError};

use crate::configs::settings;

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
        })
    }
}
