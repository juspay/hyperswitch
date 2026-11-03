//! Card on file: which leg of a stored-credential agreement a payment is, and what that leg
//! declares in `standingInstruction.*`.
//!
//! An agreement starts with a cardholder-initiated INITIAL leg that registers the card (or, for a
//! merchant that stores credentials itself, only declares the series), and continues with
//! REPEATED charges against the registration — one-click with the shopper present, or
//! merchant-initiated without. A MIT quotes the scheme references its CIT returned so the
//! acquirer can match it to the series; see [`MitLinkage`].

use common_enums::enums;
use hyperswitch_domain_models::{
    mandates::MandateReferenceId, payment_methods::storage_enums::MitCategory,
    router_request_types::PaymentsAuthorizeData,
};
use hyperswitch_masking::PeekInterface;

use super::{authentication::ShopperPresence, channel::Channel, PaymentItem};
use crate::{
    connectors::aci::{
        cof_record::{AciCofRecord, CofRecordView},
        scheme_trace_id::{BanknetPacking, SchemeTraceId},
        transformers::{
            metadata::AciMetadata,
            request::{
                AciIndustryPractice, AciRecurringAmountType, AciRecurringType, Instruction,
                InstructionMode, InstructionSource, InstructionType,
            },
        },
    },
    utils::{self, PaymentsAuthorizeRequestData},
};

/// Which leg of a card-on-file agreement an authorize request is, decided from the request
/// alone.
///
/// Shared by the request builder and the response path, so what is sent and what is recorded
/// cannot drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::connectors::aci::transformers) enum CofLeg {
    /// No agreement.
    OneOff,
    /// The first leg of an agreement, registering the card with ACI (`createRegistration=true`):
    /// a `customer_acceptance` or `mandate_data` with `setup_future_usage=off_session`.
    Register,
    /// The first leg of an agreement ACI keeps no registration for: a bare
    /// `setup_future_usage=off_session` save-card confirm, with neither of the above.
    Initial,
    /// A charge against a stored credential (`mandate_id`). `off_session` is `Some(true)`
    /// whenever a mandate id is present, so the shopper counts as present only on an explicit
    /// `off_session=false`: a one-click charge.
    Repeat { shopper_present: bool },
}

impl CofLeg {
    pub(in crate::connectors::aci::transformers) fn of(request: &PaymentsAuthorizeData) -> Self {
        if request.mandate_id.is_some() {
            Self::Repeat {
                shopper_present: request.off_session == Some(false),
            }
        } else if request.is_cit_mandate_payment() {
            Self::Register
        } else if request.setup_future_usage == Some(enums::FutureUsage::OffSession) {
            Self::Initial
        } else {
            Self::OneOff
        }
    }

    /// Only a merchant-initiated repeat has the shopper absent.
    pub(super) fn shopper_presence(self) -> ShopperPresence {
        match self {
            Self::Repeat {
                shopper_present: false,
            } => ShopperPresence::Absent,
            Self::OneOff | Self::Register | Self::Initial | Self::Repeat { .. } => {
                ShopperPresence::Present
            }
        }
    }
}

/// The card-on-file leg of a payment, with everything its `standingInstruction.*` block declares.
#[derive(Debug)]
pub(super) enum CardOnFile {
    OneOff,
    /// See [`CofLeg::Register`].
    Register(SeriesTerms),
    /// See [`CofLeg::Initial`].
    Initial(SeriesTerms),
    /// A repeat with the shopper in session: the spec's one-click flow, `mode=REPEATED` +
    /// `source=CIT`. Its sample carries neither the scheme reference nor the agreement id.
    OneClick(RepeatTerms),
    /// A repeat the merchant initiates with the shopper absent, quoting the CIT's references.
    MerchantInitiated(RepeatTerms, MitLinkage),
}

impl CardOnFile {
    pub(super) fn resolve(leg: CofLeg, item: PaymentItem<'_>, metadata: AciMetadata) -> Self {
        match leg {
            CofLeg::OneOff => Self::OneOff,
            CofLeg::Register => Self::Register(SeriesTerms::of(item)),
            CofLeg::Initial => Self::Initial(SeriesTerms::of(item)),
            CofLeg::Repeat { shopper_present } => {
                let linkage = MitLinkage::resolve(
                    item,
                    metadata
                        .acquirer
                        .and_then(|acquirer| acquirer.banknet_packing()),
                );
                let terms = RepeatTerms {
                    series: linkage.series.clone(),
                    industry_practice: match item.router_data.request.mit_category {
                        Some(MitCategory::Resubmission) => Some(AciIndustryPractice::Resubmission),
                        _ => None,
                    },
                };
                if shopper_present {
                    Self::OneClick(terms)
                } else {
                    Self::MerchantInitiated(terms, linkage)
                }
            }
        }
    }

