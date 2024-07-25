use crate::{
    router_data::RouterData,
    router_flow_types::{Authorize, Capture, PSync, RSync, SetupMandate, Void},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
};

pub type PaymentsAuthorizeRouterData =
    RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsSyncRouterData = RouterData<PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureRouterData = RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsCancelRouterData = RouterData<Void, PaymentsCancelData, PaymentsResponseData>;
pub type SetupMandateRouterData =
    RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>;
pub type RefundsRouterData<F> = RouterData<F, RefundsData, RefundsResponseData>;
pub type RefundSyncRouterData = RouterData<RSync, RefundsData, RefundsResponseData>;
