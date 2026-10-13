#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use base64::{engine::general_purpose::STANDARD, Engine};
use common_utils::proxy::{ProxyBody, ProxyMultipartPart};
use serde_json::{json, Value};

fn vault() -> Value {
    json!({"card": {"card_number": "4111111111111111", "card_holder_name": "A&B <\"O'Neil\"> + café", "card_exp_month": "09", "card_exp_year": "2030", "card_cvc": "123", "number": 42}})
}

fn text(body: ProxyBody, content_type: Option<&str>) -> String {
    String::from_utf8(body.prepare(&vault(), content_type).unwrap().bytes.unwrap()).unwrap()
}

#[test]
fn legacy_json_is_compatible_and_arrays_and_embedded_tokens_are_resolved() {
    let body = ProxyBody::from_legacy(json!({"card": "{{$card_number}}", "items": [{"name": "prefix {{ $card_holder_name }} suffix", "number": "{{$number}}"}], "literal": "{{ordinary text}}", "amount": 100}), Some("Application/JSON; charset=UTF-8")).unwrap();
    let actual: Value =
        serde_json::from_str(&text(body, Some("application/json; charset=UTF-8"))).unwrap();
    assert_eq!(actual["card"], "4111111111111111");
    assert_eq!(actual["items"][0]["number"], 42);
    assert_eq!(
        actual["items"][0]["name"],
        "prefix A&B <\"O'Neil\"> + café suffix"
    );
    assert_eq!(actual["literal"], "{{ordinary text}}");
    assert_eq!(actual["amount"], 100);
}

#[test]
fn whole_json_placeholders_preserve_objects_arrays_and_null_without_recursion() {
    let values = json!({"object": {"name": "{{$missing}}"}, "array": [1, false], "empty": null});
    let template = json!({"object": "{{$object}}", "array": "{{$array}}", "empty": "{{$empty}}"});
    for body in [
        ProxyBody::Json(template.to_string()),
        ProxyBody::from_legacy(template, None).unwrap(),
    ] {
        let actual: Value =
            serde_json::from_slice(&body.prepare(&values, None).unwrap().bytes.unwrap()).unwrap();
        assert_eq!(actual, values);
    }
    assert!(
        ProxyBody::Json(json!({"object": "prefix {{$object}}"}).to_string())
            .prepare(&values, None)
            .is_err()
    );
}

