//! Narrow GET/conditional PUT adapter for ONE pre-provisioned Lease UID. No create/delete/list,
//! kubeconfig, ambient token lookup, kubectl, redirects, retry or rollout/barrier fabrication.
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use reqwest::{
    header::{HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    Method,
};
use serde::de::DeserializeOwned;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::{
    authority::*,
    coordination::*,
    journal::{RestrictedJson, SecretGuard},
    raw_json::Json,
    Name,
};

const STATE_KEY: &str = "required-fields.hyperswitch.io/state";
const MAX_RESPONSE: usize = 1024 * 1024;

pub struct KubernetesCoordinator {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    target: TargetRegistration,
    authorization: Option<HeaderValue>,
    fixture_owner: Option<Name>,
    fixture_verified: AtomicBool,
}
impl KubernetesCoordinator {
    pub fn from_registered(
        target_id: &Name,
        _authorization: HeaderValue,
    ) -> CoordinationResult<Self> {
        // Registration must bind API endpoint/CA/principal, cluster/server build, namespace,
        // immutable object UID, permissions and no-delete guarantees. Q3/Q4/Q6 remain open.
        let _target = CertifiedExecutionTarget::from_registry(target_id)?;
        Err(refuse(CoordinationCode::CertificationPending))
    }
    fn path(&self) -> String {
        format!(
            "/apis/coordination.k8s.io/v1/namespaces/{}/leases/{}",
            self.target.coordinator.namespace.as_str(),
            self.target.coordinator.object.as_str()
        )
    }
    async fn exchange(&self, method: Method, body: Option<Vec<u8>>) -> CoordinationResult<Vec<u8>> {
        self.target.validate()?;
        if method == Method::PUT
            && self.fixture_owner.is_some()
            && !self.fixture_verified.load(Ordering::SeqCst)
        {
            return Err(refuse(CoordinationCode::PermissionDenied));
        }
        let mut url = self.endpoint.clone();
        url.set_path(&self.path());
        let mut builder = self
            .client
            .request(method.clone(), url)
            .header(ACCEPT, "application/json")
            .timeout(Duration::from_secs(5));
        if let Some(authorization) = &self.authorization {
            builder = builder.header(AUTHORIZATION, authorization.clone());
        }
        if method == Method::PUT {
            if let Some(owner) = &self.fixture_owner {
                builder = builder.header("x-required-fields-fixture-write", owner.as_str());
            }
        }
        if let Some(bytes) = body {
            if bytes.len() > MAX_RESPONSE {
                return Err(refuse(CoordinationCode::LimitExceeded));
            }
            builder = builder.header(CONTENT_TYPE, "application/json").body(bytes);
        }
        let mut response = builder
            .send()
            .await
            .map_err(|_| refuse(CoordinationCode::UnknownOutcome))?;
        if let Some(owner) = &self.fixture_owner {
            if response
                .headers()
                .get("x-required-fields-fixture")
                .and_then(|v| v.to_str().ok())
                != Some(owner.as_str())
            {
                return Err(refuse(CoordinationCode::PermissionDenied));
            }
        }
        match response.status().as_u16() {
            200 => (),
            409 => return Err(refuse(CoordinationCode::Conflict)),
            401 | 403 => return Err(refuse(CoordinationCode::PermissionDenied)),
            404 => return Err(refuse(CoordinationCode::RecoveryRequired)),
            _ => return Err(refuse(CoordinationCode::UnknownOutcome)),
        }
        if response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(';').next())
            != Some("application/json")
            || response.headers().contains_key("content-encoding")
            || response.headers().contains_key("content-range")
            || response
                .content_length()
                .is_some_and(|n| n > u64::try_from(MAX_RESPONSE).unwrap_or(u64::MAX))
        {
            return Err(refuse(CoordinationCode::MalformedResponse));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| refuse(CoordinationCode::UnknownOutcome))?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE {
                return Err(refuse(CoordinationCode::LimitExceeded));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
    #[cfg(test)]
    pub(super) fn fixture(
        target: TargetRegistration,
        endpoint: reqwest::Url,
        owner: Name,
    ) -> CoordinationResult<Self> {
        target.validate()?;
        if endpoint.scheme() != "http"
            || endpoint.host_str() != Some("127.0.0.1")
            || endpoint.port().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || endpoint.path() != "/"
        {
            return Err(refuse(CoordinationCode::TargetMismatch));
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(2))
            .build()
            .map_err(|_| refuse(CoordinationCode::InvalidInput))?;
        Ok(Self {
            client,
            endpoint,
            target,
            authorization: None,
            fixture_owner: Some(owner),
            fixture_verified: AtomicBool::new(false),
        })
    }
}
impl CoordinatorStore for KubernetesCoordinator {
    async fn read(&self) -> CoordinationResult<ObservedState> {
        let result = decode(&self.exchange(Method::GET, None).await?, &self.target)?;
        self.fixture_verified.store(true, Ordering::SeqCst);
        Ok(result)
    }
    async fn compare_exchange(
        &self,
        observed: &ObservedState,
        next: &CoordinationState,
    ) -> CoordinationResult<ObservedState> {
        let body = replacement(observed, next, &self.target)?;
        decode(&self.exchange(Method::PUT, Some(body)).await?, &self.target)
    }
}
fn value<T: DeserializeOwned>(node: &Json) -> CoordinationResult<T> {
    serde_json::from_str(node.raw()).map_err(|_| refuse(CoordinationCode::MalformedResponse))
}
fn field<'a>(node: &'a Json, key: &str) -> CoordinationResult<&'a Json> {
    node.get(key)
        .map_err(|_| refuse(CoordinationCode::MalformedResponse))
}
fn json<T: serde::Serialize>(value: &T) -> CoordinationResult<Json> {
    Json::parse(&encode(value)?).map_err(|_| refuse(CoordinationCode::MalformedResponse))
}
fn put(node: Json, key: &str, replacement: Json) -> CoordinationResult<Json> {
    node.replacing(key, replacement)
        .map_err(|_| refuse(CoordinationCode::MalformedResponse))
}
fn timestamp(millis: u64) -> CoordinationResult<String> {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000)
        .map_err(|_| refuse(CoordinationCode::ClockInvalid))?
        .format(&Rfc3339)
        .map_err(|_| refuse(CoordinationCode::ClockInvalid))
}
fn holder(state: &CoordinationState) -> CoordinationResult<Option<String>> {
    state
        .lease
        .as_ref()
        .map(|lease| value::<String>(&json(&hash(&lease.owner)?)?))
        .transpose()
}
pub(super) fn decode(
    bytes: &[u8],
    target: &TargetRegistration,
) -> CoordinationResult<ObservedState> {
    if bytes.len() > MAX_RESPONSE {
        return Err(refuse(CoordinationCode::LimitExceeded));
    }
    let raw = Json::parse(bytes).map_err(|_| refuse(CoordinationCode::MalformedResponse))?;
    let metadata = field(&raw, "metadata")?;
    let spec = field(&raw, "spec")?;
    if value::<String>(field(&raw, "apiVersion")?)? != "coordination.k8s.io/v1"
        || value::<String>(field(&raw, "kind")?)? != "Lease"
        || value::<Name>(field(metadata, "name")?)? != target.coordinator.object
        || value::<Name>(field(metadata, "namespace")?)? != target.coordinator.namespace
        || value::<Name>(field(metadata, "uid")?)? != target.coordinator.uid
        || metadata
            .optional("deletionTimestamp")
            .map_err(|_| refuse(CoordinationCode::MalformedResponse))?
            .is_some_and(|v| !v.null())
        || metadata
            .optional("ownerReferences")
            .map_err(|_| refuse(CoordinationCode::MalformedResponse))?
            .is_some_and(|v| v.array().map_or(true, |a| !a.is_empty()))
        || ["strategy", "preferredHolder"].iter().any(|key| {
            spec.optional(key)
                .map_or(true, |v| v.is_some_and(|v| !v.null()))
        })
    {
        return Err(refuse(CoordinationCode::TargetMismatch));
    }
    let state_json: String = value(field(field(metadata, "annotations")?, STATE_KEY)?)?;
    let state: CoordinationState = parse_contract(state_json.as_bytes())?;
    state.validate(target)?;
    if value::<Option<String>>(field(spec, "holderIdentity")?)? != holder(&state)?
        || value::<u64>(field(spec, "leaseDurationSeconds")?)? != LEASE_MS / 1000
    {
        return Err(refuse(CoordinationCode::MalformedResponse));
    }
    let time: Option<String> = value(field(spec, "renewTime")?)?;
    match (&state.lease, time) {
        (Some(lease), Some(time)) => {
            let nanos = OffsetDateTime::parse(&time, &Rfc3339)
                .map_err(|_| refuse(CoordinationCode::MalformedResponse))?
                .unix_timestamp_nanos();
            if nanos != i128::from(lease.renewed_at_ms) * 1_000_000 {
                return Err(refuse(CoordinationCode::MalformedResponse));
            }
        }
        (None, None) => (),
        _ => return Err(refuse(CoordinationCode::MalformedResponse)),
    }
    Ok(ObservedState {
        version: value(field(metadata, "resourceVersion")?)?,
        state,
        wire: Some(
            RestrictedJson::parse(bytes, &SecretGuard::default())
                .map_err(|_| refuse(CoordinationCode::MalformedResponse))?,
        ),
    })
}
pub(super) fn replacement(
    before: &ObservedState,
    next: &CoordinationState,
    target: &TargetRegistration,
) -> CoordinationResult<Vec<u8>> {
    next.validate(target)?;
    let raw = before
        .wire
        .as_ref()
        .ok_or_else(|| refuse(CoordinationCode::MalformedResponse))?;
    let verified = decode(raw.bytes(), target)?;
    if verified.version != before.version || encode(&verified.state)? != encode(&before.state)? {
        return Err(refuse(CoordinationCode::Conflict));
    }
    let mut root =
        Json::parse(raw.bytes()).map_err(|_| refuse(CoordinationCode::MalformedResponse))?;
    let mut metadata = field(&root, "metadata")?.clone();
    let annotations = put(
        field(&metadata, "annotations")?.clone(),
        STATE_KEY,
        json(
            &String::from_utf8(encode(next)?)
                .map_err(|_| refuse(CoordinationCode::InvalidInput))?,
        )?,
    )?;
    metadata = put(metadata, "annotations", annotations)?;
    let mut spec = field(&root, "spec")?.clone();
    let previous_holder = holder(&before.state)?;
    let next_holder = holder(next)?;
    spec = put(spec, "holderIdentity", json(&next_holder)?)?;
    spec = put(spec, "leaseDurationSeconds", json(&(LEASE_MS / 1000))?)?;
    let renewed = next
        .lease
        .as_ref()
        .map(|l| timestamp(l.renewed_at_ms))
        .transpose()?;
    spec = put(spec, "renewTime", json(&renewed)?)?;
    if previous_holder != next_holder {
        let count: u32 = match spec
            .optional("leaseTransitions")
            .map_err(|_| refuse(CoordinationCode::MalformedResponse))?
        {
            Some(v) => value(v)?,
            None => 0,
        };
        spec = put(
            spec,
            "leaseTransitions",
            json(
                &count
                    .checked_add(1)
                    .ok_or_else(|| refuse(CoordinationCode::LimitExceeded))?,
            )?,
        )?;
        spec = put(spec, "acquireTime", json(&renewed)?)?;
    }
    root = put(root, "metadata", metadata)?;
    root = put(root, "spec", spec)?;
    let bytes = root.raw().as_bytes().to_vec();
    if bytes.len() > MAX_RESPONSE {
        return Err(refuse(CoordinationCode::LimitExceeded));
    }
    Ok(bytes)
}

#[cfg(test)]
#[path = "kubernetes_coordination_tests.rs"]
mod tests;
