//! Real-server assertions. Only verified disposable execution can supply server
//! evidence. Negative bootstrap unit tests never certify a scripted server.
use serde_json::{json, Value};

use crate::{
    api::Api,
    contract::{self, CaseResult, MAX_PAGES},
    refuse, Result,
};

fn ensure(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(refuse(message))
    }
}
async fn success(api: &Api, op: &str, input: Value) -> Result<Value> {
    api.call(op, input).await?.success()
}
async fn version(api: &Api) -> Result<Value> {
    let v = success(api, "versions", json!({"count": 1})).await?;
    ensure(v["data"].is_array(), "version list missing data")?;
    Ok(v["data"][0].clone())
}
pub async fn inventory(api: &Api, kind: &str, id: &str) -> Result<Vec<Value>> {
    let before = version(api).await?;
    let first = inventory_pass(api, kind, id).await?;
    let second = inventory_pass(api, kind, id).await?;
    ensure(
        before == version(api).await?,
        "inventory version changed between passes",
    )?;
    contract::stable_rows(first, second)
}
async fn inventory_pass(api: &Api, kind: &str, id: &str) -> Result<Vec<Value>> {
    let before = version(api).await?;
    let first = success(api, kind, json!({"page":1,"count":2})).await?;
    let n = first["total_pages"]
        .as_u64()
        .ok_or_else(|| refuse("missing pagination total"))? as usize;
    ensure(n <= MAX_PAGES, "page cap exceeded")?;
    let mut pages = vec![first];
    for page in 2..=n {
        pages.push(success(api, kind, json!({"page":page,"count":2})).await?);
    }
    contract::pages(&pages, 2, &before, &version(api).await?, id)
}
fn row<'a>(rows: &'a [Value], id: &str) -> Result<&'a Value> {
    rows.iter()
        .find(|r| r["id"] == id)
        .ok_or_else(|| refuse("owned row absent"))
}
fn context(value: &str, fields: Value) -> Value {
    json!({"context":{"rf_scope":value},"override":fields,"description":"synthetic description preserved"})
}
fn get_id(row: &Value) -> Result<String> {
    row["id"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| refuse("response missing server ID"))
}

