use router::types::{self, api, storage::enums};
use router::services::connector_integration_interface::{ConnectorEnum, BoxedConnectorIntegrationInterface};
use router::services::{self, ProxyClient};
use router::configs::settings::{Settings, Proxy};
use std::sync::Arc;
use tokio::sync::oneshot;
use wiremock::{
    matchers::{method, path, path_regex},
    Mock, ResponseTemplate,
};
use serial_test::serial;
use serde_json::json;

use crate::utils::{self, ConnectorActions, Connector, LocalMock, MockConfig};
use test_utils::connector_auth;

#[derive(Clone, Copy)]
struct PeachpaymentsTest;
impl ConnectorActions for PeachpaymentsTest {}
impl LocalMock for PeachpaymentsTest {}
impl Connector for PeachpaymentsTest {
    fn get_data(&self) -> api::ConnectorData {
        utils::construct_connector_data_old(
            Box::new(hyperswitch_connectors::connectors::Peachpayments::new()),
            types::Connector::Peachpayments,
            api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .peachpayments
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "peachpayments".to_string()
    }
}

// Function to make real HTTP requests for testing
async fn call_connector_with_real_http<
    T: std::fmt::Debug + Clone + 'static,
    ResourceCommonData: std::fmt::Debug
        + Clone
        + services::connector_integration_interface::RouterDataConversion<T, Req, Resp>
        + 'static,
    Req: std::fmt::Debug + Clone + 'static,
    Resp: std::fmt::Debug + Clone + 'static,
>(
    request: types::RouterData<T, Req, Resp>,
    integration: BoxedConnectorIntegrationInterface<T, ResourceCommonData, Req, Resp>,
) -> Result<types::RouterData<T, Req, Resp>, error_stack::Report<hyperswitch_interfaces::errors::ConnectorError>> {
    let conf = Settings::new().unwrap();
    let tx: oneshot::Sender<()> = oneshot::channel().0;

    // Create ProxyClient instead of MockApiClient for real HTTP requests
    let proxy_client = ProxyClient::new(&Proxy::default()).unwrap();
    
    let app_state = Box::pin(router::routes::AppState::with_storage(
        conf,
        router::db::StorageImpl::PostgresqlTest,
        tx,
        Box::new(proxy_client),
    ))
    .await;
    let state = Arc::new(app_state)
        .get_session_state(
            &common_utils::id_type::TenantId::try_from_string("public".to_string()).unwrap(),
            None,
            || {},
        )
        .unwrap();
    services::api::execute_connector_processing_step(
        &state,
        integration,
        &request,
        router::core::payments::CallConnectorAction::Trigger,
        None,
    )
    .await
}

impl PeachpaymentsTest {
    // Custom method to make real HTTP requests for testing
    async fn authorize_payment_with_real_http(
        &self,
        payment_data: Option<types::PaymentsAuthorizeData>,
        payment_info: Option<utils::PaymentInfo>,
    ) -> Result<types::PaymentsAuthorizeRouterData, error_stack::Report<hyperswitch_interfaces::errors::ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            types::PaymentsAuthorizeData {
                confirm: true,
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..(payment_data.unwrap_or(utils::PaymentAuthorizeType::default().0))
            },
            payment_info,
        );
        call_connector_with_real_http(request, integration).await
    }
}

static CONNECTOR: PeachpaymentsTest = PeachpaymentsTest {};

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    None
}

