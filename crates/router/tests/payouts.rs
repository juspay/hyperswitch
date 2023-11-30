#![allow(clippy::unwrap_used)]

mod utils;

#[actix_web::test]
async fn payouts_todo() {
    Box::pin(utils::setup()).await;

    let client = awc::Client::default();
    let mut response;
    let mut response_body;
    let get_endpoints = vec!["retrieve", "accounts"];
    let post_endpoints = vec!["create", "update", "reverse", "cancel"];

    for endpoint in get_endpoints {
        response = client
            .get(format!("http://127.0.0.1:8080/payouts/{endpoint}"))
            .send()
            .await
            .unwrap();
        response_body = response.body().await;
        println!("{endpoint} =:= {response:?} : {response_body:?}");
        assert_eq!(response.status(), awc::http::StatusCode::OK);
    }

    for endpoint in post_endpoints {
        response = client
            .post(format!("http://127.0.0.1:8080/payouts/{endpoint}"))
            .send()
            .await
            .unwrap();
        response_body = response.body().await;
        println!("{endpoint} =:= {response:?} : {response_body:?}");
        assert_eq!(response.status(), awc::http::StatusCode::OK);
    }
}
