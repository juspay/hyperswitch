//! Execute the locked generated client's serializers against an in-memory connector.
//! No response from this fixture certifies a server or permission scope.
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use aws_smithy_runtime_api::{
    client::{
        http::{http_client_fn, HttpConnector, HttpConnectorFuture, SharedHttpConnector},
        orchestrator::{HttpRequest, HttpResponse},
    },
    http::StatusCode,
};
use aws_smithy_types::{body::SdkBody, retry::RetryConfig, timeout::TimeoutConfig};

use crate::{
    administration::tests::request as direct_request,
    inventory::ReadRequest,
    types::{Resource, PAGED},
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Seen {
    method: String,
    path: String,
    query: BTreeMap<String, String>,
    body: Vec<u8>,
    org: Option<String>,
    workspace: Option<String>,
}
#[derive(Debug, Clone)]
struct Capture(Arc<Mutex<Vec<Seen>>>);
impl HttpConnector for Capture {
    fn call(&self, request: HttpRequest) -> HttpConnectorFuture {
        let trace = self.0.clone();
        HttpConnectorFuture::new(async move {
            let url = reqwest::Url::parse(request.uri()).unwrap();
            trace.lock().unwrap().push(Seen {
                method: request.method().to_owned(),
                path: url.path().to_owned(),
                query: url
                    .query_pairs()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect(),
                body: request.body().bytes().unwrap().to_vec(),
                org: request.headers().get("x-org-id").map(str::to_owned),
                workspace: request.headers().get("x-workspace").map(str::to_owned),
            });
            // Only serialization is asserted. Empty list output; workspace decoding may refuse.
            let mut response = HttpResponse::new(
                StatusCode::try_from(200).unwrap(),
                SdkBody::from(r#"{"total_pages":0,"total_items":0,"data":[]}"#),
            );
            response
                .headers_mut()
                .insert("content-type", "application/json");
            Ok(response)
        })
    }
}

#[tokio::test]
async fn verification_exports_use_locked_get_config_without_projection_filters() {
    let capture = Capture(Arc::new(Mutex::new(Vec::new())));
    let connector = SharedHttpConnector::new(capture.clone());
    let client = superposition_sdk::Client::from_conf(
        superposition_sdk::Config::builder()
            .behavior_version(superposition_sdk::config::BehaviorVersion::latest())
            .endpoint_url("https://synthetic.invalid")
            .bearer_token(superposition_sdk::config::Token::new(
                "synthetic-fixture",
                None,
            ))
            .http_client(http_client_fn(move |_, _| connector.clone()))
            .retry_config(RetryConfig::standard().with_max_attempts(1))
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_timeout(Duration::from_secs(2))
                    .build(),
            )
            .sleep_impl(aws_smithy_async::rt::sleep::TokioSleep::new())
            .build(),
    );
    for version in [None, Some("v-1")] {
        // Fixture deliberately does not return a config. This test certifies serialization only.
        let _ = client
            .get_config()
            .org_id("synthetic-org")
            .workspace_id("synthetic-workspace")
            .set_version(version.map(str::to_owned))
            .send()
            .await;
        let seen = capture.0.lock().unwrap().pop().unwrap();
        assert_eq!(seen.method, "POST");
        assert_eq!(seen.path, "/config");
        assert_eq!(seen.body, b"{}");
        assert_eq!(
            seen.query,
            version
                .into_iter()
                .map(|v| ("version".to_owned(), v.to_owned()))
                .collect()
        );
        assert_eq!(seen.org.as_deref(), Some("synthetic-org"));
        assert_eq!(seen.workspace.as_deref(), Some("synthetic-workspace"));
    }
}

#[tokio::test]
async fn all_seven_direct_read_operations_match_the_actual_locked_sdk_requests() {
    let capture = Capture(Arc::new(Mutex::new(Vec::new())));
    let connector = SharedHttpConnector::new(capture.clone());
    let client = superposition_sdk::Client::from_conf(
        superposition_sdk::Config::builder()
            .behavior_version(superposition_sdk::config::BehaviorVersion::latest())
            .endpoint_url("https://synthetic.invalid/sp")
            .bearer_token(superposition_sdk::config::Token::new(
                "synthetic-fixture",
                None,
            ))
            .http_client(http_client_fn(move |_, _| connector.clone()))
            .retry_config(RetryConfig::standard().with_max_attempts(1))
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_timeout(Duration::from_secs(2))
                    .build(),
            )
            .sleep_impl(aws_smithy_async::rt::sleep::TokioSleep::new())
            .build(),
    );
    let org = "synthetic_org";
    let workspace = "synthetic_workspace";
    for resource in PAGED
        .into_iter()
        .chain([Resource::Workspace, Resource::VersionHead])
    {
        match resource {
            Resource::Contexts => {
                let _ = client
                    .list_contexts()
                    .org_id(org)
                    .workspace_id(workspace)
                    .count(17)
                    .page(2)
                    .all(false)
                    .send()
                    .await;
            }
            Resource::Defaults => {
                let _ = client
                    .list_default_configs()
                    .org_id(org)
                    .workspace_id(workspace)
                    .count(17)
                    .page(2)
                    .all(false)
                    .send()
                    .await;
            }
            Resource::Dimensions => {
                let _ = client
                    .list_dimensions()
                    .org_id(org)
                    .workspace_id(workspace)
                    .count(17)
                    .page(2)
                    .all(false)
                    .send()
                    .await;
            }
            Resource::Experiments => {
                let _ = client
                    .list_experiment()
                    .org_id(org)
                    .workspace_id(workspace)
                    .count(17)
                    .page(2)
                    .all(false)
                    .send()
                    .await;
            }
            Resource::ExperimentGroups => {
                let _ = client
                    .list_experiment_groups()
                    .org_id(org)
                    .workspace_id(workspace)
                    .count(17)
                    .page(2)
                    .all(false)
                    .send()
                    .await;
            }
            Resource::Workspace => {
                let _ = client
                    .get_workspace()
                    .org_id(org)
                    .workspace_name(workspace)
                    .send()
                    .await;
            }
            Resource::VersionHead => {
                let _ = client
                    .list_versions()
                    .org_id(org)
                    .workspace_id(workspace)
                    .count(1)
                    .page(1)
                    .send()
                    .await;
            }
        }
        let read = match resource {
            Resource::Workspace => ReadRequest::Workspace,
            Resource::VersionHead => ReadRequest::VersionHead,
            _ => ReadRequest::Page {
                resource,
                number: 2,
                count: 17,
            },
        };
        let request = direct_request(read);
        let expected = Seen {
            method: request.method().to_string(),
            path: request.url().path().to_owned(),
            query: request
                .url()
                .query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect(),
            body: request
                .body()
                .and_then(|b| b.as_bytes())
                .unwrap_or_default()
                .to_vec(),
            org: request
                .headers()
                .get("x-org-id")
                .map(|v| v.to_str().unwrap().to_owned()),
            workspace: request
                .headers()
                .get("x-workspace")
                .map(|v| v.to_str().unwrap().to_owned()),
        };
        let mut calls = capture.0.lock().unwrap();
        assert_eq!(calls.len(), 1, "{resource:?}");
        assert_eq!(calls.remove(0), expected, "{resource:?}");
    }
}
