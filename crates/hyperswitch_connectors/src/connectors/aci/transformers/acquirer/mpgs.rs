//! MPGS (Mastercard Payment Gateway Services).
//!
//! MPGS reports its references as top-level `resultDetails` fields rather than inside the
//! `ConnectorTxID` composites, and its sheet documents no Banknet packing, so trace ids are
//! replayed as received.
//!
//! Known limitation: MPGS is reported to reject zero-amount transactions, which is how mandate
//! setup verifies a card. An MPGS entity that needs card-on-file setup will need a non-zero
//! auth-and-reverse here instead; left unimplemented until an MPGS entity actually fails.

use super::{AciResponseResultDetails, Acquirer, AcquirerReferences};

pub(super) struct Mpgs;

impl Acquirer for Mpgs {
    const NAME_MARKERS: &'static [&'static str] = &["mpgs", "mastercard payment gateway"];

    fn references(details: &AciResponseResultDetails) -> AcquirerReferences {
        AcquirerReferences::top_level(details)
    }
}
