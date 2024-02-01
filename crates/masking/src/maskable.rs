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
        /// Formats the value based on the enum variant.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Masked(secret_value) => std::fmt::Debug::fmt(secret_value, f),
            Self::Normal(value) => std::fmt::Debug::fmt(value, f),
        }
    }
}

impl<T: Eq + PartialEq + Clone + std::hash::Hash> std::hash::Hash for Maskable<T> {
        /// Hashes the value using the provided hasher `H`. If the value is a `Masked`, it uses the `peek` method from `PeekInterface` to hash the inner value. If the value is a `Normal`, it directly hashes the value using the provided hasher `H`.
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
        /// Converts the value into a `Maskable` instance with the value wrapped in a `Masked` enum variant.
    fn into_masked(self) -> Maskable<Self::Output> {
        Maskable::new_masked(self.into())
    }
}

impl Mask for Secret<String> {
    type Output = String;
        /// Converts the value into a Maskable type, which can be used to perform masked operations on the value.
    fn into_masked(self) -> Maskable<Self::Output> {
        Maskable::new_masked(self)
    }
}

impl<T: Eq + PartialEq + Clone> From<T> for Maskable<T> {
        /// Creates a new instance of Self using the provided value. This method is a convenience
    /// wrapper around the new_normal method, allowing for a more concise syntax when creating
    /// instances of Self with a single value.
    fn from(value: T) -> Self {
        Self::new_normal(value)
    }
}

impl From<&str> for Maskable<String> {
        /// Creates a new instance of Self from the given string value.
    fn from(value: &str) -> Self {
        Self::new_normal(value.to_string())
    }
}
