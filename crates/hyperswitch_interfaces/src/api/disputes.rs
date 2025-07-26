//! Disputes interface

use hyperswitch_domain_models::{
    router_flow_types::dispute::{Accept, Defend, Dsync, Evidence, Fetch},
    router_request_types::{
        AcceptDisputeRequestData, DefendDisputeRequestData, DisputeSyncData,
        FetchDisputesRequestData, SubmitEvidenceRequestData,
    },
    router_response_types::{
        AcceptDisputeResponse, DefendDisputeResponse, DisputeSyncResponse, FetchDisputesResponse,
        SubmitEvidenceResponse,
    },
};

use crate::api::ConnectorIntegration;

/// trait AcceptDispute
pub trait AcceptDispute:
    ConnectorIntegration<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>
{
}

/// trait SubmitEvidence
pub trait SubmitEvidence:
    ConnectorIntegration<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>
{
}

/// trait DefendDispute
pub trait DefendDispute:
    ConnectorIntegration<Defend, DefendDisputeRequestData, DefendDisputeResponse>
{
}

/// trait Dispute
pub trait Dispute:
    super::ConnectorCommon
    + AcceptDispute
    + SubmitEvidence
    + DefendDispute
    + FetchDisputes
    + DisputeSync
{
}

/// trait FetchDisputes
pub trait FetchDisputes:
    ConnectorIntegration<Fetch, FetchDisputesRequestData, FetchDisputesResponse>
{
}

/// trait SyncDisputes
pub trait DisputeSync: ConnectorIntegration<Dsync, DisputeSyncData, DisputeSyncResponse> {}
