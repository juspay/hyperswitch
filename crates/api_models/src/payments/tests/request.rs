use common_utils::generate_customer_id_of_default_length;

use super::super::*;

#[test]
fn test_valid_case_where_customer_details_are_passed_only_once() {
    let customer_id = generate_customer_id_of_default_length();
    let payments_request = PaymentsRequest {
        customer_id: Some(customer_id),
        ..Default::default()
    };

    assert!(payments_request
        .validate_customer_details_in_request()
        .is_none());
}

#[test]
fn test_valid_case_where_customer_id_is_passed_in_both_places() {
    let customer_id = generate_customer_id_of_default_length();

    let customer_object = CustomerDetails {
        id: Some(customer_id.clone()),
        name: None,
        email: None,
        phone: None,
        phone_country_code: None,
        tax_registration_id: None,
        document_details: None,
        date_of_birth: None,
    };

    let payments_request = PaymentsRequest {
        customer_id: Some(customer_id),
        customer: Some(customer_object),
        ..Default::default()
    };

    assert!(payments_request
        .validate_customer_details_in_request()
        .is_none());
}

#[test]
fn test_invalid_case_where_customer_id_is_passed_in_both_places() {
    let customer_id = generate_customer_id_of_default_length();
    let another_customer_id = generate_customer_id_of_default_length();

    let customer_object = CustomerDetails {
        id: Some(customer_id.clone()),
        name: None,
        email: None,
        phone: None,
        phone_country_code: None,
        tax_registration_id: None,
        document_details: None,
        date_of_birth: None,
    };

    let payments_request = PaymentsRequest {
        customer_id: Some(another_customer_id),
        customer: Some(customer_object),
        ..Default::default()
    };

    assert_eq!(
        payments_request.validate_customer_details_in_request(),
        Some(vec!["customer_id and customer.id"])
    );
}
