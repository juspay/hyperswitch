use std::borrow::Cow;

use common_enums::CountryAlpha2;
use hyperswitch_interfaces::errors::ConnectorError;
use hyperswitch_masking::Secret;

use crate::connectors::stripe::transformers::{
    validate_shipping_address_against_payment_method, StripePaymentMethodType,
    StripeShippingAddress,
};

#[test]
fn should_return_ok() {
    // Arrange
    let stripe_shipping_address = create_stripe_shipping_address(
        "name".to_string(),
        Some("line1".to_string()),
        Some(CountryAlpha2::AD),
        Some("zip".to_string()),
    );

    let payment_method = &StripePaymentMethodType::AfterpayClearpay;

    //Act
    let result = validate_shipping_address_against_payment_method(
        &Some(stripe_shipping_address),
        Some(payment_method),
    );

    // Assert
    assert!(result.is_ok());
}

#[test]
fn should_return_err_for_empty_line1() {
    // Arrange
    let stripe_shipping_address = create_stripe_shipping_address(
        "name".to_string(),
        None,
        Some(CountryAlpha2::AD),
        Some("zip".to_string()),
    );

    let payment_method = &StripePaymentMethodType::AfterpayClearpay;

    //Act
    let result = validate_shipping_address_against_payment_method(
        &Some(stripe_shipping_address),
        Some(payment_method),
    );

    // Assert
    assert!(result.is_err());
    let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
    assert_eq!(missing_fields.len(), 1);
    assert_eq!(*missing_fields.first().unwrap(), "shipping.address.line1");
}

#[test]
fn should_return_err_for_empty_country() {
    // Arrange
    let stripe_shipping_address = create_stripe_shipping_address(
        "name".to_string(),
        Some("line1".to_string()),
        None,
        Some("zip".to_string()),
    );

    let payment_method = &StripePaymentMethodType::AfterpayClearpay;

    //Act
    let result = validate_shipping_address_against_payment_method(
        &Some(stripe_shipping_address),
        Some(payment_method),
    );

    // Assert
    assert!(result.is_err());
    let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
    assert_eq!(missing_fields.len(), 1);
    assert_eq!(*missing_fields.first().unwrap(), "shipping.address.country");
}

#[test]
fn should_return_err_for_empty_zip() {
    // Arrange
    let stripe_shipping_address = create_stripe_shipping_address(
        "name".to_string(),
        Some("line1".to_string()),
        Some(CountryAlpha2::AD),
        None,
    );
    let payment_method = &StripePaymentMethodType::AfterpayClearpay;

    //Act
    let result = validate_shipping_address_against_payment_method(
        &Some(stripe_shipping_address),
        Some(payment_method),
    );

    // Assert
    assert!(result.is_err());
    let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
    assert_eq!(missing_fields.len(), 1);
    assert_eq!(*missing_fields.first().unwrap(), "shipping.address.zip");
}

#[test]
fn should_return_error_when_missing_multiple_fields() {
    // Arrange
    let expected_missing_field_names: Vec<&str> =
        vec!["shipping.address.zip", "shipping.address.country"];
    let stripe_shipping_address =
        create_stripe_shipping_address("name".to_string(), Some("line1".to_string()), None, None);
    let payment_method = &StripePaymentMethodType::AfterpayClearpay;

    //Act
    let result = validate_shipping_address_against_payment_method(
        &Some(stripe_shipping_address),
        Some(payment_method),
    );

    // Assert
    assert!(result.is_err());
    let missing_fields = get_missing_fields(result.unwrap_err().current_context()).to_owned();
    for field in missing_fields {
        assert!(expected_missing_field_names.contains(&field.as_ref()));
    }
}

fn get_missing_fields(connector_error: &ConnectorError) -> Vec<Cow<'static, str>> {
    if let ConnectorError::MissingRequiredFields { field_names } = connector_error {
        return field_names.to_vec();
    }

    vec![]
}

fn create_stripe_shipping_address(
    name: String,
    line1: Option<String>,
    country: Option<CountryAlpha2>,
    zip: Option<String>,
) -> StripeShippingAddress {
    StripeShippingAddress {
        name: Some(Secret::new(name)),
        line1: line1.map(Secret::new),
        country,
        zip: zip.map(Secret::new),
        city: Some(String::from("city")),
        line2: Some(Secret::new(String::from("line2"))),
        state: Some(Secret::new(String::from("state"))),
        phone: Some(Secret::new(String::from("pbone number"))),
    }
}
