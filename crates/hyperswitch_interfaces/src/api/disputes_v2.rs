//! Disputes V2 interface
use hyperswitch_domain_models::{
    router_data_v2::DisputesFlowData,
    router_flow_types::dispute::{Accept, Defend, Evidence},
    router_request_types::{
        AcceptDisputeRequestData, DefendDisputeRequestData, SubmitEvidenceRequestData,
    },
    router_response_types::{AcceptDisputeResponse, DefendDisputeResponse, SubmitEvidenceResponse},
};

use crate::{api::ConnectorIntegrationV2, errors};

/// trait AcceptDisputeV2
pub trait AcceptDisputeV2:
    ConnectorIntegrationV2<
    Accept,
    DisputesFlowData,
    AcceptDisputeRequestData,
    AcceptDisputeResponse,
    Error = errors::ConnectorError,
>
{
}

/// trait SubmitEvidenceV2
pub trait SubmitEvidenceV2:
    ConnectorIntegrationV2<
    Evidence,
    DisputesFlowData,
    SubmitEvidenceRequestData,
    SubmitEvidenceResponse,
    Error = errors::ConnectorError,
>
{
}

/// trait DefendDisputeV2
pub trait DefendDisputeV2:
    ConnectorIntegrationV2<
    Defend,
    DisputesFlowData,
    DefendDisputeRequestData,
    DefendDisputeResponse,
    Error = errors::ConnectorError,
>
{
}

/// trait DisputeV2
pub trait DisputeV2:
    super::ConnectorCommon + AcceptDisputeV2 + SubmitEvidenceV2 + DefendDisputeV2
{
}
