//! Domain status types for connector-response deserialization resilience.
//!
//! Connectors occasionally return status values we do not yet model (a new
//! processor state, a typo, a backward-incompatible API change). Mapping such a
//! value straight onto a storage enum (e.g. [`crate::AttemptStatus`]) hard-fails
//! deserialization and drops the payment.
//!
//! To make this resilient we introduce a parallel *domain* status type for each
//! storage status enum. The domain type mirrors every storage variant and adds a
//! single `Unknown` catch-all (`#[serde(other)]`) so unrecognised values
//! deserialize gracefully instead of erroring.
//!
//! Contract:
//! - `Unknown` is **internal-only**. It is never persisted to the database and
//!   never surfaced to merchants — the storage enums deliberately do *not* gain
//!   an `Unknown` variant.
//! - Before a domain status is converted back to its storage representation, an
//!   `Unknown` must be resolved to the previously known state via
//!   [`resolve_or_keep`](AttemptStatusDomain::resolve_or_keep).
//! - If an `Unknown` ever reaches the `Domain -> storage` conversion it means the
//!   previous-state resolution was skipped or is buggy, so the conversion returns
//!   [`UnknownStatusError`] instead of silently corrupting state.

/// Error returned when a domain status that is still `Unknown` is converted to
/// its storage representation. Surfacing this (rather than defaulting) makes a
/// missing previous-state resolution loud and testable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "domain status `{domain}` was still `Unknown` at storage conversion; \
     previous-state resolution did not run for this connector response"
)]
pub struct UnknownStatusError {
    /// Name of the domain status type that failed to convert.
    pub domain: &'static str,
}

impl UnknownStatusError {
    /// Construct the error for the named domain status type.
    #[must_use]
    pub const fn new(domain: &'static str) -> Self {
        Self { domain }
    }
}

/// Generate a domain status enum that mirrors a storage status enum and adds an
/// internal-only `Unknown` catch-all.
///
/// Generated for `$domain` mirroring `$storage`:
/// - the enum itself, with `#[serde(other)] Unknown`;
/// - `From<$storage> for $domain` (infallible — every storage state is known);
/// - `TryFrom<$domain> for $storage` (errors only on `Unknown`);
/// - `resolve_or_keep`, `is_unknown`, and `to_storage` helpers.
#[macro_export]
macro_rules! domain_status_enum {
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $domain:ident mirrors $storage:path {
            $($variant:ident),+ $(,)?
        }
    ) => {
        $(#[$enum_meta])*
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(rename_all = "snake_case")]
        $vis enum $domain {
            $($variant,)+
            /// Connector returned a status we do not model. Internal-only:
            /// must be resolved to the previous state before persistence and is
            /// never written to storage nor sent to merchants.
            #[serde(other)]
            Unknown,
        }

        impl ::core::convert::From<$storage> for $domain {
            fn from(value: $storage) -> Self {
                match value {
                    $(<$storage>::$variant => Self::$variant,)+
                }
            }
        }

        impl ::core::convert::TryFrom<$domain> for $storage {
            type Error = $crate::domain_status::UnknownStatusError;

            fn try_from(value: $domain) -> ::core::result::Result<Self, Self::Error> {
                match value {
                    $(<$domain>::$variant => Ok(Self::$variant),)+
                    <$domain>::Unknown => Err(
                        $crate::domain_status::UnknownStatusError::new(stringify!($domain))
                    ),
                }
            }
        }

        impl $domain {
            /// Replace an `Unknown` status with the previously known storage
            /// state; known statuses pass through unchanged. This is the merge
            /// of connector response and previous state.
            #[must_use]
            pub fn resolve_or_keep(self, previous: $storage) -> Self {
                match self {
                    Self::Unknown => Self::from(previous),
                    known => known,
                }
            }

            /// `true` when the connector status could not be recognised.
            #[must_use]
            pub fn is_unknown(self) -> bool {
                matches!(self, Self::Unknown)
            }

            /// Convert to the storage representation, erroring if still `Unknown`.
            pub fn to_storage(
                self,
            ) -> ::core::result::Result<$storage, $crate::domain_status::UnknownStatusError> {
                <$storage as ::core::convert::TryFrom<$domain>>::try_from(self)
            }
        }
    };
}

domain_status_enum! {
    /// Domain mirror of [`crate::AttemptStatus`] with an internal-only `Unknown`.
    pub enum AttemptStatusDomain mirrors crate::AttemptStatus {
        Started,
        AuthenticationFailed,
        RouterDeclined,
        AuthenticationPending,
        AuthenticationSuccessful,
        Authorized,
        AuthorizationFailed,
        Charged,
        Authorizing,
        CodInitiated,
        Voided,
        VoidedPostCharge,
        VoidInitiated,
        CaptureInitiated,
        CaptureFailed,
        VoidFailed,
        AutoRefunded,
        PartialCharged,
        PartiallyAuthorized,
        PartialChargedAndChargeable,
        Unresolved,
        Pending,
        Failure,
        PaymentMethodAwaited,
        ConfirmationAwaited,
        DeviceDataCollectionPending,
        IntegrityFailure,
        Expired,
        CaptureReview,
    }
}

