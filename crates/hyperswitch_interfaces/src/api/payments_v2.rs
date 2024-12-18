//! Payments V2 interface

use hyperswitch_domain_models::{
    router_data_v2::PaymentFlowData,
    router_flow_types::payments::{
        Approve, Authorize, AuthorizeSessionToken, CalculateTax, Capture, CompleteAuthorize,
        CreateConnectorCustomer, IncrementalAuthorization, PSync, PaymentMethodToken,
        PostProcessing, PostSessionTokens, PreProcessing, Reject, SdkSessionUpdate, Session,
        SetupMandate, Void,
    },
    router_request_types::{
        AuthorizeSessionTokenData, CompleteAuthorizeData, ConnectorCustomerData,
        PaymentMethodTokenizationData, PaymentsApproveData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsIncrementalAuthorizationData,
        PaymentsPostProcessingData, PaymentsPostSessionTokensData, PaymentsPreProcessingData,
        PaymentsRejectData, PaymentsSessionData, PaymentsSyncData, PaymentsTaxCalculationData,
        SdkPaymentsSessionUpdateData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, TaxCalculationResponseData},
};

use crate::api::{
    ConnectorCommon, ConnectorIntegrationV2, ConnectorSpecifications, ConnectorValidation,
};

/// trait PaymentAuthorizeV2
pub trait PaymentAuthorizeV2:
    ConnectorIntegrationV2<Authorize, PaymentFlowData, PaymentsAuthorizeData, PaymentsResponseData>
{
}

/// trait PaymentAuthorizeSessionTokenV2
pub trait PaymentAuthorizeSessionTokenV2:
    ConnectorIntegrationV2<
    AuthorizeSessionToken,
    PaymentFlowData,
    AuthorizeSessionTokenData,
    PaymentsResponseData,
>
{
}

/// trait PaymentSyncV2
pub trait PaymentSyncV2:
    ConnectorIntegrationV2<PSync, PaymentFlowData, PaymentsSyncData, PaymentsResponseData>
{
}

/// trait PaymentVoidV2
pub trait PaymentVoidV2:
    ConnectorIntegrationV2<Void, PaymentFlowData, PaymentsCancelData, PaymentsResponseData>
{
}

/// trait PaymentApproveV2
pub trait PaymentApproveV2:
    ConnectorIntegrationV2<Approve, PaymentFlowData, PaymentsApproveData, PaymentsResponseData>
{
}

/// trait PaymentRejectV2
pub trait PaymentRejectV2:
    ConnectorIntegrationV2<Reject, PaymentFlowData, PaymentsRejectData, PaymentsResponseData>
{
}

/// trait PaymentCaptureV2
pub trait PaymentCaptureV2:
    ConnectorIntegrationV2<Capture, PaymentFlowData, PaymentsCaptureData, PaymentsResponseData>
{
}

/// trait PaymentSessionV2
pub trait PaymentSessionV2:
    ConnectorIntegrationV2<Session, PaymentFlowData, PaymentsSessionData, PaymentsResponseData>
{
}

/// trait MandateSetupV2
pub trait MandateSetupV2:
    ConnectorIntegrationV2<SetupMandate, PaymentFlowData, SetupMandateRequestData, PaymentsResponseData>
{
}

/// trait PaymentIncrementalAuthorizationV2
pub trait PaymentIncrementalAuthorizationV2:
    ConnectorIntegrationV2<
    IncrementalAuthorization,
    PaymentFlowData,
    PaymentsIncrementalAuthorizationData,
    PaymentsResponseData,
>
{
}

///trait TaxCalculationV2
pub trait TaxCalculationV2:
    ConnectorIntegrationV2<
    CalculateTax,
    PaymentFlowData,
    PaymentsTaxCalculationData,
    TaxCalculationResponseData,
>
{
}

///trait PaymentSessionUpdateV2
pub trait PaymentSessionUpdateV2:
    ConnectorIntegrationV2<
    SdkSessionUpdate,
    PaymentFlowData,
    SdkPaymentsSessionUpdateData,
    PaymentsResponseData,
>
{
}

///trait PaymentPostSessionTokensV2
pub trait PaymentPostSessionTokensV2:
    ConnectorIntegrationV2<
    PostSessionTokens,
    PaymentFlowData,
    PaymentsPostSessionTokensData,
    PaymentsResponseData,
>
{
}

/// trait PaymentsCompleteAuthorizeV2
pub trait PaymentsCompleteAuthorizeV2:
    ConnectorIntegrationV2<
    CompleteAuthorize,
    PaymentFlowData,
    CompleteAuthorizeData,
    PaymentsResponseData,
>
{
}

/// trait PaymentTokenV2
pub trait PaymentTokenV2:
    ConnectorIntegrationV2<
    PaymentMethodToken,
    PaymentFlowData,
    PaymentMethodTokenizationData,
    PaymentsResponseData,
>
{
}

/// trait ConnectorCustomerV2
pub trait ConnectorCustomerV2:
    ConnectorIntegrationV2<
    CreateConnectorCustomer,
    PaymentFlowData,
    ConnectorCustomerData,
    PaymentsResponseData,
>
{
}

/// trait PaymentsPreProcessingV2
pub trait PaymentsPreProcessingV2:
    ConnectorIntegrationV2<
    PreProcessing,
    PaymentFlowData,
    PaymentsPreProcessingData,
    PaymentsResponseData,
>
{
}

/// trait PaymentsPostProcessingV2
pub trait PaymentsPostProcessingV2:
    ConnectorIntegrationV2<
    PostProcessing,
    PaymentFlowData,
    PaymentsPostProcessingData,
    PaymentsResponseData,
>
{
}

/// trait PaymentV2
pub trait PaymentV2:
    ConnectorCommon
    + ConnectorSpecifications
    + ConnectorValidation
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
    + PaymentsPostProcessingV2
    + ConnectorCustomerV2
    + PaymentIncrementalAuthorizationV2
    + TaxCalculationV2
    + PaymentSessionUpdateV2
    + PaymentPostSessionTokensV2
{
}
