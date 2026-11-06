//! Planet Payment, a multi-currency pricing platform.
//!
//! Its merchant account configuration is centred on User Id / Password / Terminal Id, and its
//! `ConnectorTxID` composite layout is undocumented, so it gets the same conservative top-level
//! reading as MPGS and CyberSource.

use super::{AciResponseResultDetails, Acquirer, AcquirerReferences};

pub(super) struct Planet;

impl Acquirer for Planet {
    const NAME_MARKERS: &'static [&'static str] = &["planet"];

    fn references(details: &AciResponseResultDetails) -> AcquirerReferences {
        AcquirerReferences::top_level(details)
    }
}
