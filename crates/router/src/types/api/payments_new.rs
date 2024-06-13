use hyperswitch_domain_models::{
    router_data_new::PaymentFlowData,
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

pub trait PaymentAuthorizeNew:
    api::ConnectorIntegrationNew<
    Authorize,
    PaymentFlowData,
    types::PaymentsAuthorizeData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentAuthorizeSessionTokenNew:
    api::ConnectorIntegrationNew<
    AuthorizeSessionToken,
    PaymentFlowData,
    types::AuthorizeSessionTokenData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentSyncNew:
    api::ConnectorIntegrationNew<
    PSync,
    PaymentFlowData,
    types::PaymentsSyncData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentVoidNew:
    api::ConnectorIntegrationNew<
    Void,
    PaymentFlowData,
    types::PaymentsCancelData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentApproveNew:
    api::ConnectorIntegrationNew<
    Approve,
    PaymentFlowData,
    types::PaymentsApproveData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentRejectNew:
    api::ConnectorIntegrationNew<
    Reject,
    PaymentFlowData,
    types::PaymentsRejectData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentCaptureNew:
    api::ConnectorIntegrationNew<
    Capture,
    PaymentFlowData,
    types::PaymentsCaptureData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentSessionNew:
    api::ConnectorIntegrationNew<
    Session,
    PaymentFlowData,
    types::PaymentsSessionData,
    types::PaymentsResponseData,
>
{
}

pub trait MandateSetupNew:
    api::ConnectorIntegrationNew<
    SetupMandate,
    PaymentFlowData,
    types::SetupMandateRequestData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentIncrementalAuthorizationNew:
    api::ConnectorIntegrationNew<
    IncrementalAuthorization,
    PaymentFlowData,
    types::PaymentsIncrementalAuthorizationData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentsCompleteAuthorizeNew:
    api::ConnectorIntegrationNew<
    CompleteAuthorize,
    PaymentFlowData,
    types::CompleteAuthorizeData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentTokenNew:
    api::ConnectorIntegrationNew<
    PaymentMethodToken,
    PaymentFlowData,
    types::PaymentMethodTokenizationData,
    types::PaymentsResponseData,
>
{
}

pub trait ConnectorCustomerNew:
    api::ConnectorIntegrationNew<
    CreateConnectorCustomer,
    PaymentFlowData,
    types::ConnectorCustomerData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentsPreProcessingNew:
    api::ConnectorIntegrationNew<
    PreProcessing,
    PaymentFlowData,
    types::PaymentsPreProcessingData,
    types::PaymentsResponseData,
>
{
}

pub trait PaymentNew:
    api_types::ConnectorCommon
    + api_types::ConnectorValidation
    + PaymentAuthorizeNew
    + PaymentAuthorizeSessionTokenNew
    + PaymentsCompleteAuthorizeNew
    + PaymentSyncNew
    + PaymentCaptureNew
    + PaymentVoidNew
    + PaymentApproveNew
    + PaymentRejectNew
    + MandateSetupNew
    + PaymentSessionNew
    + PaymentTokenNew
    + PaymentsPreProcessingNew
    + ConnectorCustomerNew
    + PaymentIncrementalAuthorizationNew
{
}
