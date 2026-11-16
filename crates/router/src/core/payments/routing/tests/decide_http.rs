use std::time::Duration;

use api_models::routing::{RoutableChoiceKind, RoutableConnectorChoice};
use common_utils::id_type;
use euclid::enums::RoutableConnectors;
use external_services::http_client::send_request;
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use hyperswitch_masking::Secret;
use serde_json::{json, Value};
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

use super::super::{
    decide::{normalize_decide_routing_response, DecideRequest, DecideResponse},
    utils::build_decision_engine_http_request,
    RoutingConnectorOutcomeWithApproach,
};
use crate::{configs::settings::OpenRouter, services};

fn profile_id() -> id_type::ProfileId {
    "pro_decide_http".parse().expect("valid test profile")
}

fn config(server: &MockServer) -> OpenRouter {
    OpenRouter {
        // Keep the trailing slash: the shared builder must join the base URL correctly.
        url: format!("{}/", server.uri()),
        admin_secret: Secret::new("test_decision_engine_service_secret".to_string()),
        ..OpenRouter::default()
    }
}

fn proxy() -> Proxy {
    // Match the binary's explicit provider selection when multiple Rustls providers are enabled.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    Proxy {
        bypass_proxy_hosts: Some("127.0.0.1,localhost".to_string()),
        ..Proxy::default()
    }
}

fn request_body() -> Value {
    json!({
        "payment_id": "pay_decide_http",
        "amount": 1000,
        "currency": "USD",
        "payment_method_type": "card",
        "payment_method_sub_type": "credit",
        "card": {"card_isin": "001234", "card_issuing_country_code": "US"},
        "routing_config": {
            "algorithm_for": "payment",
            "eligible_connectors": [
                {"connector": "stripe", "merchant_connector_id": "mca_Second"},
                {"connector": "stripe", "merchant_connector_id": "mca_First"}
            ]
        }
    })
}

fn response_body() -> Value {
    json!({
        "id": "dec_http_test",
        "payment_id": "pay_decide_http",
        "created_at": "2026-11-13T00:00:00Z",
        "connectors": [
            {"connector": "stripe", "merchant_connector_id": "mca_First"},
            {"connector": "stripe", "merchant_connector_id": "mca_Second"}
        ],
        "outcome": {"type": "routed", "strategy": "rule", "reason": "rule_matched"},
        "success_rate": null,
        "warnings": [{"code": "future_warning", "message": "Additional diagnostics"}],
        "future_optional_section": {"status": "new_value"}
    })
}

#[tokio::test]
async fn decide_http_sends_profile_scope_and_preserves_ranked_mca_order() {
    let server = MockServer::start().await;
    let body = request_body();
    Mock::given(method("POST"))
        .and(path("/decide"))
        .and(header("content-type", "application/json"))
        .and(header("x-connected-merchant-id", "pro_decide_http"))
        .and(header(
            "x-admin-secret",
            "test_decision_engine_service_secret",
        ))
        .and(body_json(body.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body()))
        .expect(1)
        .mount(&server)
        .await;

    let request = build_decision_engine_http_request(
        &config(&server),
        services::Method::Post,
        "decide",
        Some(serde_json::from_value::<DecideRequest>(body).expect("typed HS request")),
        Some(&profile_id()),
    )
    .expect("build scoped decide request");
    let response = send_request(&proxy(), request, Some(2))
        .await
        .expect("mock DE must respond");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let response: DecideResponse = response.json().await.expect("valid DE response");
    assert_eq!(
        serde_json::to_value(&response).expect("serialize recorded DE response"),
        response_body(),
        "audit serialization must retain the decision ID, warnings and additional sections"
    );
    let outcome = normalize_decide_routing_response(&response).expect("normalize DE response");
    assert_eq!(
        outcome
            .connectors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["stripe:mca_First", "stripe:mca_Second"]
    );
    let requests = server.received_requests().await.expect("recorded request");
    let sent: Value = requests
        .first()
        .expect("one decide request")
        .body_json()
        .expect("request JSON");
    assert!(sent.get("merchant_id").is_none());
}

