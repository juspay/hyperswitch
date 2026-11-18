use std::{collections::HashMap, time::Duration};

use aws_smithy_runtime_api::client::http::{http_client_fn, SharedHttpConnector};
use aws_smithy_types::{retry::RetryConfig, timeout::TimeoutConfig, Document, Number};
use serde_json::{json, Value};
use superposition_sdk::{
    config::{BehaviorVersion, Token},
    types::{
        ContextAction, ContextIdentifier, ContextPut, UpdateContextOverrideRequest, Variant,
        VariantType,
    },
    Client,
};

use crate::{contract::bounded_bulk, refuse, transport::Trace, Result};

pub fn document(value: &Value) -> Document {
    match value {
        Value::Null => Document::Null,
        Value::Bool(v) => Document::Bool(*v),
        Value::String(v) => Document::String(v.clone()),
        Value::Array(v) => Document::Array(v.iter().map(document).collect()),
        Value::Object(v) => {
            Document::Object(v.iter().map(|(k, v)| (k.clone(), document(v))).collect())
        }
        Value::Number(v) => Document::Number(if let Some(n) = v.as_u64() {
            Number::PosInt(n)
        } else if let Some(n) = v.as_i64() {
            Number::NegInt(n)
        } else {
            Number::Float(v.as_f64().expect("JSON number"))
        }),
    }
}
pub fn object(value: &Value) -> Result<HashMap<String, Document>> {
    if let Document::Object(v) = document(value) {
        Ok(v)
    } else {
        Err(refuse("expected object"))
    }
}
pub fn client(
    connector: SharedHttpConnector,
    endpoint: &str,
    attempts: u32,
    timeout: Duration,
) -> Client {
    Client::from_conf(
        superposition_sdk::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url(endpoint)
            .bearer_token(Token::new("required-fields-synthetic-fixture", None))
            .http_client(http_client_fn(move |_, _| connector.clone()))
            .retry_config(RetryConfig::standard().with_max_attempts(attempts))
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_timeout(timeout)
                    .operation_attempt_timeout(timeout)
                    .build(),
            )
            .sleep_impl(aws_smithy_async::rt::sleep::TokioSleep::new())
            .build(),
    )
}
pub fn put(input: &Value) -> Result<ContextPut> {
    ContextPut::builder()
        .set_context(Some(object(&input["context"])?))
        .set_override(Some(object(&input["override"])?))
        .description(input["description"].as_str().unwrap_or("synthetic fixture"))
        .change_reason("required-fields disposable API contract")
        .build()
        .map_err(|_| refuse("invalid context fixture"))
}
pub fn replacement(input: &Value) -> Result<UpdateContextOverrideRequest> {
    UpdateContextOverrideRequest::builder()
        .context(ContextIdentifier::Id(
            input["id"]
                .as_str()
                .ok_or_else(|| refuse("missing id"))?
                .into(),
        ))
        .set_override(Some(object(&input["override"])?))
        .description(input["description"].as_str().unwrap_or("synthetic fixture"))
        .change_reason("required-fields disposable API contract")
        .build()
        .map_err(|_| refuse("invalid replacement fixture"))
}

