//! Compile frozen, lossless request bytes; send at most once through the closed administration
//! adapter. SDK 0.119.0 wire comparison lives in the isolated test crate, not a second serializer
//! on the post-journal dispatch path. This adapter never uses the serving client/cache.
use std::{collections::BTreeMap, future::Future, time::Duration};

use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use super::{
    apply_contract::*,
    journal::{
        Method, PreparedRequest, RecordedIntent, RestrictedJson, SecretGuard, TransportOutcome,
    },
    planner_types::{ActionKind, PlannedAction},
    raw_json::Json,
    types::EvidenceScope,
    Digest, Name,
};

pub const CHANGE_REASON: &str = "Required-fields approved plan";
pub(super) fn json<T: serde::Serialize>(value: &T) -> ApplyResult<Json> {
    checked(Json::parse(&checked(serde_json::to_vec(value))?))
}
fn object(fields: impl IntoIterator<Item = (String, Json)>) -> ApplyResult<Json> {
    checked(Json::members(&fields.into_iter().collect()))
}
pub(super) fn array(values: &[Json]) -> ApplyResult<Json> {
    checked(Json::parse(
        format!(
            "[{}]",
            values.iter().map(Json::raw).collect::<Vec<_>>().join(",")
        )
        .as_bytes(),
    ))
}
fn get(row: &Json, key: &str) -> ApplyResult<Json> {
    Ok(checked(row.get(key))?.clone())
}
fn description(row: &Json) -> ApplyResult<Json> {
    let value = get(row, "description")?;
    if checked(value.string())?.trim().is_empty() {
        return Err(fail(ApplyCode::InvalidInput));
    }
    Ok(value)
}

pub(super) fn compile(
    actions: &[PlannedAction],
    contract: &CertifiedMutationTarget,
    guard: &SecretGuard,
) -> ApplyResult<Vec<PreparedRequest>> {
    contract.validate()?;
    let mut requests = Vec::new();
    let mut contexts = Vec::new();
    let mut indices = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        let context = matches!(
            action.kind,
            ActionKind::Create | ActionKind::Replace | ActionKind::Delete
        );
        if !context && !contexts.is_empty() {
            return Err(fail(ApplyCode::InvalidInput));
        }
        let reason = json(&CHANGE_REASON)?;
        let (method, path, body) = match action.kind {
            ActionKind::ExtendDimension => {
                let row = action
                    .after
                    .as_ref()
                    .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
                let name = action
                    .server_id
                    .as_ref()
                    .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
                // Existing dimension names use this deliberately narrow path alphabet; no encoding ambiguity.
                if !name
                    .as_str()
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    return Err(fail(ApplyCode::InvalidInput));
                }
                (
                    Method::Patch,
                    format!("/dimension/{}", name.as_str()),
                    object([
                        ("schema".into(), get(row, "schema")?),
                        ("description".into(), description(row)?),
                        ("change_reason".into(), reason),
                    ])?,
                )
            }
            ActionKind::CreateDefault => {
                let row = action
                    .after
                    .as_ref()
                    .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
                let mut fields = BTreeMap::new();
                for key in ["key", "value", "schema", "description"] {
                    fields.insert(key.to_owned(), get(row, key)?);
                }
                description(row)?;
                checked(checked(row.get("schema"))?.object())?;
                fields.insert("change_reason".into(), reason);
                (Method::Post, "/default-config".into(), object(fields)?)
            }
            ActionKind::Create | ActionKind::Replace => {
                let row = action
                    .after
                    .as_ref()
                    .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
                let context = if action.kind == ActionKind::Create {
                    if action.server_id.is_some() || action.before.is_some() {
                        return Err(fail(ApplyCode::InvalidInput));
                    }
                    get(row, "value")?
                } else {
                    let id = action
                        .server_id
                        .as_ref()
                        .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
                    object([("id".into(), json(id)?)])?
                };
                let values = get(row, "override")?;
                if checked(values.object())?.is_empty() {
                    return Err(fail(ApplyCode::InvalidInput));
                }
                let payload = object([
                    ("context".into(), context),
                    ("override".into(), values),
                    ("description".into(), description(row)?),
                    ("change_reason".into(), reason),
                ])?;
                contexts.push(object([(
                    if action.kind == ActionKind::Create {
                        "PUT"
                    } else {
                        "REPLACE"
                    }
                    .into(),
                    payload,
                )])?);
                indices.push(index);
                continue;
            }
            ActionKind::Delete => {
                if action.after.is_some() || action.before.is_none() {
                    return Err(fail(ApplyCode::InvalidInput));
                }
                contexts.push(object([(
                    "DELETE".into(),
                    json(
                        action
                            .server_id
                            .as_ref()
                            .ok_or_else(|| fail(ApplyCode::InvalidInput))?,
                    )?,
                )])?);
                indices.push(index);
                continue;
            }
        };
        requests.push(request(method, path, body, vec![index], contract, guard)?);
    }
    if requests.len() > contract.max_prerequisites || contexts.len() > contract.max_context_actions
    {
        return Err(fail(ApplyCode::LimitExceeded));
    }
    if !contexts.is_empty() {
        requests.push(request(
            Method::Put,
            "/context/bulk-operations".into(),
            object([("operations".into(), array(&contexts)?)])?,
            indices,
            contract,
            guard,
        )?);
    }
    if requests.len() > super::journal::MAX_REQUESTS {
        return Err(fail(ApplyCode::LimitExceeded));
    }
    Ok(requests)
}
fn request(
    method: Method,
    path: String,
    body: Json,
    indices: Vec<usize>,
    contract: &CertifiedMutationTarget,
    guard: &SecretGuard,
) -> ApplyResult<PreparedRequest> {
    if body.raw().len() > contract.max_body {
        return Err(fail(ApplyCode::LimitExceeded));
    }
    checked(PreparedRequest::new(
        method,
        path,
        BTreeMap::from([("content-type".into(), "application/json".into())]),
        checked(RestrictedJson::parse(body.raw().as_bytes(), guard))?,
        indices,
        contract.id.clone(),
    ))
}