    /// The leg a mandate setup always is, whatever the request otherwise looks like.
    ///
    /// Forced rather than derived through [`CofLeg::of`], which only sees a registering leg when
    /// the request *looks* like a CIT mandate payment. A setup intent can reach the flow with
    /// neither `customer_acceptance` nor `mandate_data`, since storing the credential is the whole
    /// request rather than a rider on a charge; deriving would silently drop
    /// `createRegistration` and store nothing.
    pub(super) fn mandate_setup(item: PaymentItem<'_>) -> Self {
        Self::Register(SeriesTerms::of(item))
    }

    /// The `standingInstruction.*` block (and `createRegistration`) this leg sends on `channel`.
    pub(super) fn instruction(&self, channel: Channel) -> Option<Instruction> {
        let instruction = match self {
            Self::OneOff => return None,
            Self::Register(terms) => Instruction {
                create_registration: Some(true),
                ..terms.initial_leg()
            },
            Self::Initial(terms) => terms.initial_leg(),
            Self::OneClick(terms) => {
                terms.repeat_leg(InstructionSource::CardholderInitiatedTransaction)
            }
            // Mastercard requires both the CITI and the agreementId (TLID) during the
            // transitional period; Visa has no agreementId.
            Self::MerchantInitiated(terms, linkage) => Instruction {
                initial_transaction_id: linkage.initial_transaction_id.clone(),
                agreement_id: linkage.agreement_id.clone(),
                ..terms.repeat_leg(InstructionSource::MerchantInitiatedTransaction)
            },
        };
        channel.declare(instruction)
    }

    /// The pre-`standingInstruction` top-level `recurringType`, still stated alongside the block.
    pub(super) fn legacy_recurring_type(&self) -> Option<AciRecurringType> {
        match self {
            Self::Register(_) => Some(AciRecurringType::Initial),
            Self::OneClick(_) | Self::MerchantInitiated(..) => Some(AciRecurringType::Repeated),
            Self::OneOff | Self::Initial(_) => None,
        }
    }
}

/// What the first leg of an agreement declares about the series.
#[derive(Debug)]
pub(super) struct SeriesTerms {
    /// The type the acquirer registers the agreement under, so it must match what the later MIT
    /// bills. Hardcoding UNSCHEDULED registered every agreement as unscheduled and then billed it
    /// as RECURRING/INSTALLMENT.
    series: InstructionType,
    amount_type: Option<AciRecurringAmountType>,
    expiry: Option<String>,
}

impl SeriesTerms {
    fn of(item: PaymentItem<'_>) -> Self {
        let request = &item.router_data.request;
        Self {
            series: InstructionType::for_mit_category(request.mit_category),
            amount_type: AciRecurringAmountType::for_mit_category(request.mit_category),
            expiry: agreement_expiry(request),
        }
    }

    /// The INITIAL leg. Its source is CIT unconditionally: the spec defines INITIAL as the leg
    /// that carries the CVV / 3DS data "to enable an initial authentication" — it presupposes the
    /// cardholder — and no tutorial flow or acquirer sheet pairs INITIAL with MIT.
    fn initial_leg(&self) -> Instruction {
        Instruction {
            mode: Some(InstructionMode::Initial),
            transaction_type: Some(self.series.clone()),
            source: Some(InstructionSource::CardholderInitiatedTransaction),
            recurring_amount_type: self.amount_type.clone(),
            expiry: self.expiry.clone(),
            ..Instruction::default()
        }
    }
}

/// What every repeat leg declares about the charge.
#[derive(Debug)]
pub(super) struct RepeatTerms {
    series: InstructionType,
    industry_practice: Option<AciIndustryPractice>,
}

impl RepeatTerms {
    /// The amount-variability agreement is established on the CIT, not restated per charge.
    fn repeat_leg(&self, source: InstructionSource) -> Instruction {
        Instruction {
            mode: Some(InstructionMode::Repeated),
            transaction_type: Some(self.series.clone()),
            source: Some(source),
            industry_practice: self.industry_practice.clone(),
            ..Instruction::default()
        }
    }
}

/// `standingInstruction.expiry` — the date after which no further authorizations are performed.
///
/// Sourced from the mandate's end date. Absent for a plain `setup_future_usage=off_session` CIT
/// that carries no `mandate_data`, which is legal, so the lookup is swallowed rather than
/// propagated: `get_mandate_details` errors for `MultiUse(None)` and for an absent mandate type,
/// neither of which is a request failure.
fn agreement_expiry(request: &PaymentsAuthorizeData) -> Option<String> {
    utils::get_mandate_details(request.setup_mandate_details.clone())
        .ok()
        .flatten()?
        .end_date?
        .format(&time::macros::format_description!("[year]-[month]-[day]"))
        .ok()
}

