use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use aws_smithy_runtime_api::client::{
    http::{HttpConnector, HttpConnectorFuture},
    orchestrator::{HttpRequest, HttpResponse},
    result::ConnectorError,
};
use aws_smithy_types::body::SdkBody;
use serde::Serialize;
use serde_json::Value;

use crate::contract::{Registration, MAX_BODY, MAX_REQUESTS};

#[derive(Clone, Debug, Serialize)]
pub struct Exchange {
    pub method: String,
    pub path: String,
    pub request: Value,
    pub status: Option<u16>,
    pub body: Option<Value>,
    pub version: Option<String>,
    pub last_modified: Option<String>,
}
pub type Trace = Arc<Mutex<Vec<Exchange>>>;
pub fn trace() -> Trace {
    Arc::new(Mutex::new(Vec::new()))
}
pub fn error(message: impl Into<String>) -> ConnectorError {
    ConnectorError::other(std::io::Error::other(message.into()).into(), None)
}
pub fn response(status: u16, bytes: Vec<u8>, version: Option<&str>) -> HttpResponse {
    let mut result = HttpResponse::new(
        status.try_into().expect("HTTP status"),
        SdkBody::from(bytes),
    );
    result
        .headers_mut()
        .insert("content-type", "application/json");
    if let Some(v) = version {
        result
            .headers_mut()
            .insert("x-config-version", v.to_owned());
    }
    result
}

/// Network transport exists only after local Docker ownership proof. It has no
/// proxy, redirects, credential discovery, ambient endpoint or HTTPS support.
#[derive(Clone, Debug)]
pub struct LocalConnector {
    registration: Registration,
    registration_path: PathBuf,
    guard_script: PathBuf,
    client: reqwest::Client,
    pub trace: Trace,
    dispatched: Arc<AtomicUsize>,
}
impl LocalConnector {
    pub fn new(
        registration: Registration,
        registration_path: PathBuf,
        guard_script: PathBuf,
    ) -> crate::Result<Self> {
        registration.validate()?;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| crate::refuse("local HTTP client construction failed"))?;
        Ok(Self {
            registration,
            registration_path,
            guard_script,
            client,
            trace: trace(),
            dispatched: Arc::new(AtomicUsize::new(0)),
        })
    }
}
impl HttpConnector for LocalConnector {
    fn call(&self, request: HttpRequest) -> HttpConnectorFuture {
        let this = self.clone();
        HttpConnectorFuture::new(async move {
            let r = &this.registration;
            let url = reqwest::Url::parse(request.uri()).map_err(|_| error("bad SDK URL"))?;
            let registered =
                reqwest::Url::parse(&r.endpoint).map_err(|_| error("bad registration"))?;
            if url.origin() != registered.origin()
                || !url.username().is_empty()
                || url.fragment().is_some()
                || request.headers().get("x-org-id") != Some(r.org.as_str())
            {
                return Err(error("request escaped registered origin/organisation"));
            }
            let bytes = request
                .body()
                .bytes()
                .ok_or_else(|| error("streaming request refused"))?;
            if bytes.len() > MAX_BODY {
                return Err(error("request body cap"));
            }
            let body: Value = if bytes.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(bytes).map_err(|_| error("non-JSON request refused"))?
            };
            let workspace_admin = url.path().starts_with("/workspaces");
            if workspace_admin {
                let expected = format!("/workspaces/{}", r.workspace);
                let create = request.method() == "POST"
                    && url.path() == "/workspaces"
                    && body["workspace_name"] == r.workspace;
                let read = request.method() == "GET" && url.path() == expected;
                if !create && !read {
                    return Err(error("workspace administration outside owned workspace"));
                }
            } else if request.headers().get("x-workspace") != Some(r.workspace.as_str()) {
                return Err(error("request escaped fixture workspace"));
            }
            // Re-inspect exact container/image/network identities before every
            // dispatch, including the first workspace creation. No trust flag.
            let registration_bytes = std::fs::read(&this.registration_path)
                .map_err(|_| error("registration missing"))?;
            if registration_bytes.len() > 16384
                || serde_json::from_slice::<Registration>(&registration_bytes)
                    .ok()
                    .as_ref()
                    != Some(r)
            {
                return Err(error("registration changed during run"));
            }
            let mut guard = tokio::process::Command::new("python3");
            guard
                .arg(&this.guard_script)
                .arg("verify")
                .arg("--registration")
                .arg(&this.registration_path)
                .kill_on_drop(true);
            let guard = tokio::time::timeout(Duration::from_secs(5), guard.output())
                .await
                .map_err(|_| error("fixture proof deadline"))?
                .map_err(|_| error("cannot verify fixture"))?;
            if !guard.status.success() {
                return Err(error("fixture ownership/image/network verification failed"));
            }
            if this.dispatched.fetch_add(1, Ordering::SeqCst) >= MAX_REQUESTS {
                return Err(error("request count cap"));
            }
            let mut call = this
                .client
                .request(
                    request.method().parse().map_err(|_| error("method"))?,
                    url.clone(),
                )
                .body(bytes.to_vec());
            for (key, value) in request.headers().iter() {
                call = call.header(key, value);
            }
            let index = {
                let mut trace = this.trace.lock().expect("trace lock");
                trace.push(Exchange {
                    method: request.method().into(),
                    path: url.path().into(),
                    request: body,
                    status: None,
                    body: None,
                    version: None,
                    last_modified: None,
                });
                trace.len() - 1
            };
            let mut reply = call
                .send()
                .await
                .map_err(|_| error("local dispatch failed; outcome unknown"))?;
            let status = reply.status().as_u16();
            let version = reply
                .headers()
                .get("x-config-version")
                .and_then(|h| h.to_str().ok())
                .map(str::to_owned);
            let headers = reply.headers().clone();
            if headers.len() > 128
                || headers
                    .iter()
                    .map(|(k, v)| k.as_str().len() + v.len())
                    .sum::<usize>()
                    > 16384
            {
                return Err(error("response header cap; outcome unknown"));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = reply
                .chunk()
                .await
                .map_err(|_| error("partial response; outcome unknown"))?
            {
                if bytes.len() + chunk.len() > MAX_BODY {
                    return Err(error("response byte cap; outcome unknown"));
                }
                bytes.extend_from_slice(&chunk);
            }
            {
                let mut trace = this.trace.lock().expect("trace lock");
                trace[index].status = Some(status);
                trace[index].body = serde_json::from_slice(&bytes).ok();
                trace[index].version = version.clone();
                trace[index].last_modified = headers
                    .get("last-modified")
                    .and_then(|h| h.to_str().ok())
                    .map(str::to_owned);
            }
            let mut result = response(status, bytes, version.as_deref());
            // The generated client needs response headers such as Last-Modified
            // as well as JSON. Never fabricate missing server metadata.
            for (key, value) in &headers {
                result
                    .headers_mut()
                    .try_insert(
                        key.as_str().to_owned(),
                        value
                            .to_str()
                            .map_err(|_| error("non-text response header"))?
                            .to_owned(),
                    )
                    .map_err(|_| error("invalid response header"))?;
            }
            Ok(result)
        })
    }
}
