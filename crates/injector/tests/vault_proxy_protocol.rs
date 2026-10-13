#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

#[path = "../../common_utils/tests/support/mock_http.rs"]
mod mock_http;

use base64::{engine::general_purpose::STANDARD, Engine};
use hyperswitch_masking::{ExposeInterface, PeekInterface};
use injector::{
    injector_core, HttpMethod, HyperswitchVaultProxyRequest, InjectorRequest, InjectorResponse,
    ProxyBody, ProxyMultipartPart, TokenData, VaultConnectorAuth, VaultConnectorType,
    VaultConnectors,
};
use serde_json::{json, Value};

fn request(endpoint: String, body: ProxyBody, content_type: &str) -> InjectorRequest {
    let mut request = InjectorRequest::new(
        url::Url::parse("https://connector.example/pay?reference=fixture").unwrap(),
        HttpMethod::POST,
        String::new(),
        TokenData {
            specific_token_data: common_utils::pii::SecretSerdeValue::new(
                json!({"card_number":"fixture-vault-token"}),
            ),
        },
        Some(std::collections::HashMap::from([
            ("Content-Type".to_owned(), content_type.to_owned().into()),
            (
                "Authorization".to_owned(),
                "Bearer connector-fixture".to_owned().into(),
            ),
        ])),
        None,
        None,
        None,
        None,
    );
    request.connection_config.vault_connector_type = Some(VaultConnectorType::Transformation);
    request.connection_config.vault_connector_id = Some(VaultConnectors::HyperswitchVault);
    request.connection_config.vault_endpoint = Some(url::Url::parse(&endpoint).unwrap());
    request.connection_config.vault_auth_data = Some(VaultConnectorAuth {
        api_key: "saas-fixture-key".to_owned().into(),
        profile_id: "profile-fixture".to_owned().into(),
    });
    request.connector_payload.body = Some(body);
    request
}

