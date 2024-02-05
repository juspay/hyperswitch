use std::marker::PhantomData;

use serde::{Deserialize, Deserializer};

/// Trait defining encryption states
pub trait EncryptionState {}

/// Decrypted state
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Decrypted {}

/// Encrypted state
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Encrypted {}

impl EncryptionState for Decrypted {}
impl EncryptionState for Encrypted {}

/// Struct for managing the encrypted and decrypted states of a given type
#[derive(Debug, Clone, Default)]
pub struct Decryptable<T, S: EncryptionState> {
    inner: T,
    marker: PhantomData<S>,
}

impl<T: Clone, S: EncryptionState> Decryptable<T, S> {
    ///
    /// Get the inner data while consuming self
    ///
    #[inline]
    pub fn into_inner(self) -> T {
        self.inner
    }

    ///
    /// Get the reference to inner value
    ///
    #[inline]
    pub fn get_inner(&self) -> &T {
        &self.inner
    }
}

impl<'de, T: Deserialize<'de>, S: EncryptionState> Deserialize<'de> for Decryptable<T, S> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let val = Deserialize::deserialize(deserializer)?;
        Ok(Self {
            inner: val,
            marker: PhantomData,
        })
    }
}

impl<T> Decryptable<T, Encrypted> {
    /// Decrypts the inner value using the provided decryption function
    pub fn decrypt(mut self, decryptor_fn: impl FnOnce(T) -> T) -> Decryptable<T, Decrypted> {
        self.inner = decryptor_fn(self.inner);
        Decryptable {
            inner: self.inner,
            marker: PhantomData,
        }
    }
}