async fn phase(api: &Api, name: &str) -> Result<Value> {
    match name {
        "target_identity" => {
            let absent = api.call("get_workspace", json!({})).await?;
            ensure(
                absent.status == Some(404),
                "workspace must be absent before create; never reuse",
            )?;
            success(api, "create_workspace", json!({})).await?;
            let owned = success(api, "get_workspace", json!({})).await?;
            ensure(
                owned["workspace_name"] == api.workspace
                    && owned["organisation_id"] == api.org
                    && owned["workspace_admin_email"] == "rf-fixture@example.invalid",
                "workspace ownership identity mismatch",
            )?;
            for op in ["contexts", "defaults", "experiments"] {
                let v = success(api, op, json!({"count":2})).await?;
                ensure(
                    v["total_items"] == 0 && v["data"] == json!([]),
                    "fixture must start empty",
                )?;
            }
            Ok(json!({"fresh_workspace":true,"synthetic_only":true}))
        }
        "prerequisites" => {
            let schema = json!({"type":"string","enum":["a","b"],"minLength":1});
            success(
                api,
                "create_dimension",
                json!({"key":"rf_scope","schema":schema}),
            )
            .await?;
            let expanded = json!({"type":"string","enum":["a","b","c","d","rollback","experiment","unrelated"],"minLength":1});
            success(
                api,
                "update_dimension",
                json!({"key":"rf_scope","schema":expanded}),
            )
            .await?;
            let dimensions = inventory(api, "dimensions", "dimension").await?;
            let dim = dimensions
                .iter()
                .find(|v| v["dimension"] == "rf_scope")
                .ok_or_else(|| refuse("dimension missing"))?;
            ensure(
                dim["schema"] == expanded && dim["description"] == "synthetic dimension",
                "dimension schema/description not preserved",
            )?;
            for (key, value, schema) in [
                ("rf.required", json!(false), json!({"type":"boolean"})),
                (
                    "rf.manual",
                    json!("baseline"),
                    json!({"type":"string","maxLength":128}),
                ),
                (
                    "rf.options",
                    json!(["A", "B"]),
                    json!({"type":"array","items":{"type":"string"}}),
                ),
            ] {
                success(
                    api,
                    "create_default",
                    json!({"key":key,"value":value,"schema":schema}),
                )
                .await?;
            }
            let before = inventory(api, "defaults", "key").await?;
            let conflict = api
                .call(
                    "create_default",
                    json!({"key":"rf.required","value":true,"schema":{"type":"boolean"}}),
                )
                .await?;
            ensure(
                conflict.status.is_some_and(|s| (400..500).contains(&s)) && !conflict.sdk_ok,
                "default conflict must not silently succeed",
            )?;
            ensure(
                inventory(api, "defaults", "key").await? == before,
                "default conflict modified existing row/schema/functions",
            )?;
            let invalid = api
                .call(
                    "put",
                    context("a", json!({"rf.required":"invalid boolean"})),
                )
                .await?;
            ensure(
                invalid.status.is_some_and(|s| (400..500).contains(&s)),
                "invalid override accepted",
            )?;
            ensure(
                inventory(api, "contexts", "id").await?.is_empty(),
                "invalid prerequisite-dependent write persisted",
            )?;
            Ok(
                json!({"dimension_schema":expanded,"defaults":before,"separate_prerequisite_commits":true}),
            )
        }
        "pagination" => {
            for value in ["a", "b", "c"] {
                success(
                    api,
                    "put",
                    context(value, json!({"rf.required":true,"rf.manual":"keep me"})),
                )
                .await?;
            }
            let rows = inventory(api, "contexts", "id").await?;
            ensure(
                rows.len() == 3,
                "three fixture rows across two pages expected",
            )?;
            for r in &rows {
                ensure(
                    r["override"]["rf.manual"] == "keep me"
                        && r["description"] == "synthetic description preserved",
                    "full list lost metadata",
                )?;
            }
            Ok(json!({"count":rows.len(),"page_size":2,"full_rows":rows}))
        }
        "prefix_preservation" => {
            let full = inventory(api, "contexts", "id").await?;
            let filtered =
                success(api, "contexts", json!({"prefix":"rf.required","count":2})).await?;
            ensure(
                filtered["data"].as_array().is_some_and(|rows| {
                    !rows.is_empty()
                        && rows
                            .iter()
                            .all(|r| r["override"].get("rf.manual").is_none())
                }),
                "prefix stripping contract changed",
            )?;
            ensure(
                inventory(api, "contexts", "id").await? == full,
                "filtered read changed full inventory",
            )?;
            Ok(json!({"filtered_is_lossy":true,"never_used_for_replacement":true}))
        }
        "merge_replace_delete" => {
            let before = inventory(api, "contexts", "id").await?;
            let a = before
                .iter()
                .find(|r| r["value"] == json!({"rf_scope":"a"}))
                .ok_or_else(|| refuse("context a missing"))?;
            let id = get_id(a)?;
            success(api, "put", context("a", json!({"rf.options":["A","B"]}))).await?;
            let merged = inventory(api, "contexts", "id").await?;
            ensure(
                row(&merged, &id)?["override"]
                    == json!({"rf.required":true,"rf.manual":"keep me","rf.options":["A","B"]}),
                "create-on-existing did not merge",
            )?;
            success(api, "replace", json!({"id":id,"override":{"rf.manual":"keep me","rf.options":["A","B"]},"description":a["description"]})).await?;
            let after = inventory(api, "contexts", "id").await?;
            let replaced = row(&after, &id)?;
            ensure(
                replaced["override"] == json!({"rf.manual":"keep me","rf.options":["A","B"]})
                    && replaced["description"] == a["description"]
                    && replaced["value"] == a["value"],
                "replacement/removal lost unrelated state",
            )?;
            success(api, "delete", json!({"id":id})).await?;
            let missing = api.call("delete", json!({"id":id})).await?;
            ensure(
                missing.status.is_some_and(|s| (400..500).contains(&s)) && !missing.sdk_ok,
                "delete-missing must report rejection",
            )?;
            ensure(
                inventory(api, "contexts", "id")
                    .await?
                    .iter()
                    .all(|r| r["id"] != id),
                "deleted context returned",
            )?;
            Ok(
                json!({"merge":true,"replacement_removes_key":true,"unrelated_preserved":true,"delete_missing_status":missing.status}),
            )
        }
        "bulk_rollback" => {
            let before = inventory(api, "contexts", "id").await?;
            let version_before = version(api).await?;
            let defaults = inventory(api, "defaults", "key").await?;
            let reply = api
                .call(
                    "bulk",
                    json!([
                        {"PUT": context("rollback",json!({"rf.required":true}))},
                        {"PUT": context("a",json!({"rf.required":"invalid boolean"}))}
                    ]),
                )
                .await?;
            ensure(
                reply.status.is_some_and(|s| (400..500).contains(&s)) && reply.dispatches == 1,
                "invalid bulk must reject in one dispatch",
            )?;
            ensure(
                inventory(api, "contexts", "id").await? == before
                    && version(api).await? == version_before,
                "bulk partially committed",
            )?;
            ensure(
                inventory(api, "defaults", "key").await? == defaults,
                "context rollback changed prior prerequisites",
            )?;
            Ok(
                json!({"context_rollback":true,"prerequisites_remain":true,"dispatches":1,"classification":"PartialApplied when prerequisite commits belong to this run"}),
            )
        }
        "versions_order" => {
            let old = success(api, "config", json!({})).await?;
            let response = api
                .call(
                    "bulk",
                    json!([
                        {"PUT":context("a",json!({"rf.required":true}))},
                        {"PUT":context("d",json!({"rf.manual":"independent"}))}
                    ]),
                )
                .await?;
            let version = response.version.clone();
            response.success()?;
            let latest = success(api, "config", json!({})).await?;
            let repeat = success(api, "config", json!({})).await?;
            ensure(
                latest == repeat && old != latest && version.is_some(),
                "missing config version or unstable export order",
            )?;
            let at_version = success(api, "config", json!({"version":version})).await?;
            ensure(at_version == latest, "versioned/latest export disagreement")?;
            Ok(json!({"version":version,"context_order":latest["contexts"],"export":latest}))
        }
        "ordered_array_identity" => {
            success(api, "put", context("a", json!({"rf.options":["A","B"]}))).await?;
            success(api, "put", context("d", json!({"rf.options":["B","A"]}))).await?;
            // Replace both with otherwise identical maps to isolate array order.
            let rows = inventory(api, "contexts", "id").await?;
            for (scope, options) in [("a", json!(["A", "B"])), ("d", json!(["B", "A"]))] {
                let r = rows
                    .iter()
                    .find(|r| r["value"] == json!({"rf_scope":scope}))
                    .ok_or_else(|| refuse("array context missing"))?;
                success(api,"replace",json!({"id":r["id"],"override":{"rf.options":options},"description":r["description"]})).await?;
            }
            let rows = inventory(api, "contexts", "id").await?;
            let a = rows
                .iter()
                .find(|r| r["value"] == json!({"rf_scope":"a"}))
                .ok_or_else(|| refuse("array row a missing"))?;
            let d = rows
                .iter()
                .find(|r| r["value"] == json!({"rf_scope":"d"}))
                .ok_or_else(|| refuse("array row d missing"))?;
            let exported = success(api, "config", json!({})).await?;
            let collision = a["override_id"] == d["override_id"];
            Ok(
                json!({"order_only_hash_collision":collision,"direct_rows":[a,d],"exported_overrides":exported["overrides"],"safe_independent_order":!collision}),
            )
        }
        "experiments" => {
            let unrelated = success(
                api,
                "create_experiment",
                json!({"context":{"rf_scope":"unrelated"},"name":"rf_unrelated_experiment"}),
            )
            .await?;
            let active = success(
                api,
                "create_experiment",
                json!({"context":{"rf_scope":"experiment"}}),
            )
            .await?;
            let id = get_id(&active)?;
            success(api, "ramp_experiment", json!({"id":id})).await?;
            let before = inventory(api, "experiments", "id").await?;
            let contexts = inventory(api, "contexts", "id").await?;
            let owned = before
                .iter()
                .find(|e| e["id"] == id)
                .ok_or_else(|| refuse("active experiment absent"))?;
            let context_id = owned["variants"][0]["context_id"]
                .as_str()
                .ok_or_else(|| refuse("experiment context reference absent"))?;
            let rejected = api.call("delete", json!({"id":context_id})).await?;
            ensure(
                rejected.status.is_some_and(|s| (400..500).contains(&s)),
                "experiment-referenced context deletion accepted",
            )?;
            ensure(
                inventory(api, "contexts", "id").await? == contexts
                    && inventory(api, "experiments", "id").await? == before,
                "experiment rejection changed context/experiment state",
            )?;
            let overlapping = api
                .call("put", context("experiment", json!({"rf.required":true})))
                .await?;
            // Observe policy; never assume all builds reject all overlap.
            if overlapping.status.is_none_or(|s| s >= 500) {
                return Err(refuse("unknown overlap write outcome"));
            }
            let after = inventory(api, "experiments", "id").await?;
            let unrelated_id = get_id(&unrelated)?;
            ensure(
                row(&before, &unrelated_id)? == row(&after, &unrelated_id)?,
                "unrelated experiment changed",
            )?;
            Ok(
                json!({"referenced_delete_status":rejected.status,"overlapping_put_status":overlapping.status,"unrelated_preserved":true}),
            )
        }
        _ => Err(refuse(
            "phase requires unavailable server instrumentation or scoped identity",
        )),
    }
}

pub async fn run(api: &Api) -> Vec<CaseResult> {
    let mut results = contract::pending_cases();
    // Dependencies are explicit; stop writes on any unknown/failed outcome.
    for name in [
        "target_identity",
        "prerequisites",
        "pagination",
        "prefix_preservation",
        "merge_replace_delete",
        "bulk_rollback",
        "versions_order",
        "ordered_array_identity",
        "experiments",
    ] {
        let r = results
            .iter_mut()
            .find(|r| r.id == name)
            .expect("declared case");
        match phase(api, name).await {
            Ok(observation) => {
                r.status = if name == "ordered_array_identity"
                    && observation["order_only_hash_collision"] == true
                {
                    "unsupported"
                } else {
                    "passed"
                }
                .into();
                r.detail = "Executed with the locked SDK against the verified disposable server; see observation".into();
                r.observation = observation;
            }
            Err(e) => {
                r.status = "failed".into();
                r.detail = format!("{e:?}").chars().take(1024).collect();
                break;
            }
        }
    }
    results
}
