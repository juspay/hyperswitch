use external_services::hashicorp_vault::{
    decrypt::VaultFetch, Engine, HashiCorpError, HashiCorpVault,
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
            self.locker_encryption_key1,
            self.locker_encryption_key2,
            self.locker_decryption_key1,
            self.locker_decryption_key2,
            self.vault_encryption_key,
            self.rust_locker_encryption_key,
            self.vault_private_key,
            self.tunnel_private_key,
        ) = (
            masking::Secret::new(self.locker_encryption_key1)
                .fetch_inner::<En>(client)
                .await?
                .expose(),
            masking::Secret::new(self.locker_encryption_key2)
                .fetch_inner::<En>(client)
                .await?
                .expose(),
            masking::Secret::new(self.locker_decryption_key1)
                .fetch_inner::<En>(client)
                .await?
                .expose(),
            masking::Secret::new(self.locker_decryption_key2)
                .fetch_inner::<En>(client)
                .await?
                .expose(),
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
