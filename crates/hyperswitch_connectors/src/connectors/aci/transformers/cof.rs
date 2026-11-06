//! Card on file, response side: what a payment response records about the agreement its request
//! declared.

use common_enums::enums;
use hyperswitch_domain_models::{
    mandates,
    router_data::RouterData,
    router_response_types::{MandateReference, PaymentsResponseData},
};

use super::{acquirer::AcquirerResolution, flow::PaymentFlow, metadata::AciMetadata};
use crate::connectors::aci::cof_record::{AciCofRecord, AciCofStash, CofRole};

/// What a request was sent as, for the card-on-file record. Decided from what ACI was SENT,
/// never from the response.
#[derive(Debug, Clone)]
pub(super) enum CofRequestRole {
    /// A registration-creating CIT: `createRegistration=true` went out (or, on a sync, the
    /// attempt's stash says it did).
    Cit,
    /// A charge against an existing registration. Carries the request's own record, echoed back
    /// verbatim so every copy core makes of it holds the CIT's values.
    Mit(Option<MandateReference>),
    /// A sync of an attempt that was sent as a MIT.
    MitSync,
    /// A sync of an attempt with no stash: authorized before the stash existed.
    Unknown,
    /// Not a card-on-file leg: a plain payment, a void, or a `setup_future_usage` CIT that asked
    /// for no registration.
    Unrelated,
}

/// The charged registration's record, echoed back on a MIT response.
pub(super) fn echo_charged_record(
    reference: Option<mandates::ConnectorMandateReferenceId>,
) -> Option<MandateReference> {
    let reference = reference?;
    reference
        .get_connector_mandate_id()
        .map(|rg| MandateReference {
            connector_mandate_id: Some(rg),
            payment_method_id: None,
            mandate_metadata: reference.get_mandate_metadata(),
            connector_mandate_request_reference_id: reference
                .get_connector_mandate_request_reference_id(),
        })
}

/// `connector_mandate_request_reference_id` for a sync whose attempt has no card-on-file stash
/// ([`CofRequestRole::Unknown`]): an attempt authorized before the stash existed. Every other
/// response emits the reference through [`card_on_file_emission`], and only a stamped record's
/// `initialTransactionId` is authoritative on the MIT path.
///
/// INVARIANT: this slot may only ever hold a genuine network/scheme trace id captured from the
/// CIT response. A format-plausible substitute (a payment id, a registration id) is strictly
/// worse than absence: it passes parameter validation, the scheme cannot resolve it, and every
/// MIT on the mandate silently never converts — the exact failure the PT-3882 cleanup migration
/// exists to repair. When no source yielded a trace id, store `None`.
///
/// `is_mandate_payment` is the sync's `mandate_id`, which is only a best guess at MIT-ness: core's
/// `new.or(existing)` merge means any `Some` written by a MIT would chain the next MIT to it.
fn mandate_request_reference(citi: Option<String>, is_mandate_payment: bool) -> Option<String> {
    if is_mandate_payment {
        None
    } else {
        citi
    }
}

/// A CIT that finished without a scheme trace id produces a record whose MITs will go out
/// without `standingInstruction.initialTransactionId`. Surface that at capture time, per payment,
/// so an acquirer that stops (or never starts) returning trace ids is caught from logs — not
/// discovered weeks later through MIT decline rates. Only a final response can say this: a 3DS
/// redirect never carries a trace id.
fn warn_missing_cit_trace(payment_id: &str, acquirer: Option<AcquirerResolution>) {
    router_env::logger::warn!(
        connector_transaction_id = payment_id,
        acquirer = ?acquirer.map(|r| r.acquirer),
        acquirer_positively_identified = acquirer.map(|r| r.positively_identified),
        "aci: CIT finalised but no scheme trace id was captured from the response; MITs on this \
         registration will omit standingInstruction.initialTransactionId"
    );
}

/// Whether the attempt has reached an outcome a card-on-file record may be written from.
fn is_final_attempt_status(status: enums::AttemptStatus) -> bool {
    matches!(
        status,
        enums::AttemptStatus::Charged
            | enums::AttemptStatus::Authorized
            | enums::AttemptStatus::PartiallyAuthorized
    )
}

/// The card-on-file fields of one ACI payment response.
pub(super) struct CofEmission {
    pub(super) mandate_reference: Option<MandateReference>,
    pub(super) connector_metadata: Option<serde_json::Value>,
    pub(super) network_txn_id: Option<String>,
    pub(super) network_txn_link_id: Option<String>,
}