impl InstructionType {
    /// `standingInstruction.type` for a card-on-file series.
    ///
    /// `MitCategory::Resubmission` is a merchant-initiated *reason*, not a series type: it rides
    /// on `standingInstruction.industryPractice` and leaves the series UNSCHEDULED.
    pub(in crate::connectors::aci::transformers) fn for_mit_category(
        mit_category: Option<MitCategory>,
    ) -> Self {
        match mit_category {
            Some(MitCategory::Installment) => Self::Installment,
            Some(MitCategory::Recurring | MitCategory::Subscription) => Self::Recurring,
            Some(MitCategory::Unscheduled) | Some(MitCategory::Resubmission) | None => {
                Self::Unscheduled
            }
        }
    }

    /// The wire value, as recorded in the card-on-file record's `siType`.
    pub(in crate::connectors::aci::transformers) fn as_wire_str(&self) -> &'static str {
        match self {
            Self::Unscheduled => "UNSCHEDULED",
            Self::Recurring => "RECURRING",
            Self::Installment => "INSTALLMENT",
        }
    }

    /// A wire value read back from storage.
    fn from_wire_str(value: &str) -> Option<Self> {
        [Self::Installment, Self::Recurring, Self::Unscheduled]
            .into_iter()
            .find(|ty| ty.as_wire_str() == value)
    }
}

impl AciRecurringAmountType {
    /// `standingInstruction.recurringType` — whether the amounts in a recurring series vary.
    ///
    /// Nothing in hyperswitch states whether a series uses a fixed amount:
    /// `MandateAmountData.amount` is a per-mandate cap rather than a fixed instalment. So this
    /// only ever asserts the permissive value, and stays silent where OPPWA's own documented
    /// default already says the right thing (installments default to FIXED, everything else to
    /// VARIABLE).
    fn for_mit_category(mit_category: Option<MitCategory>) -> Option<Self> {
        match mit_category {
            Some(MitCategory::Recurring) => Some(Self::StandingOrder),
            _ => None,
        }
    }
}

/// The `standingInstruction.type` an INITIAL leg on `payment_channel` sends for `mit_category`,
/// as the card-on-file record keeps it: none on a MOTO channel, whose block goes out without a
/// type.
pub(in crate::connectors::aci::transformers) fn recorded_series_type(
    mit_category: Option<MitCategory>,
    payment_channel: Option<&enums::PaymentChannel>,
    metadata: AciMetadata,
) -> Option<String> {
    (!Channel::resolve(metadata, payment_channel).is_moto()).then(|| {
        InstructionType::for_mit_category(mit_category)
            .as_wire_str()
            .to_string()
    })
}

/// What a MIT quotes back so the acquirer can match it to the CIT that registered the series.
#[derive(Debug)]
pub(super) struct MitLinkage {
    initial_transaction_id: Option<String>,
    agreement_id: Option<String>,
    series: InstructionType,
}

