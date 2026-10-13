#[path = "../../common_utils/tests/support/mock_http.rs"]
mod mock_http;

use base64::{engine::general_purpose::STANDARD, Engine};
use common_utils::{
    proxy::{ProxyBody, ProxyMultipartPart},
    request::Method,
};
use external_services::http_client::send_request;
use hyperswitch_interfaces::types::Proxy;
use serde_json::json;

#[tokio::test]
async fn proxy_prepared_bodies_reach_the_connector_in_their_actual_format() {
    let binary = b"\x00\xff\xfe{{$card_number}}";
    let cases = [
        (
            ProxyBody::Json(json!({"card":"{{$card_number}}"}).to_string()),
            "application/json",
            b"{\"card\":\"4111111111111111\"}".to_vec(),
        ),
        (
            ProxyBody::Xml("<?xml version=\"1.0\"?><card>{{$card_number}}</card>".into()),
            "text/xml; charset=utf-8",
            b"<?xml version=\"1.0\"?><card>4111111111111111</card>".to_vec(),
        ),
        (
            ProxyBody::FormUrlEncoded("card=%7B%7B%24card_number%7D%7D&tag=1&tag=2".into()),
            "application/x-www-form-urlencoded",
            b"card=4111111111111111&tag=1&tag=2".to_vec(),
        ),
        (
            ProxyBody::Text("card={{$card_number}}".into()),
            "text/plain; charset=UTF-8",
            b"card=4111111111111111".to_vec(),
        ),
        (
            ProxyBody::RawBytes(STANDARD.encode(binary)),
            "application/octet-stream",
            binary.to_vec(),
        ),
    ];
    for (body, content_type, expected) in cases {
        let response_bytes = b"\x00\xff<connector-error/>".to_vec();
        let expected_response = response_bytes.clone();
        let (url, server) = mock_http::serve(move |_| mock_http::MockResponse {
            status: 422,
            headers: vec![("Content-Type".into(), "application/octet-stream".into())],
            body: response_bytes,
        });
        let prepared = body
            .prepare(
                &json!({"card_number":"4111111111111111"}),
                Some(content_type),
            )
            .unwrap();
        let request = prepared.into_request(
            &url,
            Method::Post,
            vec![
                ("Authorization".into(), "Bearer fixture".into()),
                ("Content-Length".into(), "1".into()),
            ],
        );
        let reply = send_request(&Proxy::default(), request, Some(5))
            .await
            .unwrap();
        assert_eq!(reply.status().as_u16(), 422);
        assert_eq!(reply.bytes().await.unwrap().as_ref(), expected_response);
        let captured = server.join().unwrap();
        assert_eq!(captured.method, "POST");
        assert_eq!(captured.target, "/connector?merchant=fixture");
        assert_eq!(captured.header("content-type"), Some(content_type));
        assert_eq!(captured.header("authorization"), Some("Bearer fixture"));
        assert_eq!(captured.body, expected);
        assert_eq!(
            captured
                .header("content-length")
                .unwrap()
                .parse::<usize>()
                .unwrap(),
            expected.len()
        );
    }
}

#[tokio::test]
async fn multipart_file_bytes_and_text_fields_survive_real_http_forwarding() {
    let file = b"\x00\xff\xfe{{$card_number}}";
    let body = ProxyBody::Multipart(vec![
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
    ]);
    let prepared = body
        .prepare(
            &json!({"card_number":"4111111111111111"}),
            Some("multipart/form-data; boundary=fixture-boundary"),
        )
        .unwrap();
    let (url, server) = mock_http::serve(|_| mock_http::MockResponse {
        status: 204,
        headers: vec![],
        body: vec![],
    });
    let request = prepared.into_request(&url, Method::Post, vec![]);
    assert_eq!(
        send_request(&Proxy::default(), request, Some(5))
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    let captured = server.join().unwrap();
    assert_eq!(
        captured.header("content-type"),
        Some("multipart/form-data; boundary=fixture-boundary")
    );
    assert!(captured.body.windows(file.len()).any(|value| value == file));
    assert!(captured
        .body
        .windows(16)
        .any(|value| value == b"4111111111111111"));
}

#[tokio::test]
async fn bodyless_get_and_raw_delete_and_get_are_not_silently_changed() {
    for (method, body, expected) in [
        (Method::Get, ProxyBody::None, vec![]),
        (
            Method::Delete,
            ProxyBody::RawBytes(STANDARD.encode(b"\x00\xff")),
            vec![0, 255],
        ),
        (
            Method::Get,
            ProxyBody::RawBytes(STANDARD.encode(b"query-body")),
            b"query-body".to_vec(),
        ),
    ] {
        let (url, server) = mock_http::serve(|_| mock_http::MockResponse {
            status: 200,
            headers: vec![],
            body: vec![],
        });
        let prepared = body.prepare(&json!({}), None).unwrap();
        let request = prepared.into_request(&url, method, vec![]);
        send_request(&Proxy::default(), request, Some(5))
            .await
            .unwrap();
        let captured = server.join().unwrap();
        assert_eq!(captured.method, method.to_string());
        assert_eq!(captured.body, expected);
        if matches!(body, ProxyBody::None) {
            assert!(captured.header("content-type").is_none());
        }
    }
}
