#![allow(clippy::panic_in_result_fn)]
use api_models::types::Percentage;
use common_utils::errors::ApiModelsError;
const PRECISION_2: u8 = 2;
const PRECISION_0: u8 = 0;

#[test]
fn invalid_range_more_than_100() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let percentage = Percentage::<PRECISION_2>::from_float(100.01);
    assert!(percentage.is_err());
    if let Err(err) = percentage {
        assert_eq!(
            *err.current_context(),
            ApiModelsError::InvalidPercentageValue
        )
    }
    Ok(())
}
#[test]
fn invalid_range_less_than_0() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let percentage = Percentage::<PRECISION_2>::from_float(-0.01);
    assert!(percentage.is_err());
    if let Err(err) = percentage {
        assert_eq!(
            *err.current_context(),
            ApiModelsError::InvalidPercentageValue
        )
    }
    Ok(())
}
#[test]
fn valid_range() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let percentage = Percentage::<PRECISION_2>::from_float(2.22);
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 2.22)
    }

    let percentage = Percentage::<PRECISION_2>::from_float(0.0);
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 0.0)
    }

    let percentage = Percentage::<PRECISION_2>::from_float(100.0);
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 100.0)
    }
    Ok(())
}
#[test]
fn valid_precision() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let percentage = Percentage::<PRECISION_2>::from_float(2.2);
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 2.2)
    }

    let percentage = Percentage::<PRECISION_2>::from_float(2.20000);
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 2.2)
    }

    let percentage = Percentage::<PRECISION_0>::from_float(2.0);
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 2.0)
    }

    Ok(())
}

#[test]
fn invalid_precision() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let percentage = Percentage::<PRECISION_2>::from_float(2.221);
    assert!(percentage.is_err());
    if let Err(err) = percentage {
        assert_eq!(
            *err.current_context(),
            ApiModelsError::InvalidPercentageValue
        )
    }
    Ok(())
}

#[test]
fn deserialization_test_ok() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json_string = r#"
        {
            "percentage" : 12.4
        }
    "#;
    let percentage = serde_json::from_str::<Percentage<PRECISION_2>>(json_string);
    assert!(percentage.is_ok());
    if let Ok(percentage) = percentage {
        assert_eq!(percentage.get_percentage(), 12.4)
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
        assert_eq!(err.to_string(), "invalid value: percentage value `12.4`, expected value should be between 0 to 100 and precise to only upto 0 decimal digits at line 4 column 9".to_string())
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
        assert_eq!(err.to_string(), "invalid value: percentage value `123.42`, expected value should be between 0 to 100 and precise to only upto 2 decimal digits at line 4 column 9".to_string())
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
        dbg!(err.to_string());
        assert_eq!(
            err.to_string(),
            "missing field `percentage` at line 4 column 9".to_string()
        )
    }
    Ok(())
}