impl MitLinkage {
    /// The CIT's linkage values for the registration this MIT charges.
    ///
    /// A record stamped for this registration ([`AciCofRecord`]) is the whole answer: it was
    /// written once, from the CIT's final response, and a value it lacks was absent on the CIT.
    /// Nothing PM-wide is consulted for it, because a payment method's `network_transaction_id`
    /// is shared by every MCA and connector that card was used on, and core rewrites it from
    /// later syncs.
    ///
    /// Records written before stamping fall back to a chain that prefers per-registration
    /// storage:
    ///   * CITI: the record's `connector_mandate_request_reference_id`, then a network mandate
    ///     id, then the payment method's `network_transaction_id`. The first value that is a
    ///     scheme reference wins, so a placeholder or ACI id in the record falls through to the
    ///     next.
    ///   * TLID: the record's `agreementId`, then the payment method's
    ///     `network_transaction_link_id` only when the CITI chosen above IS the payment method's
    ///     `network_transaction_id`. The two payment-method columns are then a pair from the same
    ///     CIT; otherwise the TLID could belong to another agreement, and sending none is the
    ///     documented-safe fallback.
    ///   * Type: the record's `siType`, then this request's own `mit_category`.
    ///
    /// A stamped record without `siType` is a CIT that sent no type (MOTO); the MIT then declares
    /// its own `mit_category`, as there is no registered series type to match.
    ///
    /// Every fallback to a payment-method column is logged, to size the population left
    /// unstamped. `packing` is the destination acquirer's Banknet packing, see [`replay`].
    fn resolve(item: PaymentItem<'_>, packing: Option<BanknetPacking>) -> Self {
        let router_data = item.router_data;
        let request = &router_data.request;
        let payment_id = router_data.payment_id.as_str();
        let replay = |raw: &str, source: &'static str| replay(raw, packing, payment_id, source);

        let recurring = router_data.recurring_mandate_payment_data.as_ref();
        let charged = request.get_connector_mandate_data();
        let charged_rg = charged
            .as_ref()
            .and_then(|reference| reference.get_connector_mandate_id());
        let metadata = charged
            .as_ref()
            .and_then(|reference| reference.get_mandate_metadata())
            .or_else(|| recurring.and_then(|data| data.mandate_metadata.clone()));

        let view = AciCofRecord::from_metadata(
            metadata.as_ref().map(|meta| meta.peek()),
            charged_rg.as_deref(),
        );
        let (record_agreement_id, record_si_type) = match view {
            CofRecordView::Stamped(record) => {
                let series = match record.si_type.as_deref() {
                    // A stamp without a type records a CIT that sent none — a MOTO registration,
                    // whose block carries no `standingInstruction.type`. No series type was
                    // registered, so there is nothing to match and this MIT declares its own.
                    None => InstructionType::for_mit_category(request.mit_category),
                    Some(raw) => InstructionType::from_wire_str(raw).unwrap_or_else(|| {
                        // Every stamp records the type it sent, and only ever a known one.
                        router_env::logger::warn!(
                            payment_id,
                            si_type = raw,
                            "aci: stamped card-on-file record has an unreadable siType; \
                             sending UNSCHEDULED"
                        );
                        InstructionType::Unscheduled
                    }),
                };
                return Self {
                    initial_transaction_id: record
                        .initial_transaction_id
                        .as_deref()
                        .and_then(|raw| replay(raw, "stamped_record")),
                    agreement_id: record.agreement_id,
                    series,
                };
            }
            CofRecordView::Unbound { stamped_rg } => {
                router_env::logger::warn!(
                    payment_id,
                    ?stamped_rg,
                    ?charged_rg,
                    "aci: card-on-file record is stamped for another registration; ignoring it"
                );
                (None, None)
            }
            CofRecordView::Legacy {
                agreement_id,
                si_type,
            } => (agreement_id, si_type),
        };

        let network_mandate_id =
            request
                .mandate_id
                .as_ref()
                .and_then(|mandate| match &mandate.mandate_reference_id {
                    Some(MandateReferenceId::NetworkMandateId(ntid)) => {
                        Some(ntid.network_transaction_id.clone())
                    }
                    _ => None,
                });
        let pm_network_transaction_id =
            recurring.and_then(|data| data.network_transaction_id.clone());
        let pm_network_transaction_link_id =
            recurring.and_then(|data| data.network_transaction_link_id.clone());

        const FROM_PAYMENT_METHOD: &str = "payment_method.network_transaction_id";
        let chosen = [
            (
                "mandate_record.connector_mandate_request_reference_id",
                request.get_connector_mandate_request_reference_id().ok(),
            ),
            ("network_mandate_id", network_mandate_id),
            (FROM_PAYMENT_METHOD, pm_network_transaction_id.clone()),
        ]
        .into_iter()
        .find_map(|(source, raw)| {
            let raw = raw?;
            replay(&raw, source).map(|wire| (source, raw, wire))
        });

        match &chosen {
            Some((source, _, _)) if *source == FROM_PAYMENT_METHOD => {
                router_env::logger::warn!(
                    payment_id,
                    fallback = FROM_PAYMENT_METHOD,
                    "aci: unstamped mandate record holds no replayable CITI; using the payment \
                     method's"
                );
            }
            Some(_) => {}
            None => {
                router_env::logger::warn!(
                    payment_id,
                    "aci: no scheme trace id is stored for this registration; the MIT will omit \
                     standingInstruction.initialTransactionId"
                );
            }
        }

        let chosen_is_pm_pair = chosen
            .as_ref()
            .zip(pm_network_transaction_id.as_ref())
            .is_some_and(|((_, raw, _), pm_nti)| raw == pm_nti);
        let agreement_id = record_agreement_id
            .or_else(|| {
                let pm_tlid = pm_network_transaction_link_id?;
                if chosen_is_pm_pair {
                    router_env::logger::warn!(
                        payment_id,
                        fallback = "payment_method.network_transaction_link_id",
                        "aci: unstamped mandate record holds no agreementId; using the payment \
                         method's"
                    );
                    Some(pm_tlid)
                } else {
                    router_env::logger::warn!(
                        payment_id,
                        fallback = "payment_method.network_transaction_link_id",
                        "aci: withholding the payment method's agreementId: it is not paired \
                         with the CITI this MIT replays"
                    );
                    None
                }
            })
            .or_else(|| request.get_optional_transaction_link_id());

        Self {
            initial_transaction_id: chosen.map(|(_, _, wire)| wire),
            agreement_id,
            series: record_si_type
                .as_deref()
                .and_then(InstructionType::from_wire_str)
                .unwrap_or_else(|| InstructionType::for_mit_category(request.mit_category)),
        }
    }
}

