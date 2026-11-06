use super::*;

struct TestConnector;

impl ConnectorErrorTypeMapping for TestConnector {
    fn get_connector_error_type(
        &self,
        error_code: String,
        error_message: String,
    ) -> ConnectorErrorType {
        match (error_code.as_str(), error_message.as_str()) {
            ("01", "INVALID_MERCHANT") => ConnectorErrorType::BusinessError,
            ("03", "INVALID_CVV") => ConnectorErrorType::UserError,
            ("04", "04") => ConnectorErrorType::TechnicalError,
            _ => ConnectorErrorType::UnknownError,
        }
    }
}

#[test]
fn test_get_error_code_error_message_based_on_priority() {
    let error_code_message_list_unknown = vec![
        ErrorCodeAndMessage {
            error_code: "01".to_string(),
            error_message: "INVALID_MERCHANT".to_string(),
        },
        ErrorCodeAndMessage {
            error_code: "05".to_string(),
            error_message: "05".to_string(),
        },
        ErrorCodeAndMessage {
            error_code: "03".to_string(),
            error_message: "INVALID_CVV".to_string(),
        },
        ErrorCodeAndMessage {
            error_code: "04".to_string(),
            error_message: "04".to_string(),
        },
    ];
    let error_code_message_list_user = vec![
        ErrorCodeAndMessage {
            error_code: "01".to_string(),
            error_message: "INVALID_MERCHANT".to_string(),
        },
        ErrorCodeAndMessage {
            error_code: "03".to_string(),
            error_message: "INVALID_CVV".to_string(),
        },
    ];
    let error_code_error_message_unknown = get_error_code_error_message_based_on_priority(
        TestConnector,
        error_code_message_list_unknown,
    );
    let error_code_error_message_user =
        get_error_code_error_message_based_on_priority(TestConnector, error_code_message_list_user);
    let error_code_error_message_none =
        get_error_code_error_message_based_on_priority(TestConnector, vec![]);
    assert_eq!(
        error_code_error_message_unknown,
        Some(ErrorCodeAndMessage {
            error_code: "05".to_string(),
            error_message: "05".to_string(),
        })
    );
    assert_eq!(
        error_code_error_message_user,
        Some(ErrorCodeAndMessage {
            error_code: "03".to_string(),
            error_message: "INVALID_CVV".to_string(),
        })
    );
    assert_eq!(error_code_error_message_none, None);
}
