use hyperswitch_domain_models::{
    router_data_new::DisputesFlowData,
    router_flow_types::dispute::{Accept, Defend, Evidence},
};

use crate::{services, types};

pub trait AcceptDisputeNew:
    services::ConnectorIntegrationNew<
    Accept,
    DisputesFlowData,
    types::AcceptDisputeRequestData,
    types::AcceptDisputeResponse,
>
{
}

pub trait SubmitEvidenceNew:
    services::ConnectorIntegrationNew<
    Evidence,
    DisputesFlowData,
    types::SubmitEvidenceRequestData,
    types::SubmitEvidenceResponse,
>
{
}

pub trait DefendDisputeNew:
    services::ConnectorIntegrationNew<
    Defend,
    DisputesFlowData,
    types::DefendDisputeRequestData,
    types::DefendDisputeResponse,
>
{
}

pub trait DisputeNew:
    super::ConnectorCommon + AcceptDisputeNew + SubmitEvidenceNew + DefendDisputeNew
{
}