/// A stored scheme trace id, formatted for the acquirer this MIT goes to.
///
/// The value is introspected on its own, wherever it was captured. The packing it is written in
/// comes from the destination: the acquirer declared in connector metadata. A request carries no
/// other signal of where it is going, so an undeclared destination gets the value as stored.
///
/// A stored value that is not a scheme reference at all (an ACI id, or the placeholder PT-4180
/// left on mandates saved during a 3DS challenge) yields `None`: an unresolvable
/// `initialTransactionId` fails the scheme's linkage check, while an absent one is the
/// documented-safe fallback. `source` names where the value was read from, for the log.
fn replay(
    raw: &str,
    packing: Option<BanknetPacking>,
    payment_id: &str,
    source: &'static str,
) -> Option<String> {
    SchemeTraceId::parse(raw)
        .map_err(|rejection| {
            router_env::logger::warn!(
                payment_id,
                source,
                ?rejection,
                "aci: stored scheme trace id is not replayable"
            );
        })
        .ok()
        .map(|trace_id| trace_id.format_for(packing))
}

#[cfg(test)]
mod tests {
    use common_utils::types::StringMajorUnit;
    use hyperswitch_domain_models::{
        mandates, router_data::RecurringMandatePaymentData, types::PaymentsAuthorizeRouterData,
    };
    use hyperswitch_masking::Secret;

    use super::*;
    use crate::connectors::aci::{
        test_fixtures::*,
        transformers::{request::AciTransactionCategory, AciRouterData},
    };

    fn moto() -> Channel {
        Channel::resolve(
            AciMetadata {
                transaction_category: Some(AciTransactionCategory::MailOrTelephoneOrder),
                ..AciMetadata::default()
            },
            None,
        )
    }

    fn encoded(instruction: Option<Instruction>) -> String {
        instruction
            .map(|instruction| {
                serde_urlencoded::to_string(instruction).expect("instruction should form-urlencode")
            })
            .unwrap_or_default()
    }

    fn registering(mit_category: Option<MitCategory>) -> CardOnFile {
        CardOnFile::Register(SeriesTerms {
            series: InstructionType::for_mit_category(mit_category),
            amount_type: AciRecurringAmountType::for_mit_category(mit_category),
            expiry: Some("2027-12-31".to_string()),
        })
    }

    fn merchant_initiated() -> CardOnFile {
        CardOnFile::MerchantInitiated(
            RepeatTerms {
                series: InstructionType::Recurring,
                industry_practice: None,
            },
            MitLinkage {
                initial_transaction_id: Some("CITI-123".to_string()),
                agreement_id: Some("TLID-456".to_string()),
                series: InstructionType::Recurring,
            },
        )
    }

    #[test]
    fn the_leg_is_read_from_the_request() {
        let cases = [
            (cof_authorize_data(), CofLeg::OneOff),
            (cit_authorize_data(), CofLeg::Register),
            (
                PaymentsAuthorizeData {
                    setup_future_usage: Some(enums::FutureUsage::OffSession),
                    ..cof_authorize_data()
                },
                CofLeg::Initial,
            ),
            (
                mit_authorize_data(COF_RG, None, None),
                CofLeg::Repeat {
                    shopper_present: false,
                },
            ),
            (
                PaymentsAuthorizeData {
                    off_session: Some(false),
                    ..mit_authorize_data(COF_RG, None, None)
                },
                CofLeg::Repeat {
                    shopper_present: true,
                },
            ),
            // A mandate id makes it a repeat even with the CIT markers present.
            (
                PaymentsAuthorizeData {
                    mandate_id: mit_authorize_data(COF_RG, None, None).mandate_id,
                    ..cit_authorize_data()
                },
                CofLeg::Repeat {
                    shopper_present: false,
                },
            ),
        ];
        for (request, expected) in cases {
            assert_eq!(CofLeg::of(&request), expected);
        }
    }

    /// Only a merchant-initiated repeat has the shopper absent; a one-click repeat is in session.
    #[test]
    fn only_a_merchant_initiated_repeat_is_unattended() {
        assert_eq!(
            CofLeg::Repeat {
                shopper_present: false
            }
            .shopper_presence(),
            ShopperPresence::Absent
        );
        for leg in [
            CofLeg::OneOff,
            CofLeg::Register,
            CofLeg::Initial,
            CofLeg::Repeat {
                shopper_present: true,
            },
        ] {
            assert_eq!(leg.shopper_presence(), ShopperPresence::Present, "{leg:?}");
        }
    }

