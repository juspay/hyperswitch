//! The flows whose response is a payment response — authorize, sync, void and mandate setup —
//! and what that response needs to know about the request it answers.

use common_enums::enums;
use common_utils::types::MinorUnit;
use hyperswitch_domain_models::router_request_types::{
    AuthenticationData, PaymentsAuthorizeData, PaymentsCancelData, PaymentsSyncData,
    SetupMandateRequestData,
};

use super::{
    cof::{echo_charged_record, CofRequestRole},
    metadata::AciMetadata,
    payment::{recorded_series_type, CofLeg},
    result::ResultClass,
};
use crate::{
    connectors::aci::cof_record::{AciCofStash, CofRole},
    utils::PaymentsAuthorizeRequestData,
};

/// A flow answered by [`super::response::AciPaymentsResponse`].
pub(super) trait PaymentFlow {
    fn capture_method(&self) -> Option<enums::CaptureMethod>;

    /// The attempt status a success settles at, given the intent's capture method.
    fn success_status(&self, auto_capture: bool) -> enums::AttemptStatus {
        if auto_capture {
            enums::AttemptStatus::Charged
        } else {
            enums::AttemptStatus::Authorized
        }
    }

    /// The attempt status a response reports when it carries no redirect.
    fn attempt_status(&self, class: ResultClass) -> enums::AttemptStatus {
        let auto_capture = matches!(
            self.capture_method(),
            Some(enums::CaptureMethod::Automatic) | None
        );
        match class {
            ResultClass::Success => self.success_status(auto_capture),
            ResultClass::Pending => enums::AttemptStatus::Authorizing,
            ResultClass::Failure => enums::AttemptStatus::Failure,
        }
    }

    /// The 3DS result this request carried in, when the authentication happened outside the
    /// payment call — an external authentication connector, or ACI's own standalone
    /// `/v1/threeDSecure`.
    ///
    /// On that path ACI is handed the evidence rather than producing it, so its payment response
    /// need not echo a `threeDSecure` block, and the merchant-visible `authentication_data` would
    /// otherwise stay null for a payment that was demonstrably authenticated. Only the flows that
    /// can carry one override this; a sync or a void never does.
    fn external_authentication(&self) -> Option<&AuthenticationData> {
        None
    }

    /// What this request was sent as. See [`CofRequestRole`].
    fn cof_role(&self) -> CofRequestRole {
        CofRequestRole::Unrelated
    }

    /// Whether this request charges an existing mandate.
    ///
    /// Only consulted for a sync whose attempt carries no card-on-file stash
    /// ([`CofRequestRole::Unknown`]), to keep that path exactly as it was. For a sync the value
    /// is the attempt's `mandate_id`, which is set for a legacy-mandate CIT and unset for a token
    /// MIT, so it is not a CIT/MIT discriminator; [`Self::cof_role`] is.
    fn is_mandate_payment(&self) -> bool {
        false
    }

    /// The `standingInstruction.type` this request sent, UNSCHEDULED default included. Recorded
    /// on the CIT because the acquirer registers the series under exactly that type, and a MIT
    /// billing a different one does not match it.
    ///
    /// `None` when no type went out, which on a CIT means a MOTO channel: the block is stripped
    /// to `createRegistration`. The channel resolves MCA first, so the account's metadata is
    /// needed to answer.
    fn cof_series_type_sent(&self, _metadata: AciMetadata) -> Option<String> {
        None
    }

    /// The card-on-file state the authorize response left on the attempt. Only a sync has one.
    fn cof_stash(&self) -> Option<AciCofStash> {
        None
    }

    /// The registration core already holds for this attempt, from an earlier response.
    fn cof_known_registration(&self) -> Option<String> {
        None
    }
}

impl PaymentFlow for PaymentsAuthorizeData {
    fn capture_method(&self) -> Option<enums::CaptureMethod> {
        self.capture_method
    }

    fn external_authentication(&self) -> Option<&AuthenticationData> {
        self.authentication_data.as_ref()
    }

    fn cof_role(&self) -> CofRequestRole {
        match CofLeg::of(self) {
            CofLeg::Repeat { .. } => {
                CofRequestRole::Mit(echo_charged_record(self.get_connector_mandate_data()))
            }
            CofLeg::Register => CofRequestRole::Cit,
            CofLeg::OneOff | CofLeg::Initial => CofRequestRole::Unrelated,
        }
    }

    fn cof_series_type_sent(&self, metadata: AciMetadata) -> Option<String> {
        recorded_series_type(self.mit_category, self.payment_channel.as_ref(), metadata)
    }
}

impl PaymentFlow for PaymentsSyncData {
    fn capture_method(&self) -> Option<enums::CaptureMethod> {
        self.capture_method
    }

