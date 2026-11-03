//! ABSA.
//!
//! `resultDetails` composites, per the ABSA integration sheet:
//!
//! | Field        | Holds                                          |
//! |--------------|------------------------------------------------|
//! | `TxID1[0]`   | ACI client terminal sequence number (P-37)     |
//! | `TxID1[1]`   | Draft capture flag                             |
//! | `TxID1[2]`   | Terminal owner ID                              |
//! | `TxID1[3]`   | Card issuer category response code data       |
//! | `TxID1[4]`   | Token OH                                       |
//! | `TxID1[5]`   | Original ID                                    |
//! | `TxID2[0]`   | `merchantTransactionId` / invoice ID           |
//! | `TxID2[1]`   | Action code (P-39)                             |
//! | `TxID2[2]`   | Approval code (P-38) — not the RRN             |
//! | `TxID2[3..]` | Various scheme tokens                          |
//! | `TxID3[0]`   | STAN (P-11)                                    |
//! | `TxID3[1]`   | Token 17                                       |
//! | `TxID3[2]`   | Token 20 (scheme trace ID)                     |
//!
//! ABSA does not expose an RRN inside the composites. For card-on-file the CITI is the scheme
//! trace ID (Token 20), surfaced as `resultDetails.SchemeTransactionId`.

use super::{AciResponseResultDetails, Acquirer, AcquirerReferences};

pub(super) struct Absa;

impl Acquirer for Absa {
    const NAME_MARKERS: &'static [&'static str] = &["absa"];

    /// A "PIM" tag in `ConnectorTxID1` or `ConnectorTxID2`, which marks the ACI sandbox's ABSA
    /// simulator format.
    fn has_response_shape(details: &AciResponseResultDetails) -> bool {
        details.connector_tx_id1().contains("PIM") || details.connector_tx_id2().contains("PIM")
    }

    fn references(details: &AciResponseResultDetails) -> AcquirerReferences {
        let tx_id3 = details.connector_tx_id3();
        AcquirerReferences {
            rrn: None,
            citi: details
                .scheme_transaction_id
                .clone()
                .or_else(|| tx_id3.field(2))
                .or_else(|| details.network_reference()),
            stan: tx_id3.field(0),
            auth_code: details
                .authorization_code()
                .or_else(|| details.connector_tx_id2().field(2)),
            original_transaction_id: details.connector_tx_id1().field(5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Token 20 is mapped to `SchemeTransactionId` by the sheet and also sits at
    /// `ConnectorTxID3[2]`, which is the fallback.
    #[test]
    fn citi_prefers_scheme_transaction_id_then_token_20() {
        let with_trace = AciResponseResultDetails {
            scheme_transaction_id: Some("016153570198200".to_string()),
            connector_tx_id3: Some("001234|17TOKEN|020TRACE".to_string()),
            ..Default::default()
        };
        assert_eq!(
            Absa::references(&with_trace).citi.as_deref(),
            Some("016153570198200")
        );

        let without_trace = AciResponseResultDetails {
            scheme_transaction_id: None,
            ..with_trace
        };
        let references = Absa::references(&without_trace);
        assert_eq!(references.citi.as_deref(), Some("020TRACE"));
        assert_eq!(
            references.stan.as_deref(),
            Some("001234"),
            "STAN (P-11) is ConnectorTxID3[0]"
        );
    }

    #[test]
    fn approval_code_and_original_id_come_from_their_composites() {
        let details = AciResponseResultDetails {
            connector_tx_id1: Some("37|Y|OWNER|CAT|OH|ORIGINAL01".to_string()),
            connector_tx_id2: Some("INV1|00|654321".to_string()),
            ..Default::default()
        };
        let references = Absa::references(&details);
        assert_eq!(references.auth_code.as_deref(), Some("654321"));
        assert_eq!(
            references.original_transaction_id.as_deref(),
            Some("ORIGINAL01")
        );
        assert_eq!(references.rrn, None);
    }
}