    /// The CIT registers the agreement under a type, and the MIT bills against that same series.
    /// Hardcoding UNSCHEDULED on the CIT while deriving RECURRING on the MIT made the two legs
    /// disagree.
    #[test]
    fn standing_instruction_type_follows_the_mit_category() {
        let cases = [
            (Some(MitCategory::Recurring), "RECURRING"),
            (Some(MitCategory::Installment), "INSTALLMENT"),
            (Some(MitCategory::Unscheduled), "UNSCHEDULED"),
            // A resubmission is a MIT *reason*, not a series type — it rides on
            // industryPractice and leaves the series unscheduled.
            (Some(MitCategory::Resubmission), "UNSCHEDULED"),
            (None, "UNSCHEDULED"),
        ];

        for (category, expected) in cases {
            assert_eq!(
                InstructionType::for_mit_category(category).as_wire_str(),
                expected,
                "unexpected standingInstruction.type for {category:?}"
            );
        }
    }

    /// The stored `siType` is the serialized wire value, read back.
    #[test]
    fn instruction_type_wire_strings_match_serde() {
        for ty in [
            InstructionType::Unscheduled,
            InstructionType::Recurring,
            InstructionType::Installment,
        ] {
            assert_eq!(
                serde_json::to_value(&ty).expect("serializes"),
                serde_json::json!(ty.as_wire_str())
            );
            assert_eq!(
                InstructionType::from_wire_str(ty.as_wire_str()).map(|read| read.as_wire_str()),
                Some(ty.as_wire_str())
            );
        }
        assert!(InstructionType::from_wire_str("unscheduled").is_none());
    }

    /// `recurringType` is only truthful for a recurring series, and OPPWA's documented defaults
    /// already cover the other cases, so it is asserted only where it is meaningful.
    #[test]
    fn recurring_amount_type_is_only_sent_for_recurring_series() {
        assert!(matches!(
            AciRecurringAmountType::for_mit_category(Some(MitCategory::Recurring)),
            Some(AciRecurringAmountType::StandingOrder)
        ));
        for category in [
            Some(MitCategory::Installment),
            Some(MitCategory::Unscheduled),
            Some(MitCategory::Resubmission),
            None,
        ] {
            assert!(
                AciRecurringAmountType::for_mit_category(category).is_none(),
                "recurringType should be omitted for {category:?}"
            );
        }
    }

    /// Mandate setup runs as a zero-amount CIT on `/v1/payments`, so the instruction block it
    /// sends is what tells the acquirer a card-on-file agreement was established and what makes
    /// the gateway hand back a registration id. `/v1/registrations` sent none of this, which is
    /// why the resulting mandates had no scheme reference for their MITs to quote.
    #[test]
    fn a_registering_leg_declares_an_initial_cit() {
        let encoded =
            encoded(registering(Some(MitCategory::Recurring)).instruction(Channel::Undeclared));

        assert_eq!(
            encoded,
            "standingInstruction.mode=INITIAL&standingInstruction.type=RECURRING\
             &standingInstruction.source=CIT&createRegistration=true\
             &standingInstruction.recurringType=STANDING_ORDER\
             &standingInstruction.expiry=2027-12-31"
        );
    }

    #[test]
    fn repeat_legs_quote_the_linkage_only_when_merchant_initiated() {
        assert_eq!(
            encoded(merchant_initiated().instruction(Channel::Undeclared)),
            "standingInstruction.mode=REPEATED&standingInstruction.type=RECURRING\
             &standingInstruction.source=MIT&standingInstruction.initialTransactionId=CITI-123\
             &standingInstruction.agreementId=TLID-456"
        );

        let one_click = CardOnFile::OneClick(RepeatTerms {
            series: InstructionType::Unscheduled,
            industry_practice: Some(AciIndustryPractice::Resubmission),
        });
        assert_eq!(
            encoded(one_click.instruction(Channel::Undeclared)),
            "standingInstruction.mode=REPEATED&standingInstruction.type=UNSCHEDULED\
             &standingInstruction.source=CIT&standingInstruction.industryPractice=RESUBMISSION"
        );
    }

    // ---------------------------------------------------------------------
    // MOTO: the channel is the whole statement
    //
    // A mail or telephone order declares no card-on-file agreement. `transactionCategory`
    // goes out, every `standingInstruction.*` field is dropped, and only
    // `createRegistration` survives so the card is still stored.
    // ---------------------------------------------------------------------

    /// A MOTO vaulting leg keeps `createRegistration` and loses everything else, so the card is
    /// still stored while no agreement is declared.
    #[test]
    fn moto_vaulting_keeps_only_create_registration() {
        assert_eq!(
            encoded(registering(Some(MitCategory::Recurring)).instruction(moto())),
            "createRegistration=true"
        );
    }

