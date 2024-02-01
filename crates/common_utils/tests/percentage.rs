#![allow(clippy::panic_in_result_fn)]
use common_utils::{errors::PercentageError, types::Percentage};
const PRECISION_2: u8 = 2;
const PRECISION_0: u8 = 0;

#[test]
/// This method tests the behavior of the Percentage::from_string method when an invalid percentage value greater than 100 is provided.
fn invalid_range_more_than_100() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let percentage = Percentage::<PRECISION_2>::from_string("100.01".to_string());
    assert!(percentage.is_err());
    if let Err(err) = percentage {
        assert_eq!(
            *err.current_context(),
            PercentageError::InvalidPercentageValue
        )
    }
    Ok(())
}
#[test]
/// This method tests for the case when the range of percentage value is less than 0.
fn invalid_range_less_than_0() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let percentage = Percentage::<PRECISION_2>::from_string("-0.01".to_string());
    assert!(percentage.is_err());
    if let Err(err) = percentage {
        assert_eq!(
            *err.current_context(),
            PercentageError::InvalidPercentageValue
        )
    }
    Ok(())
}

#[test]
/// This method creates an invalid Percentage value from a string and asserts that it is indeed invalid. It then checks if the correct error is returned and returns a Result indicating success.
fn invalid_string() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let percentage = Percentage::<PRECISION_2>::from_string("-0.01ed".to_string());
    assert!(percentage.is_err());
    if let Err(err) = percentage {
        assert_eq!(
            *err.current_context(),
            PercentageError::InvalidPercentageValue
        )
    }
    Ok(())
}

#[test]
/// This method validates the functionality of the Percentage struct by creating instances with different percentage values and asserting that the values are correctly stored and retrieved.
fn valid_range() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let percentage = Percentage::<PRECISION_2>::from_string("2.22".to_string());
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 2.22)
    }

    let percentage = Percentage::<PRECISION_2>::from_string("0.05".to_string());
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 0.05)
    }

    let percentage = Percentage::<PRECISION_2>::from_string("100.0".to_string());
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 100.0)
    }
    Ok(())
}
#[test]
/// This method tests the validity of the precision of the Percentage type by creating Percentage instances with different precision values and checking if the parsed percentage values match the expected values. It returns a Result indicating success or an error if the precision is not valid.
fn valid_precision() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let percentage = Percentage::<PRECISION_2>::from_string("2.2".to_string());
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 2.2)
    }

    let percentage = Percentage::<PRECISION_2>::from_string("2.20000".to_string());
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 2.2)
    }

    let percentage = Percentage::<PRECISION_0>::from_string("2.0".to_string());
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 2.0)
    }

    Ok(())
}

#[test]
/// This method tests if a Percentage instance with invalid precision can be created 
/// from a string and returns a Result based on the success of the test.
fn invalid_precision() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let percentage = Percentage::<PRECISION_2>::from_string("2.221".to_string());
    assert!(percentage.is_err());
    if let Err(err) = percentage {
        assert_eq!(
            *err.current_context(),
            PercentageError::InvalidPercentageValue
        )
    }
    Ok(())
}

#[test]
/// This method tests the deserialization of percentage values from 0 to 100 with varying decimal precision, as well as specific percentage values such as 18.7 and 12.0. It constructs JSON strings representing percentage values and uses serde_json to deserialize them into Percentage structs with different precision levels, asserting that the deserialization is successful and that the deserialized percentage values match the expected values. 
fn deserialization_test_ok() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut decimal = 0;
    let mut integer = 0;
    // check for all percentage values from 0 to 100
    while integer <= 100 {
        let json_string = format!(
            r#"
            {{
                "percentage" : {}.{}
            }}
        "#,
            integer, decimal
        );
        let percentage = serde_json::from_str::<Percentage<PRECISION_2>>(&json_string);
        assert!(percentage.is_ok());
        if let Ok(percentage) = percentage {
            assert_eq!(
                percentage.get_percentage(),
                format!("{}.{}", integer, decimal)
                    .parse::<f32>()
                    .unwrap_or_default()
            )
        }
        if integer == 100 {
            break;
        }
        decimal += 1;
        if decimal == 100 {
            decimal = 0;
            integer += 1;
        }
    }

    let json_string = r#"
        {
            "percentage" : 18.7
        }
    "#;
    let percentage = serde_json::from_str::<Percentage<PRECISION_2>>(json_string);
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 18.7)
    }

    let json_string = r#"
        {
            "percentage" : 12.0
        }
    "#;
    let percentage = serde_json::from_str::<Percentage<PRECISION_0>>(json_string);
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 12.0)
    }
    Ok(())
}

#[test]
/// This method tests the deserialization of Percentage struct with different error scenarios, such as invalid percentage precision, invalid percentage value, and missing percentage field.
fn deserialization_test_err() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // invalid percentage precision
    let json_string = r#"
        {
            "percentage" : 12.4
        }
    "#;
    let percentage = serde_json::from_str::<Percentage<PRECISION_0>>(json_string);
    assert!(percentage.is_err());
    if let Err(err) = percentage {
        assert_eq!(err.to_string(), "invalid value: percentage value 12.4, expected value should be a float between 0 to 100 and precise to only upto 0 decimal digits at line 4 column 9".to_string())
    }

    // invalid percentage value
    let json_string = r#"
        {
            "percentage" : 123.42
        }
    "#;
    let percentage = serde_json::from_str::<Percentage<PRECISION_2>>(json_string);
    assert!(percentage.is_err());
    if let Err(err) = percentage {
        assert_eq!(err.to_string(), "invalid value: percentage value 123.42, expected value should be a float between 0 to 100 and precise to only upto 2 decimal digits at line 4 column 9".to_string())
    }

    // missing percentage field
    let json_string = r#"
        {
            "percent": 22.0
        }
    "#;
    let percentage = serde_json::from_str::<Percentage<PRECISION_2>>(json_string);
    assert!(percentage.is_err());
    if let Err(err) = percentage {
        assert_eq!(
            err.to_string(),
            "missing field `percentage` at line 4 column 9".to_string()
        )
    }
    Ok(())
}