// Mock configurations based on real API responses
fn get_mock_config() -> MockConfig {
    // Successful authorization response based on real API
    let authorize_success = json!({
        "transactionId": "838b98b4-195d-48d3-be9a-6b316589ad13",
        "referenceId": "7b1db2b8-6e87-45d8-9035-01c447e09502",
        "transactionResult": "authorized",
        "cardNotPresentRefundableStatus": "not_refundable",
        "transactionType": {
            "value": 0,
            "description": "Goods and Services"
        },
        "responseCode": {
            "value": "00",
            "description": "Approved or completed successfully",
            "terminalOutcomeString": "Approved",
            "receiptString": "Approved"
        },
        "ecommerceCardPaymentOnlyTransactionData": {
            "rrn": "256465872011",
            "traceId": "001747978177345",
            "stan": "323669",
            "approvalCode": "856721",
            "amount": {
                "amount": 100,
                "currencyCode": "USD",
                "displayValue": "$1.00"
            },
            "card": {
                "maskedPan": "420000******0000",
                "binNumber": "420000",
                "scheme": "Visa",
                "cardholderName": "card holder name",
                "expiryYear": "25",
                "expiryMonth": "10"
            }
        },
        "voidableUntilTime": "2025-05-23T22:00:00Z",
        "transactionTime": "2025-05-23T05:29:36.472272189Z",
        "paymentMethod": "ecommerce_card_payment_only"
    });

    // Successful capture response - NOTE: transactionResult is "successful" for captures
    let capture_success = json!({
        "transactionId": "838b98b4-195d-48d3-be9a-6b316589ad13",
        "referenceId": "7b1db2b8-6e87-45d8-9035-01c447e09502",
        "transactionResult": "successful",  // This is key - "successful" not "authorized"
        "responseCode": {
            "value": "00",
            "description": "Approved or completed successfully"
        },
        "ecommerceCardPaymentOnlyTransactionData": {
            "amount": {
                "amount": 100,
                "currencyCode": "USD"
            }
        }
    });

    // Payment sync response
    let sync_response = json!({
        "transactionId": "838b98b4-195d-48d3-be9a-6b316589ad13",
        "transactionResult": "authorized",
        "responseCode": {
            "value": "00",
            "description": "Approved or completed successfully"
        }
    });

    // Refund success response
    let refund_success = json!({
        "transactionId": "refund-txn-123",
        "originalTransactionId": "838b98b4-195d-48d3-be9a-6b316589ad13",
        "transactionResult": "successful",
        "responseCode": {
            "value": "00",
            "description": "Approved or completed successfully"
        },
        "ecommerceCardPaymentOnlyTransactionData": {
            "amount": {
                "amount": 100,
                "currencyCode": "USD"
            }
        }
    });

    MockConfig {
        address: Some("127.0.0.1:9090".to_string()),
        mocks: vec![
            // CRITICAL: Most specific patterns MUST come first
            
            // 1. Capture endpoints - very specific to avoid conflicts
            Mock::given(method("POST"))
                .and(path_regex(r".*/confirm$"))  // Any path ending with /confirm
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(&capture_success)
                        .insert_header("x-mock-matched", "capture-confirm")
                ),

            // 2. Refund endpoints  
            Mock::given(method("POST"))
                .and(path_regex(r".*/refund$"))  // Any path ending with /refund
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(&refund_success)
                        .insert_header("x-mock-matched", "refund")
                ),

            // 3. Payment sync (GET requests with transaction ID)
            Mock::given(method("GET"))
                .and(path_regex(r"^/transactions/[a-f0-9\-]{36}$"))  // GET /transactions/{uuid}
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(&sync_response)
                        .insert_header("x-mock-matched", "sync-get")
                ),

            // 4. Authorization endpoint (POST to /transactions without suffix)
            Mock::given(method("POST"))
                .and(path("/transactions"))  // Exact match
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(&authorize_success)
                        .insert_header("x-mock-matched", "auth-exact")
                ),

            // 5. Authorization endpoint with trailing slash
            Mock::given(method("POST"))
                .and(path("/transactions/"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(&authorize_success)
                        .insert_header("x-mock-matched", "auth-slash")
                ),

            // 6. Catch-all fallback for any unmatched requests (use auth response)
            Mock::given(wiremock::matchers::any())
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(&authorize_success)
                        .insert_header("x-mock-matched", "fallback-auth")
                ),
        ],
    }
}

// Debug mock configuration to catch ALL requests and see what's being called
fn get_debug_mock_config() -> MockConfig {
    // This response has a unique identifier so we can tell if it was hit
    let debug_response = json!({
        "transactionId": "DEBUG-MOCK-HIT",
        "transactionResult": "successful",
        "responseCode": {
            "value": "00", 
            "description": "Debug mock response"
        }
    });

    MockConfig {
        address: Some("127.0.0.1:9090".to_string()),
        mocks: vec![
            // Catch ALL requests and log them
            Mock::given(wiremock::matchers::any())
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(&debug_response)
                        .insert_header("x-mock-matched", "debug-catch-all")
                        .insert_header("x-debug-info", "all-requests-hit-this-mock")
                ),
        ],
    }
}

// =============================================================================
// MOCK-BASED INTEGRATION TESTS
// =============================================================================

