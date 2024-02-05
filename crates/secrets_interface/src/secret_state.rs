//! Module to manage encrypted and decrypted states for a given type.

use std::marker::PhantomData;

use serde::{Deserialize, Deserializer};

/// Trait defining the states of a secret
pub trait SecretState {}

/// Decrypted state
#[derive(Debug, Clone, Deserialize)]
pub enum RawSecret {}

/// Encrypted state
#[derive(Debug, Clone, Deserialize)]
pub enum SecuredSecret {}

impl SecretState for RawSecret {}
impl SecretState for SecuredSecret {}

/// Struct for managing the encrypted and decrypted states of a given type
#[derive(Debug, Clone, Default)]
pub struct SecretStateContainer<T, S: SecretState> {
    inner: T,
    marker: PhantomData<S>,
}

impl<T: Clone, S: SecretState> SecretStateContainer<T, S> {
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

impl<'de, T: Deserialize<'de>, S: SecretState> Deserialize<'de> for SecretStateContainer<T, S> {
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

impl<T> SecretStateContainer<T, SecuredSecret> {
    /// Transition the secret state from `SecuredSecret` to `RawSecret`
    pub fn transition_state(
        mut self,
        decryptor_fn: impl FnOnce(T) -> T,
    ) -> SecretStateContainer<T, RawSecret> {
        self.inner = decryptor_fn(self.inner);
        SecretStateContainer {
            inner: self.inner,
            marker: PhantomData,
        }
    }
}
