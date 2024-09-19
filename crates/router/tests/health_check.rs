mod utils;

use utils::{mk_service, AppClient};

#[actix_web::test]
async fn health_check() {
    let server = Box::pin(mk_service()).await;
    let client = AppClient::guest();

    assert_eq!(client.health(&server).await, "health is good");
}
