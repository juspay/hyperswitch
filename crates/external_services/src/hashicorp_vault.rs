//! Interactions with the HashiCorp Vault

use std::{collections::HashMap, future::Future, pin::Pin};

use error_stack::{Report, ResultExt};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

/// Utilities for supporting decryption of data
pub mod decrypt;

static HC_CLIENT: tokio::sync::OnceCell<HashiCorpVault> = tokio::sync::OnceCell::const_new();

#[allow(missing_debug_implementations)]
/// A struct representing a connection to HashiCorp Vault.
pub struct HashiCorpVault {
    /// The underlying client used for interacting with HashiCorp Vault.
    client: VaultClient,
}

/// Configuration for connecting to HashiCorp Vault.
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct HashiCorpVaultConfig {
    /// The URL of the HashiCorp Vault server.
    pub url: String,
    /// The authentication token used to access HashiCorp Vault.
    pub token: String,
}

/// Asynchronously retrieves a HashiCorp Vault client based on the provided configuration.
///
/// # Parameters
///
/// - `config`: A reference to a `HashiCorpVaultConfig` containing the configuration details.
pub async fn get_hashicorp_client(
    config: &HashiCorpVaultConfig,
) -> error_stack::Result<&'static HashiCorpVault, HashiCorpError> {
    HC_CLIENT
        .get_or_try_init(|| async { HashiCorpVault::new(config) })
        .await
}

/// A trait defining an engine for interacting with HashiCorp Vault.
pub trait Engine: Sized {
    /// The associated type representing the return type of the engine's operations.
    type ReturnType<'b, T>
    where
        T: 'b,
        Self: 'b;
    /// Reads data from HashiCorp Vault at the specified location.
    ///
    /// # Parameters
    ///
    /// - `client`: A reference to the HashiCorpVault client.
    /// - `location`: The location in HashiCorp Vault to read data from.
    ///
    /// # Returns
    ///
    /// A future representing the result of the read operation.
    fn read(client: &HashiCorpVault, location: String) -> Self::ReturnType<'_, String>;
}

/// An implementation of the `Engine` trait for the Key-Value version 2 (Kv2) engine.
#[derive(Debug)]
pub enum Kv2 {}

impl Engine for Kv2 {
    type ReturnType<'b, T: 'b> =
        Pin<Box<dyn Future<Output = error_stack::Result<T, HashiCorpError>> + Send + 'b>>;
        /// Reads a value from a specified location in HashiCorp Vault using the provided client.
    /// 
    /// # Arguments
    /// 
    /// * `client` - A reference to a HashiCorpVault client
    /// * `location` - A String representing the location in the vault from which to read the value
    /// 
    /// # Returns
    /// 
    /// Returns a Future that resolves to the value read from the specified location in the vault.
    fn read(client: &HashiCorpVault, location: String) -> Self::ReturnType<'_, String> {
        Box::pin(async move {
            let mut split = location.split(':');
            let mount = split.next().ok_or(HashiCorpError::IncompleteData)?;
            let path = split.next().ok_or(HashiCorpError::IncompleteData)?;
            let key = split.next().unwrap_or("value");

            let mut output =
                vaultrs::kv2::read::<HashMap<String, String>>(&client.client, mount, path)
                    .await
                    .map_err(Into::<Report<_>>::into)
                    .change_context(HashiCorpError::FetchFailed)?;

            Ok(output.remove(key).ok_or(HashiCorpError::ParseError)?)
        })
    }
}

impl HashiCorpVault {
    /// Creates a new instance of HashiCorpVault based on the provided configuration.
    ///
    /// # Parameters
    ///
    /// - `config`: A reference to a `HashiCorpVaultConfig` containing the configuration details.
    ///
    pub fn new(config: &HashiCorpVaultConfig) -> error_stack::Result<Self, HashiCorpError> {
        VaultClient::new(
            VaultClientSettingsBuilder::default()
                .address(&config.url)
                .token(&config.token)
                .build()
                .map_err(Into::<Report<_>>::into)
                .change_context(HashiCorpError::ClientCreationFailed)
                .attach_printable("Failed while building vault settings")?,
        )
        .map_err(Into::<Report<_>>::into)
        .change_context(HashiCorpError::ClientCreationFailed)
        .map(|client| Self { client })
    }

