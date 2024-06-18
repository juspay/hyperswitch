use hyperswitch_domain_models::{
    router_data_v2::PaymentFlowData,
    router_flow_types::payments::{
        Approve, Authorize, AuthorizeSessionToken, Capture, CompleteAuthorize,
        CreateConnectorCustomer, IncrementalAuthorization, PSync, PaymentMethodToken,
        PreProcessing, Reject, Session, SetupMandate, Void,
    },
};

use crate::{
    services::api,
    types::{self, api as api_types},
};

pub trait PaymentAuthorizeV2:
    api::ConnectorIntegrationV2<
    Authorize,
    PaymentFlowData,
    types::PaymentsAuthorizeData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentAuthorizeSessionTokenV2:
    api::ConnectorIntegrationV2<
    AuthorizeSessionToken,
    PaymentFlowData,
    types::AuthorizeSessionTokenData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentSyncV2:
    api::ConnectorIntegrationV2<
    PSync,
    PaymentFlowData,
    types::PaymentsSyncData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentVoidV2:
    api::ConnectorIntegrationV2<
    Void,
    PaymentFlowData,
    types::PaymentsCancelData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentApproveV2:
    api::ConnectorIntegrationV2<
    Approve,
    PaymentFlowData,
    types::PaymentsApproveData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentRejectV2:
    api::ConnectorIntegrationV2<
    Reject,
    PaymentFlowData,
    types::PaymentsRejectData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentCaptureV2:
    api::ConnectorIntegrationV2<
    Capture,
    PaymentFlowData,
    types::PaymentsCaptureData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentSessionV2:
    api::ConnectorIntegrationV2<
    Session,
    PaymentFlowData,
    types::PaymentsSessionData,
    types::PaymentsResponseData,
>
{
}

pub trait MandateSetupV2:
    api::ConnectorIntegrationV2<
    SetupMandate,
    PaymentFlowData,
    types::SetupMandateRequestData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentIncrementalAuthorizationV2:
    api::ConnectorIntegrationV2<
    IncrementalAuthorization,
    PaymentFlowData,
    types::PaymentsIncrementalAuthorizationData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentsCompleteAuthorizeV2:
    api::ConnectorIntegrationV2<
    CompleteAuthorize,
    PaymentFlowData,
    types::CompleteAuthorizeData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentTokenV2:
    api::ConnectorIntegrationV2<
    PaymentMethodToken,
    PaymentFlowData,
    types::PaymentMethodTokenizationData,
    types::PaymentsResponseData,
>
{
}

pub trait ConnectorCustomerV2:
    api::ConnectorIntegrationV2<
    CreateConnectorCustomer,
    PaymentFlowData,
    types::ConnectorCustomerData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentsPreProcessingV2:
    api::ConnectorIntegrationV2<
    PreProcessing,
    PaymentFlowData,
    types::PaymentsPreProcessingData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentV2:
    api_types::ConnectorCommon
    + api_types::ConnectorValidation
    + PaymentAuthorizeV2
    + PaymentAuthorizeSessionTokenV2
    + PaymentsCompleteAuthorizeV2
    + PaymentSyncV2
    + PaymentCaptureV2
    + PaymentVoidV2
    + PaymentApproveV2
    + PaymentRejectV2
    + MandateSetupV2
    + PaymentSessionV2
    + PaymentTokenV2
    + PaymentsPreProcessingV2
    + ConnectorCustomerV2
    + PaymentIncrementalAuthorizationV2
{
}