    /// A zero-amount authorization is an account verification: there is nothing to capture, so it
    /// settles as `Charged` whatever the capture method says.
    ///
    /// This is the path a 3DS-challenged mandate setup finishes on — the shopper returns from the
    /// redirect and the attempt is finalized by PSync rather than by the setup response — so
    /// deferring to `auto_capture` here would park a successfully stored credential in
    /// `requires_capture`, awaiting a capture that can never be made.
    fn success_status(&self, auto_capture: bool) -> enums::AttemptStatus {
        if auto_capture || self.amount == MinorUnit::zero() {
            enums::AttemptStatus::Charged
        } else {
            enums::AttemptStatus::Authorized
        }
    }

    fn cof_role(&self) -> CofRequestRole {
        match self.cof_stash().map(|stash| stash.role) {
            Some(CofRole::Cit) => CofRequestRole::Cit,
            Some(CofRole::Mit) => CofRequestRole::MitSync,
            None => CofRequestRole::Unknown,
        }
    }

    fn is_mandate_payment(&self) -> bool {
        self.mandate_id.is_some()
    }

    fn cof_stash(&self) -> Option<AciCofStash> {
        AciCofStash::from_connector_meta(self.connector_meta.as_ref())
    }

    fn cof_known_registration(&self) -> Option<String> {
        self.connector_mandate_id.clone()
    }
}

impl PaymentFlow for PaymentsCancelData {
    fn capture_method(&self) -> Option<enums::CaptureMethod> {
        None
    }

    fn success_status(&self, _auto_capture: bool) -> enums::AttemptStatus {
        enums::AttemptStatus::Voided
    }
}

impl PaymentFlow for SetupMandateRequestData {
    fn capture_method(&self) -> Option<enums::CaptureMethod> {
        self.capture_method
    }

    /// A successful setup is `Charged` whatever the intent's capture method says.
    ///
    /// The setup itself is a zero-amount preauthorization, so there is nothing to capture and no
    /// funds to settle. Deferring to `auto_capture` would report `Authorized` on a manual-capture
    /// intent, parking the attempt in `requires_capture` awaiting a capture that can never be
    /// made — and hiding a stored credential that is in fact ready to bill.
    fn success_status(&self, _auto_capture: bool) -> enums::AttemptStatus {
        enums::AttemptStatus::Charged
    }

    fn external_authentication(&self) -> Option<&AuthenticationData> {
        self.authentication_data.as_ref()
    }

    /// Always a CIT: the setup is sent as `mode=INITIAL` with `createRegistration=true`
    /// whatever else the request carries.
    fn cof_role(&self) -> CofRequestRole {
        CofRequestRole::Cit
    }

    fn cof_series_type_sent(&self, metadata: AciMetadata) -> Option<String> {
        recorded_series_type(self.mit_category, self.payment_channel.as_ref(), metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::aci::test_fixtures::*;

    #[test]
    fn a_pending_result_is_still_authorizing_and_a_failure_fails() {
        let request = cof_authorize_data();
        assert_eq!(
            request.attempt_status(ResultClass::Pending),
            enums::AttemptStatus::Authorizing
        );
        assert_eq!(
            request.attempt_status(ResultClass::Failure),
            enums::AttemptStatus::Failure
        );
    }

    /// Only an automatic (or unstated) capture method charges on success; sequential automatic
    /// capture is treated as a manual one here.
    #[test]
    fn success_settles_by_capture_method_and_flow() {
        let authorize = |capture_method| PaymentsAuthorizeData {
            capture_method,
            ..cof_authorize_data()
        };
        let charged = enums::AttemptStatus::Charged;
        let authorized = enums::AttemptStatus::Authorized;

        assert_eq!(
            authorize(None).attempt_status(ResultClass::Success),
            charged
        );
        assert_eq!(
            authorize(Some(enums::CaptureMethod::Automatic)).attempt_status(ResultClass::Success),
            charged
        );
        assert_eq!(
            authorize(Some(enums::CaptureMethod::Manual)).attempt_status(ResultClass::Success),
            authorized
        );
        assert_eq!(
            authorize(Some(enums::CaptureMethod::SequentialAutomatic))
                .attempt_status(ResultClass::Success),
            authorized
        );

        let manual_zero_amount_sync = PaymentsSyncData {
            capture_method: Some(enums::CaptureMethod::Manual),
            amount: MinorUnit::zero(),
            ..Default::default()
        };
        assert_eq!(
            manual_zero_amount_sync.attempt_status(ResultClass::Success),
            charged
        );
        assert_eq!(
            PaymentsCancelData::default().attempt_status(ResultClass::Success),
            enums::AttemptStatus::Voided
        );
    }
}