#[test]
fn json_keeps_original_order_whitespace_numeric_spelling_and_untouched_escapes() {
    let template = r#" { "z": 1e+02, "a": "\u0041", "items": ["{{$card_number}}", "prefix {{ $card_holder_name }}"], "{{$card_number}}": "literal key" } "#;
    let actual = text(ProxyBody::Json(template.into()), None);
    assert!(actual.starts_with(r#" { "z": 1e+02, "a": "\u0041", "items": ["4111111111111111", "#));
    assert!(actual.ends_with(r#"], "{{$card_number}}": "literal key" } "#));
    let value: Value = serde_json::from_str(&actual).unwrap();
    assert_eq!(value["items"][1], "prefix A&B <\"O'Neil\"> + café");
    let untouched = r#" { "z": 1e+02, "a": "\u0041", "items": [null, 1.0] } "#;
    assert_eq!(text(ProxyBody::Json(untouched.into()), None), untouched);
}

#[test]
fn xml_preserves_declaration_doctype_namespaces_and_existing_entities() {
    let template = r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE payment SYSTEM "payment.dtd"><p:payment xmlns:p="urn:payment" owner="{{$card_holder_name}}"><p:card>{{$card_number}}</p:card><p:existing>A&amp;B</p:existing></p:payment>"#;
    let expected = template
        .replace(
            "{{$card_holder_name}}",
            "A&amp;B &lt;&quot;O&apos;Neil&quot;&gt; + café",
        )
        .replace("{{$card_number}}", "4111111111111111");
    assert_eq!(
        text(
            ProxyBody::Xml(template.into()),
            Some("text/xml; charset=utf-8")
        ),
        expected
    );
}

#[test]
fn soap_and_cdata_survive_without_json_quotes() {
    let template = "<soap:Envelope xmlns:soap=\"urn:soap\"><soap:Body><![CDATA[{{$card_holder_name}}]]></soap:Body></soap:Envelope>";
    let body = ProxyBody::Xml(template.into())
        .prepare(
            &json!({"card_holder_name":"a]]>b"}),
            Some("application/soap+xml; charset=UTF-8"),
        )
        .unwrap();
    assert_eq!(String::from_utf8(body.bytes.unwrap()).unwrap(), "<soap:Envelope xmlns:soap=\"urn:soap\"><soap:Body><![CDATA[a]]]]><![CDATA[>b]]></soap:Body></soap:Envelope>");
}

#[test]
fn xml_attribute_whitespace_is_not_normalized_away() {
    let body = ProxyBody::Xml("<card name=\"{{$name}}\"/>".into())
        .prepare(&json!({"name":"a\nb\tc\rd"}), None)
        .unwrap();
    assert_eq!(
        String::from_utf8(body.bytes.unwrap()).unwrap(),
        "<card name=\"a&#10;b&#9;c&#13;d\"/>"
    );
}

#[test]
fn encoded_forms_preserve_duplicate_keys_order_and_literal_fields() {
    let template = "tag=one&name=%7B%7B%24card_holder_name%7D%7D&tag=two&card=%7b%7b%24card_number%7d%7d&unchanged=a+b%2fc&empty=&flag";
    let actual = text(ProxyBody::FormUrlEncoded(template.into()), None);
    let fields: Vec<_> = url::form_urlencoded::parse(actual.as_bytes())
        .into_owned()
        .collect();
    assert_eq!(fields[0], ("tag".into(), "one".into()));
    assert_eq!(fields[1], ("name".into(), "A&B <\"O'Neil\"> + café".into()));
    assert_eq!(fields[2], ("tag".into(), "two".into()));
    assert_eq!(fields[3], ("card".into(), "4111111111111111".into()));
    assert!(actual.ends_with("&unchanged=a+b%2fc&empty=&flag"));
    assert!(!actual.contains("%257B"));
}

#[test]
fn plain_text_uses_literal_substitution() {
    assert_eq!(
        text(
            ProxyBody::Text("holder={{$card_holder_name}}\ncard={{$card_number}}".into()),
            Some("text/plain; charset=UTF-8")
        ),
        "holder=A&B <\"O'Neil\"> + café\ncard=4111111111111111"
    );
}

#[test]
fn multipart_resolves_text_and_preserves_file_bytes_and_boundary() {
    let file = b"\x00\xff\xfe\r\n{{$card_number}}";
    let body = ProxyBody::Multipart(vec![
        ProxyMultipartPart::Text {
            name: "card".into(),
            value: "{{$card_number}}".into(),
        },
        ProxyMultipartPart::File {
            name: "evidence".into(),
            filename: "receipt.bin".into(),
            mime_type: "application/octet-stream".into(),
            data_base64: STANDARD.encode(file),
        },
    ])
    .prepare(
        &vault(),
        Some("multipart/form-data; boundary=fixture-boundary"),
    )
    .unwrap();
    assert_eq!(
        body.content_type.as_deref(),
        Some("multipart/form-data; boundary=fixture-boundary")
    );
    let bytes = body.bytes.unwrap();
    assert!(bytes.starts_with(b"--fixture-boundary\r\n"));
    assert!(bytes.ends_with(b"--fixture-boundary--\r\n"));
    assert!(bytes.windows(file.len()).any(|part| part == file));
    assert!(bytes.windows(16).any(|part| part == b"4111111111111111"));
}

#[test]
fn multipart_adds_a_boundary_when_prism_only_supplies_the_media_type() {
    let body = ProxyBody::Multipart(vec![])
        .prepare(&vault(), Some("multipart/form-data"))
        .unwrap();
    let boundary = body.content_type.unwrap();
    let boundary = boundary
        .strip_prefix("multipart/form-data; boundary=")
        .unwrap();
    assert_eq!(
        body.bytes.unwrap(),
        format!("--{boundary}--\r\n").as_bytes()
    );
}

#[test]
fn multipart_boundary_header_and_body_cannot_disagree() {
    for content_type in [
        "multipart/form-data; boundary=one; boundary=two",
        "multipart/form-data; boundary=one; BOUNDARY=one",
        "multipart/form-data; boundary",
        "multipart/form-data; boundary=",
        "multipart/form-data; boundary=\"unterminated",
        "multipart/form-data; boundary=unopened\"",
        "multipart/form-data; boundary=\"trailing \"",
        "multipart/form-data; boundary=unquoted:separator",
    ] {
        assert!(ProxyBody::Multipart(vec![])
            .prepare(&vault(), Some(content_type))
            .is_err());
    }
    let content_type = "multipart/form-data; boundary=\"fixture: boundary\"";
    let body = ProxyBody::Multipart(vec![])
        .prepare(&vault(), Some(content_type))
        .unwrap();
    assert_eq!(body.content_type.as_deref(), Some(content_type));
    assert_eq!(body.bytes.unwrap(), b"--fixture: boundary--\r\n");
    let generated = ProxyBody::Multipart(vec![])
        .prepare(&vault(), Some("multipart/form-data; charset=utf-8"))
        .unwrap();
    assert!(generated
        .content_type
        .unwrap()
        .starts_with("multipart/form-data; charset=utf-8; boundary=hs-proxy-"));
}

#[test]
fn utf8_templates_reject_conflicting_charset_and_xml_encoding_declarations() {
    let cases = [
        (ProxyBody::Json("{}".into()), "application/json"),
        (ProxyBody::Xml("<card/>".into()), "text/xml"),
        (
            ProxyBody::FormUrlEncoded("name=caf%C3%A9".into()),
            "application/x-www-form-urlencoded",
        ),
        (ProxyBody::Text("café".into()), "text/plain"),
        (ProxyBody::Multipart(vec![]), "multipart/form-data"),
    ];
    for (body, content_type) in cases {
        assert!(body
            .prepare(
                &vault(),
                Some(&format!("{content_type}; charset=ISO-8859-1"))
            )
            .is_err());
        assert!(body
            .prepare(
                &vault(),
                Some(&format!("{content_type}; charset=\"UTF-8\""))
            )
            .is_ok());
        assert!(body
            .prepare(
                &vault(),
                Some(&format!("{content_type}; charset=UTF-8; charset=UTF-16"))
            )
            .is_err());
    }
    assert!(ProxyBody::Xml(
        "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><card>café</card>".into()
    )
    .prepare(&vault(), Some("text/xml; charset=UTF-8"))
    .is_err());
    assert_eq!(
        text(
            ProxyBody::Xml("<?xml version=\"1.0\" encoding=\"UTF-8\"?><card>café</card>".into()),
            None
        ),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><card>café</card>"
    );
    assert_eq!(
        ProxyBody::RawBytes(STANDARD.encode(b"caf\xe9"))
            .prepare(&vault(), Some("text/plain; charset=ISO-8859-1"))
            .unwrap()
            .bytes
            .unwrap(),
        b"caf\xe9"
    );
}

#[test]
fn quoted_mime_parameters_do_not_become_boundary_or_charset_parameters() {
    let content_type =
        "multipart/form-data; hint=\"marker; boundary=fake; charset=ISO-8859-1\"; boundary=actual";
    let body = ProxyBody::Multipart(vec![])
        .prepare(&vault(), Some(content_type))
        .unwrap();
    assert_eq!(body.content_type.as_deref(), Some(content_type));
    assert_eq!(body.bytes.unwrap(), b"--actual--\r\n");
}

#[test]
fn raw_and_absent_bodies_are_distinct_and_binary_safe() {
    let bytes = b"\x00\xff{{$card_number}}";
    let body = ProxyBody::RawBytes(STANDARD.encode(bytes))
        .prepare(&vault(), None)
        .unwrap();
    assert_eq!(body.bytes.unwrap(), bytes);
    assert!(ProxyBody::None
        .prepare(&vault(), None)
        .unwrap()
        .bytes
        .is_none());
    assert_eq!(
        ProxyBody::Json("null".into())
            .prepare(&vault(), None)
            .unwrap()
            .bytes
            .unwrap(),
        b"null"
    );
    assert_eq!(
        ProxyBody::RawBytes(String::new())
            .prepare(&vault(), None)
            .unwrap()
            .bytes
            .unwrap(),
        b""
    );
}

#[test]
fn invalid_templates_fail_without_exposing_vault_values() {
    for body in [
        ProxyBody::Json(json!({"card":"{{$missing}}"}).to_string()),
        ProxyBody::Text("{{$card_number".into()),
        ProxyBody::Xml("<card>{{$missing}}</card>".into()),
        ProxyBody::Xml("<card><x></card>".into()),
        ProxyBody::Xml("<a/><b/>".into()),
        ProxyBody::Xml("<{{$card_number}}/>".into()),
        ProxyBody::Xml("<a><!--{{\t$card_number}}--></a>".into()),
        ProxyBody::FormUrlEncoded("card=%FF".into()),
        ProxyBody::FormUrlEncoded("card=%7".into()),
        ProxyBody::RawBytes("not-base64".into()),
    ] {
        let error = body.prepare(&vault(), None).unwrap_err().to_string();
        assert!(!error.contains("4111111111111111"));
    }
    assert!(ProxyBody::Xml("<card>{{$name}}</card>".into())
        .prepare(&json!({"name":"\u{0}"}), None)
        .is_err());
    assert!(ProxyBody::Xml("<card/>".into())
        .prepare(&vault(), Some("application/json"))
        .is_err());
}

#[test]
fn multipart_rejects_header_injection_and_boundary_collisions() {
    for mime_type in ["", "not a media type", "image/png\r\nInjected: yes"] {
        assert!(ProxyBody::Multipart(vec![ProxyMultipartPart::File {
            name: "file".into(),
            filename: "fixture.bin".into(),
            mime_type: mime_type.into(),
            data_base64: String::new(),
        }])
        .prepare(&vault(), None)
        .is_err());
    }
    let part = ProxyMultipartPart::Text {
        name: "bad\r\nInjected: yes".into(),
        value: "value".into(),
    };
    assert!(ProxyBody::Multipart(vec![part])
        .prepare(&vault(), None)
        .is_err());
    let part = ProxyMultipartPart::Text {
        name: "name".into(),
        value: "contains-boundary".into(),
    };
    assert!(ProxyBody::Multipart(vec![part])
        .prepare(&vault(), Some("multipart/form-data; boundary=boundary"))
        .is_err());
}

#[test]
fn transport_round_trips_all_body_variants() {
    for body in [
        ProxyBody::Json(json!({"card":"{{$card_number}}"}).to_string()),
        ProxyBody::Xml("<card>{{$card_number}}</card>".into()),
        ProxyBody::FormUrlEncoded("card=%7B%7B%24card_number%7D%7D".into()),
        ProxyBody::Text("{{$card_number}}".into()),
        ProxyBody::Multipart(vec![ProxyMultipartPart::Text {
            name: "card".into(),
            value: "{{$card_number}}".into(),
        }]),
        ProxyBody::RawBytes(STANDARD.encode(b"\xff\x00")),
        ProxyBody::None,
    ] {
        let envelope = serde_json::to_vec(&body).unwrap();
        let received: ProxyBody = serde_json::from_slice(&envelope).unwrap();
        assert!(received.prepare(&vault(), None).is_ok());
    }
}

#[test]
fn form_escapes_are_validated_and_plus_signs_survive_decoding() {
    assert_eq!(
        text(
            ProxyBody::FormUrlEncoded("card=%7B%7B%24card_number%7D%7D%2Bx+y".into()),
            None
        ),
        "card=4111111111111111%2Bx+y"
    );
    for template in [
        "card=%",
        "card=%4",
        "card=%zz{{$card_number}}",
        "%7B%7B%24card_number%7D%7D=1",
    ] {
        assert!(ProxyBody::FormUrlEncoded(template.into())
            .prepare(&vault(), None)
            .is_err());
    }
}

#[test]
fn only_json_bodies_are_duplicated_into_the_legacy_request_field() {
    assert_eq!(
        ProxyBody::Json("{\"card\":\"{{$card_number}}\"}".into()).legacy_value(),
        json!({"card": "{{$card_number}}"})
    );
    for body in [
        ProxyBody::Xml("<card/>".into()),
        ProxyBody::FormUrlEncoded("card=1".into()),
        ProxyBody::Text("card".into()),
        ProxyBody::Multipart(vec![]),
        ProxyBody::RawBytes(STANDARD.encode(b"card")),
        ProxyBody::None,
    ] {
        assert_eq!(body.legacy_value(), Value::Null);
    }
}