domain_status_enum! {
    /// Domain mirror of [`crate::RefundStatus`] with an internal-only `Unknown`.
    pub enum RefundStatusDomain mirrors crate::RefundStatus {
        Failure,
        ManualReview,
        Pending,
        Success,
        TransactionFailure,
    }
}

domain_status_enum! {
    /// Domain mirror of [`crate::DisputeStatus`] with an internal-only `Unknown`.
    pub enum DisputeStatusDomain mirrors crate::DisputeStatus {
        DisputeOpened,
        DisputeExpired,
        DisputeAccepted,
        DisputeCancelled,
        DisputeChallenged,
        DisputeWon,
        DisputeLost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AttemptStatus, DisputeStatus, RefundStatus};

    // ---- Tier 1: graceful deserialization (the core incident fix) ----

    #[test]
    fn unrecognised_status_deserializes_to_unknown_not_error() {
        let parsed: AttemptStatusDomain =
            serde_json::from_str("\"some_brand_new_state\"").expect("must not hard-fail");
        assert_eq!(parsed, AttemptStatusDomain::Unknown);
    }

    #[test]
    fn known_status_deserializes_to_its_variant() {
        let parsed: AttemptStatusDomain =
            serde_json::from_str("\"charged\"").expect("known status parses");
        assert_eq!(parsed, AttemptStatusDomain::Charged);
    }

    // ---- Tier 2: storage conversion guard + previous-state resolution ----

    #[test]
    fn unknown_to_storage_errors_loudly() {
        // This is the bug guard: an Unknown that was never resolved must NOT be
        // silently coerced into a storage state.
        let err = AttemptStatus::try_from(AttemptStatusDomain::Unknown).unwrap_err();
        assert_eq!(err.domain, "AttemptStatusDomain");
    }

    #[test]
    fn known_domain_converts_to_matching_storage() {
        assert_eq!(
            AttemptStatus::try_from(AttemptStatusDomain::Authorized).unwrap(),
            AttemptStatus::Authorized
        );
        assert_eq!(
            AttemptStatus::try_from(AttemptStatusDomain::Failure).unwrap(),
            AttemptStatus::Failure
        );
    }

    #[test]
    fn storage_to_domain_roundtrips_for_known() {
        for storage in [
            AttemptStatus::Pending,
            AttemptStatus::Charged,
            AttemptStatus::Voided,
            AttemptStatus::CaptureReview,
        ] {
            let domain = AttemptStatusDomain::from(storage);
            assert!(!domain.is_unknown());
            assert_eq!(AttemptStatus::try_from(domain).unwrap(), storage);
        }
    }

    #[test]
    fn resolve_or_keep_replaces_unknown_with_previous_state() {
        // Simulates: connector returned Unknown, Hyperswitch already held the
        // pre-call state (Pending) and merges it back in -> no error downstream.
        let resolved = AttemptStatusDomain::Unknown.resolve_or_keep(AttemptStatus::Pending);
        assert_eq!(resolved, AttemptStatusDomain::Pending);
        assert_eq!(resolved.to_storage().unwrap(), AttemptStatus::Pending);
    }

    #[test]
    fn resolve_or_keep_preserves_known_status() {
        let resolved = AttemptStatusDomain::Charged.resolve_or_keep(AttemptStatus::Pending);
        assert_eq!(resolved, AttemptStatusDomain::Charged);
        assert_eq!(resolved.to_storage().unwrap(), AttemptStatus::Charged);
    }

    // ---- Same contract holds for refund + dispute domain mirrors ----

    #[test]
    fn refund_domain_contract() {
        let unknown: RefundStatusDomain = serde_json::from_str("\"weird_refund_state\"").unwrap();
        assert_eq!(unknown, RefundStatusDomain::Unknown);
        assert!(RefundStatus::try_from(unknown).is_err());

        let resolved = RefundStatusDomain::Unknown.resolve_or_keep(RefundStatus::Pending);
        assert_eq!(resolved.to_storage().unwrap(), RefundStatus::Pending);

        assert_eq!(
            RefundStatus::try_from(RefundStatusDomain::Success).unwrap(),
            RefundStatus::Success
        );
    }

    #[test]
    fn dispute_domain_contract() {
        let unknown: DisputeStatusDomain = serde_json::from_str("\"weird_dispute_state\"").unwrap();
        assert_eq!(unknown, DisputeStatusDomain::Unknown);
        assert!(DisputeStatus::try_from(unknown).is_err());

        let resolved = DisputeStatusDomain::Unknown.resolve_or_keep(DisputeStatus::DisputeOpened);
        assert_eq!(resolved.to_storage().unwrap(), DisputeStatus::DisputeOpened);

        assert_eq!(
            DisputeStatus::try_from(DisputeStatusDomain::DisputeWon).unwrap(),
            DisputeStatus::DisputeWon
        );
    }
}
