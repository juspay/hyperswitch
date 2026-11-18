use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{refuse, Result};

pub const MAX_BODY: usize = 1_048_576;
pub const MAX_ACTIONS: usize = 16;
pub const MAX_PAGES: usize = 32;
pub const MAX_REQUESTS: usize = 512;
pub const CASES: &[&str] = &[
    "target_identity",
    "pagination",
    "prefix_preservation",
    "merge_replace_delete",
    "prerequisites",
    "bulk_rollback",
    "versions_order",
    "ordered_array_identity",
    "experiments",
    "auth_scope",
    "limits",
    "post_commit_webhook",
    "cache_publication",
    "dropped_response",
    "dispatch_timeout",
    "old_request_exclusion",
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Registration {
    pub schema_version: u32,
    pub run_id: String,
    pub docker_host: String,
    pub server_image: String,
    pub server_image_id: String,
    pub server_version: String,
    pub source_revision: String,
    pub server_container: String,
    pub redis_container: String,
    pub network: String,
    pub endpoint: String,
    pub org: String,
    pub workspace: String,
}
impl Registration {
    pub fn validate(&self) -> Result<()> {
        let hex = |s: &str, n| s.len() == n && s.bytes().all(|b| b.is_ascii_hexdigit());
        let url = reqwest::Url::parse(&self.endpoint).map_err(|_| refuse("invalid endpoint"))?;
        if self.schema_version != 1
            || !hex(&self.run_id, 32)
            || !self.docker_host.starts_with("unix:///")
            || !self
                .server_image
                .starts_with("ghcr.io/juspay/superposition-demo@sha256:")
            || !hex(self.server_image.rsplit(':').next().unwrap_or(""), 64)
            || !self.server_image_id.starts_with("sha256:")
            || !hex(&self.server_image_id[7..], 64)
            || !hex(&self.source_revision, 40)
            || !["0.102.0", "0.113.0", "0.115.5"].contains(&self.server_version.as_str())
            || !hex(&self.server_container, 64)
            || !hex(&self.redis_container, 64)
            || !hex(&self.network, 64)
            || self.org != "localorg"
            || self.workspace != format!("rf_contract_{}", self.run_id)
            || url.scheme() != "http"
            || url.host_str() != Some("127.0.0.1")
            || url.port().is_none()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(refuse("not an exact registered local disposable fixture"));
        }
        Ok(())
    }
}

/// Test-bound completeness guard, not the production CompleteInventory adapter.
pub fn pages(
    pages: &[Value],
    count: usize,
    before: &Value,
    after: &Value,
    id: &str,
) -> Result<Vec<Value>> {
    if pages.is_empty() || pages.len() > MAX_PAGES || before != after || count == 0 {
        return Err(refuse("missing, unbounded or changing inventory"));
    }
    let total = pages[0]["total_items"]
        .as_u64()
        .ok_or_else(|| refuse("missing total"))? as usize;
    let expected_pages = pages[0]["total_pages"]
        .as_u64()
        .ok_or_else(|| refuse("missing pages"))? as usize;
    // Some servers represent an empty set with zero pages; still inspect page one.
    if expected_pages != total.div_ceil(count) || pages.len() != expected_pages.max(1) {
        return Err(refuse("pagination totals do not account for every page"));
    }
    let mut rows = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, page) in pages.iter().enumerate() {
        if serde_json::to_vec(page)
            .map_err(|_| refuse("invalid JSON"))?
            .len()
            > MAX_BODY
            || page["total_items"].as_u64() != Some(total as u64)
            || page["total_pages"].as_u64() != Some(expected_pages as u64)
        {
            return Err(refuse("partial, oversized or changing page"));
        }
        let data = page["data"]
            .as_array()
            .ok_or_else(|| refuse("missing data"))?;
        if data.len() != count.min(total.saturating_sub(index * count)) {
            return Err(refuse("missing page rows"));
        }
        for row in data {
            let key = row[id]
                .as_str()
                .ok_or_else(|| refuse("missing row identity"))?;
            if !ids.insert(key.to_owned()) {
                return Err(refuse("duplicate row/page"));
            }
            rows.push(row.clone());
        }
    }
    if rows.len() != total {
        return Err(refuse("incomplete inventory"));
    }
    Ok(rows)
}

pub fn bounded_bulk(body: &Value) -> Result<()> {
    let ops = body
        .as_array()
        .ok_or_else(|| refuse("bulk must be one action array"))?;
    if ops.is_empty()
        || ops.len() > MAX_ACTIONS
        || serde_json::to_vec(body)
            .map_err(|_| refuse("invalid bulk"))?
            .len()
            > MAX_BODY
    {
        return Err(refuse("bulk exceeds local bound; no batching fallback"));
    }
    Ok(())
}

pub fn stable_rows(first: Vec<Value>, second: Vec<Value>) -> Result<Vec<Value>> {
    if first != second {
        return Err(refuse("full rows/order changed across inventory passes"));
    }
    Ok(first)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observation {
    Before,
    After,
    Diverged,
    Unavailable,
}
/// Test classifier only. A before-image is never permission to retry an old request.
pub fn outcome(
    dispatched: bool,
    prior_commit: bool,
    definite_rollback: bool,
    observation: Observation,
) -> &'static str {
    if !dispatched {
        return if prior_commit {
            "PartialApplied"
        } else {
            "Refused"
        };
    }
    match observation {
        Observation::After => "CommittedNeedsVerification",
        Observation::Before if definite_rollback => {
            if prior_commit {
                "PartialApplied"
            } else {
                "RejectedNoChange"
            }
        }
        _ => "CommitUnknown",
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CaseResult {
    pub id: String,
    pub status: String,
    pub detail: String,
    pub observation: Value,
}
pub fn pending_cases() -> Vec<CaseResult> {
    CASES
        .iter()
        .map(|id| CaseResult {
            id: (*id).into(),
            status: "pending".into(),
            detail:
                "Requires exact disposable server build and applicable fault/permission fixture"
                    .into(),
            observation: Value::Null,
        })
        .collect()
}
pub fn validate_results(results: &[CaseResult], real_server: bool) -> Result<()> {
    let ids: BTreeSet<_> = results.iter().map(|r| r.id.as_str()).collect();
    if ids != CASES.iter().copied().collect()
        || results.len() != CASES.len()
        || results.iter().any(|r| {
            !["passed", "failed", "pending", "unsupported"].contains(&r.status.as_str())
                || r.detail.len() > 1024
        })
        || (!real_server && results.iter().any(|r| r.status == "passed"))
    {
        return Err(refuse(
            "incomplete, mislabeled or synthetic server certification",
        ));
    }
    Ok(())
}
