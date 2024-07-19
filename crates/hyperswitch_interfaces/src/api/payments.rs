//! Payments interface

use hyperswitch_domain_models::{
    router_flow_types::payments::{
        Approve, Authorize, AuthorizeSessionToken, Capture, CompleteAuthorize,
        CreateConnectorCustomer, IncrementalAuthorization, PSync, PaymentMethodToken,
        PostProcessing, PreProcessing, Reject, Session, SetupMandate, Void,
    },
    router_request_types::{
        AuthorizeSessionTokenData, CompleteAuthorizeData, ConnectorCustomerData,
        PaymentMethodTokenizationData, PaymentsApproveData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsIncrementalAuthorizationData,
        PaymentsPostProcessingData, PaymentsPreProcessingData, PaymentsRejectData,
        PaymentsSessionData, PaymentsSyncData, SetupMandateRequestData,
    },
    router_response_types::PaymentsResponseData,
};

use crate::api;

/// trait Payment
pub trait Payment:
    api::ConnectorCommon
    + api::ConnectorValidation
    + PaymentAuthorize
    + PaymentAuthorizeSessionToken
    + PaymentsCompleteAuthorize
    + PaymentSync
    + PaymentCapture
    + PaymentVoid
    + PaymentApprove
    + PaymentReject
    + MandateSetup
    + PaymentSession
    + PaymentToken
    + PaymentsPreProcessing
    + PaymentsPostProcessing
    + ConnectorCustomer
    + PaymentIncrementalAuthorization
{
}

/// trait PaymentSession
pub trait PaymentSession:
    api::ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData>
{
}

/// trait MandateSetup
pub trait MandateSetup:
    api::ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
{
}

/// trait PaymentAuthorize
pub trait PaymentAuthorize:
    api::ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
{
}

/// trait PaymentCapture
pub trait PaymentCapture:
    api::ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData>
{
}

/// trait PaymentSync
pub trait PaymentSync:
    api::ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData>
{
}

/// trait PaymentVoid
pub trait PaymentVoid:
    api::ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData>
{
}

/// trait PaymentApprove
pub trait PaymentApprove:
    api::ConnectorIntegration<Approve, PaymentsApproveData, PaymentsResponseData>
{
}

/// trait PaymentReject
pub trait PaymentReject:
    api::ConnectorIntegration<Reject, PaymentsRejectData, PaymentsResponseData>
{
}

/// trait PaymentToken
pub trait PaymentToken:
    api::ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
{
}

/// trait PaymentAuthorizeSessionToken
pub trait PaymentAuthorizeSessionToken:
    api::ConnectorIntegration<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>
{
}

/// trait PaymentIncrementalAuthorization
pub trait PaymentIncrementalAuthorization:
    api::ConnectorIntegration<
    IncrementalAuthorization,
    PaymentsIncrementalAuthorizationData,
    PaymentsResponseData,
>
{
}

/// trait PaymentsCompleteAuthorize
pub trait PaymentsCompleteAuthorize:
    api::ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>
{
}

/// trait ConnectorCustomer
pub trait ConnectorCustomer:
    api::ConnectorIntegration<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>
{
}

/// trait PaymentsPreProcessing
pub trait PaymentsPreProcessing:
    api::ConnectorIntegration<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>
{
}

/// trait PaymentsPostProcessing
pub trait PaymentsPostProcessing:
    api::ConnectorIntegration<PostProcessing, PaymentsPostProcessingData, PaymentsResponseData>
{
}