/// What an ACI payment response's values become, given what the request was sent as.
///
/// Core writes a payment method's mandate record from the first successful response that carries
/// a `mandate_reference`, and rewrites an Active ACI record only when the response finalising a
/// CIT carries a different registration (a re-CIT supersedes the old agreement); a re-sync of an
/// already-final CIT never rewrites it. A legacy mandate row is filled only while it has no
/// connector mandate id. So the first `mandate_reference` emitted for a registration must already
/// be complete, no stamp may carry another registration's values, and nothing a MIT returns may
/// ever be emitted into it:
///
/// * CIT, not final (3DS redirect, pending): no `mandate_reference` at all, even with a
///   `registrationId`. The CITI and TLID arrive only once the shopper authenticates; emitting the
///   registration now would let core lock a record (or a mandate row) without them. The role and
///   sent type are stashed on the attempt for the sync that finishes it.
/// * CIT, final: the complete [`AciCofRecord`] stamp. The registration is the response's, else
///   the stash's, else the one core already holds, so the record is written even when the sync
///   does not echo `registrationId`.
/// * MIT authorize: the request's own record, echoed verbatim. The MIT's identifiers stay on the
///   attempt (`network_txn_id`) where merchants see them, and never reach the record.
/// * MIT sync: nothing. Core copies a sync's `network_txn_id` onto the payment method whenever
///   the intent was `setup_future_usage=off_session`, and that column is the legacy CITI
///   fallback; the attempt keeps the values its authorize response wrote.
/// * Unknown (an attempt from before the stash): the previous behaviour.
pub(super) fn card_on_file_emission<F, Req: PaymentFlow>(
    router_data: &RouterData<F, Req, PaymentsResponseData>,
    status: enums::AttemptStatus,
    response_rg: Option<String>,
    citi: Option<String>,
    agreement_id: Option<String>,
    connector_transaction_id: &str,
    acquirer: Option<AcquirerResolution>,
) -> CofEmission {
    let request = &router_data.request;
    // Only a CIT whose series type is not yet recorded needs the account's settings.
    let metadata = || AciMetadata::from_connector_meta(router_data.connector_meta_data.as_ref());
    match request.cof_role() {
        CofRequestRole::Cit => {
            let stash = request.cof_stash();
            if !is_final_attempt_status(status) {
                // A sync that is still not final keeps the stash its authorize wrote: `None`
                // never clears the attempt's metadata.
                let connector_metadata = stash.is_none().then(|| {
                    AciCofStash {
                        rg: response_rg,
                        si_type: request.cof_series_type_sent(metadata()),
                        ..AciCofStash::role(CofRole::Cit)
                    }
                    .to_connector_meta()
                });
                return CofEmission {
                    mandate_reference: None,
                    connector_metadata,
                    network_txn_id: citi,
                    network_txn_link_id: agreement_id,
                };
            }

            let stash = stash.unwrap_or_else(|| AciCofStash::role(CofRole::Cit));
            if let (Some(response), Some(stashed)) = (response_rg.as_ref(), stash.rg.as_ref()) {
                if response != stashed {
                    router_env::logger::warn!(
                        connector_transaction_id,
                        "aci: CIT finalised with a different registrationId than its authorize \
                         returned; recording the final one"
                    );
                }
            }
            let rg = response_rg
                .or(stash.rg)
                .or_else(|| request.cof_known_registration());
            // A stash already marked final is what the record was written from; re-emitting it
            // verbatim keeps every later write of the record identical.
            let (citi, agreement_id) = if stash.is_final {
                (stash.citi, stash.agreement_id)
            } else {
                (citi, agreement_id)
            };
            let si_type = stash
                .si_type
                .or_else(|| request.cof_series_type_sent(metadata()));

            if rg.is_none() {
                router_env::logger::warn!(
                    connector_transaction_id,
                    "aci: CIT finalised without registrationId; no card-on-file record can be \
                     written"
                );
            }
            if citi.is_none() {
                warn_missing_cit_trace(connector_transaction_id, acquirer);
            }

            let mandate_reference = rg.clone().map(|rg| {
                let record = AciCofRecord::new(
                    rg.clone(),
                    citi.clone(),
                    agreement_id.clone(),
                    si_type.clone(),
                );
                MandateReference {
                    connector_mandate_id: Some(rg),
                    payment_method_id: None,
                    // Written for compatibility with builds that read the CITI from here. The
                    // stamp in `mandate_metadata` is what MITs read.
                    connector_mandate_request_reference_id: record.initial_transaction_id.clone(),
                    mandate_metadata: Some(record.to_metadata()),
                }
            });
            let connector_metadata = AciCofStash {
                role: CofRole::Cit,
                is_final: true,
                rg,
                citi: citi.clone(),
                agreement_id: agreement_id.clone(),
                si_type,
            }
            .to_connector_meta();

            CofEmission {
                mandate_reference,
                connector_metadata: Some(connector_metadata),
                network_txn_id: citi,
                network_txn_link_id: agreement_id,
            }
        }
        CofRequestRole::Mit(charged_record) => CofEmission {
            mandate_reference: charged_record,
            connector_metadata: Some(AciCofStash::role(CofRole::Mit).to_connector_meta()),
            network_txn_id: citi,
            network_txn_link_id: agreement_id,
        },
        CofRequestRole::MitSync => CofEmission {
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            network_txn_link_id: None,
        },
        CofRequestRole::Unknown => {
            let is_mandate_payment = request.is_mandate_payment();
            let mandate_reference = response_rg.map(|rg| MandateReference {
                connector_mandate_id: Some(rg),
                payment_method_id: None,
                mandate_metadata: None,
                connector_mandate_request_reference_id: mandate_request_reference(
                    citi.clone(),
                    is_mandate_payment,
                ),
            });
            if mandate_reference.is_some()
                && citi.is_none()
                && !is_mandate_payment
                && is_final_attempt_status(status)
            {
                warn_missing_cit_trace(connector_transaction_id, acquirer);
            }
            CofEmission {
                mandate_reference,
                connector_metadata: None,
                network_txn_id: citi,
                network_txn_link_id: agreement_id,
            }
        }
        CofRequestRole::Unrelated => CofEmission {
            mandate_reference: response_rg.map(|rg| MandateReference {
                connector_mandate_id: Some(rg),
                payment_method_id: None,
                mandate_metadata: None,
                connector_mandate_request_reference_id: citi.clone(),
            }),
            connector_metadata: None,
            network_txn_id: citi,
            network_txn_link_id: agreement_id,
        },
    }
}

