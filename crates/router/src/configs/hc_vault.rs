use external_services::hashicorp_vault::{
    core::{Engine, HashiCorpError, HashiCorpVault},
    decrypt::VaultFetch,
};
use masking::ExposeInterface;

use crate::configs::settings;

#[async_trait::async_trait]
impl VaultFetch for settings::Jwekey {
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
