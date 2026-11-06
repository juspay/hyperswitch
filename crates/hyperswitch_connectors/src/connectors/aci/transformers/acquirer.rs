//! The acquirer behind an ACI entity, and everything that differs by acquirer.
//!
//! ACI fronts several acquirers, and each one reports its references differently: the
//! pipe-delimited `resultDetails.ConnectorTxID1/2/3` composites have acquirer-specific layouts,
//! and a Mastercard trace id is concatenated in an acquirer-specific order. Each acquirer is a
//! type implementing [`Acquirer`], in its own module alongside the integration-sheet layout it
//! encodes; [`AcquirerKind`] names them and dispatches to them.

mod absa;
mod cybersource;
mod mpgs;
mod nedbank;
mod planet;
mod unknown;

use super::response::{AciResponseResultDetails, AciResponseStandingInstruction};
use crate::connectors::aci::scheme_trace_id::{BanknetPacking, SchemeTraceId, TraceIdRejection};

/// What sets one acquirer apart from another.
trait Acquirer {
    /// Lowercase fragments that identify this acquirer in a merchant-declared name or in ACI's
    /// `clearingInstituteName`.
    const NAME_MARKERS: &'static [&'static str];

    /// The order this acquirer concatenates a Mastercard trace id's Banknet reference and
    /// settlement date in, when its sheet documents one. Undocumented packings are replayed as
    /// received. See [`crate::connectors::aci::scheme_trace_id`].
    const BANKNET_PACKING: Option<BanknetPacking> = None;

    /// Whether an anonymous response is shaped like this acquirer's. A guess, never an
    /// identification.
    fn has_response_shape(_details: &AciResponseResultDetails) -> bool {
        false
    }

    /// The references this acquirer reports in `resultDetails`.
    fn references(details: &AciResponseResultDetails) -> AcquirerReferences;
}

