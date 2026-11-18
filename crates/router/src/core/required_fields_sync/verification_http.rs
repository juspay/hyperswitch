//! Bounded raw reads for V1 exports and the two existing authenticated SDK routes.
//! Locked SDK 0.119.0 uses POST /config with an empty object for unfiltered export reads.
//! Direct-pod identity is a platform capability; a response header cannot manufacture it.
use std::time::Duration;

use reqwest::{
    header::{HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_ENCODING, CONTENT_RANGE, CONTENT_TYPE},
    Method,
};

use super::{
    apply_contract as ac,
    verification::{bytes_hash, ProbeKey, SdkRoute},
    Digest, Name,
};

pub enum VerificationReadOperation {
    LatestConfig,
    VersionedConfig(Name),
    PaymentSdk,
    ProfileSdk(Name),
}
/// Issued by a target registration only after the endpoint/pod tunnel and auth scope are proved.
/// No URL, credential, permission or pod-UID discovery is inferred from a CLI/environment input.
pub struct RegisteredRead {
    endpoint: reqwest::Url,
    operation: VerificationReadOperation,
    credential: Digest,
    organisation: Option<Name>,
    workspace: Option<Name>,
    pod: Option<ProbeKey>,
}
impl RegisteredRead {
    pub fn from_registry(_: &Name) -> ac::ApplyResult<Self> {
        Err(ac::fail(ac::ApplyCode::VerificationFailed))
    }
}
pub struct VerificationHttp {
    client: reqwest::Client,
    target: RegisteredRead,
    credential: HeaderValue,
}
impl VerificationHttp {
    pub fn new(target: RegisteredRead, mut credential: HeaderValue) -> ac::ApplyResult<Self> {
        let local_fixture = cfg!(test)
            && target.endpoint.scheme() == "http"
            && target.endpoint.host_str() == Some("127.0.0.1");
        if (target.endpoint.scheme() != "https" && !local_fixture)
            || target.endpoint.host_str().is_none()
            || !target.endpoint.username().is_empty()
            || target.endpoint.password().is_some()
            || target.endpoint.query().is_some()
            || target.endpoint.fragment().is_some()
            || target.endpoint.path() != "/"
            || credential.is_empty()
            || credential.as_bytes().len() > 8192
            || bytes_hash(credential.as_bytes())? != target.credential
        {
            return Err(ac::fail(ac::ApplyCode::VerificationFailed));
        }
        match &target.operation {
            VerificationReadOperation::LatestConfig
            | VerificationReadOperation::VersionedConfig(_) => {
                if target.organisation.is_none()
                    || target.workspace.is_none()
                    || target.pod.is_some()
                    || !credential.to_str().is_ok_and(|s| s.starts_with("Bearer "))
                {
                    return Err(ac::fail(ac::ApplyCode::VerificationFailed));
                }
            }
            VerificationReadOperation::PaymentSdk | VerificationReadOperation::ProfileSdk(_) => {
                let expected = if matches!(target.operation, VerificationReadOperation::PaymentSdk)
                {
                    SdkRoute::Payment
                } else {
                    SdkRoute::Profile
                };
                if target.organisation.is_some()
                    || target.workspace.is_some()
                    || target.pod.as_ref().is_none_or(|p| p.route != expected)
                {
                    return Err(ac::fail(ac::ApplyCode::VerificationFailed));
                }
            }
        }
        credential.set_sensitive(true);
        let client = ac::checked(
            reqwest::Client::builder()
                .no_proxy()
                .https_only(!local_fixture)
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .no_gzip()
                .no_brotli()
                .no_deflate()
                .no_zstd()
                .connect_timeout(Duration::from_secs(5))
                .build(),
        )?;
        Ok(Self {
            client,
            target,
            credential,
        })
    }
    fn request(&self, timeout: Duration) -> ac::ApplyResult<reqwest::Request> {
        if timeout.is_zero() || timeout > Duration::from_secs(30) {
            return Err(ac::fail(ac::ApplyCode::VerificationFailed));
        }
        let mut url = self.target.endpoint.clone();
        let mut api_key = false;
        match &self.target.operation {
            VerificationReadOperation::LatestConfig => url.set_path("/config"),
            VerificationReadOperation::VersionedConfig(id) => {
                url.set_path("/config");
                url.query_pairs_mut().append_pair("version", id.as_str());
            }
            VerificationReadOperation::PaymentSdk => {
                url.set_path("/v1/sdk/configs/web/sdk_config.json")
            }
            VerificationReadOperation::ProfileSdk(profile) => {
                if !profile
                    .as_str()
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                {
                    return Err(ac::fail(ac::ApplyCode::VerificationFailed));
                }
                url.set_path(&format!(
                    "/v1/sdk/configs/web/{}/sdk_config.json",
                    profile.as_str()
                ));
                api_key = true;
            }
        }
        let method = if self.target.pod.is_some() {
            Method::GET
        } else {
            Method::POST
        };
        let mut request = self
            .client
            .request(method.clone(), url)
            .timeout(timeout)
            .header(ACCEPT, "application/json")
            .header(
                if api_key {
                    "api-key"
                } else {
                    AUTHORIZATION.as_str()
                },
                self.credential.clone(),
            );
        if let (Some(org), Some(workspace)) = (&self.target.organisation, &self.target.workspace) {
            request = request
                .header("x-org-id", org.as_str())
                .header("x-workspace", workspace.as_str());
        }
        if method == Method::POST {
            request = request.header(CONTENT_TYPE, "application/json").body("{}");
        }
        ac::checked(request.build())
    }
    pub async fn read(&self, timeout: Duration) -> ac::ApplyResult<Vec<u8>> {
        let request = self.request(timeout)?;
        // Request timeout includes streamed body consumption. No URL/header/response values escape errors.
        let mut response = self
            .client
            .execute(request)
            .await
            .map_err(|_| ac::fail(ac::ApplyCode::VerificationFailed))?;
        if response.status() != reqwest::StatusCode::OK
            || response.headers().contains_key(CONTENT_RANGE)
            || response
                .headers()
                .get(CONTENT_ENCODING)
                .is_some_and(|v| v != "identity")
            || !response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.split(';').next() == Some("application/json"))
            || response
                .content_length()
                .is_some_and(|n| n > 4 * 1024 * 1024)
        {
            return Err(ac::fail(ac::ApplyCode::VerificationFailed));
        }
        let expected = response.content_length();
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ac::fail(ac::ApplyCode::VerificationFailed))?
        {
            if bytes.len().saturating_add(chunk.len()) > 4 * 1024 * 1024 {
                return Err(ac::fail(ac::ApplyCode::VerificationFailed));
            }
            bytes.extend_from_slice(&chunk);
        }
        let length = ac::checked(u64::try_from(bytes.len()))?;
        if expected.is_some_and(|n| n != length) {
            return Err(ac::fail(ac::ApplyCode::VerificationFailed));
        }
        ac::checked(super::raw_json::Json::parse(&bytes))?;
        Ok(bytes)
    }
    pub async fn read_pod(
        &self,
        timeout: Duration,
    ) -> ac::ApplyResult<super::verification::PodRead> {
        let key = self
            .target
            .pod
            .as_ref()
            .ok_or_else(|| ac::fail(ac::ApplyCode::VerificationFailed))?;
        Ok(super::verification::PodRead {
            key: key.clone(),
            addressed_pod: key.pod_uid.clone(),
            status: 200,
            body: self.read(timeout).await?,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
    use super::*;
    fn name(s: &str) -> Name {
        s.to_owned().try_into().unwrap()
    }
    #[tokio::test]
    #[ignore = "requires explicit RF_VERIFICATION_FIXTURE_ROOT and loopback-only network"]
    async fn bounded_registered_reads_reject_partial_redirect_encoded_and_malformed_responses() {
        use std::io::{Read, Write};
        let root =
            std::fs::canonicalize(std::env::var("RF_VERIFICATION_FIXTURE_ROOT").unwrap()).unwrap();
        let owner: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("owner.json")).unwrap()).unwrap();
        assert_eq!(owner["purpose"], "Step 11 isolated verification checks");
        assert_eq!(owner["root"], root.to_str().unwrap());
        assert_eq!(owner["cleanup_scope"], owner["root"]);
        assert!(owner["hosted_target"].is_null());
        let dir = (0..32)
            .find_map(|i| {
                let d = root.join(format!("http-fixture-{i}"));
                match std::fs::create_dir(&d) {
                    Ok(()) => Some(d),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => None,
                    Err(e) => panic!("fixture creation: {e}"),
                }
            })
            .unwrap();
        std::fs::write(dir.join("owner.json"), serde_json::to_vec(&owner).unwrap()).unwrap();
        let responses=[
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}",
            "HTTP/1.1 302 Found\r\nLocation: https://forbidden.invalid/\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 206 Partial Content\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}",
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 2\r\n\r\n{}",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Encoding: gzip\r\nContent-Length: 2\r\n\r\n{}",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 9999999\r\n\r\n{}",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 4\r\n\r\n{}",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 13\r\n\r\n{\"a\":1,\"a\":2}",
            "HTTP/1.1 503 Unavailable\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}",
        ];
        for (i, response) in responses.into_iter().enumerate() {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let endpoint = format!("http://{}/", listener.local_addr().unwrap());
            // Explicit ownership/endpoint record is durable before the first read. No server-side write exists.
            std::fs::write(dir.join(format!("target-{i}.json")),serde_json::to_vec(&serde_json::json!({"owner":owner["owner"],"endpoint":endpoint,"methods":["POST /config (read only)"],"synthetic":true,"cleanup":"owned listener only"})).unwrap()).unwrap();
            let credential = HeaderValue::from_static("Bearer synthetic-read-only");
            let target = RegisteredRead {
                endpoint: endpoint.parse().unwrap(),
                operation: VerificationReadOperation::LatestConfig,
                credential: bytes_hash(credential.as_bytes()).unwrap(),
                organisation: Some(name("synthetic-org")),
                workspace: Some(name("synthetic-workspace")),
                pod: None,
            };
            let transport = VerificationHttp::new(target, credential).unwrap();
            let server = std::thread::spawn(move || {
                let mut accepted = None;
                for _ in 0..200 {
                    match listener.accept() {
                        Ok((s, _)) => {
                            accepted = Some(s);
                            break;
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(10))
                        }
                        Err(e) => panic!("fixture accept: {e}"),
                    }
                }
                let mut stream = accepted.expect("bounded fixture accepted read");
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut received = Vec::new();
                let mut buf = [0; 1024];
                while !received.ends_with(b"{}") {
                    let n = stream.read(&mut buf).unwrap();
                    assert!(n > 0 && received.len() + n <= 8192);
                    received.extend_from_slice(&buf[..n]);
                }
                assert!(received.starts_with(b"POST /config HTTP/1.1\r\n"));
                stream.write_all(response.as_bytes()).unwrap();
            });
            let result = transport.read(Duration::from_secs(1)).await;
            server.join().unwrap();
            assert_eq!(result.is_ok(), i == 0, "case {i}");
        }
        let marker: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("owner.json")).unwrap()).unwrap();
        assert_eq!(marker["owner"], owner["owner"]);
        assert!(dir.starts_with(root));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn exact_registered_reads_use_existing_routes_without_mutation_filters_or_secret_queries() {
        for (op, path) in [
            (VerificationReadOperation::LatestConfig, "/config"),
            (
                VerificationReadOperation::VersionedConfig(name("v-1")),
                "/config?version=v-1",
            ),
            (
                VerificationReadOperation::PaymentSdk,
                "/v1/sdk/configs/web/sdk_config.json",
            ),
            (
                VerificationReadOperation::ProfileSdk(name("synthetic-profile")),
                "/v1/sdk/configs/web/synthetic-profile/sdk_config.json",
            ),
        ] {
            let sdk = matches!(
                op,
                VerificationReadOperation::PaymentSdk | VerificationReadOperation::ProfileSdk(_)
            );
            let profile = matches!(op, VerificationReadOperation::ProfileSdk(_));
            let credential = HeaderValue::from_static("Bearer synthetic-read-only");
            let hash = bytes_hash(credential.as_bytes()).unwrap();
            let target = RegisteredRead {
                endpoint: "https://synthetic.invalid/".parse().unwrap(),
                operation: op,
                credential: hash.clone(),
                organisation: (!sdk).then(|| name("synthetic-org")),
                workspace: (!sdk).then(|| name("synthetic-workspace")),
                pod: sdk.then(|| ProbeKey {
                    pod_uid: name("synthetic-pod"),
                    reader: hash.clone(),
                    route: if profile {
                        SdkRoute::Profile
                    } else {
                        SdkRoute::Payment
                    },
                    input: hash,
                }),
            };
            let transport = VerificationHttp::new(target, credential).unwrap();
            let request = transport.request(Duration::from_secs(1)).unwrap();
            assert_eq!(
                request.method(),
                if sdk { Method::GET } else { Method::POST }
            );
            assert_eq!(
                request.body().and_then(|b| b.as_bytes()),
                if sdk { None } else { Some(b"{}".as_slice()) }
            );
            assert_eq!(
                request.url().as_str(),
                format!("https://synthetic.invalid{path}")
            );
            assert!(request
                .headers()
                .get(if profile { "api-key" } else { "authorization" })
                .unwrap()
                .is_sensitive());
            assert_eq!(request.headers().contains_key("x-workspace"), !sdk);
            assert!(transport.request(Duration::ZERO).is_err());
        }
        assert!(RegisteredRead::from_registry(&name("unregistered")).is_err());
    }
}
