#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{
    super::inventory::tests::{n, target},
    *,
};

pub(crate) fn request(request: ReadRequest) -> reqwest::Request {
    DirectAdminReader::new(
        target(),
        HeaderValue::from_static("Bearer synthetic-fixture"),
    )
    .unwrap()
    .request(request, Duration::from_secs(1))
    .unwrap()
}

#[test]
fn wire_surface_is_closed_unfiltered_and_preserves_registered_base_path() {
    for (resource, method, path) in [
        (Resource::Contexts, "GET", "/sp/context"),
        (Resource::Defaults, "GET", "/sp/default-config"),
        (Resource::Dimensions, "GET", "/sp/dimension"),
        (Resource::Experiments, "POST", "/sp/experiments/list"),
        (
            Resource::ExperimentGroups,
            "POST",
            "/sp/experiment-groups/list",
        ),
    ] {
        let request = request(ReadRequest::Page {
            resource,
            number: 2,
            count: 17,
        });
        assert_eq!(request.method(), method);
        assert_eq!(request.url().path(), path);
        assert_eq!(
            request.url().query_pairs().collect::<Vec<_>>(),
            vec![
                ("count".into(), "17".into()),
                ("page".into(), "2".into()),
                ("all".into(), "false".into())
            ]
        );
        assert_eq!(request.headers()["x-org-id"], "synthetic_org");
        assert_eq!(request.headers()["x-workspace"], "synthetic_workspace");
        assert!(request.headers()[AUTHORIZATION].is_sensitive());
        assert!(!request.headers().contains_key("if-modified-since"));
        assert!(!request.headers().contains_key("if-none-match"));
        if method == "POST" {
            assert_eq!(request.body().unwrap().as_bytes(), Some(b"{}".as_slice()));
        } else {
            assert!(request.body().is_none());
        }
    }
    let workspace = request(ReadRequest::Workspace);
    assert_eq!(workspace.url().path(), "/sp/workspaces/synthetic_workspace");
    assert!(workspace.url().query().is_none());
    assert!(!workspace.headers().contains_key("x-workspace"));
    let versions = request(ReadRequest::VersionHead);
    assert_eq!(versions.url().path(), "/sp/config/versions");
    assert_eq!(versions.url().query(), Some("count=1&page=1"));
}

#[test]
fn real_registry_has_no_certified_targets_and_no_ambient_default() {
    let error = DirectAdminReader::from_registered(
        &n("candidate-0-115-5"),
        HeaderValue::from_static("Bearer synthetic-fixture"),
    )
    .err()
    .unwrap();
    assert_eq!(
        error.current_context().kind,
        InventoryErrorKind::CertificationPending
    );
}

#[test]
fn a_different_credential_cannot_inherit_registered_principal_permissions() {
    let error = DirectAdminReader::new(
        target(),
        HeaderValue::from_static("Bearer another-synthetic-reader"),
    )
    .err()
    .unwrap();
    assert_eq!(
        error.current_context().kind,
        InventoryErrorKind::PermissionScopeUnproven
    );
    assert!(!format!("{error:?}").contains("another-synthetic-reader"));
}

#[tokio::test]
async fn synthetic_proof_cannot_dispatch_an_http_request() {
    let reader = DirectAdminReader::new(
        target(),
        HeaderValue::from_static("Bearer synthetic-fixture"),
    )
    .unwrap();
    let error = reader
        .read(ReadRequest::VersionHead, Duration::from_millis(1), 100)
        .await
        .err()
        .unwrap();
    assert_eq!(
        error.current_context().kind,
        InventoryErrorKind::CertificationPending
    );
}

#[test]
fn target_injection_unsupported_requests_and_non_bearer_headers_refuse() {
    for endpoint in [
        "https://synthetic.invalid/sp?prefix=dynamic_fields",
        "https://synthetic.invalid/sp#fragment",
        "https://user:password@synthetic.invalid/sp",
        "http://127.0.0.1/sp",
    ] {
        let mut target = target();
        target.endpoint = endpoint.parse().unwrap();
        assert!(DirectAdminReader::new(
            target,
            HeaderValue::from_static("Bearer synthetic-fixture")
        )
        .is_err());
    }
    let mut invalid = target();
    invalid.identity.workspace = n("workspace/escape");
    assert!(invalid.validate().is_err());
    assert!(DirectAdminReader::new(target(), HeaderValue::from_static("Basic synthetic")).is_err());
    let reader = DirectAdminReader::new(
        target(),
        HeaderValue::from_static("Bearer synthetic-fixture"),
    )
    .unwrap();
    for query in [
        ReadRequest::Page {
            resource: Resource::Workspace,
            number: 1,
            count: 2,
        },
        ReadRequest::Page {
            resource: Resource::Contexts,
            number: 0,
            count: 2,
        },
        ReadRequest::Page {
            resource: Resource::Contexts,
            number: 1,
            count: 0,
        },
    ] {
        assert!(reader.request(query, Duration::from_secs(1)).is_err());
    }
}
