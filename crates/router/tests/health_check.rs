mod utils;

use utils::{mk_service, AppClient};

#[actix_web::test]
/// Asynchronously performs a health check on the server by creating a service and a client, then asserting that the health status returned by the client is "health is good".
async fn health_check() {
    let server = mk_service().await;
    let client = AppClient::guest();

    assert_eq!(client.health(&server).await, "health is good");
}