/// Declares the acquirers: the [`AcquirerKind`] enum and its dispatch to each [`Acquirer`].
///
/// Declaration order is name-matching order, see [`AcquirerKind::from_name`].
macro_rules! acquirers {
    ($($(#[$doc:meta])* $kind:ident => $acquirer:ty;)+) => {
        /// The downstream acquirer routing a transaction.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(super) enum AcquirerKind {
            $($(#[$doc])* $kind,)+
        }

        impl AcquirerKind {
            const ALL: &'static [Self] = &[$(Self::$kind),+];

            fn name_markers(self) -> &'static [&'static str] {
                match self {
                    $(Self::$kind => <$acquirer as Acquirer>::NAME_MARKERS,)+
                }
            }

            /// See [`Acquirer::BANKNET_PACKING`].
            pub(super) fn banknet_packing(self) -> Option<BanknetPacking> {
                match self {
                    $(Self::$kind => <$acquirer as Acquirer>::BANKNET_PACKING,)+
                }
            }

            fn has_response_shape(self, details: &AciResponseResultDetails) -> bool {
                match self {
                    $(Self::$kind => <$acquirer as Acquirer>::has_response_shape(details),)+
                }
            }

            /// The references this acquirer reports in `details`.
            pub(super) fn references(
                self,
                details: &AciResponseResultDetails,
            ) -> AcquirerReferences {
                match self {
                    $(Self::$kind => <$acquirer as Acquirer>::references(details),)+
                }
            }
        }
    };
}

acquirers! {
    /// MPGS (Mastercard Payment Gateway Services).
    Mpgs => mpgs::Mpgs;
    /// CyberSource REST API via ACI's EMC (also called VDP — Visa Direct Platform — when
    /// configured for that routing).
    Cybersource => cybersource::Cybersource;
    /// Nedbank, via Postilion.
    Nedbank => nedbank::Nedbank;
    Absa => absa::Absa;
    /// Planet Payment (multi-currency pricing platform; supports VISA, MASTER, AMEX, DINERS,
    /// DISCOVER, JCB, UNIONPAY, UNIONPAY_SMS).
    Planet => planet::Planet;
    /// An acquirer nobody declared and nothing recognised.
    Unknown => unknown::Unknown;
}

/// How the downstream acquirer was determined, and how much to trust it.
///
/// `positively_identified` is true only when the acquirer was declared by the merchant or named
/// by ACI itself — the shape heuristics are guesses, and values extracted from composites under a
/// guessed identity must not outrank fields the response documents explicitly.
#[derive(Debug, Clone, Copy)]
pub(super) struct AcquirerResolution {
    pub(super) acquirer: AcquirerKind,
    pub(super) positively_identified: bool,
}

impl AcquirerKind {
    /// Shape heuristics in the order they are tried. ABSA first: the "PIM" tag is specific to
    /// the ACI sandbox's ABSA simulator, whereas the Nedbank and CyberSource shapes are looser.
    const SHAPE_PRECEDENCE: [Self; 3] = [Self::Absa, Self::Nedbank, Self::Cybersource];

    /// Match an acquirer from a name — a merchant-declared metadata value or ACI's
    /// `clearingInstituteName`.
    ///
    /// Gateway names are tested before downstream-acquirer names. The integration sheets document
    /// the naming *conventions* `MPGS_<Required Acquirer Name>` and `EMC_<Required Bank Name>`
    /// without showing any concrete value, so any such name whose acquirer/bank part contains a
    /// marker of a downstream acquirer (e.g. a hypothetical `MPGS_ABSA`) would classify as that
    /// acquirer if it were tested first — and every composite for that entity would be parsed
    /// under the wrong rules.
    pub(super) fn from_name(name: &str) -> Option<Self> {
        let name = name.to_lowercase();
        Self::ALL.iter().copied().find(|kind| {
            kind.name_markers()
                .iter()
                .any(|marker| name.contains(marker))
        })
    }
}

impl AcquirerResolution {
    /// Resolve the downstream acquirer, most-trustworthy source first.
    ///
    /// 1. The `acquirer` the merchant declared in connector metadata. Acquirer identity is known
    ///    at onboarding, so configuration is the durable source — response-shape inference exists
    ///    only for accounts that have not declared it.
    /// 2. `resultDetails.clearingInstituteName`, when ACI populates it. Note none of the four
    ///    acquirer integration sheets document this field or show it in a sample response, so its
    ///    production coverage is unverified.
    /// 3. Shape heuristics, as guesses (see each acquirer's [`Acquirer::has_response_shape`]).
    pub(super) fn resolve(
        declared: Option<AcquirerKind>,
        details: &AciResponseResultDetails,
    ) -> Self {
        let named = || {
            details
                .clearing_institute_name
                .as_deref()
                .and_then(AcquirerKind::from_name)
        };
        match declared.or_else(named) {
            Some(acquirer) => Self {
                acquirer,
                positively_identified: true,
            },
            None => Self {
                acquirer: AcquirerKind::SHAPE_PRECEDENCE
                    .into_iter()
                    .find(|kind| kind.has_response_shape(details))
                    .unwrap_or(AcquirerKind::Unknown),
                positively_identified: false,
            },
        }
    }
}

/// The references an acquirer reports in `resultDetails`, read under its own layout.
#[derive(Debug, Default)]
pub(super) struct AcquirerReferences {
    /// Read for each acquirer's layout, though nothing downstream consumes it yet.
    #[allow(dead_code)]
    pub(super) rrn: Option<String>,
    pub(super) citi: Option<String>,
    pub(super) stan: Option<String>,
    pub(super) auth_code: Option<String>,
    pub(super) original_transaction_id: Option<String>,
}

impl AcquirerReferences {
    /// For acquirers that report their references as top-level `resultDetails` fields rather than
    /// inside the `ConnectorTxID` composites.
    fn top_level(details: &AciResponseResultDetails) -> Self {
        Self {
            citi: details
                .scheme_transaction_id
                .clone()
                .or_else(|| details.network_reference()),
            auth_code: details.authorization_code(),
            ..Self::default()
        }
    }
}

/// One of the pipe-delimited `resultDetails.ConnectorTxID1/2/3` composites.
#[derive(Debug, Clone, Copy)]
struct Composite<'a>(Option<&'a str>);

impl Composite<'_> {
    /// Field `index`, when present and non-empty.
    fn field(self, index: usize) -> Option<String> {
        self.0?
            .split('|')
            .nth(index)
            .filter(|field| !field.is_empty())
            .map(str::to_string)
    }

    fn field_count(self) -> usize {
        self.0.map_or(0, |composite| composite.split('|').count())
    }

    fn contains(self, needle: &str) -> bool {
        self.0.is_some_and(|composite| composite.contains(needle))
    }
}

