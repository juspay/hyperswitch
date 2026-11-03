//! An acquirer nobody declared and no heuristic recognised.
//!
//! Reads the composites with the best-effort heuristics that predate per-acquirer parsing, so
//! acquirers that have never been explicitly characterised do not regress.

use super::{AciResponseResultDetails, Acquirer, AcquirerReferences};

pub(super) struct Unknown;

impl Acquirer for Unknown {
    const NAME_MARKERS: &'static [&'static str] = &[];

    fn references(details: &AciResponseResultDetails) -> AcquirerReferences {
        let tx_id2 = details.connector_tx_id2();
        let tx_id3 = details.connector_tx_id3();
        AcquirerReferences {
            rrn: tx_id2.field(2),
            citi: tx_id3
                .field(4)
                .or_else(|| details.scheme_transaction_id.clone())
                .or_else(|| details.network_reference()),
            stan: tx_id3.field(0).or_else(|| tx_id2.field(1)),
            auth_code: details.authorization_code(),
            original_transaction_id: details.connector_tx_id1().field(5),
        }
    }
}
