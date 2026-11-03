//! Nedbank, via Postilion.
//!
//! `resultDetails` composites, per the Postilion integration sheet:
//!
//! | Field          | Holds                                          |
//! |----------------|------------------------------------------------|
//! | `TxID1[0]`     | Terminal ID                                    |
//! | `TxID2[0]`     | STAN                                           |
//! | `TxID2[1]`     | RRN                                            |
//! | `TxID3[0]`     | Authorization code                             |
//! | `TxID3[1]`     | Response code                                  |
//! | `TxID3[2..=3]` | Initial transaction date/time                  |
//! | `TxID3[4]`     | CITI (`CardholderInitiatedTransactionID`)      |
//! | `TxID3[5]`     | Merchant ID                                    |

use super::{AciResponseResultDetails, Acquirer, AcquirerReferences, BanknetPacking};

pub(super) struct Nedbank;

impl Acquirer for Nedbank {
    const NAME_MARKERS: &'static [&'static str] = &["nedbank", "postilion"];

    /// The Postilion sheet documents 127.22 as `<BANKNET_REF_NR><BANKNET_DATE>`.
    const BANKNET_PACKING: Option<BanknetPacking> = Some(BanknetPacking::ReferenceThenDate);

    /// A `ConnectorTxID3` of five or more pipe-separated fields.
    fn has_response_shape(details: &AciResponseResultDetails) -> bool {
        details.connector_tx_id3().field_count() >= 5
    }

    fn references(details: &AciResponseResultDetails) -> AcquirerReferences {
        let tx_id2 = details.connector_tx_id2();
        let tx_id3 = details.connector_tx_id3();
        AcquirerReferences {
            rrn: tx_id2.field(1),
            citi: tx_id3
                .field(4)
                .or_else(|| details.scheme_transaction_id.clone())
                .or_else(|| details.network_reference()),
            stan: tx_id2.field(0),
            auth_code: details.authorization_code().or_else(|| tx_id3.field(0)),
            original_transaction_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Payload and expected values are the sheet's own sample, which pairs this composite with
    /// `"CardholderInitiatedTransactionID":"MCC00013N0417"`.
    #[test]
    fn citi_is_read_from_connector_tx_id3_field_4() {
        let details = AciResponseResultDetails {
            connector_tx_id2: Some("000123|612345678901".to_string()),
            connector_tx_id3: Some(
                "543324|00|0417|075308|MCC00013N0417|000000001816511|".to_string(),
            ),
            ..Default::default()
        };
        let references = Nedbank::references(&details);

        assert_eq!(references.citi.as_deref(), Some("MCC00013N0417"));
        assert_eq!(
            references.auth_code.as_deref(),
            Some("543324"),
            "authorization code is ConnectorTxID3[0]"
        );
        assert_eq!(references.stan.as_deref(), Some("000123"));
        assert_eq!(references.rrn.as_deref(), Some("612345678901"));
    }

    #[test]
    fn a_direct_auth_code_outranks_the_composite() {
        let details = AciResponseResultDetails {
            approval_code: Some("888888".to_string()),
            connector_tx_id3: Some("543324|00".to_string()),
            ..Default::default()
        };
        assert_eq!(
            Nedbank::references(&details).auth_code.as_deref(),
            Some("888888")
        );
    }
}
