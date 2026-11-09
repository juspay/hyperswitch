use super::*;

fn tonic_status(code: tonic::Code) -> UnifiedConnectorServiceError {
    UnifiedConnectorServiceError::TonicStatus {
        code,
        message: "from ucs".to_string(),
        transport: None,
    }
}

#[test]
fn ucs_wide_grpc_codes_still_qualify() {
    // While UCS is unwell the payment has no fallback and fails outright, so these qualify
    // like any other. They share a label so a run of them reads as one UCS-wide event.
    for code in [
        tonic::Code::Unavailable,
        tonic::Code::DeadlineExceeded,
        tonic::Code::ResourceExhausted,
        tonic::Code::Aborted,
        tonic::Code::Cancelled,
    ] {
        assert_eq!(
            tonic_status(code).ucs_kill_switch_reason(),
            Some(UcsKillSwitchReason::UcsUnreachable),
            "{code:?}"
        );
    }
}

#[test]
fn ucs_side_grpc_codes_are_kill_switch_worthy() {
    // `from_grpc_error` extracts connector errors and timeouts before producing TonicStatus,
    // so what reaches here is UCS-side and repeats identically for this scope.
    let cases = [
        (
            tonic::Code::InvalidArgument,
            UcsKillSwitchReason::UcsRejectedRequest,
        ),
        (
            tonic::Code::FailedPrecondition,
            UcsKillSwitchReason::UcsRejectedRequest,
        ),
        (
            tonic::Code::Unauthenticated,
            UcsKillSwitchReason::UcsRejectedRequest,
        ),
        (
            tonic::Code::Unimplemented,
            UcsKillSwitchReason::UcsFlowUnsupported,
        ),
        (tonic::Code::Internal, UcsKillSwitchReason::UcsInternalError),
        (tonic::Code::Unknown, UcsKillSwitchReason::UcsInternalError),
    ];

    for (code, expected) in cases {
        assert_eq!(
            tonic_status(code).ucs_kill_switch_reason(),
            Some(expected),
            "{code:?}"
        );
    }
}
