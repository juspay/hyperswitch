//! This file contains the examples to be used for openapi

/// Creating the payment with minimal fields
pub const PAYMENTS_CREATE_MINIMUM_FIELDS: &str = r#"{
    "amount": 6540,
    "currency": "USD",
}"#;

/// Creating a manual capture payment
pub const PAYMENTS_CREATE_WITH_MANUAL_CAPTURE: &str = r#"{
    "amount": 6540,
    "currency": "USD",
    "capture_method":"manual"
}"#;

/// Creating a payment with billing and shipping address
pub const PAYMENTS_CREATE_WITH_ADDRESS: &str = r#"{
    "amount": 6540,
    "currency": "USD",
    "customer": {
        "id" : "cus_abcdefgh"
    },
    "billing": {
        "address": {
            "line1": "1467",
            "line2": "Harrison Street",
            "line3": "Harrison Street",
            "city": "San Fransico",
            "state": "California",
            "zip": "94122",
            "country": "US",
            "first_name": "joseph",
            "last_name": "Doe"
        },
        "phone": {
            "number": "8056594427",
            "country_code": "+91"
        }
  }
}"#;

/// Creating a payment with customer details
pub const PAYMENTS_CREATE_WITH_CUSTOMER_DATA: &str = r#"{
    "amount": 6540,
    "currency": "USD",
    "customer": {
        "id":"cus_abcdefgh",
        "name":"John Dough",
        "phone":"9999999999",
        "email":"john@example.com"
    }
}"#;

/// 3DS force payment
pub const PAYMENTS_CREATE_WITH_FORCED_3DS: &str = r#"{
    "amount": 6540,
    "currency": "USD",
    "authentication_type" : "three_ds"
}"#;

/// A payment with other fields
pub const PAYMENTS_CREATE: &str = r#"{
    "amount": 6540,
    "currency": "USD",
    "payment_id": "abcdefghijklmnopqrstuvwxyz",
    "customer": {
        "id":"cus_abcdefgh",
        "name":"John Dough",
        "phone":"9999999999",
        "email":"john@example.com"
    },
    "description": "Its my first payment request",
    "statement_descriptor_name": "joseph",
    "statement_descriptor_suffix": "JS",
    "metadata": {
        "udf1": "some-value",
        "udf2": "some-value"
    }
}"#;

/// Creating the payment with order details
pub const PAYMENTS_CREATE_WITH_ORDER_DETAILS: &str = r#"{
    "amount": 6540,
    "currency": "USD",
    "order_details": [
        {
            "product_name": "Apple iPhone 15",
            "quantity": 1,
            "amount" : 6540
        }
    ]
}"#;

/// Creating the payment with connector metadata for noon
pub const PAYMENTS_CREATE_WITH_NOON_ORDER_CATETORY: &str = r#"{
    "amount": 6540,
    "currency": "USD",
    "connector_metadata": {
        "noon": {
            "order_category":"shoes"
        }
    }
}"#;
