use external_services::hashicorp_vault::{
    decrypt::VaultFetch, Engine, HashiCorpError, HashiCorpVault,
};
use masking::ExposeInterface;

use crate::configs::settings;

#[async_trait::async_trait]
impl VaultFetch for settings::Jwekey {
        /// Asynchronously fetches encryption keys from HashiCorp Vault using the provided client and updates the internal state of the struct with the fetched keys. Returns a Result containing the updated struct or an error if the fetching process fails.
    async fn fetch_inner<En>(
        mut self,
        client: &HashiCorpVault,
    ) -> error_stack::Result<Self, HashiCorpError>
    where
        for<'a> En: Engine<
                ReturnType<'a, String> = std::pin::Pin<
                    Box<
                        dyn std::future::Future<
                                Output = error_stack::Result<String, HashiCorpError>,
                            > + Send
                            + 'a,
                    >,
                >,
            > + 'a,
    {
        (
            self.vault_encryption_key,
            self.rust_locker_encryption_key,
            self.vault_private_key,
            self.tunnel_private_key,
        ) = (
            masking::Secret::new(self.vault_encryption_key)
                .fetch_inner::<En>(client)
                .await?
                .expose(),
            masking::Secret::new(self.rust_locker_encryption_key)
                .fetch_inner::<En>(client)
                .await?
                .expose(),
            masking::Secret::new(self.vault_private_key)
                .fetch_inner::<En>(client)
                .await?
                .expose(),
            masking::Secret::new(self.tunnel_private_key)
                .fetch_inner::<En>(client)
                .await?
                .expose(),
        );
        Ok(self)
    }
}

#[async_trait::async_trait]
impl VaultFetch for settings::Database {
        /// Asynchronously fetches the inner state of the current object by fetching the password using the specified HashiCorpVault client. 
    /// Returns a Result containing the updated object with the fetched password or an error if the fetching process fails.
    async fn fetch_inner<En>(
        mut self,
        client: &HashiCorpVault,
    ) -> error_stack::Result<Self, HashiCorpError>
    where
        for<'a> En: Engine<
                ReturnType<'a, String> = std::pin::Pin<
                    Box<
                        dyn std::future::Future<
                                Output = error_stack::Result<String, HashiCorpError>,
                            > + Send
                            + 'a,
                    >,
                >,
            > + 'a,
    {
        Ok(Self {
            host: self.host,
            port: self.port,
            dbname: self.dbname,
            username: self.username,
            password: self.password.fetch_inner::<En>(client).await?,
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
impl VaultFetch for settings::PayPalOnboarding {
        /// Asynchronously fetches the inner value for the client ID, client secret, and partner ID using the provided HashiCorpVault client. 
    /// 
    /// # Arguments
    /// 
    /// * `client` - A reference to the HashiCorpVault client
    /// 
    /// # Returns
    /// 
    /// The updated Self after fetching the inner values for client ID, client secret, and partner ID, or an error of type HashiCorpError wrapped in a Result.
    async fn fetch_inner<En>(
        mut self,
        client: &HashiCorpVault,
    ) -> error_stack::Result<Self, HashiCorpError>
    where
        for<'a> En: Engine<
                ReturnType<'a, String> = std::pin::Pin<
                    Box<
                        dyn std::future::Future<
                                Output = error_stack::Result<String, HashiCorpError>,
                            > + Send
                            + 'a,
                    >,
                >,
            > + 'a,
    {
        self.client_id = self.client_id.fetch_inner::<En>(client).await?;
        self.client_secret = self.client_secret.fetch_inner::<En>(client).await?;
        self.partner_id = self.partner_id.fetch_inner::<En>(client).await?;
        Ok(self)
    }
}

#[cfg(feature = "olap")]
#[async_trait::async_trait]
impl VaultFetch for settings::ConnectorOnboarding {
        /// Asynchronously fetches inner data using the specified client and updates the current instance with the fetched data.
    async fn fetch_inner<En>(
        mut self,
        client: &HashiCorpVault,
    ) -> error_stack::Result<Self, HashiCorpError>
    where
        for<'a> En: Engine<
                ReturnType<'a, String> = std::pin::Pin<
                    Box<
                        dyn std::future::Future<
                                Output = error_stack::Result<String, HashiCorpError>,
                            > + Send
                            + 'a,
                    >,
                >,
            > + 'a,
    {
        self.paypal = self.paypal.fetch_inner::<En>(client).await?;
        Ok(self)
    }
}
