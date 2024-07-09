//!
//! This module contains Masking objects and traits
//!

use crate::{ExposeInterface, Secret};

///
/// An Enum that allows us to optionally mask data, based on which enum variant that data is stored
/// in.
///
#[derive(Clone, Eq, PartialEq)]
pub enum Maskable<T: Eq + PartialEq + Clone> {
    /// Variant which masks the data by wrapping in a Secret
    Masked(Secret<T>),
    /// Varant which doesn't mask the data
    Normal(T),
}

impl<T: std::fmt::Debug + Clone + Eq + PartialEq> std::fmt::Debug for Maskable<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Masked(secret_value) => std::fmt::Debug::fmt(secret_value, f),
            Self::Normal(value) => std::fmt::Debug::fmt(value, f),
        }
    }
}

impl<T: Eq + PartialEq + Clone + std::hash::Hash> std::hash::Hash for Maskable<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Masked(value) => crate::PeekInterface::peek(value).hash(state),
            Self::Normal(value) => value.hash(state),
        }
    }
}

impl<T: Eq + PartialEq + Clone> Maskable<T> {
    ///
    /// Get the inner data while consuming self
    ///
    pub fn into_inner(self) -> T {
        match self {
            Self::Masked(inner_secret) => inner_secret.expose(),
            Self::Normal(inner) => inner,
        }
    }

    ///
    /// Create a new Masked data
    ///
    pub fn new_masked(item: Secret<T>) -> Self {
        Self::Masked(item)
    }

    ///
    /// Create a new non-masked data
    ///
    pub fn new_normal(item: T) -> Self {
        Self::Normal(item)
    }

    ///
    /// Checks whether the data is masked.
    /// Returns `true` if the data is wrapped in the `Masked` variant,
    /// returns `false` otherwise.
    ///
    pub fn is_masked(&self) -> bool {
        matches!(self, Self::Masked(_))
    }

    ///
    /// Checks whether the data is normal (not masked).
    /// Returns `true` if the data is wrapped in the `Normal` variant,
    /// returns `false` otherwise.
    ///
    pub fn is_normal(&self) -> bool {
        matches!(self, Self::Normal(_))
    }
}

/// Trait for providing a method on custom types for constructing `Maskable`

pub trait Mask {
    /// The type returned by the `into_masked()` method. Must implement `PartialEq`, `Eq` and `Clone`

    type Output: Eq + Clone + PartialEq;

    ///
    /// Construct a `Maskable` instance that wraps `Self::Output` by consuming `self`
    ///
    fn into_masked(self) -> Maskable<Self::Output>;
}

impl Mask for String {
    type Output = Self;
    fn into_masked(self) -> Maskable<Self::Output> {
        Maskable::new_masked(self.into())
    }
}

impl Mask for Secret<String> {
    type Output = String;
    fn into_masked(self) -> Maskable<Self::Output> {
        Maskable::new_masked(self)
    }
}

impl<T: Eq + PartialEq + Clone> From<T> for Maskable<T> {
    fn from(value: T) -> Self {
        Self::new_normal(value)
    }
}

impl From<&str> for Maskable<String> {
    fn from(value: &str) -> Self {
        Self::new_normal(value.to_string())
    }
}