#[tokio::test]
async fn injector_calls_saas_with_a_json_envelope_for_every_connector_body_format() {
    let file = b"\x00\xff{{$card_number}}";
    let cases = [
        (
            ProxyBody::Json(json!({"card":"{{$card_number}}"}).to_string()),
            "application/json; charset=UTF-8",
        ),
        (
            ProxyBody::Xml("<?xml version=\"1.0\"?><card>{{$card_number}}</card>".into()),
            "text/xml; charset=UTF-8",
        ),
        (
            ProxyBody::FormUrlEncoded("card=%7B%7B%24card_number%7D%7D&tag=1&tag=2".into()),
            "application/x-www-form-urlencoded",
        ),
        (
            ProxyBody::Text("card={{$card_number}}".into()),
            "text/plain; charset=UTF-8",
        ),
        (
            ProxyBody::Multipart(vec![
                ProxyMultipartPart::Text {
                    name: "card".into(),
                    value: "{{$card_number}}".into(),
                },
                ProxyMultipartPart::File {
                    name: "file".into(),
                    filename: "fixture.bin".into(),
                    mime_type: "application/octet-stream".into(),
                    data_base64: STANDARD.encode(file),
                },
            ]),
            "multipart/form-data",
        ),
        (
            ProxyBody::RawBytes(STANDARD.encode(file)),
            "application/octet-stream",
        ),
        (ProxyBody::None, "application/json"),
    ];
    for (body, content_type) in cases {
        let (endpoint, server) = mock_http::serve(move |captured| {
            assert_eq!(captured.method, "POST");
            assert_eq!(captured.header("content-type"), Some("application/json"));
            assert_eq!(
                captured.header("authorization"),
                Some("api-key=saas-fixture-key")
            );
            assert_eq!(captured.header("x-profile-id"), Some("profile-fixture"));
            let envelope: HyperswitchVaultProxyRequest =
                serde_json::from_slice(&captured.body).unwrap();
            assert_eq!(
                envelope.destination_url.as_str(),
                "https://connector.example/pay?reference=fixture"
            );
            assert_eq!(envelope.token.peek(), "fixture-vault-token");
            assert_eq!(envelope.token_type, "payment_method_token");
            assert_eq!(envelope.method, "POST");
            // SaaS-side interpolation uses the same shared code as the real handler.
            let prepared = envelope
                .body
                .unwrap()
                .expose()
                .prepare(
                    &json!({"card_number":"4111111111111111"}),
                    Some(content_type),
                )
                .unwrap();
            let connector_bytes = prepared.bytes.unwrap_or_default();
            let reply = json!({"response":null,"status_code":422,"response_body_base64":STANDARD.encode(&connector_bytes),"response_headers":{"Content-Type":content_type},"response_headers_multi":[["Content-Type",content_type],["Set-Cookie","one=1"],["Set-Cookie","two=2"]]});
            mock_http::MockResponse {
                status: 200,
                headers: vec![("Content-Type".into(), "application/json".into())],
                body: serde_json::to_vec(&reply).unwrap(),
            }
        });
        let original = body.clone();
        let reply = injector_core(request(endpoint, body, content_type))
            .await
            .unwrap()
            .into_connector_response(true)
            .unwrap();
        assert_eq!(reply.status_code, 422);
        let expected = original
            .prepare(
                &json!({"card_number":"4111111111111111"}),
                Some(content_type),
            )
            .unwrap();
        if !matches!(original, ProxyBody::Multipart(_)) {
            assert_eq!(reply.body, expected.bytes.unwrap_or_default());
        } else {
            assert!(reply.body.windows(file.len()).any(|part| part == file));
        }
        assert_eq!(
            reply
                .headers
                .unwrap()
                .iter()
                .filter(|(name, _)| name == "Set-Cookie")
                .count(),
            2
        );
        let captured = server.join().unwrap();
        let envelope: Value = serde_json::from_slice(&captured.body).unwrap();
        assert!(envelope.get("body").is_some());
        // Non-JSON formats travel once; only JSON is duplicated for older servers.
        let expected_legacy = match &original {
            ProxyBody::Json(json) => serde_json::from_str(json).unwrap(),
            _ => Value::Null,
        };
        assert_eq!(envelope["request_body"], expected_legacy);
        assert!(!String::from_utf8(captured.body)
            .unwrap()
            .contains("4111111111111111"));
    }
}

fn response(response: Value) -> InjectorResponse {
    InjectorResponse {
        status_code: 200,
        headers: None,
        response,
        response_body_base64: None,
        response_headers_multi: None,
    }
}

#[tokio::test]
async fn connector_tls_credentials_travel_in_the_envelope_and_are_not_used_for_saas() {
    let (endpoint, server) = mock_http::serve(|captured| {
        let envelope: Value = serde_json::from_slice(&captured.body).unwrap();
        assert_eq!(envelope["certificate"], "connector-cert-fixture");
        assert_eq!(envelope["certificate_key"], "connector-key-fixture");
        assert_eq!(envelope["ca_certificate"], "connector-ca-fixture");
        mock_http::MockResponse {
            status: 200,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: serde_json::to_vec(&json!({"response":null,"status_code":204,"response_headers":{},"response_body_base64":""})).unwrap(),
        }
    });
    let mut request = request(endpoint, ProxyBody::Json("{}".into()), "application/json");
    // Invalid certificate fixtures would fail before HTTP if applied to the SaaS client.
    request.connection_config.client_cert = Some("connector-cert-fixture".to_owned().into());
    request.connection_config.client_key = Some("connector-key-fixture".to_owned().into());
    request.connection_config.ca_cert = Some("connector-ca-fixture".to_owned().into());
    let reply = injector_core(request)
        .await
        .unwrap()
        .into_connector_response(true)
        .unwrap();
    assert_eq!(reply.status_code, 204);
    assert!(reply.body.is_empty());
    server.join().unwrap();
}

