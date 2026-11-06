//! The scheme trace id (CITI): the network reference that links every merchant-initiated
//! transaction back to the cardholder-initiated transaction that set up the card-on-file
//! agreement. ACI captures it from the CIT response and replays it on each MIT as
//! `standingInstruction.initialTransactionId`.
//!
//! The scheme parts are the same on every acquirer, but how they appear on the wire is not:
//!
//! * Visa: the 15-digit transaction identifier (F62.2), identical on every rail.
//! * Mastercard: two parts, the 9-character Banknet reference (DE 63) and the MMDD settlement
//!   date (DE 15). Acquirers concatenate them in different orders. Nedbank (Postilion 127.22
//!   `<BANKNET_REF_NR><BANKNET_DATE>`) sends `MDGXVJV4N0910`. CyberSource via EMC sends the date
//!   first, `0912MDUK0QGM7`, and every EMC bank packs the same way. Replaying one rail's packing
//!   on the other fails the scheme's linkage check.
//!
//! [`SchemeTraceId`] introspects a raw value, holds its parts rather than the string, and formats
//! them for whichever acquirer the value is sent to next. Nothing about the acquirer that produced
//! the value is needed, which is what lets a reference captured on one rail be used on another.

/// The order an acquirer concatenates a Mastercard trace id's two parts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BanknetPacking {
    /// `<BANKNET_REF_NR><BANKNET_DATE>`, e.g. `MDGXVJV4N0910`.
    ReferenceThenDate,
    /// `<BANKNET_DATE><BANKNET_REF_NR>`, e.g. `0912MDUK0QGM7`.
    DateThenReference,
}

const BANKNET_REFERENCE_LEN: usize = 9;
const FINANCIAL_NETWORK_CODE_LEN: usize = 3;
const SETTLEMENT_DATE_LEN: usize = 4;
const VISA_TRANSACTION_ID_LEN: usize = 15;

/// A scheme trace id, held as its parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SchemeTraceId {
    Visa(VisaTransactionId),
    Mastercard(MastercardTraceId),
    /// An uppercase alphanumeric reference in a shape no acquirer sheet documents: ABSA's
    /// Token 20, MPGS, or a network other than Visa and Mastercard. Its parts are unknown, so it
    /// is replayed exactly as received rather than guessed at.
    Unrecognised(String),
}

/// Visa's transaction identifier (F62.2): 15 digits, opaque, and never reordered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VisaTransactionId {
    transaction_id: String,
}

/// A Mastercard trace id's two scheme parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MastercardTraceId {
    /// DE 63 Banknet reference: a 3-character financial network code and a 6-character
    /// reference number, e.g. `MDGXVJV4N`.
    banknet_reference: String,
    /// DE 15 settlement date, `MMDD`.
    settlement_date: String,
    /// The packing the value arrived in, used to replay it when the destination acquirer's
    /// packing is undocumented.
    received_as: BanknetPacking,
}

/// Why a captured value was refused as a scheme trace id.
///
/// Each of these is an identifier from the wrong plane that has been seen in the CITI slot. It
/// passes OPPWA's parameter validation, the scheme cannot resolve it, and every MIT on the
/// mandate silently fails to link, so omitting `initialTransactionId` is strictly better.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TraceIdRejection {
    Empty,
    /// A 32-character lowercase hex id: an ACI payment or registration id.
    AciIdentifier,
    /// An 18-character mixed-case id: the placeholder Hyperswitch seeds into
    /// `connector_mandate_request_reference_id` at confirm (PT-4180).
    HyperswitchPlaceholder,
    /// Contains a lowercase letter or a non-alphanumeric character. Scheme references are
    /// uppercase alphanumeric; this catches every other id that can end up in the slot, such as
    /// Hyperswitch's `pm_…`/`pay_…`/`man_…` ids.
    NotSchemeShaped,
}

impl SchemeTraceId {
    /// Parse a captured value into its scheme parts by introspecting its shape alone.
    ///
    /// No knowledge of the acquirer that produced it is needed, so a value captured on one rail
    /// can be read anywhere and formatted for any other. The Mastercard packing is unambiguous
    /// from the value: reference-first starts with the three letters of the financial network
    /// code, date-first starts with four digits, and no value does both.
    pub(super) fn parse(raw: &str) -> Result<Self, TraceIdRejection> {
        let value = raw.trim();
        if value.is_empty() {
            return Err(TraceIdRejection::Empty);
        }
        if is_aci_identifier(value) {
            return Err(TraceIdRejection::AciIdentifier);
        }
        if is_hyperswitch_placeholder(value) {
            return Err(TraceIdRejection::HyperswitchPlaceholder);
        }
        if !value
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        {
            return Err(TraceIdRejection::NotSchemeShaped);
        }

        if value.len() == VISA_TRANSACTION_ID_LEN && value.bytes().all(|b| b.is_ascii_digit()) {
            return Ok(Self::Visa(VisaTransactionId {
                transaction_id: value.to_string(),
            }));
        }
        if let Some(mastercard) = MastercardTraceId::parse(value) {
            return Ok(Self::Mastercard(mastercard));
        }
        // Accepted, but in no documented shape: logged so the shapes that really occur can be
        // learned. The value itself is a scheme reference, not card data.
        router_env::logger::warn!(
            length = value.len(),
            value,
            "aci: accepted a scheme trace id in an unrecognised shape"
        );
        Ok(Self::Unrecognised(value.to_string()))
    }