    /// A MOTO leg that was only ever going to declare an agreement — a later MIT, or a
    /// DB-only mandate — sends no instruction fields at all rather than an empty block.
    #[test]
    fn moto_charge_without_registration_sends_no_instruction() {
        assert!(merchant_initiated().instruction(moto()).is_none());
        let initial = CardOnFile::Initial(SeriesTerms {
            series: InstructionType::Recurring,
            amount_type: None,
            expiry: None,
        });
        assert!(initial.instruction(moto()).is_none());
    }

    /// The legacy flag says INITIAL only for a registering leg and REPEATED for every repeat.
    #[test]
    fn legacy_recurring_type_follows_the_leg() {
        let initial = CardOnFile::Initial(SeriesTerms {
            series: InstructionType::Unscheduled,
            amount_type: None,
            expiry: None,
        });
        assert!(CardOnFile::OneOff.legacy_recurring_type().is_none());
        assert!(initial.legacy_recurring_type().is_none());
        assert!(matches!(
            registering(None).legacy_recurring_type(),
            Some(AciRecurringType::Initial)
        ));
        assert!(matches!(
            merchant_initiated().legacy_recurring_type(),
            Some(AciRecurringType::Repeated)
        ));
    }

    #[test]
    fn a_moto_registration_records_no_series_type() {
        let moto = AciMetadata {
            transaction_category: Some(AciTransactionCategory::MailOrder),
            ..AciMetadata::default()
        };
        assert_eq!(
            recorded_series_type(Some(MitCategory::Recurring), None, moto),
            None
        );
        assert_eq!(
            recorded_series_type(
                Some(MitCategory::Recurring),
                Some(&enums::PaymentChannel::Ecommerce),
                AciMetadata::default()
            )
            .as_deref(),
            Some("RECURRING")
        );
    }

    // ---- MIT linkage ----

    fn linkage(
        request: PaymentsAuthorizeData,
        recurring: Option<RecurringMandatePaymentData>,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let mut router_data: PaymentsAuthorizeRouterData = cof_router_data(request);
        router_data.recurring_mandate_payment_data = recurring;
        let item = AciRouterData::from((StringMajorUnit::zero(), &router_data));
        let MitLinkage {
            initial_transaction_id,
            agreement_id,
            series,
        } = MitLinkage::resolve(&item, None);
        (
            initial_transaction_id,
            agreement_id,
            Some(series.as_wire_str().to_string()),
        )
    }

    fn payment_method_columns(nti: &str, tlid: &str) -> RecurringMandatePaymentData {
        RecurringMandatePaymentData {
            network_transaction_id: Some(nti.to_string()),
            network_transaction_link_id: Some(tlid.to_string()),
            ..Default::default()
        }
    }

    const PM_NTI: &str = "476253540570401";

    const PM_TLID: &str = "TLIDPM";

    const PLACEHOLDER: &str = "PLSWDsdWh5GALKpIZs";

    /// A record stamped for the charged registration is the whole answer: payment-method
    /// columns are never consulted, even for a value the stamp lacks.
    #[test]
    fn stamped_record_for_the_charged_registration_is_authoritative() {
        let stamp = |citi: Option<&str>, agreement_id: Option<&str>| {
            AciCofRecord::new(
                COF_RG.to_string(),
                citi.map(String::from),
                agreement_id.map(String::from),
                Some("RECURRING".to_string()),
            )
            .to_metadata()
            .peek()
            .clone()
        };
        let pm = || Some(payment_method_columns(PM_NTI, PM_TLID));

        assert_eq!(
            linkage(
                mit_authorize_data(COF_RG, Some(stamp(Some(COF_CITI), Some(COF_TLID))), None),
                pm()
            ),
            (
                Some(COF_CITI.to_string()),
                Some(COF_TLID.to_string()),
                Some("RECURRING".to_string())
            )
        );
        // A stamp without CITI/TLID really was absent on the CIT; nothing is filled in.
        assert_eq!(
            linkage(
                mit_authorize_data(COF_RG, Some(stamp(None, None)), None),
                pm()
            ),
            (None, None, Some("RECURRING".to_string()))
        );
        // The record may also arrive through the recurring mandate data.
        assert_eq!(
            linkage(
                mit_authorize_data(COF_RG, None, None),
                Some(RecurringMandatePaymentData {
                    mandate_metadata: Some(Secret::new(stamp(Some(COF_CITI), None))),
                    ..payment_method_columns(PM_NTI, PM_TLID)
                })
            ),
            (
                Some(COF_CITI.to_string()),
                None,
                Some("RECURRING".to_string())
            )
        );
    }