#[cfg(test)]
mod tests {

    use hyperswitch_domain_models::router_request_types::PaymentsSyncData;
    use hyperswitch_masking::PeekInterface;

    use super::*;
    use crate::connectors::aci::test_fixtures::*;

    /// The stored MIT replay reference may only ever hold a genuine scheme trace id captured
    /// from the CIT: a fabricated stand-in (payment id, registration id) passes validation but
    /// can never be resolved by the scheme, so MITs never convert. When capture fails, the slot
    /// stays empty (and the capture gap is logged). A MIT must never write the slot at all — the
    /// `new.or(existing)` merge would chain each MIT to the previous MIT instead of the CIT.
    #[test]
    fn mandate_request_reference_holds_trace_ids_only_and_is_never_written_by_mits() {
        // CIT with a captured scheme reference: stored as-is.
        assert_eq!(
            mandate_request_reference(Some("MCC00013N0417".to_string()), false),
            Some("MCC00013N0417".to_string())
        );
        // CIT where no source yielded a trace id: stays empty — never fabricated.
        assert_eq!(mandate_request_reference(None, false), None);
        // MIT: never written, even when the MIT response carried its own scheme reference —
        // the stored CIT value survives via the core's merge.
        assert_eq!(
            mandate_request_reference(Some("MIT-OWN-TRACE".to_string()), true),
            None
        );
        assert_eq!(mandate_request_reference(None, true), None);
    }

    // ---- card-on-file emission and MIT linkage ----

    fn emit<F, Req: PaymentFlow>(
        router_data: &RouterData<F, Req, PaymentsResponseData>,
        status: enums::AttemptStatus,
        response_rg: Option<&str>,
        citi: Option<&str>,
        agreement_id: Option<&str>,
    ) -> CofEmission {
        card_on_file_emission(
            router_data,
            status,
            response_rg.map(String::from),
            citi.map(String::from),
            agreement_id.map(String::from),
            "8ac7a4a1payment",
            None,
        )
    }

    /// `(connector_mandate_id, connector_mandate_request_reference_id, mandate_metadata)`.
    fn reference_parts(
        reference: &MandateReference,
    ) -> (Option<String>, Option<String>, Option<serde_json::Value>) {
        assert_eq!(reference.payment_method_id, None);
        (
            reference.connector_mandate_id.clone(),
            reference.connector_mandate_request_reference_id.clone(),
            reference
                .mandate_metadata
                .as_ref()
                .map(|meta| meta.peek().clone()),
        )
    }

    fn complete_stamp() -> (Option<String>, Option<String>, Option<serde_json::Value>) {
        (
            Some(COF_RG.to_string()),
            Some(COF_CITI.to_string()),
            Some(
                AciCofRecord::new(
                    COF_RG.to_string(),
                    Some(COF_CITI.to_string()),
                    Some(COF_TLID.to_string()),
                    Some("UNSCHEDULED".to_string()),
                )
                .to_metadata()
                .peek()
                .clone(),
            ),
        )
    }

