use std::{future::Future, pin::Pin};

use masking::ExposeInterface;

/// A trait for types that can be asynchronously fetched and decrypted from HashiCorp Vault.
#[async_trait::async_trait]
pub trait VaultFetch: Sized {
    /// Asynchronously decrypts the inner content of the type.
    ///
    /// # Returns
    ///
    /// An `Result<Self, super::HashiCorpError>` representing the decrypted instance if successful,
    /// or an `super::HashiCorpError` with details about the encountered error.
    ///
    async fn fetch_inner<En>(
        self,
        client: &super::HashiCorpVault,
    ) -> error_stack::Result<Self, super::HashiCorpError>
    where
        for<'a> En: super::Engine<
                ReturnType<'a, String> = Pin<
                    Box<
                        dyn Future<Output = error_stack::Result<String, super::HashiCorpError>>
                            + Send
                            + 'a,
                    >,
                >,
            > + 'a;
}

#[async_trait::async_trait]
impl VaultFetch for masking::Secret<String> {
        /// Asynchronously fetches data from the specified client using the provided engine, and returns the result
    async fn fetch_inner<En>(
        self,
        client: &super::HashiCorpVault,
    ) -> error_stack::Result<Self, super::HashiCorpError>
    where
        for<'a> En: super::Engine<
                ReturnType<'a, String> = Pin<
                    Box<
                        dyn Future<Output = error_stack::Result<String, super::HashiCorpError>>
                            + Send
                            + 'a,
                    >,
                >,
            > + 'a,
    {
        client.fetch::<En, Self>(self.expose()).await
    }
}
