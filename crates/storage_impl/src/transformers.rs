//! Trait definitions and implementations for converting between storage models and domain models
//!
//! This module provides ForeignFrom and ForeignTryFrom traits that allow conversions
//! between diesel_models and hyperswitch_domain_models types.

use common_utils::{
    errors::{CustomResult, ValidationError},
    types::keymanager::{Identifier, KeyManagerState},
};
use error_stack::ResultExt;
use hyperswitch_masking::Secret;

/// Trait for converting from a foreign type
///
/// This trait allows implementing conversions for types that are defined in other crates,
/// bypassing Rust's orphan rules which prevent implementing foreign traits for foreign types.
pub trait ForeignFrom<F> {
    fn foreign_from(from: F) -> Self;
}

/// Trait for fallible conversion from a foreign type
///
/// Similar to ForeignFrom, but allows for fallible conversions.
pub trait ForeignTryFrom<F>: Sized {
    type Error;
    fn foreign_try_from(from: F) -> error_stack::Result<Self, Self::Error>;
}

/// Trait for async conversion from a foreign type with decryption
///
/// This trait is specifically designed for converting storage models (from diesel_models)
/// to domain models (from hyperswitch_domain_models) with decryption.
#[async_trait::async_trait]
pub trait AsyncForeignTryFrom<F>: Sized {
    type Error;
    async fn async_foreign_try_from(
        from: F,
        state: &KeyManagerState,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError>;
}

/// Extension trait for ForeignFrom
pub trait ForeignInto<T> {
    fn foreign_into(self) -> T;
}

impl<F, T> ForeignInto<T> for F
where
    T: ForeignFrom<F>,
{
    fn foreign_into(self) -> T {
        T::foreign_from(self)
    }
}

impl<F, T> ForeignFrom<Option<F>> for Option<T>
where
    T: ForeignFrom<F>,
{
    fn foreign_from(from: Option<F>) -> Self {
        from.map(|v| T::foreign_from(v))
    }
}

/// Extension trait for ForeignTryFrom
pub trait ForeignTryInto<T> {
    type Error;
    fn foreign_try_into(self) -> error_stack::Result<T, Self::Error>;
}

impl<F, T> ForeignTryInto<T> for F
where
    T: ForeignTryFrom<F>,
{
    type Error = <T as ForeignTryFrom<F>>::Error;
    fn foreign_try_into(self) -> error_stack::Result<T, Self::Error> {
        T::foreign_try_from(self)
    }
}