    fn final_cit_stash() -> AciCofStash {
        AciCofStash {
            role: CofRole::Cit,
            is_final: true,
            rg: Some(COF_RG.to_string()),
            citi: Some(COF_CITI.to_string()),
            agreement_id: Some(COF_TLID.to_string()),
            si_type: Some("UNSCHEDULED".to_string()),
        }
    }

    fn emitted_stash(emission: &CofEmission) -> Option<AciCofStash> {
        AciCofStash::from_connector_meta(emission.connector_metadata.as_ref())
    }

    /// A 3DS-challenged CIT: the registration is known but the CITI/TLID are not, so nothing may
    /// reach the mandate record yet. The role, registration and sent type are stashed for the sync.
    #[test]
    fn pending_cit_emits_no_mandate_reference_and_stashes_its_role() {
        let router_data = cof_router_data::<(), _, PaymentsResponseData>(cit_authorize_data());
        let emission = emit(
            &router_data,
            enums::AttemptStatus::AuthenticationPending,
            Some(COF_RG),
            None,
            None,
        );
        assert!(emission.mandate_reference.is_none());
        assert_eq!(
            emitted_stash(&emission),
            Some(AciCofStash {
                rg: Some(COF_RG.to_string()),
                si_type: Some("UNSCHEDULED".to_string()),
                ..AciCofStash::role(CofRole::Cit)
            })
        );
        assert_eq!(emission.network_txn_id, None);
        assert_eq!(emission.network_txn_link_id, None);
    }

    /// A sync that is still not final leaves the authorize's stash in place (`None` never clears
    /// the attempt's metadata) and still emits no mandate reference.
    #[test]
    fn still_pending_cit_sync_keeps_the_existing_stash() {
        let stash = AciCofStash {
            rg: Some(COF_RG.to_string()),
            si_type: Some("UNSCHEDULED".to_string()),
            ..AciCofStash::role(CofRole::Cit)
        };
        let router_data = cof_router_data::<(), _, PaymentsResponseData>(sync_data(Some(&stash)));
        let emission = emit(
            &router_data,
            enums::AttemptStatus::AuthenticationPending,
            Some(COF_RG),
            None,
            None,
        );
        assert!(emission.mandate_reference.is_none());
        assert!(emission.connector_metadata.is_none());
    }

    /// A frictionless CIT is final on its authorize response: the complete stamp is emitted at
    /// once.
    #[test]
    fn final_cit_authorize_emits_the_complete_stamp() {
        let router_data = cof_router_data::<(), _, PaymentsResponseData>(cit_authorize_data());
        let emission = emit(
            &router_data,
            enums::AttemptStatus::Charged,
            Some(COF_RG),
            Some(COF_CITI),
            Some(COF_TLID),
        );
        let reference = emission.mandate_reference.as_ref().expect("a stamp");
        assert_eq!(reference_parts(reference), complete_stamp());
        assert_eq!(emitted_stash(&emission), Some(final_cit_stash()));
        assert_eq!(emission.network_txn_id.as_deref(), Some(COF_CITI));
        assert_eq!(emission.network_txn_link_id.as_deref(), Some(COF_TLID));
    }

    /// The sync that finishes a challenged CIT writes the complete stamp, taking the registration
    /// from the stash when the sync response does not echo `registrationId`.
    #[test]
    fn final_cit_sync_takes_the_registration_from_the_stash() {
        let stash = AciCofStash {
            rg: Some(COF_RG.to_string()),
            si_type: Some("UNSCHEDULED".to_string()),
            ..AciCofStash::role(CofRole::Cit)
        };
        let router_data = cof_router_data::<(), _, PaymentsResponseData>(sync_data(Some(&stash)));
        let emission = emit(
            &router_data,
            enums::AttemptStatus::Charged,
            None,
            Some(COF_CITI),
            Some(COF_TLID),
        );
        let reference = emission.mandate_reference.as_ref().expect("a stamp");
        assert_eq!(reference_parts(reference), complete_stamp());
        assert_eq!(emitted_stash(&emission), Some(final_cit_stash()));
    }