#[actix_web::test]
#[serial]
async fn should_authorize_payment() {
    let connector = PeachpaymentsTest {};
    let _mock = connector.start_server(get_mock_config()).await;
    let response = connector
        .authorize_payment(None, get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    assert_eq!(
        utils::get_connector_transaction_id(response.response),
        Some("838b98b4-195d-48d3-be9a-6b316589ad13".to_string())
    );
}

#[actix_web::test]
#[serial]
async fn should_capture_authorized_payment() {
    let connector = PeachpaymentsTest {};
    let _mock = connector.start_server(get_mock_config()).await;
    
    // First authorize
    let authorize_response = connector
        .authorize_payment(None, get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    
    // Then capture
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let capture_response = connector
        .capture_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(capture_response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
#[serial]
async fn should_sync_payment() {
    let connector = PeachpaymentsTest {};
    let _mock = connector.start_server(get_mock_config()).await;
    let response = connector
        .sync_payment(
            Some(types::PaymentsSyncData {
                connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                    "838b98b4-195d-48d3-be9a-6b316589ad13".to_string(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Sync payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
#[serial]
async fn should_refund_payment() {
    let connector = PeachpaymentsTest {};
    let _mock = connector.start_server(get_mock_config()).await;
    let response = connector
        .refund_payment(
            "838b98b4-195d-48d3-be9a-6b316589ad13".to_string(),
            None,
            get_default_payment_info(),
        )
        .await
        .expect("Refund payment response");
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success
    );
}

#[actix_web::test]
#[serial]
async fn should_authorize_and_capture_payment() {
    let connector = PeachpaymentsTest {};
    let _mock = connector.start_server(get_mock_config()).await;
    let response = connector
        .authorize_and_capture_payment(None, None, get_default_payment_info())
        .await
        .expect("Authorize and capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
#[serial]
async fn debug_capture_flow() {
    let connector = PeachpaymentsTest {};
    let _mock = connector.start_server(get_mock_config()).await;
    
    println!("🔍 DEBUG: Testing capture flow with enhanced mock matching...");
    
    // First authorize
    let authorize_response = connector
        .authorize_payment(None, get_default_payment_info())
        .await
        .expect("Authorize payment response");
    
    println!("✅ Authorization status: {:?}", authorize_response.status);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    println!("📋 Transaction ID from authorize: {:?}", txn_id);
    
    // Then capture with debug info
    if let Some(transaction_id) = txn_id {
        println!("🚀 Calling capture with transaction ID: {}", transaction_id);
        println!("🔗 Expected capture URL: /transactions/{}/confirm", transaction_id);
        
        let capture_response = connector
            .capture_payment(transaction_id, None, get_default_payment_info())
            .await
            .expect("Capture payment response");
            
        println!("📊 Capture status: {:?}", capture_response.status);
        println!("📄 Capture response: {:?}", capture_response.response);
        
        // Check if this is the expected "Charged" status
        if capture_response.status == enums::AttemptStatus::Charged {
            println!("🎉 SUCCESS: Capture returned Charged status (as expected)");
        } else {
            println!("❌ ISSUE: Capture returned {:?} instead of Charged", capture_response.status);
        }
    }
}

#[actix_web::test]
#[serial]
async fn debug_with_catch_all_mock() {
    let connector = PeachpaymentsTest {};
    let _mock = connector.start_server(get_debug_mock_config()).await;
    
    println!("🔍 DEBUG: Testing with catch-all mock...");
    
    // Try to authorize - this should hit our debug mock
    let authorize_response = connector
        .authorize_payment(None, get_default_payment_info())
        .await
        .expect("Authorize payment response");
    
    println!("📊 Auth status: {:?}", authorize_response.status);
    
    // Check if we got the debug response
    let txn_id = utils::get_connector_transaction_id(authorize_response.response.clone());
    println!("🆔 Transaction ID: {:?}", txn_id);
    
    if let Some(id) = &txn_id {
        if id == "DEBUG-MOCK-HIT" {
            println!("🎯 SUCCESS: Authorization hit the mock server!");
        } else {
            println!("❌ Authorization did NOT hit mock server (got real transaction ID: {})", id);
        }
    }
    
    // Now try capture
    if let Some(transaction_id) = txn_id {
        let capture_response = connector
            .capture_payment(transaction_id, None, get_default_payment_info())
            .await
            .expect("Capture payment response");
            
        println!("📊 Capture status: {:?}", capture_response.status);
        
        let capture_txn_id = utils::get_connector_transaction_id(capture_response.response);
        println!("🆔 Capture transaction ID: {:?}", capture_txn_id);
        
        if let Some(id) = &capture_txn_id {
            if id == "DEBUG-MOCK-HIT" {
                println!("🎯 SUCCESS: Capture hit the mock server!");
            } else {
                println!("❌ Capture did NOT hit mock server (got transaction ID: {})", id);
            }
        }
    }
}

// =============================================================================
// REAL API TESTS (for development/verification)
// =============================================================================

// Real HTTP requests test to verify integration and capture API behavior
#[tokio::test]
async fn should_authorize_payment_with_real_api() {
    use router::services::{self, ProxyClient};
    use router::configs::settings::{Settings, Proxy};
    use std::sync::Arc;
    use tokio::sync::oneshot;
    
    println!("🚀 Testing PeachPayments connector with REAL API requests...");
    
    let conf = Settings::new().unwrap();
    let tx: oneshot::Sender<()> = oneshot::channel().0;

    // Create ProxyClient instead of MockApiClient for real HTTP requests
    let proxy_client = ProxyClient::new(&Proxy::default()).unwrap();
    
    let app_state = Box::pin(router::routes::AppState::with_storage(
        conf,
        router::db::StorageImpl::PostgresqlTest,
        tx,
        Box::new(proxy_client),
    ))
    .await;
    let state = Arc::new(app_state)
        .get_session_state(
            &common_utils::id_type::TenantId::try_from_string("public".to_string()).unwrap(),
            None,
            || {},
        )
        .unwrap();
        
    let integration = CONNECTOR.get_data().connector.get_connector_integration();
    let request = CONNECTOR.generate_data(
        types::PaymentsAuthorizeData {
            confirm: true,
            capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
            ..utils::PaymentAuthorizeType::default().0
        },
        get_default_payment_info(),
    );
    
    let response = services::api::execute_connector_processing_step(
        &state,
        integration,
        &request,
        router::core::payments::CallConnectorAction::Trigger,
        None,
    )
    .await;
    
    match response {
        Ok(res) => {
            println!("✅ Authorization successful!");
            println!("Status: {:?}", res.status);
            println!("Response: {:#?}", res.response);
            
            // Record the request/response for creating mocks
            println!("\n🔍 RAW REQUEST/RESPONSE DATA FOR MOCK CREATION:");
            println!("Status: {:?}", res.status);
            if let Ok(resp_data) = &res.response {
                println!("Response Data: {:#?}", resp_data);
            } else if let Err(err_data) = &res.response {
                println!("Error Data: {:#?}", err_data);
            }
        }
        Err(err) => {
            println!("❌ Authorization failed: {:#?}", err);
            println!("\n🔍 ERROR DATA FOR ANALYSIS:");
            println!("Error: {:#?}", err);
            
            // Even failures are useful - they tell us about the API's error responses
            println!("\nThis is still valuable - we can see the actual API error structure!");
        }
    }
}

#[tokio::test]
async fn test_mock_api_client_directly() {
    use router::services::MockApiClient;
    use reqwest::Method;
    
    println!("🧪 Testing MockApiClient directly...");
    
    // Start our mock server
    let connector = PeachpaymentsTest {};
    let mock_server = connector.start_server(get_mock_config()).await;
    println!("🖥️  Mock server started at: {}", mock_server.address());
    
    // Create MockApiClient
    let mock_client = MockApiClient::new();
    
    // Test URL redirection logic
    let original_url = "https://apitest.bankingapi.peachpayments.com/transactions/838b98b4-195d-48d3-be9a-6b316589ad13/confirm";
    println!("🔗 Original URL: {}", original_url);
    
    // Make a direct request using reqwest to see if the mock server works
    let test_response = reqwest::Client::new()
        .post("http://127.0.0.1:9090/transactions/838b98b4-195d-48d3-be9a-6b316589ad13/confirm")
        .json(&serde_json::json!({"test": "data"}))
        .send()
        .await
        .expect("Direct mock server request failed");
    
    println!("📊 Direct mock server response status: {}", test_response.status());
    
    if let Some(mock_matched) = test_response.headers().get("x-mock-matched") {
        println!("🎯 Mock matched: {:?}", mock_matched.to_str().unwrap_or("invalid"));
    } else {
        println!("❌ No mock matched header found");
    }
    
    let response_text = test_response.text().await.expect("Failed to read response");
    println!("📄 Response preview: {}", if response_text.len() > 200 { 
        format!("{}...", &response_text[..200])
    } else { 
        response_text 
    });
}

#[tokio::test] 
async fn test_mock_api_client_url_redirection() {
    use router::services::MockApiClient;
    
    println!("🔍 Testing MockApiClient URL redirection...");
    
    // Create MockApiClient
    let mock_client = MockApiClient::new();
    
    // Test cases for URL redirection
    let test_cases = vec![
        (
            "https://apitest.bankingapi.peachpayments.com/transactions",
            "http://127.0.0.1:9090/transactions"
        ),
        (
            "https://apitest.bankingapi.peachpayments.com/transactions/838b98b4-195d-48d3-be9a-6b316589ad13/confirm",
            "http://127.0.0.1:9090/transactions/838b98b4-195d-48d3-be9a-6b316589ad13/confirm"
        ),
        (
            "https://apitest.bankingapi.peachpayments.com/transactions/838b98b4-195d-48d3-be9a-6b316589ad13",
            "http://127.0.0.1:9090/transactions/838b98b4-195d-48d3-be9a-6b316589ad13"
        ),
    ];
    
    for (original, expected) in test_cases {
        // We need to use reflection or make the method public to test this
        println!("🔗 Testing: {} -> expected: {}", original, expected);
        
        // Let's test by making actual HTTP calls to see what happens
        let response_result = reqwest::Client::new()
            .get(original)
            .send()
            .await;
            
        match response_result {
            Ok(response) => {
                println!("📊 Response status: {} (this should fail since it's hitting real API)", response.status());
            }
            Err(e) => {
                println!("❌ Request failed (expected): {}", e);
            }
        }
    }
    
    println!("🧪 MockApiClient URL redirection test completed");
} 