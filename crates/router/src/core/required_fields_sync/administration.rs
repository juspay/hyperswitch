//! Dedicated lossless administration reads, separate from SuperpositionClient's serving cache.
//! Wire operations are pinned to SDK 0.119.0. The two POST routes are list operations with an
//! empty JSON body; there is no arbitrary method, filter, body, URL or mutation forwarding.

use std::time::Duration;

use reqwest::{
    header::{HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_ENCODING, CONTENT_RANGE, CONTENT_TYPE},
    Method,
};

use super::{
    inventory::{self, RawResponse, ReadRequest, ReadSource},
    raw_json::fingerprint,
    types::*,
    Name,
};

pub struct DirectAdminReader {
    client: reqwest::Client,
    target: CertifiedReadTarget,
    authorization: HeaderValue,
}

impl DirectAdminReader {
    /// Authorization is supplied explicitly; this never reads secrets, ambient Settings or
    /// credentials. No real target is currently registered/certified, so this refuses before I/O.
    pub fn from_registered(target_id: &Name, authorization: HeaderValue) -> InventoryResult<Self> {
        Self::new(
            CertifiedReadTarget::from_registry(target_id)?,
            authorization,
        )
    }

    fn new(target: CertifiedReadTarget, mut authorization: HeaderValue) -> InventoryResult<Self> {
        target.validate()?;
        if !authorization
            .to_str()
            .ok()
            .is_some_and(|value| value.starts_with("Bearer ") && value.len() > 7)
        {
            return Err(failure(InventoryErrorKind::InvalidTarget));
        }
        authorization.set_sensitive(true);
        if fingerprint("reader-authorization", authorization.as_bytes())?
            != target.authorization_fingerprint
        {
            return Err(failure(InventoryErrorKind::PermissionScopeUnproven));
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| failure(InventoryErrorKind::Transport))?;
        Ok(Self {
            client,
            target,
            authorization,
        })
    }

    pub async fn inventory(&self, limits: InventoryLimits) -> InventoryResult<CompleteInventory> {
        inventory::discover(self, &self.target, limits).await
    }

    fn request(
        &self,
        request: ReadRequest,
        timeout: Duration,
    ) -> InventoryResult<reqwest::Request> {
        let mut url = self.target.endpoint.clone();
        let (resource, page, count) = match request {
            ReadRequest::Page {
                resource,
                number,
                count,
            } if PAGED.contains(&resource)
                && number > 0
                && count > 0
                && number <= 128
                && count <= 1_000 =>
            {
                (resource, Some(number), Some(count))
            }
            ReadRequest::Page { .. } => return Err(failure(InventoryErrorKind::InvalidRequest)),
            ReadRequest::Workspace => (Resource::Workspace, None, None),
            ReadRequest::VersionHead => (Resource::VersionHead, Some(1), Some(1)),
        };
        let method = match resource {
            Resource::Experiments | Resource::ExperimentGroups => Method::POST,
            _ => Method::GET,
        };
        let suffix: &[&str] = match resource {
            Resource::Contexts => &["context"],
            Resource::Defaults => &["default-config"],
            Resource::Dimensions => &["dimension"],
            Resource::Experiments => &["experiments", "list"],
            Resource::ExperimentGroups => &["experiment-groups", "list"],
            Resource::VersionHead => &["config", "versions"],
            Resource::Workspace => &["workspaces", self.target.identity.workspace.as_str()],
        };
        url.path_segments_mut()
            .map_err(|_| failure(InventoryErrorKind::InvalidTarget))?
            .pop_if_empty()
            .extend(suffix);
        if let (Some(page), Some(count)) = (page, count) {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("count", &count.to_string())
                .append_pair("page", &page.to_string());
            if resource != Resource::VersionHead {
                query.append_pair("all", "false");
            }
        }
        let mut request = self
            .client
            .request(method.clone(), url)
            .timeout(timeout)
            .header(ACCEPT, "application/json")
            .header(AUTHORIZATION, self.authorization.clone())
            .header("x-org-id", self.target.identity.organisation.as_str());
        if resource != Resource::Workspace {
            request = request.header("x-workspace", self.target.identity.workspace.as_str());
        }
        if method == Method::POST {
            request = request.header(CONTENT_TYPE, "application/json").body("{}");
        }
        request
            .build()
            .map_err(|_| failure(InventoryErrorKind::InvalidRequest))
    }
}

fn transport(error: reqwest::Error) -> error_stack::Report<InventoryError> {
    // reqwest errors can carry URLs/headers. Keep only a static classification in reports.
    failure(if error.is_timeout() {
        InventoryErrorKind::Deadline
    } else {
        InventoryErrorKind::Transport
    })
}

impl ReadSource for DirectAdminReader {
    async fn read(
        &self,
        request: ReadRequest,
        timeout: Duration,
        body_cap: usize,
    ) -> InventoryResult<RawResponse> {
        // A test certificate can exercise validation, never the HTTP dispatch path.
        if self.target.evidence.scope != EvidenceScope::CertifiedServer {
            return Err(failure(InventoryErrorKind::CertificationPending));
        }
        let request = self.request(request, timeout)?;
        let mut response = self.client.execute(request).await.map_err(transport)?;
        let header = |key| {
            response
                .headers()
                .get(key)
                .map(|value| {
                    value
                        .to_str()
                        .map(str::to_owned)
                        .map_err(|_| failure(InventoryErrorKind::MalformedResponse))
                })
                .transpose()
        };
        let status = response.status().as_u16();
        let content_type = header(CONTENT_TYPE)?;
        let content_encoding = header(CONTENT_ENCODING)?;
        let content_range = response.headers().contains_key(CONTENT_RANGE);
        let age = header(reqwest::header::AGE)?;
        let content_length = header(reqwest::header::CONTENT_LENGTH)?
            .map(|v| {
                v.parse::<u64>()
                    .map_err(|_| failure(InventoryErrorKind::MalformedResponse))
            })
            .transpose()?;
        if content_length
            .is_some_and(|value| usize::try_from(value).map_or(true, |value| value > body_cap))
        {
            return Err(failure(InventoryErrorKind::ByteLimit));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport)? {
            if body
                .len()
                .checked_add(chunk.len())
                .is_none_or(|len| len > body_cap)
            {
                return Err(failure(InventoryErrorKind::ByteLimit));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(RawResponse {
            status,
            content_type,
            content_length,
            content_encoding,
            content_range,
            age,
            body,
        })
    }
}

#[cfg(test)]
#[path = "administration_tests.rs"]
pub(super) mod tests;