    /// A manual-capture CIT synced again after it was finalised re-emits exactly the stamp it was
    /// first written with, whatever the later response says.
    #[test]
    fn resync_of_a_final_cit_re_emits_the_identical_stamp() {
        let router_data =
            cof_router_data::<(), _, PaymentsResponseData>(sync_data(Some(&final_cit_stash())));
        for (citi, agreement_id) in [(None, None), (Some("0912MDUK0QGM7"), Some("TLIDOTHER"))] {
            let emission = emit(
                &router_data,
                enums::AttemptStatus::Authorized,
                None,
                citi,
                agreement_id,
            );
            let reference = emission.mandate_reference.as_ref().expect("a stamp");
            assert_eq!(reference_parts(reference), complete_stamp());
            assert_eq!(emitted_stash(&emission), Some(final_cit_stash()));
            assert_eq!(emission.network_txn_id.as_deref(), Some(COF_CITI));
            assert_eq!(emission.network_txn_link_id.as_deref(), Some(COF_TLID));
        }
    }

    /// A MIT echoes the record it charged verbatim; its own identifiers stay on the attempt.
    #[test]
    fn mit_authorize_echoes_the_charged_record_verbatim() {
        let stamp = AciCofRecord::new(
            COF_RG.to_string(),
            Some(COF_CITI.to_string()),
            Some(COF_TLID.to_string()),
            Some("RECURRING".to_string()),
        )
        .to_metadata()
        .peek()
        .clone();
        let router_data = cof_router_data::<(), _, PaymentsResponseData>(mit_authorize_data(
            COF_RG,
            Some(stamp.clone()),
            Some(COF_CITI),
        ));
        let emission = emit(
            &router_data,
            enums::AttemptStatus::Charged,
            Some(COF_RG),
            Some("0912MDUK0QGM7"),
            Some("TLIDMIT"),
        );
        let reference = emission
            .mandate_reference
            .as_ref()
            .expect("the echoed record");
        assert_eq!(
            reference_parts(reference),
            (
                Some(COF_RG.to_string()),
                Some(COF_CITI.to_string()),
                Some(stamp)
            )
        );
        assert_eq!(
            emitted_stash(&emission),
            Some(AciCofStash::role(CofRole::Mit))
        );
        assert_eq!(emission.network_txn_id.as_deref(), Some("0912MDUK0QGM7"));
        assert_eq!(emission.network_txn_link_id.as_deref(), Some("TLIDMIT"));
    }

    /// A sync of a MIT emits nothing: core would copy its `network_txn_id` onto the payment method.
    #[test]
    fn mit_sync_emits_nothing() {
        let router_data = cof_router_data::<(), _, PaymentsResponseData>(sync_data(Some(
            &AciCofStash::role(CofRole::Mit),
        )));
        let emission = emit(
            &router_data,
            enums::AttemptStatus::Charged,
            Some(COF_RG),
            Some(COF_CITI),
            Some(COF_TLID),
        );
        assert!(emission.mandate_reference.is_none());
        assert!(emission.connector_metadata.is_none());
        assert_eq!(emission.network_txn_id, None);
        assert_eq!(emission.network_txn_link_id, None);
    }

    /// A sync of an attempt authorized before the stash existed keeps the previous behaviour: the
    /// registration with the CITI as request reference, which a mandate payment never writes.
    #[test]
    fn unknown_sync_keeps_the_legacy_behaviour() {
        let router_data = cof_router_data::<(), _, PaymentsResponseData>(sync_data(None));
        let emission = emit(
            &router_data,
            enums::AttemptStatus::Charged,
            Some(COF_RG),
            Some(COF_CITI),
            Some(COF_TLID),
        );
        let reference = emission.mandate_reference.as_ref().expect("a reference");
        assert_eq!(
            reference_parts(reference),
            (Some(COF_RG.to_string()), Some(COF_CITI.to_string()), None)
        );
        assert!(emission.connector_metadata.is_none());
        assert_eq!(emission.network_txn_id.as_deref(), Some(COF_CITI));
        assert_eq!(emission.network_txn_link_id.as_deref(), Some(COF_TLID));

        let mandate_sync = PaymentsSyncData {
            mandate_id: Some(mandates::MandateIds::new("man_test".to_string())),
            ..sync_data(None)
        };
        let router_data = cof_router_data::<(), _, PaymentsResponseData>(mandate_sync);
        let emission = emit(
            &router_data,
            enums::AttemptStatus::Charged,
            Some(COF_RG),
            Some(COF_CITI),
            None,
        );
        let reference = emission.mandate_reference.as_ref().expect("a reference");
        assert_eq!(
            reference_parts(reference),
            (Some(COF_RG.to_string()), None, None)
        );

        let emission = emit(
            &router_data,
            enums::AttemptStatus::Charged,
            None,
            Some(COF_CITI),
            None,
        );
        assert!(emission.mandate_reference.is_none());
    }
}
