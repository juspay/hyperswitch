#[cfg(test)]
mod flat_struct_test {
    use std::collections::HashMap;

    use router_derive::FlatStruct;
    use serde::Serialize;

    #[test]
    fn test_flat_struct() {
        #[derive(FlatStruct, Serialize)]
        struct User {
            address: Address,
        }

        #[derive(Serialize)]
        struct Address {
            line1: String,
            zip: String,
            city: String,
        }

        let line1 = "1397".to_string();
        let zip = "Some street".to_string();
        let city = "941222".to_string();

        let address = Address {
            line1: line1.clone(),
            zip: zip.clone(),
            city: city.clone(),
        };
        let user = User { address };
        let flat_user_map = user.flat_struct();

        let mut required_map = HashMap::new();
        required_map.insert("address.line1".to_string(), line1);
        required_map.insert("address.zip".to_string(), zip);
        required_map.insert("address.city".to_string(), city);

        assert_eq!(flat_user_map, required_map);
    }
}

#[cfg(test)]
mod validate_schema_test {
    use router_derive::ValidateSchema;
    use url::Url;

    #[test]
    fn test_validate_schema() {
        #[derive(ValidateSchema)]
        struct Payment {
            #[schema(min_length = 5, max_length = 12)]
            payment_id: String,

            #[schema(min_length = 10, max_length = 100)]
            description: Option<String>,

            #[schema(max_length = 255)]
            return_url: Option<Url>,
        }

        // Valid case
        let valid_payment = Payment {
            payment_id: "payment_123".to_string(),
            description: Some("This is a valid description".to_string()),
            return_url: Some("https://example.com/return".parse().unwrap()),
        };
        assert!(valid_payment.validate().is_ok());

        // Invalid: payment_id too short
        let invalid_id = Payment {
            payment_id: "pay".to_string(),
            description: Some("This is a valid description".to_string()),
            return_url: Some("https://example.com/return".parse().unwrap()),
        };
        let err = invalid_id.validate().unwrap_err();
        assert!(
            err.contains("payment_id must be at least 5 characters long. Received 3 characters")
        );

        // Invalid: payment_id too long
        let invalid_desc = Payment {
            payment_id: "payment_12345".to_string(),
            description: Some("This is a valid description".to_string()),
            return_url: Some("https://example.com/return".parse().unwrap()),
        };
        let err = invalid_desc.validate().unwrap_err();
        assert!(
            err.contains("payment_id must be at most 12 characters long. Received 13 characters")
        );

        // None values should pass validation
        let none_values = Payment {
            payment_id: "payment_123".to_string(),
            description: None,
            return_url: None,
        };
        assert!(none_values.validate().is_ok());
    }
}