impl AciResponseResultDetails {
    fn connector_tx_id1(&self) -> Composite<'_> {
        Composite(self.connector_tx_id1.as_deref())
    }

    fn connector_tx_id2(&self) -> Composite<'_> {
        Composite(self.connector_tx_id2.as_deref())
    }

    fn connector_tx_id3(&self) -> Composite<'_> {
        Composite(self.connector_tx_id3.as_deref())
    }

    /// The canonical `AuthCode`, falling back to `ApprovalCode` (used by some acquirers and the
    /// ACI sandbox in test mode).
    fn authorization_code(&self) -> Option<String> {
        self.auth_code
            .clone()
            .or_else(|| self.approval_code.clone())
    }

    /// The scheme/network transaction id as CyberSource returns it (see the field docs). Every
    /// other acquirer also uses it as its last CITI fallback: its semantics are exactly what the
    /// CITI wants, and it only fires when the acquirer's documented sources are absent.
    fn network_reference(&self) -> Option<String> {
        self.network_transaction_id
            .clone()
            .or_else(|| self.transaction_id.clone())
    }
}

/// Resolve the scheme-generated CIT reference (the "CITI") from a payment response, in the
/// order OPPWA documents it.
///
/// The API reference names two response locations and defers to the per-acquirer Connector
/// Integration sheet for which one applies, so both must be consulted before falling back:
///
/// 1. `standingInstruction.initialTransactionId` — the only location documented by *all four*
///    acquirer sheets (Nedbank, ABSA, MPGS, CyberSource), so it is unambiguously first.
/// 2. The acquirer-specific value from [`AcquirerKind::references`] — `ConnectorTxID3[4]` for
///    Nedbank, `SchemeTransactionId`/Token 20 for ABSA — **but only when the acquirer was
///    positively identified** (declared in connector metadata or named by ACI). For an identified
///    ABSA this must outrank step 3: its sheet documents Token 20 as the CITI and does not
///    document `CardholderInitiatedTransactionID` at all, so letting that field win would replay
///    a Mastercard banknet reference where ABSA expects a trace ID and fail its COF linkage
///    check. When the acquirer is merely *guessed* from response shape, the order flips: a
///    5+-pipe `ConnectorTxID3` from an uncharacterised acquirer would otherwise promote whatever
///    occupies slot 4 over a field the response documents explicitly.
/// 3. `resultDetails.CardholderInitiatedTransactionID` — documented by Nedbank and MPGS. It is the
///    fallback that rescues MPGS and CyberSource, which return neither `SchemeTransactionId` nor a
///    pipe-delimited `ConnectorTxID3` and so have no step-2 value at all. Before this field was
///    consulted the CITI was permanently `None` on those two acquirers and every subsequent MIT
///    silently omitted `standingInstruction.initialTransactionId`, breaking CIT→MIT linkage.
///
/// Every source is validated as a [`SchemeTraceId`] before it can win, so a value that is not a
/// scheme reference falls through to the next source instead of shadowing it. ACI payloads really
/// do carry empty fields (the Nedbank sample composite ends in a trailing `|`), and ACI ids and
/// the PT-4180 placeholder have been seen in the CITI slot. Validating only the winner would let
/// any of them hide a genuine reference further down and leave the mandate with none at all.
/// Accepted values are returned exactly as received.
pub(super) fn resolve_citi(
    standing_instruction: Option<&AciResponseStandingInstruction>,
    result_details: Option<&AciResponseResultDetails>,
    references: Option<&AcquirerReferences>,
    acquirer_positively_identified: bool,
) -> Option<String> {
    fn scheme_reference(value: Option<String>) -> Option<String> {
        let raw = value?;
        SchemeTraceId::parse(&raw)
            .map_err(|rejection| {
                // Empty fields are routine in ACI payloads; only a wrong-plane id is news.
                if rejection != TraceIdRejection::Empty {
                    router_env::logger::warn!(
                        ?rejection,
                        "aci: skipped a CITI source that is not a scheme reference"
                    );
                }
            })
            .ok()
            .map(|trace_id| trace_id.as_received())
    }

    let documented =
        scheme_reference(standing_instruction.and_then(|si| si.initial_transaction_id.clone()));
    let composite = scheme_reference(references.and_then(|r| r.citi.clone()));
    let cardholder_initiated = scheme_reference(
        result_details.and_then(|d| d.cardholder_initiated_transaction_id.clone()),
    );

    if acquirer_positively_identified {
        documented.or(composite).or(cardholder_initiated)
    } else {
        documented.or(cardholder_initiated).or(composite)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(name: &str) -> Option<AcquirerKind> {
        AcquirerKind::from_name(name)
    }

    /// Resolve the acquirer for `details`, read its references and resolve the CITI from them,
    /// the way the payment response does.
    fn citi(
        declared: Option<AcquirerKind>,
        standing_instruction: Option<&AciResponseStandingInstruction>,
        details: &AciResponseResultDetails,
    ) -> Option<String> {
        let resolution = AcquirerResolution::resolve(declared, details);
        let references = resolution.acquirer.references(details);
        resolve_citi(
            standing_instruction,
            Some(details),
            Some(&references),
            resolution.positively_identified,
        )
    }

    fn standing_instruction(initial_transaction_id: &str) -> AciResponseStandingInstruction {
        AciResponseStandingInstruction {
            agreement_id: None,
            initial_transaction_id: Some(initial_transaction_id.to_string()),
        }
    }

    /// The Nedbank Connector Integration sheet's own sample composite, which it pairs with
    /// `"CardholderInitiatedTransactionID":"MCC00013N0417"`.
    const NEDBANK_TX_ID3: &str = "543324|00|0417|075308|MCC00013N0417|000000001816511|";

    // ---------------------------------------------------------------------
    // Card-on-file: resolving the scheme CIT reference (CITI)
    //
    // The CITI is replayed as `standingInstruction.initialTransactionId` on every
    // subsequent MIT, so getting it wrong breaks CIT→MIT linkage. OPPWA documents two
    // response locations for it and defers to the per-acquirer Connector Integration
    // sheet for which one applies, so both must be consulted before falling back to the
    // acquirer-specific pipe-delimited composites.
    // ---------------------------------------------------------------------

    /// `standingInstruction.initialTransactionId` wins: it is the location all four
    /// acquirer sheets document, so it takes precedence over everything else.
    #[test]
    fn citi_prefers_standing_instruction_initial_transaction_id() {
        let details = AciResponseResultDetails {
            cardholder_initiated_transaction_id: Some("FROMRESULTDETAILS".to_string()),
            scheme_transaction_id: Some("FROMSCHEMETRACE".to_string()),
            ..Default::default()
        };

        assert_eq!(
            citi(None, Some(&standing_instruction("SICANONICAL")), &details),
            Some("SICANONICAL".to_string())
        );
    }

    /// When the standing-instruction object is absent, `resultDetails.CardholderInitiated‑
    /// TransactionID` is the second documented location (Nedbank and MPGS both name it).
    /// The `...ID` spelling needs an explicit serde rename because the struct is
    /// `rename_all = "PascalCase"`, so this also guards the deserialization.
    #[test]
    fn citi_falls_back_to_cardholder_initiated_transaction_id() {
        let details: AciResponseResultDetails =
            serde_json::from_str(r#"{"CardholderInitiatedTransactionID": "MCC00013N0417"}"#)
                .expect("resultDetails should deserialize");
        assert_eq!(
            details.cardholder_initiated_transaction_id.as_deref(),
            Some("MCC00013N0417"),
            "the documented `...ID` spelling must deserialize despite PascalCase rename_all"
        );

        assert_eq!(
            citi(None, None, &details),
            Some("MCC00013N0417".to_string())
        );
    }

    /// MPGS and CyberSource return neither `SchemeTransactionId` nor a pipe-delimited
    /// `ConnectorTxID3`, so the composite heuristics yield nothing for them. Before the
    /// documented fields were consulted this produced `citi = None` on every CIT, which
    /// meant every MIT omitted `standingInstruction.initialTransactionId`.
    #[test]
    fn citi_is_resolved_for_acquirers_without_scheme_transaction_id() {
        // Shape of an MPGS / CyberSource response: no SchemeTransactionId, and
        // ConnectorTxID3 is a scalar (CyberSource returns ReconciliationId there).
        let details = AciResponseResultDetails {
            connector_tx_id3: Some("70719677M0J5WI97".to_string()),
            ..Default::default()
        };
        let resolution = AcquirerResolution::resolve(None, &details);
        assert_eq!(
            resolution.acquirer.references(&details).citi,
            None,
            "composite heuristics cannot recover a CITI for these acquirers"
        );

        assert_eq!(
            citi(
                None,
                Some(&standing_instruction("016153570198200")),
                &details
            ),
            Some("016153570198200".to_string()),
            "the documented standingInstruction field must be used"
        );
    }

    /// With none of the documented fields present, resolution degrades to the
    /// acquirer-specific composites rather than inventing a value.
    #[test]
    fn citi_is_none_when_no_documented_source_is_present() {
        assert_eq!(citi(None, None, &AciResponseResultDetails::default()), None);
    }

    /// The composite fallback must stay load-bearing. Nedbank and ABSA relied on it before the
    /// documented fields were consulted, so this drives a populated Nedbank composite through
    /// `resolve_citi` itself with no `standingInstruction` and no
    /// `CardholderInitiatedTransactionID` — deleting step 3 must fail this test.
    #[test]
    fn citi_falls_back_to_the_acquirer_composite() {
        let details = AciResponseResultDetails {
            clearing_institute_name: Some("NEDBANK".to_string()),
            connector_tx_id3: Some(NEDBANK_TX_ID3.to_string()),
            ..Default::default()
        };
        assert!(
            AcquirerResolution::resolve(None, &details).positively_identified,
            "a clearingInstituteName naming Nedbank must positively identify it"
        );

        assert_eq!(
            citi(None, None, &details),
            Some("MCC00013N0417".to_string())
        );
    }

    /// ABSA documents Token 20 (surfaced as `SchemeTransactionId`) as its CITI and does not
    /// document `CardholderInitiatedTransactionID` at all. If ACI populates both, the
    /// acquirer-specific value must win — replaying a Mastercard banknet reference where ABSA
    /// expects a trace ID fails its card-on-file linkage check.
    #[test]
    fn citi_prefers_the_acquirer_composite_over_cardholder_initiated_id() {
        let details = AciResponseResultDetails {
            clearing_institute_name: Some("ABSA".to_string()),
            scheme_transaction_id: Some("016153570198200".to_string()),
            cardholder_initiated_transaction_id: Some("MCC00013N0417".to_string()),
            ..Default::default()
        };

        assert_eq!(
            citi(None, None, &details),
            Some("016153570198200".to_string()),
            "ABSA's documented Token 20 must outrank the undocumented CardholderInitiatedTransactionID"
        );
    }

    /// An empty documented field must not shadow a valid later source. ACI payloads really do
    /// carry empty fields, and an empty CITI would be replayed as
    /// `standingInstruction.initialTransactionId: ""`, which OPPWA rejects.
    #[test]
    fn citi_ignores_empty_sources() {
        let details = AciResponseResultDetails {
            clearing_institute_name: Some("NEDBANK".to_string()),
            cardholder_initiated_transaction_id: Some("   ".to_string()),
            connector_tx_id3: Some(NEDBANK_TX_ID3.to_string()),
            ..Default::default()
        };

        assert_eq!(
            citi(None, Some(&standing_instruction("")), &details),
            Some("MCC00013N0417".to_string()),
            "an empty standingInstruction/resultDetails value must not shadow the composite"
        );
    }

    // ---------------------------------------------------------------------
    // Acquirer identity resolution
    // ---------------------------------------------------------------------

    /// The merchant-declared `acquirer` in connector metadata is the durable identity source —
    /// it must beat every response-shape signal, including a composite that looks like a
    /// different acquirer's format.
    #[test]
    fn declared_acquirer_outranks_response_shape() {
        // Nedbank-shaped composite (6 pipes) that the heuristic would otherwise claim.
        let details = AciResponseResultDetails {
            connector_tx_id3: Some(NEDBANK_TX_ID3.to_string()),
            scheme_transaction_id: Some("016153570198200".to_string()),
            ..Default::default()
        };

        let resolution = AcquirerResolution::resolve(declared("absa"), &details);
        assert_eq!(resolution.acquirer, AcquirerKind::Absa);
        assert!(resolution.positively_identified);

        // Parsed under the declared identity: ABSA's rule (scheme trace first), not Nedbank's
        // pipe index.
        assert_eq!(
            resolution.acquirer.references(&details).citi.as_deref(),
            Some("016153570198200")
        );
    }

    /// The onboarding conventions name clearing institutes `MPGS_<Required Acquirer Name>` /
    /// `EMC_<Required Bank Name>`. The sheets show no concrete values — the names below are
    /// synthetic instances of the convention, not observed configurations — but the property they
    /// pin is real: whatever the acquirer/bank part turns out to be, the gateway prefix must win,
    /// or every composite for that entity is parsed under the downstream acquirer's rules.
    #[test]
    fn gateway_prefixed_names_classify_as_the_gateway() {
        assert_eq!(declared("MPGS_ABSA"), Some(AcquirerKind::Mpgs));
        assert_eq!(declared("EMC_NEDBANK"), Some(AcquirerKind::Cybersource));
        assert_eq!(declared("ABSA"), Some(AcquirerKind::Absa));
        assert_eq!(declared("Postilion SA"), Some(AcquirerKind::Nedbank));
        assert_eq!(
            declared("Mastercard Payment Gateway"),
            Some(AcquirerKind::Mpgs)
        );
        assert_eq!(declared("Planet Payment"), Some(AcquirerKind::Planet));
        assert_eq!(declared("somebank"), None);
    }

    /// Each shape heuristic, and the order they are tried in.
    #[test]
    fn anonymous_responses_are_guessed_from_their_shape() {
        let guess = |details: AciResponseResultDetails| {
            let resolution = AcquirerResolution::resolve(None, &details);
            assert!(!resolution.positively_identified);
            resolution.acquirer
        };

        let absa = AciResponseResultDetails {
            connector_tx_id2: Some("INV1|00|123456|PIM".to_string()),
            // Also Nedbank-shaped: the ABSA simulator tag must win.
            connector_tx_id3: Some(NEDBANK_TX_ID3.to_string()),
            ..Default::default()
        };
        assert_eq!(guess(absa), AcquirerKind::Absa);

        let nedbank = AciResponseResultDetails {
            connector_tx_id3: Some("a|b|c|d|e".to_string()),
            network_transaction_id: Some("123456789619999".to_string()),
            ..Default::default()
        };
        assert_eq!(guess(nedbank), AcquirerKind::Nedbank);

        let four_fields = AciResponseResultDetails {
            connector_tx_id3: Some("a|b|c|d".to_string()),
            ..Default::default()
        };
        assert_eq!(guess(four_fields), AcquirerKind::Unknown);

        let cybersource = AciResponseResultDetails {
            connector_tx_id3: Some("70719677M0J5WI97".to_string()),
            network_transaction_id: Some("123456789619999".to_string()),
            ..Default::default()
        };
        assert_eq!(guess(cybersource), AcquirerKind::Cybersource);

        let no_tx_id3 = AciResponseResultDetails {
            network_transaction_id: Some("123456789619999".to_string()),
            ..Default::default()
        };
        assert_eq!(guess(no_tx_id3), AcquirerKind::Unknown);
    }

    /// When the acquirer is only guessed from response shape, a composite-extracted value must
    /// not outrank a field the response documents explicitly: a 5+-pipe `ConnectorTxID3` from an
    /// uncharacterised acquirer would otherwise promote whatever occupies slot 4 (an approval
    /// code, a merchant id) over the genuine `CardholderInitiatedTransactionID`.
    #[test]
    fn guessed_identity_demotes_the_composite_below_documented_fields() {
        // Nedbank-shaped (6 pipes) but anonymous: no metadata, no clearingInstituteName.
        let details = AciResponseResultDetails {
            connector_tx_id3: Some("543324|00|0417|075308|GARBAGESLOT4|MERCH01|".to_string()),
            cardholder_initiated_transaction_id: Some("MCC00013N0417".to_string()),
            ..Default::default()
        };

        let resolution = AcquirerResolution::resolve(None, &details);
        assert_eq!(resolution.acquirer, AcquirerKind::Nedbank, "shape guess");
        assert!(
            !resolution.positively_identified,
            "a shape match is not an identification"
        );
        assert_eq!(
            citi(None, None, &details),
            Some("MCC00013N0417".to_string()),
            "the documented field must win over a slot-4 guess"
        );

        // The same response positively identified as Nedbank keeps the composite preference —
        // that ordering is deliberate and sheet-verified for known acquirers.
        let named = AciResponseResultDetails {
            clearing_institute_name: Some("NEDBANK".to_string()),
            ..details
        };
        assert!(AcquirerResolution::resolve(None, &named).positively_identified);
        assert_eq!(
            citi(None, None, &named),
            Some("GARBAGESLOT4".to_string()),
            "an identified acquirer's composite rule stays authoritative"
        );
    }

    /// CyberSource/EMC returns the scheme reference in `resultDetails.NetworkTransactionId`
    /// (mirrored in `TransactionId`) — never in `SchemeTransactionId`, which its sheet does not
    /// mention at all. The payload below is the integration sheet's own response sample; reading
    /// `SchemeTransactionId` here left the CITI permanently unset for this acquirer (PT-3901).
    #[test]
    fn cybersource_citi_comes_from_network_transaction_id() {
        let details: AciResponseResultDetails = serde_json::from_str(
            r#"{
                "ConnectorTxID1": "6469103724256855104004",
                "ConnectorTxID3": "70719677M0J5WI97",
                "ConnectorTxID2": "8ac7a49f7f726902017f7381dd032b1d",
                "ApprovalCode": "888888",
                "TransactionId": "123456789619999",
                "ExtendedDescription": "Successful approval.",
                "avs.codeRaw": "I1",
                "ReconciliationId": "70719677M0J5WI97",
                "avs.code": "X",
                "AcquirerResponse": "AUTHORIZED",
                "NetworkTransactionId": "123456789619999"
            }"#,
        )
        .expect("sheet sample should deserialize");

        // Positively identified via merchant-declared metadata (the sample carries no
        // clearingInstituteName, like the sheet's real responses).
        let resolution = AcquirerResolution::resolve(declared("cybersource"), &details);
        assert_eq!(resolution.acquirer, AcquirerKind::Cybersource);
        assert_eq!(
            resolution.acquirer.references(&details).citi.as_deref(),
            Some("123456789619999"),
            "the scheme reference must come from NetworkTransactionId, not the scalar ReconciliationId"
        );
        assert_eq!(
            citi(declared("cybersource"), None, &details),
            Some("123456789619999".to_string())
        );

        // The documented standingInstruction echo still wins when present.
        assert_eq!(
            citi(
                declared("cybersource"),
                Some(&standing_instruction("SICANONICAL")),
                &details
            ),
            Some("SICANONICAL".to_string())
        );

        // An UNDECLARED CyberSource entity is recognised by shape (NetworkTransactionId with a
        // scalar TxID3), as a guess: the value must still be recovered under guessed-identity
        // ordering.
        let guessed = AcquirerResolution::resolve(None, &details);
        assert_eq!(guessed.acquirer, AcquirerKind::Cybersource);
        assert!(!guessed.positively_identified);
        assert_eq!(
            citi(None, None, &details),
            Some("123456789619999".to_string())
        );
    }

    #[test]
    fn only_documented_packings_are_repacked() {
        assert_eq!(
            AcquirerKind::Nedbank.banknet_packing(),
            Some(BanknetPacking::ReferenceThenDate)
        );
        assert_eq!(
            AcquirerKind::Cybersource.banknet_packing(),
            Some(BanknetPacking::DateThenReference)
        );
        for kind in [
            AcquirerKind::Absa,
            AcquirerKind::Mpgs,
            AcquirerKind::Planet,
            AcquirerKind::Unknown,
        ] {
            assert_eq!(kind.banknet_packing(), None, "{kind:?}");
        }
    }
}