#[tokio::test]
async fn legacy_bodyless_get_remains_supported_without_hyperswitch_vault() {
    let (endpoint, server) = mock_http::serve(|captured| {
        assert_eq!(captured.method, "GET");
        assert!(captured.body.is_empty());
        mock_http::MockResponse {
            status: 200,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: b"{\"synced\":true}".to_vec(),
        }
    });
    let mut request = request(endpoint.clone(), ProxyBody::None, "application/json");
    request.connection_config.endpoint = url::Url::parse(&endpoint).unwrap();
    request.connection_config.http_method = HttpMethod::GET;
    request.connection_config.vault_connector_id = None;
    request.connection_config.vault_connector_type = None;
    request.connection_config.vault_endpoint = None;
    request.connection_config.vault_auth_data = None;
    request.connector_payload.body = None;
    let reply = injector_core(request)
        .await
        .unwrap()
        .into_connector_response(false)
        .unwrap();
    assert_eq!(reply.status_code, 200);
    assert_eq!(reply.body, b"{\"synced\":true}");
    server.join().unwrap();
}

#[test]
fn injector_request_debug_masks_legacy_template_and_proxy_token() {
    let pan = "4111111111111111";
    let mut request = request(
        "https://saas.example/v2/proxy".into(),
        ProxyBody::Text(pan.into()),
        "text/plain",
    );
    request.connector_payload.template = pan.into();
    assert!(!format!("{request:?}").contains(pan));
    let payload = serde_json::to_string(pan).unwrap();
    let mut envelope = HyperswitchVaultProxyRequest::try_from_injector_request(
        &payload,
        &request.connection_config,
        &request.token_data,
    )
    .unwrap();
    envelope.body = Some(ProxyBody::Text(pan.into()).into());
    let debug = format!("{envelope:?}");
    let masked = hyperswitch_masking::masked_serialize(&envelope)
        .unwrap()
        .to_string();
    for secret in [pan, "fixture-vault-token", "Bearer connector-fixture"] {
        assert!(!debug.contains(secret));
        assert!(!masked.contains(secret));
    }
    let wire = serde_json::to_value(&envelope).unwrap();
    assert_eq!(wire["request_body"], pan);
    assert_eq!(wire["body"], json!({"type":"text","data":pan}));
    assert_eq!(wire["token"], "fixture-vault-token");
}

#[test]
fn connector_status_distinguishes_proxy_success_from_connector_failure() {
    let reply = response(json!({"status_code":422,"response":null}));
    assert_eq!(reply.connector_status_code(true).unwrap(), 422);
    assert_eq!(reply.connector_status_code(false).unwrap(), 200);
    assert_eq!(
        reply.into_connector_response(true).unwrap().status_code,
        422
    );
    for status in [json!(null), json!("422"), json!(99), json!(600)] {
        assert!(response(json!({"status_code":status}))
            .connector_status_code(true)
            .is_err());
    }
    let mut reply = response(json!({"status_code":200}));
    reply.status_code = 401;
    assert!(reply.connector_status_code(true).is_err());
}

#[test]
fn injector_response_debug_does_not_expose_body_or_header_values() {
    let mut reply = response(json!({"card":"4111111111111111"}));
    let encoded = STANDARD.encode(b"4111111111111111");
    reply.response_body_base64 = Some(encoded.clone());
    reply.headers = Some(std::collections::HashMap::from([(
        "Set-Cookie".into(),
        "session=fixture-secret".into(),
    )]));
    reply.response_headers_multi =
        Some(vec![("Set-Cookie".into(), "session=fixture-secret".into())]);
    let debug = format!("{reply:?}");
    for secret in [
        "4111111111111111",
        encoded.as_str(),
        "session=fixture-secret",
    ] {
        assert!(!debug.contains(secret));
    }
    assert!(debug.contains("status_code: 200"));
}

