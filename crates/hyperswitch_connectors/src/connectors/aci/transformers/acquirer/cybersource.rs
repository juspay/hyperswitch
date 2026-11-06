//! CyberSource REST API via ACI's EMC (sometimes referred to as VDP / NPG depending on routing
//! configuration).
//!
//! Its `ConnectorTxID3` is a scalar `ReconciliationId` (no pipe composite), and the sheet's
//! response mapping and live sample both carry the scheme reference in
//! `resultDetails.NetworkTransactionId` — `SchemeTransactionId` never appears for this acquirer,
//! which previously left the CITI permanently unset here (PT-3901).

use super::{AciResponseResultDetails, Acquirer, AcquirerReferences, BanknetPacking};

pub(super) struct Cybersource;

impl Acquirer for Cybersource {
    const NAME_MARKERS: &'static [&'static str] = &["cybersource", "emc", "vdp"];

    /// Date first. Every EMC bank packs the same way; the live `EMC_FNB` trace id sample shows
    /// it.
    const BANKNET_PACKING: Option<BanknetPacking> = Some(BanknetPacking::DateThenReference);

    /// A `NetworkTransactionId` alongside a scalar `ConnectorTxID3`. Of the four acquirer sheets
    /// only CyberSource's returns `NetworkTransactionId`.
    fn has_response_shape(details: &AciResponseResultDetails) -> bool {
        details.network_transaction_id.is_some() && details.connector_tx_id3().field_count() == 1
    }

    fn references(details: &AciResponseResultDetails) -> AcquirerReferences {
        AcquirerReferences {
            citi: details
                .network_reference()
                .or_else(|| details.scheme_transaction_id.clone()),
            auth_code: details.authorization_code(),
            ..AcquirerReferences::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_id_mirrors_network_transaction_id() {
        let details = AciResponseResultDetails {
            transaction_id: Some("123456789619999".to_string()),
            scheme_transaction_id: Some("IGNORED".to_string()),
            ..Default::default()
        };
        assert_eq!(
            Cybersource::references(&details).citi.as_deref(),
            Some("123456789619999")
        );
    }
}
