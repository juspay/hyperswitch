use crate::{
    router_data::{AccessToken, RouterData},
    router_flow_types::{
        Authorize, AuthorizeSessionToken, Capture, CalculateTax, PreProcessing, CompleteAuthorize, CreateConnectorCustomer, PSync, PaymentMethodToken,
        RSync, SetupMandate, Void,AccessTokenAuth
    },
    router_request_types::{
        CompleteAuthorizeData, ConnectorCustomerData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSyncData,
        PaymentsTaxCalculationData, RefundsData, SetupMandateRequestData,PaymentsPreProcessingData,AccessTokenRequestData, AuthorizeSessionTokenData
    },
    router_response_types::{
        PaymentsResponseData, RefundsResponseData, TaxCalculationResponseData,
    },
};

pub type PaymentsAuthorizeRouterData =
    RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsAuthorizeSessionTokenRouterData =
    RouterData<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>;
pub type PaymentsPreProcessingRouterData =
    RouterData<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>;
pub type PaymentsSyncRouterData = RouterData<PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureRouterData = RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsCancelRouterData = RouterData<Void, PaymentsCancelData, PaymentsResponseData>;
pub type SetupMandateRouterData =
    RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>;
pub type RefundsRouterData<F> = RouterData<F, RefundsData, RefundsResponseData>;
pub type RefundSyncRouterData = RouterData<RSync, RefundsData, RefundsResponseData>;
pub type TokenizationRouterData =
    RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>;
pub type ConnectorCustomerRouterData =
    RouterData<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>;
pub type PaymentsCompleteAuthorizeRouterData =
    RouterData<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>;
pub type PaymentsTaxCalculationRouterData =
    RouterData<CalculateTax, PaymentsTaxCalculationData, TaxCalculationResponseData>;
pub type RefreshTokenRouterData = RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>;
