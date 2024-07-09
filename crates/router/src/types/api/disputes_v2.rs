use hyperswitch_domain_models::{
    router_data_v2::DisputesFlowData,
    router_flow_types::dispute::{Accept, Defend, Evidence},
};

use crate::{services, types};

pub trait AcceptDisputeV2:
    services::ConnectorIntegrationV2<
    Accept,
    DisputesFlowData,
    types::AcceptDisputeRequestData,
    types::AcceptDisputeResponse,
>
{
}

pub trait SubmitEvidenceV2:
    services::ConnectorIntegrationV2<
    Evidence,
    DisputesFlowData,
    types::SubmitEvidenceRequestData,
    types::SubmitEvidenceResponse,
>
{
}

pub trait DefendDisputeV2:
    services::ConnectorIntegrationV2<
    Defend,
    DisputesFlowData,
    types::DefendDisputeRequestData,
    types::DefendDisputeResponse,
>
{
}

pub trait DisputeV2:
    super::ConnectorCommon + AcceptDisputeV2 + SubmitEvidenceV2 + DefendDisputeV2
{
}
