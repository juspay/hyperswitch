#[cfg(feature = "payouts")]
use common_enums::PayoutStatus;
use hyperswitch_domain_models::router_response_types::SurchargeCalculationResponseData;

use super::{payments_grpc, ErrorResponse, PayoutsResponseData};
use crate::types::transformers::ForeignTryFrom;

fn connector_error() -> payments_grpc::ErrorInfo {
    payments_grpc::ErrorInfo {
        connector_details: Some(payments_grpc::ConnectorErrorDetails {
            code: Some("connector_error".to_string()),
            message: Some("connector rejected the request".to_string()),
            reason: Some("invalid request".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[cfg(feature = "payouts")]
#[test]
fn payout_connector_http_error_maps_to_error_response() {
    let response = payments_grpc::PayoutServiceTransferResponse {
        payout_status: Some(payments_grpc::payout_enums::PayoutStatus::Failed.into()),
        connector_payout_id: Some("connector-payout-id".to_string()),
        error: Some(connector_error()),
        status_code: 422,
        ..Default::default()
    };

    let result =
        <Result<PayoutsResponseData, ErrorResponse> as ForeignTryFrom<_>>::foreign_try_from((
            response,
            PayoutStatus::Pending,
        ))
        .expect("the UCS payout response should be parseable");
    let error = result.expect_err("connector HTTP errors must use RouterData's error path");

    assert_eq!(error.status_code, 422);
    assert_eq!(error.code, "connector_error");
    assert_eq!(
        error.connector_transaction_id.as_deref(),
        Some("connector-payout-id")
    );
}

#[cfg(feature = "payouts")]
#[test]
fn payout_http_success_preserves_business_failure_status() {
    let response = payments_grpc::PayoutServiceTransferResponse {
        payout_status: Some(payments_grpc::payout_enums::PayoutStatus::Failed.into()),
        connector_payout_id: Some("connector-payout-id".to_string()),
        error: Some(connector_error()),
        status_code: 200,
        ..Default::default()
    };

    let result =
        <Result<PayoutsResponseData, ErrorResponse> as ForeignTryFrom<_>>::foreign_try_from((
            response,
            PayoutStatus::Pending,
        ))
        .expect("the UCS payout response should be parseable");
    let response = result.expect("HTTP-success business failures retain payout status");

    assert_eq!(response.status, Some(PayoutStatus::Failed));
    assert_eq!(response.error_code.as_deref(), Some("connector_error"));
}

#[test]
fn surcharge_connector_error_is_not_defaulted_to_zero() {
    let response = payments_grpc::SurchargeServiceCalculateResponse {
        connector_surcharge_id: Some("connector-surcharge-id".to_string()),
        status_code: 502,
        error: Some(connector_error()),
        ..Default::default()
    };

    let result = <Result<SurchargeCalculationResponseData, ErrorResponse> as ForeignTryFrom<
            _,
        >>::foreign_try_from(response)
        .expect("the UCS surcharge response should be parseable");
    let error = result.expect_err("connector errors must not become a zero surcharge");

    assert_eq!(error.status_code, 502);
    assert_eq!(error.code, "connector_error");
}