/// Constructed only by the engine after journal reservation and the fresh coordination check.
pub struct Dispatch<'a> {
    pub(super) intent: &'a RecordedIntent,
    pub(super) target: &'a CertifiedMutationTarget,
}
impl Dispatch<'_> {
    pub fn request(&self) -> &PreparedRequest {
        self.intent.request()
    }
}
pub struct WireReply {
    pub status: Option<u16>,
    pub body_hash: Option<Digest>,
    pub version: Option<Name>,
    pub request_id: Option<Name>,
    pub outcome: TransportOutcome,
    /// Malformed envelopes stop progression even when direct reads later prove a commit.
    pub valid_response: bool,
}
impl WireReply {
    pub(super) fn unknown(outcome: TransportOutcome) -> Self {
        Self {
            status: None,
            body_hash: None,
            version: None,
            request_id: None,
            outcome,
            valid_response: false,
        }
    }
}
pub trait MutationWriter: Sync {
    fn send(&self, request: Dispatch<'_>) -> impl Future<Output = WireReply> + Send;
}

pub struct AdministrationMutator {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    authorization: HeaderValue,
    contract: CertifiedMutationTarget,
}
impl AdministrationMutator {
    pub fn from_registered(id: &Name, _authorization: HeaderValue) -> ApplyResult<Self> {
        let _target = CertifiedMutationTarget::from_registry(id)?;
        let _client = Self::one_attempt_client()?;
        // No endpoint/identity transport registration is available. No ambient credential lookup.
        Err(fail(ApplyCode::UnsupportedServerContract))
    }
    fn request(&self, dispatch: &Dispatch<'_>) -> ApplyResult<reqwest::Request> {
        let request = dispatch.request();
        self.contract.validate()?;
        if self.contract.id != dispatch.target.id
            || request.wire_contract() != &self.contract.id
            || hash(&self.contract.registration)? != hash(&dispatch.target.registration)?
            || self.contract.scope != EvidenceScope::CertifiedServer
            || dispatch.target.scope != EvidenceScope::CertifiedServer
            || request.body().len() > self.contract.max_body
        {
            return Err(fail(ApplyCode::UnsupportedServerContract));
        }
        checked(request.validate())?;
        let allowed = match request.method() {
            Method::Post => request.path() == "/default-config",
            Method::Put => request.path() == "/context/bulk-operations",
            Method::Patch => request
                .path()
                .strip_prefix("/dimension/")
                .is_some_and(|name| {
                    !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                }),
            Method::Delete => false,
        };
        if !allowed
            || self.endpoint.scheme() != "https"
            || !self.endpoint.username().is_empty()
            || self.endpoint.password().is_some()
            || self.endpoint.query().is_some()
            || self.endpoint.fragment().is_some()
        {
            return Err(fail(ApplyCode::InvalidInput));
        }
        let mut url = self.endpoint.clone();
        let suffix = request
            .path()
            .strip_prefix('/')
            .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
        checked(url.path_segments_mut())?
            .pop_if_empty()
            .extend(suffix.split('/'));
        let method = match request.method() {
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Patch => reqwest::Method::PATCH,
            Method::Delete => return Err(fail(ApplyCode::InvalidInput)),
        };
        checked(
            self.client
                .request(method, url)
                .timeout(Duration::from_millis(self.contract.request_ms))
                .header(AUTHORIZATION, self.authorization.clone())
                .header(CONTENT_TYPE, "application/json")
                .header(
                    "x-org-id",
                    self.contract.registration.target.organisation.as_str(),
                )
                .header(
                    "x-workspace",
                    self.contract.registration.target.workspace.as_str(),
                )
                .body(request.body().to_vec())
                .build(),
        )
    }
    /// Shared by the eventual registered constructor; no SDK middleware reconstructs the body.
    pub(super) fn one_attempt_client() -> ApplyResult<reqwest::Client> {
        checked(
            reqwest::Client::builder()
                .https_only(true)
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .no_gzip()
                .no_brotli()
                .no_deflate()
                .no_zstd()
                .connect_timeout(Duration::from_secs(2))
                .build(),
        )
    }
}
impl MutationWriter for AdministrationMutator {
    async fn send(&self, dispatch: Dispatch<'_>) -> WireReply {
        let unknown = || WireReply::unknown(TransportOutcome::Unknown);
        let Ok(request) = self.request(&dispatch) else {
            return unknown();
        };
        let Ok(mut response) = self.client.execute(request).await else {
            return unknown();
        };
        let status = response.status().as_u16();
        let name = |key| {
            response
                .headers()
                .get(key)
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.to_owned().try_into().ok())
        };
        let version = name("x-config-version");
        let request_id = name("x-request-id");
        let json_content = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(';').next())
            == Some("application/json")
            && !response.headers().contains_key("content-encoding")
            && !response.headers().contains_key("content-range");
        if response.headers().len() > 128
            || response
                .headers()
                .iter()
                .map(|(k, v)| k.as_str().len() + v.len())
                .sum::<usize>()
                > 16384
        {
            return unknown();
        }
        let mut bytes = Vec::new();
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) if bytes.len().saturating_add(chunk.len()) <= 1024 * 1024 => {
                    bytes.extend_from_slice(&chunk)
                }
                Ok(None) => break,
                _ => return unknown(),
            }
        }
        let valid_response = json_content && version.is_some() && Json::parse(&bytes).is_ok();
        WireReply {
            status: Some(status),
            body_hash: super::journal::checksum(&bytes).ok(),
            version,
            request_id,
            outcome: TransportOutcome::Response,
            valid_response,
        }
    }
}