    /// A legacy record's CITI comes from the first scheme-shaped value in the chain: the record's
    /// request reference, then a network mandate id, then the payment method's NTI.
    #[test]
    fn legacy_citi_chain_prefers_per_registration_storage() {
        let pm = || Some(payment_method_columns(PM_NTI, PM_TLID));

        // The record's reference wins over the payment method's NTI.
        let (citi, _, _) = linkage(mit_authorize_data(COF_RG, None, Some(COF_CITI)), pm());
        assert_eq!(citi.as_deref(), Some(COF_CITI));

        // A placeholder (or ACI id) in the record falls through to the next source.
        for rejected in [PLACEHOLDER, "8acda4a6a0e7fd8b01a0f102d66819f8"] {
            let (citi, _, _) = linkage(mit_authorize_data(COF_RG, None, Some(rejected)), pm());
            assert_eq!(citi.as_deref(), Some(PM_NTI), "{rejected}");
        }

        // A network mandate id wins over the payment method's NTI.
        let network_mandate = PaymentsAuthorizeData {
            mandate_id: Some(mandates::MandateIds {
                mandate_id: None,
                mandate_reference_id: Some(MandateReferenceId::NetworkMandateId(
                    mandates::NetworkMandateIdRef {
                        network_transaction_id: "0912MDUK0QGM7".to_string(),
                        transaction_link_id: None,
                    },
                )),
            }),
            ..cof_authorize_data()
        };
        let (citi, _, _) = linkage(network_mandate, pm());
        assert_eq!(citi.as_deref(), Some("0912MDUK0QGM7"));

        // Nothing replayable anywhere: the CITI is omitted.
        assert_eq!(
            linkage(mit_authorize_data(COF_RG, None, Some(PLACEHOLDER)), None),
            (None, None, Some("UNSCHEDULED".to_string()))
        );
    }

    /// The payment method's TLID is only sent as the pair of its NTI: when the CITI chosen is
    /// that NTI. The record's own `agreementId` always wins.
    #[test]
    fn legacy_payment_method_tlid_only_travels_with_its_own_nti() {
        let pm = || Some(payment_method_columns(PM_NTI, PM_TLID));

        // The CITI came from the payment method: its TLID is its pair.
        assert_eq!(
            linkage(mit_authorize_data(COF_RG, None, Some(PLACEHOLDER)), pm()),
            (
                Some(PM_NTI.to_string()),
                Some(PM_TLID.to_string()),
                Some("UNSCHEDULED".to_string())
            )
        );
        // The CITI came from the record: the payment method's TLID may be another agreement's.
        assert_eq!(
            linkage(mit_authorize_data(COF_RG, None, Some(COF_CITI)), pm()),
            (
                Some(COF_CITI.to_string()),
                None,
                Some("UNSCHEDULED".to_string())
            )
        );
        // The record's reference equals the payment method's NTI: still that pair.
        assert_eq!(
            linkage(mit_authorize_data(COF_RG, None, Some(PM_NTI)), pm()),
            (
                Some(PM_NTI.to_string()),
                Some(PM_TLID.to_string()),
                Some("UNSCHEDULED".to_string())
            )
        );
        // Legacy hints in the record win over the payment method.
        let legacy = serde_json::json!({ "agreementId": COF_TLID, "siType": "INSTALLMENT" });
        assert_eq!(
            linkage(
                mit_authorize_data(COF_RG, Some(legacy), Some(PLACEHOLDER)),
                pm()
            ),
            (
                Some(PM_NTI.to_string()),
                Some(COF_TLID.to_string()),
                Some("INSTALLMENT".to_string())
            )
        );
    }

    /// A record stamped for another registration contributes nothing: the legacy chain runs as if
    /// there were no hints at all.
    #[test]
    fn stamp_for_another_registration_is_ignored() {
        let other = AciCofRecord::new(
            "8ac7a4a1another".to_string(),
            Some("0912MDUK0QGM7".to_string()),
            Some("TLIDSTALE".to_string()),
            Some("INSTALLMENT".to_string()),
        )
        .to_metadata()
        .peek()
        .clone();
        assert_eq!(
            linkage(
                mit_authorize_data(COF_RG, Some(other), Some(COF_CITI)),
                Some(payment_method_columns(PM_NTI, PM_TLID))
            ),
            (
                Some(COF_CITI.to_string()),
                None,
                Some("UNSCHEDULED".to_string())
            )
        );
    }

    /// A MIT replays the stored trace id in the destination acquirer's packing.
    #[test]
    fn the_trace_id_is_repacked_for_the_destination_acquirer() {
        assert_eq!(
            replay(
                COF_CITI,
                Some(BanknetPacking::DateThenReference),
                "pay",
                "test"
            )
            .as_deref(),
            Some("0910MDGXVJV4N")
        );
        assert_eq!(
            replay(COF_CITI, None, "pay", "test").as_deref(),
            Some(COF_CITI)
        );
        assert_eq!(replay(PLACEHOLDER, None, "pay", "test"), None);
    }
}
