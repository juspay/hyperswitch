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

#[cfg(feature = "payouts")]
#[test]
fn payout_purpose_maps_all_router_values_and_preserves_absence() {
    use common_enums::PayoutPurpose as RouterPurpose;
    use payments_grpc::payout_enums::PayoutPurpose as UcsPurpose;

    let mappings = [
        (RouterPurpose::FamilySupport, UcsPurpose::FamilySupport, 1),
        (RouterPurpose::Expatriation, UcsPurpose::Expatriation, 2),
        (
            RouterPurpose::TravelAndTourism,
            UcsPurpose::TravelAndTourism,
            3,
        ),
        (RouterPurpose::Education, UcsPurpose::Education, 4),
        (
            RouterPurpose::MedicalTreatment,
            UcsPurpose::MedicalTreatment,
            5,
        ),
        (RouterPurpose::EmergencyNeed, UcsPurpose::EmergencyNeed, 6),
        (RouterPurpose::Leisure, UcsPurpose::Leisure, 7),
        (RouterPurpose::Savings, UcsPurpose::Savings, 8),
        (RouterPurpose::Gifts, UcsPurpose::Gifts, 9),
        (RouterPurpose::Donations, UcsPurpose::Donations, 10),
        (
            RouterPurpose::FinancialServices,
            UcsPurpose::FinancialServices,
            11,
        ),
        (RouterPurpose::ItServices, UcsPurpose::ItServices, 12),
        (RouterPurpose::Investment, UcsPurpose::Investment, 13),
        (RouterPurpose::Insurance, UcsPurpose::Insurance, 14),
        (RouterPurpose::LoanPayment, UcsPurpose::LoanPayment, 15),
        (RouterPurpose::Pension, UcsPurpose::Pension, 16),
        (RouterPurpose::Royalties, UcsPurpose::Royalties, 17),
        (RouterPurpose::Other, UcsPurpose::Other, 18),
        (RouterPurpose::Income, UcsPurpose::Income, 19),
    ];

    for (router_purpose, ucs_purpose, wire_value) in mappings {
        let mapped = super::payout_purpose_to_ucs(Some(router_purpose));
        assert_eq!(mapped, Some(i32::from(ucs_purpose)));
        assert_eq!(mapped, Some(wire_value));
    }
    assert_eq!(i32::from(UcsPurpose::Unspecified), 0);
    assert_eq!(super::payout_purpose_to_ucs(None), None);
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