#[tokio::test]
async fn decide_http_no_connector_preserves_hs_static_fallback_order_and_approach() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/decide"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "dec_no_connector",
            "connectors": [],
            "outcome": {"type": "no_connector", "strategy": "none"},
            "success_rate": null
        })))
        .expect(1)
        .mount(&server)
        .await;
    let body = serde_json::from_value::<DecideRequest>(request_body()).expect("typed HS request");
    let request = build_decision_engine_http_request(
        &config(&server),
        services::Method::Post,
        "decide",
        Some(body),
        Some(&profile_id()),
    )
    .expect("build decide request");
    let response = send_request(&proxy(), request, Some(2))
        .await
        .expect("mock DE responds with no connector");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let response: DecideResponse = response.json().await.expect("valid no-connector response");
    let outcome = normalize_decide_routing_response(&response).expect("valid empty routing result");
    let fallback = [
        (RoutableConnectors::Adyen, "mca_HsFirst"),
        (RoutableConnectors::Stripe, "mca_HsSecond"),
    ]
    .into_iter()
    .map(|(connector, id)| RoutableConnectorChoice {
        choice_kind: RoutableChoiceKind::FullStruct,
        connector,
        merchant_connector_id: Some(
            id_type::MerchantConnectorAccountId::wrap(id.to_string()).expect("valid fallback MCA"),
        ),
    })
    .collect::<Vec<_>>();
    let (connectors, approach) = RoutingConnectorOutcomeWithApproach {
        connectors: outcome.connectors,
        routing_approach: outcome.routing_approach.into(),
    }
    .resolve_or_fallback(
        "decide-routing",
        &fallback,
        common_enums::RoutingApproach::RuleBasedRouting,
    );
    assert_eq!(
        connectors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["adyen:mca_HsFirst", "stripe:mca_HsSecond"]
    );
    assert_eq!(approach, common_enums::RoutingApproach::RuleBasedRouting);
}

#[test]
fn decide_http_requires_profile_scope_before_sending() {
    let config = OpenRouter {
        url: "http://127.0.0.1:1".to_string(),
        ..OpenRouter::default()
    };
    assert!(build_decision_engine_http_request(
        &config,
        services::Method::Post,
        "decide",
        Some(request_body()),
        None,
    )
    .is_err());
}

#[tokio::test]
async fn decide_http_keeps_hybrid_auth_without_adding_connected_merchant_scope() {
    let server = MockServer::start().await;
    let body = json!({"static_routing_request": null, "dynamic_routing_request": null});
    Mock::given(method("POST"))
        .and(path("/routing/hybrid"))
        .and(header(
            "x-admin-secret",
            "test_decision_engine_service_secret",
        ))
        .and(body_json(body.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let request = build_decision_engine_http_request(
        &config(&server),
        services::Method::Post,
        "routing/hybrid",
        Some(body),
        Some(&profile_id()),
    )
    .expect("build legacy hybrid request");
    let response = send_request(&proxy(), request, Some(2))
        .await
        .expect("mock hybrid responds");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let requests = server.received_requests().await.expect("recorded request");
    assert!(!requests
        .first()
        .expect("one hybrid request")
        .headers
        .contains_key("x-connected-merchant-id"));
}

#[tokio::test]
async fn decide_http_omits_unconfigured_service_secret() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/decide"))
        .and(header("x-connected-merchant-id", "pro_decide_http"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body()))
        .expect(1)
        .mount(&server)
        .await;
    let mut config = config(&server);
    config.admin_secret = Secret::new(String::new());
    let request = build_decision_engine_http_request(
        &config,
        services::Method::Post,
        "decide",
        Some(request_body()),
        Some(&profile_id()),
    )
    .expect("build request with unconfigured service secret");
    send_request(&proxy(), request, Some(2))
        .await
        .expect("mock DE responds");
    let requests = server.received_requests().await.expect("recorded request");
    assert!(!requests
        .first()
        .expect("one decide request")
        .headers
        .contains_key("x-admin-secret"));
}

#[tokio::test]
async fn decide_http_rejects_malformed_success_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/decide"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{malformed json"))
        .expect(1)
        .mount(&server)
        .await;
    let request = build_decision_engine_http_request(
        &config(&server),
        services::Method::Post,
        "decide",
        Some(request_body()),
        Some(&profile_id()),
    )
    .expect("build decide request");
    let response = send_request(&proxy(), request, Some(2))
        .await
        .expect("mock DE responds");
    assert!(response.json::<DecideResponse>().await.is_err());
}

#[tokio::test]
async fn decide_http_preserves_non_success_status_for_shared_error_handling() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/decide"))
        .respond_with(ResponseTemplate::new(503).set_body_json(json!({
            "message": "Decision Engine temporarily unavailable"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let request = build_decision_engine_http_request(
        &config(&server),
        services::Method::Post,
        "decide",
        Some(request_body()),
        Some(&profile_id()),
    )
    .expect("build decide request");
    let response = send_request(&proxy(), request, Some(2))
        .await
        .expect("mock DE returns HTTP failure");
    let response = services::handle_response(Ok(response))
        .await
        .expect("read HTTP failure body")
        .expect_err("503 must use the shared HTTP error path");
    assert_eq!(response.status_code, 503);
}

#[tokio::test]
async fn decide_http_respects_the_request_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/decide"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(response_body())
                .set_delay(Duration::from_secs(3)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let request = build_decision_engine_http_request(
        &config(&server),
        services::Method::Post,
        "decide",
        Some(request_body()),
        Some(&profile_id()),
    )
    .expect("build decide request");
    let error = send_request(&proxy(), request, Some(1))
        .await
        .expect_err("the delayed response must exceed the configured timeout");
    assert!(matches!(
        error.current_context(),
        HttpClientError::RequestTimeoutReceived
    ));
}