    /// The wire value for an acquirer whose Mastercard packing is `packing`.
    ///
    /// With no documented packing, a Mastercard value is replayed in the order it was received:
    /// reordering for an acquirer whose convention is unknown would be a guess.
    pub(super) fn format_for(&self, packing: Option<BanknetPacking>) -> String {
        match self {
            Self::Visa(visa) => visa.transaction_id.clone(),
            Self::Mastercard(mastercard) => {
                mastercard.pack(packing.unwrap_or(mastercard.received_as))
            }
            Self::Unrecognised(value) => value.clone(),
        }
    }

    /// The value exactly as the acquirer sent it.
    pub(super) fn as_received(&self) -> String {
        self.format_for(None)
    }
}

impl MastercardTraceId {
    fn parse(value: &str) -> Option<Self> {
        if value.len() != BANKNET_REFERENCE_LEN + SETTLEMENT_DATE_LEN {
            return None;
        }
        // At most one order can match: see `is_banknet_reference`.
        Self::split(value, BanknetPacking::ReferenceThenDate)
            .or_else(|| Self::split(value, BanknetPacking::DateThenReference))
    }

    fn split(value: &str, packing: BanknetPacking) -> Option<Self> {
        let (banknet_reference, settlement_date) = match packing {
            BanknetPacking::ReferenceThenDate => value.split_at(BANKNET_REFERENCE_LEN),
            BanknetPacking::DateThenReference => {
                let (date, reference) = value.split_at(SETTLEMENT_DATE_LEN);
                (reference, date)
            }
        };
        (is_banknet_reference(banknet_reference) && is_settlement_date(settlement_date)).then(
            || Self {
                banknet_reference: banknet_reference.to_string(),
                settlement_date: settlement_date.to_string(),
                received_as: packing,
            },
        )
    }

    fn pack(&self, packing: BanknetPacking) -> String {
        match packing {
            BanknetPacking::ReferenceThenDate => {
                format!("{}{}", self.banknet_reference, self.settlement_date)
            }
            BanknetPacking::DateThenReference => {
                format!("{}{}", self.settlement_date, self.banknet_reference)
            }
        }
    }
}

