#![allow(clippy::unwrap_used)]

mod utils;
use masking::PeekInterface;
use test_utils::connector_auth::ConnectorAuthentication;
use utils::{mk_service, ApiKey, AppClient, MerchantId, PaymentId, Status};

/// Example of unit test
/// Kind of test: output-based testing
/// 1) Create Merchant account
#[actix_web::test]
async fn create_merchant_account() {
    let server = Box::pin(mk_service()).await;
    let client = AppClient::guest();
    let admin_client = client.admin("test_admin");

    let expected = "merchant_12345";
    let hlist_pat![merchant_id, _api_key]: HList![MerchantId, ApiKey] = admin_client
        .create_merchant_account(&server, expected.to_owned())
        .await;

    assert_eq!(expected, *merchant_id);
}

/// Example of unit test
/// Kind of test: communication-based testing
/// ```pseudocode
/// mk_service =
///   app_state <- AppState(StorageImpl::Mock) // Instantiate a mock database to simulate real world SQL database.
///   actix_web::test::init_service(<..>) // Initialize service from application builder instance .
/// ```
/// ### Tests with mocks are typically structured like so:
/// 1) create mock objects and specify what values they return
/// 2) run code under test, passing mock objects to it
/// 3) assert mocks were called the expected number of times, with the expected arguments
/// ```
/// fn show_users(get_users: impl FnOnce() -> Vec<&'static str>) -> String {
///   get_users().join(", ")
/// }
/// // GIVEN[1]:
/// let get_users = || vec!["Andrey", "Alisa"];
/// // WHEN[2]:
/// let list_users = show_users(get_users);
/// // THEN[3]:
/// assert_eq!(list_users, "Andrey, Alisa");
/// ```
/// ### Test case
/// 1) Create Merchant account (Get the API key)
/// 2) Create a connector
/// 3) Create a payment for 100 USD
/// 4) Confirm a payment (let it get processed through Stripe)
/// 5) Refund for 50USD success
/// 6) Another refund for 50USD success
///
/// ### Useful resources
/// * <https://blog.ploeh.dk/2016/03/18/functional-architecture-is-ports-and-adapters>
/// * <https://www.parsonsmatt.org/2017/07/27/inverted_mocking.html>
/// * <https://www.parsonsmatt.org/2018/03/22/three_layer_haskell_cake.html>
#[actix_web::test]
async fn partial_refund() {
    let authentication = ConnectorAuthentication::new();
    let server = Box::pin(mk_service()).await;

    let client = AppClient::guest();
    let admin_client = client.admin("test_admin");

    let hlist_pat![merchant_id, api_key]: HList![MerchantId, ApiKey] =
        admin_client.create_merchant_account(&server, None).await;

    let _connector: serde_json::Value = admin_client
        .create_connector(
            &server,
            &merchant_id,
            "stripe",
            authentication.checkout.unwrap().api_key.peek(),
        )
        .await;

    let user_client = client.user(&api_key);
    let hlist_pat![payment_id]: HList![PaymentId] =
        user_client.create_payment(&server, 100, 100).await;

    let hlist_pat![status]: HList![Status] =
        user_client.create_refund(&server, &payment_id, 50).await;
    assert_eq!(&*status, "pending");

    let hlist_pat![status]: HList![Status] =
        user_client.create_refund(&server, &payment_id, 50).await;
    assert_eq!(&*status, "pending");
}

/// Example of unit test
/// Kind of test: communication-based testing
/// ```pseudocode
/// mk_service =
///   app_state <- AppState(StorageImpl::Mock) // Instantiate a mock database to simulate real world SQL database.
///   actix_web::test::init_service(<..>) // Initialize service from application builder instance .
/// ```
/// ### Tests with mocks are typically structured like so:
/// 1) create mock objects and specify what values they return
/// 2) run code under test, passing mock objects to it
/// 3) assert mocks were called the expected number of times, with the expected arguments
/// ```
/// fn show_users(get_users: impl FnOnce() -> Vec<&'static str>) -> String {
///   get_users().join(", ")
/// }
/// // GIVEN[1]:
/// let get_users = || vec!["Andrey", "Alisa"];
/// // WHEN[2]:
/// let list_users = show_users(get_users);
/// // THEN[3]:
/// assert_eq!(list_users, "Andrey, Alisa");
/// ```
/// Test case
/// 1) Create a payment for 100 USD
/// 2) Confirm a payment (let it get processed through Stripe)
/// 3) Refund for 50USD successfully
/// 4) Try another refund for 100USD
/// 5) Get an error for second refund
///
/// ### Useful resources
/// * <https://blog.ploeh.dk/2016/03/18/functional-architecture-is-ports-and-adapters>
/// * <https://www.parsonsmatt.org/2017/07/27/inverted_mocking.html>
/// * <https://www.parsonsmatt.org/2018/03/22/three_layer_haskell_cake.html>
#[actix_web::test]
async fn exceed_refund() {
    let authentication = ConnectorAuthentication::new();
    let server = Box::pin(mk_service()).await;

    let client = AppClient::guest();
    let admin_client = client.admin("test_admin");

    let hlist_pat![merchant_id, api_key]: HList![MerchantId, ApiKey] =
        admin_client.create_merchant_account(&server, None).await;

    let _connector: serde_json::Value = admin_client
        .create_connector(
            &server,
            &merchant_id,
            "stripe",
            authentication.checkout.unwrap().api_key.peek(),
        )
        .await;

    let user_client = client.user(&api_key);

    let hlist_pat![payment_id]: HList![PaymentId] =
        user_client.create_payment(&server, 100, 100).await;

    let hlist_pat![status]: HList![Status] =
        user_client.create_refund(&server, &payment_id, 50).await;
    assert_eq!(&*status, "pending");

    let message: serde_json::Value = user_client.create_refund(&server, &payment_id, 100).await;
    assert_eq!(
        message.get("error").unwrap().get("message").unwrap(),
        "The refund amount exceeds the amount captured."
    );
}