#[test]
fn response_bytes_are_exact_for_json_xml_empty_and_binary() {
    for body in [
        &b" {\"result\":true}\n"[..],
        &b"<?xml version=\"1.0\"?><result/>"[..],
        &b"plain text"[..],
        &b"\x00\xff\xfe"[..],
        &b""[..],
    ] {
        let decoded = response(json!({"response":null,"status_code":200,"response_body_base64":STANDARD.encode(body),"response_headers":{}})).into_connector_response(true).unwrap();
        assert_eq!(decoded.body, body);
    }
}

#[test]
fn legacy_json_envelopes_and_json_string_responses_remain_compatible() {
    for value in [
        json!({"approved":true}),
        json!([1, 2]),
        json!("json string"),
        json!(null),
    ] {
        let decoded = response(json!({"response":value,"status_code":200,"response_headers":{"Content-Type":"application/json"}})).into_connector_response(true).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&decoded.body).unwrap(),
            value
        );
    }
}

#[test]
fn direct_connector_json_is_never_mistaken_for_a_proxy_envelope() {
    let value = json!({"response":"a connector field","status_code":999});
    let mut reply = response(value.clone());
    reply.response_body_base64 = Some(STANDARD.encode(serde_json::to_vec(&value).unwrap()));
    let decoded = reply.into_connector_response(false).unwrap();
    assert_eq!(decoded.status_code, 200);
    assert_eq!(
        serde_json::from_slice::<Value>(&decoded.body).unwrap(),
        value
    );
}

#[test]
fn malformed_proxy_responses_and_saas_failures_are_explicit_errors() {
    for value in [
        json!({"response":null,"status_code":70000}),
        json!({"status_code":200}),
        json!({"response":null,"status_code":200,"response_body_base64":"invalid"}),
        json!({"response":null,"status_code":200,"response_body_base64":42}),
        json!({"response":null,"status_code":200,"response_headers_multi":[42]}),
    ] {
        assert!(response(value).into_connector_response(true).is_err());
    }
    let mut reply = response(json!({"error":"invalid credentials"}));
    reply.status_code = 401;
    assert!(reply.into_connector_response(true).is_err());
}

#[test]
fn saas_failure_summary_reports_status_and_api_error_without_a_connector_reply() {
    let mut reply = response(
        json!({"error":{"type":"invalid_request","code":"IR_06","message":"Missing or non-scalar vault field: cvv"}}),
    );
    reply.status_code = 400;
    assert_eq!(
        reply.vault_proxy_failure_summary().unwrap(),
        "Vault proxy returned HTTP 400, error code: IR_06, message: Missing or non-scalar vault field: cvv"
    );
    let mut reply = response(json!({"error":"invalid credentials"}));
    reply.status_code = 401;
    assert_eq!(
        reply.vault_proxy_failure_summary().unwrap(),
        "Vault proxy returned HTTP 401, error code: unknown, message: unknown"
    );
    assert!(response(json!({"status_code":422}))
        .vault_proxy_failure_summary()
        .is_none());
}

#[tokio::test]
async fn non_ascii_response_headers_do_not_discard_the_connector_reply() {
    let (endpoint, server) = mock_http::serve(|_| mock_http::MockResponse {
        status: 200,
        headers: vec![
            ("Content-Type".into(), "application/json".into()),
            ("X-Note".into(), "café".into()),
        ],
        body: b"{\"approved\":true}".to_vec(),
    });
    let mut request = request(endpoint.clone(), ProxyBody::None, "application/json");
    request.connection_config.endpoint = url::Url::parse(&endpoint).unwrap();
    request.connection_config.http_method = HttpMethod::GET;
    request.connection_config.vault_connector_id = None;
    request.connection_config.vault_connector_type = None;
    request.connection_config.vault_endpoint = None;
    request.connection_config.vault_auth_data = None;
    request.connector_payload.body = None;
    let reply = injector_core(request)
        .await
        .unwrap()
        .into_connector_response(false)
        .unwrap();
    assert_eq!(reply.body, b"{\"approved\":true}");
    assert!(reply
        .headers
        .unwrap()
        .contains(&("x-note".to_owned(), "café".to_owned())));
    server.join().unwrap();
}