    /// Asynchronously fetches data from HashiCorp Vault using the specified engine.
    ///
    /// # Parameters
    ///
    /// - `data`: A String representing the location or identifier of the data in HashiCorp Vault.
    ///
    /// # Type Parameters
    ///
    /// - `En`: The engine type that implements the `Engine` trait.
    /// - `I`: The type that can be constructed from the retrieved encoded data.
    ///
    pub async fn fetch<En, I>(&self, data: String) -> error_stack::Result<I, HashiCorpError>
    where
        for<'a> En: Engine<
                ReturnType<'a, String> = Pin<
                    Box<
                        dyn Future<Output = error_stack::Result<String, HashiCorpError>>
                            + Send
                            + 'a,
                    >,
                >,
            > + 'a,
        I: FromEncoded,
    {
        let output = En::read(self, data).await?;
        I::from_encoded(output).ok_or(error_stack::report!(HashiCorpError::HexDecodingFailed))
    }
}

/// A trait for types that can be constructed from encoded data in the form of a String.
pub trait FromEncoded: Sized {
    /// Constructs an instance of the type from the provided encoded input.
    ///
    /// # Parameters
    ///
    /// - `input`: A String containing the encoded data.
    ///
    /// # Returns
    ///
    /// An `Option<Self>` representing the constructed instance if successful, or `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use your_module::{FromEncoded, masking::Secret, Vec};
    /// let secret_instance = Secret::<String>::from_encoded("encoded_secret_string".to_string());
    /// let vec_instance = Vec::<u8>::from_encoded("68656c6c6f".to_string());
    /// ```
    fn from_encoded(input: String) -> Option<Self>;
}

impl FromEncoded for masking::Secret<String> {
        /// Converts a string into an instance of the current type, if possible.
    ///
    /// # Arguments
    ///
    /// * `input` - A string to be converted into an instance of the current type
    ///
    /// # Returns
    ///
    /// An `Option` containing the instance of the current type if conversion is successful, or `None` if the conversion fails
    ///
    fn from_encoded(input: String) -> Option<Self> {
        Some(input.into())
    }
}

impl FromEncoded for Vec<u8> {
        /// Decodes the input string from hexadecimal encoding and returns an Option containing the result.
    fn from_encoded(input: String) -> Option<Self> {
        hex::decode(input).ok()
    }
}

/// An enumeration representing various errors that can occur in interactions with HashiCorp Vault.
#[derive(Debug, thiserror::Error)]
pub enum HashiCorpError {
    /// Failed while creating hashicorp client
    #[error("Failed while creating a new client")]
    ClientCreationFailed,

    /// Failed while building configurations for hashicorp client
    #[error("Failed while building configuration")]
    ConfigurationBuildFailed,

    /// Failed while decoding data to hex format
    #[error("Failed while decoding hex data")]
    HexDecodingFailed,

    /// An error occurred when base64 decoding input data.
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,

    /// An error occurred when KMS decrypting input data.
    #[error("Failed to KMS decrypt input data")]
    DecryptionFailed,

    /// The KMS decrypted output does not include a plaintext output.
    #[error("Missing plaintext KMS decryption output")]
    MissingPlaintextDecryptionOutput,

    /// An error occurred UTF-8 decoding KMS decrypted output.
    #[error("Failed to UTF-8 decode decryption output")]
    Utf8DecodingFailed,

    /// Incomplete data provided to fetch data from hasicorp
    #[error("Provided information about the value is incomplete")]
    IncompleteData,

    /// Failed while fetching data from vault
    #[error("Failed while fetching data from the server")]
    FetchFailed,

    /// Failed while parsing received data
    #[error("Failed while parsing the response")]
    ParseError,
}