/// A 3-letter financial network code followed by a 6-character uppercase alphanumeric reference.
///
/// The letter prefix is what makes the packing self-describing: a reference-first value starts
/// with three letters, a date-first value with four digits.
fn is_banknet_reference(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == BANKNET_REFERENCE_LEN
        && bytes
            .get(..FINANCIAL_NETWORK_CODE_LEN)
            .is_some_and(|code| code.iter().all(u8::is_ascii_uppercase))
        && bytes
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

/// `MMDD` with a real month and a day that can exist in some year.
fn is_settlement_date(value: &str) -> bool {
    if value.len() != SETTLEMENT_DATE_LEN || !value.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let (month, day) = value.split_at(2);
    let (Ok(month), Ok(day)) = (month.parse::<u8>(), day.parse::<u8>()) else {
        return false;
    };
    let days_in_month = match month {
        2 => 29,
        4 | 6 | 9 | 11 => 30,
        1..=12 => 31,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

fn is_aci_identifier(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Scheme references are uppercase. An 18-character alphanumeric value containing a lowercase
/// letter is the shape `common_utils::generate_id_with_len` produces.
fn is_hyperswitch_placeholder(value: &str) -> bool {
    value.len() == 18
        && value.bytes().all(|b| b.is_ascii_alphanumeric())
        && value.bytes().any(|b| b.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACKINGS: [Option<BanknetPacking>; 3] = [
        None,
        Some(BanknetPacking::ReferenceThenDate),
        Some(BanknetPacking::DateThenReference),
    ];

    fn parse(raw: &str) -> SchemeTraceId {
        SchemeTraceId::parse(raw).expect("a scheme trace id")
    }

    #[test]
    fn visa_transaction_id_is_identical_for_every_packing() {
        let trace_id = parse("476253540570401");
        assert!(matches!(trace_id, SchemeTraceId::Visa(_)));
        for packing in PACKINGS {
            assert_eq!(trace_id.format_for(packing), "476253540570401");
        }
        assert_eq!(trace_id.as_received(), "476253540570401");
    }

    #[test]
    fn reference_first_mastercard_repacks_for_each_acquirer() {
        let trace_id = parse("MDGXVJV4N0910");
        assert_eq!(
            trace_id,
            SchemeTraceId::Mastercard(MastercardTraceId {
                banknet_reference: "MDGXVJV4N".to_string(),
                settlement_date: "0910".to_string(),
                received_as: BanknetPacking::ReferenceThenDate,
            })
        );
        assert_eq!(
            trace_id.format_for(Some(BanknetPacking::ReferenceThenDate)),
            "MDGXVJV4N0910"
        );
        assert_eq!(
            trace_id.format_for(Some(BanknetPacking::DateThenReference)),
            "0910MDGXVJV4N"
        );
        assert_eq!(trace_id.format_for(None), "MDGXVJV4N0910");
        assert_eq!(trace_id.as_received(), "MDGXVJV4N0910");
    }

    #[test]
    fn date_first_mastercard_repacks_for_each_acquirer() {
        let trace_id = parse("0912MDUK0QGM7");
        assert_eq!(
            trace_id,
            SchemeTraceId::Mastercard(MastercardTraceId {
                banknet_reference: "MDUK0QGM7".to_string(),
                settlement_date: "0912".to_string(),
                received_as: BanknetPacking::DateThenReference,
            })
        );
        assert_eq!(
            trace_id.format_for(Some(BanknetPacking::ReferenceThenDate)),
            "MDUK0QGM70912"
        );
        assert_eq!(
            trace_id.format_for(Some(BanknetPacking::DateThenReference)),
            "0912MDUK0QGM7"
        );
        assert_eq!(trace_id.format_for(None), "0912MDUK0QGM7");
        assert_eq!(trace_id.as_received(), "0912MDUK0QGM7");
    }

    /// A value repacked for another acquirer parses back to the same parts, so it can be repacked
    /// again (or back to how it arrived) without loss.
    #[test]
    fn mastercard_repacking_round_trips() {
        let cases = [
            ("MDGXVJV4N0910", BanknetPacking::ReferenceThenDate),
            ("0912MDUK0QGM7", BanknetPacking::DateThenReference),
        ];
        for (raw, received_as) in cases {
            let trace_id = parse(raw);
            for packing in [
                BanknetPacking::ReferenceThenDate,
                BanknetPacking::DateThenReference,
            ] {
                let wire = trace_id.format_for(Some(packing));
                let reparsed = parse(&wire);
                assert_eq!(reparsed.as_received(), wire);
                assert_eq!(reparsed.format_for(Some(received_as)), raw);
            }
        }
    }

    /// `0912ABCD10910` would split both ways if the reference only had to be alphanumeric:
    /// `0912ABCD1` + `0910` and `0912` + `ABCD10910`. Requiring the reference to start with the
    /// three letters of the network code leaves only the date-first reading.
    #[test]
    fn a_value_that_once_split_both_ways_parses_only_one_way() {
        assert_eq!(
            parse("0912ABCD10910"),
            SchemeTraceId::Mastercard(MastercardTraceId {
                banknet_reference: "ABCD10910".to_string(),
                settlement_date: "0912".to_string(),
                received_as: BanknetPacking::DateThenReference,
            })
        );
    }

    #[test]
    fn invalid_settlement_date_is_not_mastercard() {
        assert_eq!(
            parse("MDGXVJV4N1332"),
            SchemeTraceId::Unrecognised("MDGXVJV4N1332".to_string())
        );
    }

    #[test]
    fn reference_without_a_three_letter_network_code_is_not_mastercard() {
        assert_eq!(
            parse("MD1XVJV4N0910"),
            SchemeTraceId::Unrecognised("MD1XVJV4N0910".to_string())
        );
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        let trace_id = parse("MDGXVJV4N0910  ");
        assert!(matches!(trace_id, SchemeTraceId::Mastercard(_)));
        assert_eq!(trace_id.as_received(), "MDGXVJV4N0910");
    }

    #[test]
    fn each_wrong_plane_identifier_is_rejected_for_its_reason() {
        let cases = [
            ("", TraceIdRejection::Empty),
            ("  ", TraceIdRejection::Empty),
            (
                "8acda4a6a0e7fd8b01a0f102d66819f8",
                TraceIdRejection::AciIdentifier,
            ),
            (
                "PLSWDsdWh5GALKpIZs",
                TraceIdRejection::HyperswitchPlaceholder,
            ),
            ("pm_UmaozPE1KJaWOCxAcIyA", TraceIdRejection::NotSchemeShaped),
            ("pay_abc", TraceIdRejection::NotSchemeShaped),
            ("MDG-064NN5", TraceIdRejection::NotSchemeShaped),
            ("mdgxvjv4n0910", TraceIdRejection::NotSchemeShaped),
        ];
        for (raw, rejection) in cases {
            assert_eq!(SchemeTraceId::parse(raw), Err(rejection), "{raw:?}");
        }
    }

    #[test]
    fn an_unknown_uppercase_shape_is_replayed_verbatim() {
        let trace_id = parse("ABC123");
        assert_eq!(trace_id, SchemeTraceId::Unrecognised("ABC123".to_string()));
        for packing in PACKINGS {
            assert_eq!(trace_id.format_for(packing), "ABC123");
        }
        assert_eq!(trace_id.as_received(), "ABC123");
    }
}