pub struct Api {
    pub reader: Client,
    pub writer: Client,
    pub trace: Trace,
    pub org: String,
    pub workspace: String,
}
#[derive(Debug)]
pub struct Reply {
    pub sdk_ok: bool,
    pub status: Option<u16>,
    pub body: Value,
    pub version: Option<String>,
    pub last_modified: Option<String>,
    pub metadata_required: bool,
    pub dispatches: usize,
}
impl Reply {
    pub fn success(self) -> Result<Value> {
        let metadata_ok = !self.metadata_required
            || (self.version.as_deref().is_some_and(|s| !s.is_empty())
                && self.last_modified.as_deref().is_some_and(|s| !s.is_empty()));
        if self.sdk_ok && metadata_ok && self.status.is_some_and(|s| (200..300).contains(&s)) {
            Ok(self.body)
        } else {
            Err(refuse(format!(
                "SDK/server response incompatible: HTTP {:?}, SDK decoded {}",
                self.status, self.sdk_ok
            )))
        }
    }
}
impl Api {
    /// Closed set of SDK operations. There is no arbitrary URL/body forwarding.
    pub async fn call(&self, op: &str, input: Value) -> Result<Reply> {
        let start = self.trace.lock().expect("trace lock").len();
        let w = &self.workspace;
        let org = &self.org;
        let r = &self.reader;
        let m = &self.writer;
        let page = input["page"].as_i64().unwrap_or(1) as i32;
        let count = input["count"].as_i64().unwrap_or(2) as i32;
        let key = input["key"].as_str().unwrap_or("");
        let prefix = input
            .get("prefix")
            .map(|p| vec![p.as_str().unwrap_or("").to_owned()]);
        let ok = match op {
            "get_workspace" => r
                .get_workspace()
                .org_id(org)
                .workspace_name(w)
                .send()
                .await
                .is_ok(),
            "create_workspace" => m
                .create_workspace()
                .org_id(org)
                .workspace_name(w)
                .workspace_admin_email("rf-fixture@example.invalid")
                .allow_experiment_self_approval(true)
                .auto_populate_control(false)
                .enable_context_validation(true)
                .enable_change_reason_validation(true)
                .send()
                .await
                .is_ok(),
            "contexts" => r
                .list_contexts()
                .org_id(org)
                .workspace_id(w)
                .page(page)
                .count(count)
                .set_prefix(prefix)
                .send()
                .await
                .is_ok(),
            "defaults" => r
                .list_default_configs()
                .org_id(org)
                .workspace_id(w)
                .page(page)
                .count(count)
                .send()
                .await
                .is_ok(),
            "dimensions" => r
                .list_dimensions()
                .org_id(org)
                .workspace_id(w)
                .page(page)
                .count(count)
                .send()
                .await
                .is_ok(),
            "experiments" => r
                .list_experiment()
                .org_id(org)
                .workspace_id(w)
                .page(page)
                .count(count)
                .send()
                .await
                .is_ok(),
            "versions" => r
                .list_versions()
                .org_id(org)
                .workspace_id(w)
                .page(page)
                .count(count)
                .send()
                .await
                .is_ok(),
            "config" => r
                .get_config()
                .org_id(org)
                .workspace_id(w)
                .set_version(input["version"].as_str().map(str::to_owned))
                .send()
                .await
                .is_ok(),
            "put" => m
                .create_context()
                .org_id(org)
                .workspace_id(w)
                .request(put(&input)?)
                .send()
                .await
                .is_ok(),
            "replace" => m
                .update_override()
                .org_id(org)
                .workspace_id(w)
                .request(replacement(&input)?)
                .send()
                .await
                .is_ok(),
            "delete" => m
                .delete_context()
                .org_id(org)
                .workspace_id(w)
                .id(input["id"]
                    .as_str()
                    .ok_or_else(|| refuse("missing context id"))?)
                .send()
                .await
                .is_ok(),
            "bulk" => {
                bounded_bulk(&input)?;
                let operations: Result<Vec<_>> = input
                    .as_array()
                    .expect("validated")
                    .iter()
                    .map(|v| {
                        if let Some(v) = v.get("PUT") {
                            Ok(ContextAction::Put(put(v)?))
                        } else if let Some(v) = v.get("REPLACE") {
                            Ok(ContextAction::Replace(replacement(v)?))
                        } else if let Some(v) = v.get("DELETE").and_then(Value::as_str) {
                            Ok(ContextAction::Delete(v.into()))
                        } else {
                            Err(refuse("unsupported bulk action"))
                        }
                    })
                    .collect();
                m.bulk_operation()
                    .org_id(org)
                    .workspace_id(w)
                    .set_operations(Some(operations?))
                    .send()
                    .await
                    .is_ok()
            }
            "create_dimension" => m
                .create_dimension()
                .org_id(org)
                .workspace_id(w)
                .dimension(key)
                .position(1)
                .set_schema(Some(object(&input["schema"])?))
                .description("synthetic dimension")
                .change_reason("required-fields disposable API contract")
                .send()
                .await
                .is_ok(),
            "update_dimension" => m
                .update_dimension()
                .org_id(org)
                .workspace_id(w)
                .dimension(key)
                .set_schema(Some(object(&input["schema"])?))
                .description("synthetic dimension")
                .change_reason("required-fields disposable API contract")
                .send()
                .await
                .is_ok(),
            "create_default" => m
                .create_default_config()
                .org_id(org)
                .workspace_id(w)
                .key(key)
                .value(document(&input["value"]))
                .set_schema(Some(object(&input["schema"])?))
                .description("synthetic default; preserve schema and functions")
                .change_reason("required-fields disposable API contract")
                .send()
                .await
                .is_ok(),
            "create_experiment" => {
                let variants: Result<Vec<_>> = ["CONTROL", "EXPERIMENTAL"]
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        Variant::builder()
                            .id(format!("rf_variant_{i}"))
                            .variant_type(VariantType::from(*t))
                            .set_overrides(Some(object(&json!({"rf.required": i == 1}))?))
                            .build()
                            .map_err(|_| refuse("invalid variant"))
                    })
                    .collect();
                m.create_experiment()
                    .org_id(org)
                    .workspace_id(w)
                    .name(input["name"].as_str().unwrap_or("rf_synthetic_experiment"))
                    .set_context(Some(object(&input["context"])?))
                    .set_variants(Some(variants?))
                    .description("synthetic experiment")
                    .change_reason("required-fields disposable API contract")
                    .send()
                    .await
                    .is_ok()
            }
            "ramp_experiment" => m
                .ramp_experiment()
                .org_id(org)
                .workspace_id(w)
                .id(input["id"]
                    .as_str()
                    .ok_or_else(|| refuse("missing experiment id"))?)
                .traffic_percentage(50)
                .change_reason("required-fields disposable API contract")
                .send()
                .await
                .is_ok(),
            _ => return Err(refuse("unimplemented operation")),
        };
        let trace = self.trace.lock().expect("trace lock");
        let last = trace.get(start..).and_then(|v| v.last());
        Ok(Reply {
            sdk_ok: ok,
            status: last.and_then(|v| v.status),
            body: last.and_then(|v| v.body.clone()).unwrap_or(Value::Null),
            version: last.and_then(|v| v.version.clone()),
            last_modified: last.and_then(|v| v.last_modified.clone()),
            metadata_required: op == "config",
            dispatches: trace.len() - start,
        })
    }
}
