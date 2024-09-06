use crate::{
    router_data_v2::router_data_v2_aliases::{PaymentsRouterDataV2, RefundsRouterDataV2},
    router_flow_types::{Authorize, Capture, PSync, RSync, SetupMandate, Void},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{PaymentsResponseData, RefundsResponseData},
};

pub type PaymentsAuthorizeRouterData =
    PaymentsRouterDataV2<Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsSyncRouterData = PaymentsRouterDataV2<PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureRouterData =
    PaymentsRouterDataV2<Capture, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsCancelRouterData =
    PaymentsRouterDataV2<Void, PaymentsCancelData, PaymentsResponseData>;
pub type SetupMandateRouterData =
    PaymentsRouterDataV2<SetupMandate, SetupMandateRequestData, PaymentsResponseData>;
pub type RefundsRouterData<F> = RefundsRouterDataV2<F, RefundsData, RefundsResponseData>;
pub type RefundSyncRouterData = RefundsRouterDataV2<RSync, RefundsData, RefundsResponseData>;
