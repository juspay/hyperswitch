use std::collections::{hash_map::Entry, HashMap};

use actix_web::{HttpRequest, HttpResponse};
pub use api_models::superposition_proxy::{
    AuditLogResponse, BulkOperationResponse, BulkOperationResult, ContextResponse,
    DefaultConfigResponse, DeleteContextResponse, DimensionResponse, PaginatedListResponse,
    ResolveConfigExplanationResponse, ResolveConfigResponse,
};
use async_trait::async_trait;
use common_utils::events::ApiEventMetric;
use external_services::superposition::{
    context_put_from_request, context_response_to_struct, create_context_output_to_struct,
    doc_map_to_json, document_to_value, get_default_config_output_to_struct,
    get_dimension_output_to_struct, list_audit_logs_to_response, list_contexts_to_response,
    list_default_configs_to_response, list_dimensions_to_response, map_sdk_error, parse_datetime,
    resolve_config_explanation_to_response, update_override_output_to_struct, value_to_document,
    AuditAction, BulkOperationInputBuilder, ContextAction, ContextActionOut, ContextFilterSortOn,
    ContextIdentifier, ContextPutRequest, CreateContextInputBuilder, DateTime,
    DeleteContextInputBuilder, DimensionMatchStrategy, GetContextInputBuilder,
    GetDefaultConfigInputBuilder, GetDetailedResolvedConfigInputBuilder, GetDimensionInputBuilder,
    GetResolvedConfigExplanationInputBuilder, ListAuditLogsInputBuilder, ListContextsInputBuilder,
    ListDefaultConfigsInputBuilder, ListDimensionsInputBuilder, SortBy, SuperpositionClient,
    SuperpositionError, UpdateContextOverrideRequest, UpdateOverrideInputBuilder,
};

use crate::{
    configs::settings::SuperpositionProxySettings,
    consts::superposition::MAX_BULK_OPERATIONS,
    core::errors::{self, RouterResponse},
    services::{authentication::UserFromToken, ApplicationResponse},
    SessionState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
enum ScopingDimension {
    OrganizationId,
    MerchantId,
    ProfileId,
    ProviderMerchantId,
    ProcessorMerchantId,
}

impl ScopingDimension {
    const REQUIRED: [Self; 4] = [
        Self::OrganizationId,
        Self::ProfileId,
        Self::ProcessorMerchantId,
        Self::ProviderMerchantId,
    ];

    fn from_context_key(key: &str) -> Option<Self> {
        match key {
            "organization_id" => Some(Self::OrganizationId),
            "merchant_id" => Some(Self::MerchantId),
            "profile_id" => Some(Self::ProfileId),
            "provider_merchant_id" => Some(Self::ProviderMerchantId),
            "processor_merchant_id" => Some(Self::ProcessorMerchantId),
            _ => None,
        }
    }

    fn from_dimension_param(key: &str) -> Option<Self> {
        key.strip_prefix("dimension[")
            .and_then(|s| s.strip_suffix(']'))
            .and_then(Self::from_context_key)
    }

    fn expected_value(self, auth: &UserFromToken) -> &str {
        match self {
            Self::OrganizationId => auth.org_id.get_string_repr(),
            Self::MerchantId | Self::ProviderMerchantId | Self::ProcessorMerchantId => {
                auth.merchant_id.get_string_repr()
            }
            Self::ProfileId => auth.profile_id.get_string_repr(),
        }
    }
}

fn validate_scope<'a>(
    dimensions: impl Iterator<Item = (ScopingDimension, Option<&'a str>)>,
    auth: &UserFromToken,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let dimensions = dimensions.collect::<Vec<_>>();

    let missing = ScopingDimension::REQUIRED
        .into_iter()
        .filter(|required| {
            !dimensions
                .iter()
                .any(|(dimension, _)| dimension == required)
        })
        .map(<&'static str>::from)
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::InvalidRequestData {
                message: format!("scope is missing: {}", missing.join(", ")),
            }
        ));
    }

    let is_out_of_scope = dimensions
        .iter()
        .any(|(dimension, value)| *value != Some(dimension.expected_value(auth)));
    if is_out_of_scope {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::AccessForbidden {
                resource: "superposition".to_string(),
            }
        ));
    }
    Ok(())
}

fn validate_superposition_params(
    params: &[(String, String)],
    auth: &UserFromToken,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let dimensions = params.iter().filter_map(|(key, value)| {
        ScopingDimension::from_dimension_param(key)
            .map(|dimension| (dimension, Some(value.as_str())))
    });
    validate_scope(dimensions, auth)
}

fn validate_superposition_context_body(
    context: &serde_json::Value,
    auth: &UserFromToken,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let dimensions = context
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| {
            ScopingDimension::from_context_key(key).map(|dimension| (dimension, value.as_str()))
        });
    validate_scope(dimensions, auth)
}

/// Request checks against the allowlist of config keys the proxy may read or write.
impl SuperpositionProxySettings {
    /// Reject a request that names a key off the allowlist, without revealing whether it exists.
    fn ensure_allowed(&self, key: &str) -> errors::RouterResult<()> {
        if self.is_allowed_key(key) {
            Ok(())
        } else {
            Err(error_stack::report!(
                errors::ApiErrorResponse::ConfigNotFound
            ))
        }
    }

    /// Reject a mutation whose override sets any key off the allowlist.
    fn ensure_all_allowed<'a>(
        &self,
        keys: impl IntoIterator<Item = &'a String>,
    ) -> errors::RouterResult<()> {
        let mut disallowed_keys = keys
            .into_iter()
            .filter(|key| !self.is_allowed_key(key))
            .map(String::as_str)
            .collect::<Vec<_>>();
        if disallowed_keys.is_empty() {
            return Ok(());
        }
        disallowed_keys.sort_unstable();
        Err(error_stack::report!(
            errors::ApiErrorResponse::InvalidRequestData {
                message: format!(
                    "only allowlisted config keys can be modified, found: {}",
                    disallowed_keys.join(", ")
                ),
            }
        ))
    }
}

/// A Superposition response that can carry config keys off the allowlist.
trait HideDisallowedKeys: Sized {
    /// Remove keys off the allowlist. Returns whether anything is left to show.
    fn hide_disallowed_keys(&mut self, allowlist: &SuperpositionProxySettings) -> bool;

    /// The response with keys off the allowlist removed.
    fn without_disallowed_keys(mut self, allowlist: &SuperpositionProxySettings) -> Self {
        self.hide_disallowed_keys(allowlist);
        self
    }
}

/// An override map.
impl HideDisallowedKeys for serde_json::Value {
    fn hide_disallowed_keys(&mut self, allowlist: &SuperpositionProxySettings) -> bool {
        self.as_object_mut().is_some_and(|map| {
            map.retain(|key, _| allowlist.is_allowed_key(key));
            !map.is_empty()
        })
    }
}

impl HideDisallowedKeys for ContextResponse {
    fn hide_disallowed_keys(&mut self, allowlist: &SuperpositionProxySettings) -> bool {
        self.r#override.hide_disallowed_keys(allowlist)
    }
}

impl HideDisallowedKeys for DefaultConfigResponse {
    fn hide_disallowed_keys(&mut self, allowlist: &SuperpositionProxySettings) -> bool {
        allowlist.is_allowed_key(&self.key)
    }
}

impl HideDisallowedKeys for ResolveConfigResponse {
    fn hide_disallowed_keys(&mut self, allowlist: &SuperpositionProxySettings) -> bool {
        self.0.retain(|key, _| allowlist.is_allowed_key(key));
        !self.0.is_empty()
    }
}

/// Audit rows are raw table snapshots: context overrides are stripped to allowed keys, default
/// configs must have an allowed key, and dimension rows hold no config keys. Rows from every
/// other table (experiments, secrets, variables, webhooks, functions, ...) are hidden, and the
/// raw SQL is always cleared.
impl HideDisallowedKeys for AuditLogResponse {
    fn hide_disallowed_keys(&mut self, allowlist: &SuperpositionProxySettings) -> bool {
        self.query.clear();
        let snapshots = [self.original_data.as_mut(), self.new_data.as_mut()];
        match self.table_name.as_str() {
            "contexts" => snapshots
                .into_iter()
                .flatten()
                .filter_map(|row| row.get_mut("override"))
                .fold(false, |visible, r#override| {
                    r#override.hide_disallowed_keys(allowlist) || visible
                }),
            "default_configs" => snapshots
                .into_iter()
                .flatten()
                .filter_map(|row| row.get("key").and_then(serde_json::Value::as_str))
                .any(|key| allowlist.is_allowed_key(key)),
            "dimensions" => true,
            _ => false,
        }
    }
}

/// A page drops the items left with nothing to show. Totals still reflect the upstream page
/// counts, since Superposition cannot filter by the allowlist.
impl<T: HideDisallowedKeys> HideDisallowedKeys for PaginatedListResponse<T> {
    fn hide_disallowed_keys(&mut self, allowlist: &SuperpositionProxySettings) -> bool {
        self.data
            .retain_mut(|item| item.hide_disallowed_keys(allowlist));
        !self.data.is_empty()
    }
}

fn map_superposition_err(
    superposition_error: error_stack::Report<SuperpositionError>,
    context: &'static str,
) -> error_stack::Report<errors::ApiErrorResponse> {
    match superposition_error.current_context() {
        SuperpositionError::BadRequest(error_message) => {
            error_stack::report!(errors::ApiErrorResponse::InvalidRequestData {
                message: error_message.clone(),
            })
        }
        SuperpositionError::NotFound(_) => {
            error_stack::report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "The specified org_id or workspace_id was not found in Superposition"
                    .to_string(),
            })
        }
        _ => superposition_error
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(context),
    }
}

/// Read the `x-org-id` and `x-workspace` headers and check them against the
/// configured Superposition scope. These name a Superposition org/workspace, a
/// different namespace from the JWT's `organization_id`, so config is the only
/// trusted source to compare against.
pub fn extract_proxy_headers(
    req: &HttpRequest,
    superposition_client: &SuperpositionClient,
) -> Result<(String, String), HttpResponse> {
    let superposition_org_id = required_header(req, "x-org-id")?;
    let superposition_workspace_id = required_header(req, "x-workspace")?;

    let org_matches = superposition_org_id == superposition_client.configured_org_id();
    let workspace_matches =
        superposition_workspace_id == superposition_client.configured_workspace_id();

    match (org_matches, workspace_matches) {
        (true, true) => Ok((superposition_org_id, superposition_workspace_id)),
        _ => Err(actix_web::ResponseError::error_response(
            &errors::ApiErrorResponse::AccessForbidden {
                resource: "superposition org and workspace".to_string(),
            },
        )),
    }
}

fn required_header(req: &HttpRequest, name: &'static str) -> Result<String, HttpResponse> {
    req.headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(String::from)
        .ok_or_else(|| {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": { "message": format!("missing required header: {name}") }
            }))
        })
}

/// Typed `ListContexts` query params, parsed from the raw key/value pairs
#[derive(Debug, Default, Clone)]
pub struct ListContextsQuery {
    count: Option<i32>,
    page: Option<i32>,
    all: Option<bool>,
    prefix: Vec<String>,
    sort_on: Option<ContextFilterSortOn>,
    sort_by: Option<SortBy>,
    created_by: Vec<String>,
    last_modified_by: Vec<String>,
    plaintext: Option<String>,
    dimension_match_strategy: Option<DimensionMatchStrategy>,
    dimension_params: Vec<(String, String)>,
}

impl From<Vec<(String, String)>> for ListContextsQuery {
    fn from(params: Vec<(String, String)>) -> Self {
        let first_value = |name: &str| {
            params
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        };
        let all_values = |name: &str| {
            params
                .iter()
                .filter(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
                .collect::<Vec<_>>()
        };

        Self {
            count: first_value("count").and_then(|v| v.parse().ok()),
            page: first_value("page").and_then(|v| v.parse().ok()),
            all: first_value("all").and_then(|v| v.parse().ok()),
            prefix: all_values("prefix"),
            sort_on: first_value("sort_on").map(|v| ContextFilterSortOn::from(v.as_str())),
            sort_by: first_value("sort_by").map(|v| SortBy::from(v.as_str())),
            created_by: all_values("created_by"),
            last_modified_by: all_values("last_modified_by"),
            plaintext: first_value("plaintext"),
            dimension_match_strategy: first_value("dimension_match_strategy")
                .map(|v| DimensionMatchStrategy::from(v.as_str())),
            dimension_params: params
                .iter()
                .filter(|(k, _)| k.starts_with("dimension["))
                .cloned()
                .collect(),
        }
    }
}

/// Typed `ListDimensions` query params.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ListDimensionsQuery {
    pub count: Option<i32>,
    pub page: Option<i32>,
    pub all: Option<bool>,
}

/// Typed `ListDefaultConfigs` query params. Paging params are accepted but ignored: every
/// allowed config is returned in one page.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ListDefaultConfigsQuery {
    pub name: Option<String>,
}

/// Typed `ListAuditLogs` query params, parsed from the raw key/value pairs.
#[derive(Debug, Default, Clone)]
pub struct ListAuditLogsQuery {
    count: Option<i32>,
    page: Option<i32>,
    all: Option<bool>,
    from_date: Option<DateTime>,
    to_date: Option<DateTime>,
    table: Vec<String>,
    action: Vec<String>,
    username: Option<String>,
    sort_by: Option<SortBy>,
    dimension_params: Vec<(String, String)>,
}

impl From<Vec<(String, String)>> for ListAuditLogsQuery {
    fn from(params: Vec<(String, String)>) -> Self {
        let first_value = |name: &str| {
            params
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        };
        // `table`/`action` accept comma-separated values across one or more keys.
        let csv_values = |name: &str| {
            params
                .iter()
                .filter(|(k, _)| k == name)
                .flat_map(|(_, v)| v.split(',').map(|s| s.trim().to_owned()))
                .collect::<Vec<_>>()
        };
        // Dates are parsed leniently: malformed values are ignored rather than rejected.
        let parse_date = |name: &str| first_value(name).and_then(|v| parse_datetime(&v).ok());

        Self {
            count: first_value("count").and_then(|v| v.parse().ok()),
            page: first_value("page").and_then(|v| v.parse().ok()),
            all: first_value("all").and_then(|v| v.parse().ok()),
            from_date: parse_date("from_date"),
            to_date: parse_date("to_date"),
            table: csv_values("table"),
            action: csv_values("action"),
            username: first_value("username"),
            sort_by: first_value("sort_by").map(|v| SortBy::from(v.as_str())),
            dimension_params: params
                .iter()
                .filter(|(k, _)| k.starts_with("dimension["))
                .cloned()
                .collect(),
        }
    }
}

/// A Superposition proxy flow: a request type that knows how to validate the caller's
/// scope, build its SDK input, dispatch the upstream call, and convert the SDK output
/// into the client-facing response.
#[async_trait(?Send)]
pub trait SuperpositionProxyFlow: Sized {
    /// The Superposition SDK input builder this request maps to.
    type InputBuilder;
    /// The response payload returned to the client.
    type Response: serde::Serialize + std::fmt::Debug + ApiEventMetric;

    /// Build the Superposition SDK input builder from this request.
    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>>;

    /// Validate the request, send the built input upstream, and convert the response.
    async fn execute(
        self,
        state: &SessionState,
        auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>>;
}

/// Generic entry point shared by every Superposition proxy handler: runs the flow and
/// wraps the result in a JSON `ApplicationResponse`.
pub async fn handle_superposition_proxy_flow<R: SuperpositionProxyFlow>(
    state: SessionState,
    auth: UserFromToken,
    request: R,
    org_id: String,
    workspace_id: String,
) -> RouterResponse<R::Response> {
    let response = request.execute(&state, &auth, org_id, workspace_id).await?;
    Ok(ApplicationResponse::Json(response))
}

#[async_trait(?Send)]
impl SuperpositionProxyFlow for ListContextsQuery {
    type InputBuilder = ListContextsInputBuilder;
    type Response = PaginatedListResponse<ContextResponse>;

    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
        let mut builder = ListContextsInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .set_count(self.count)
            .set_page(self.page)
            .set_all(self.all)
            .set_sort_on(self.sort_on)
            .set_sort_by(self.sort_by)
            .set_plaintext(self.plaintext)
            .set_dimension_match_strategy(self.dimension_match_strategy)
            .set_prefix((!self.prefix.is_empty()).then_some(self.prefix))
            .set_created_by((!self.created_by.is_empty()).then_some(self.created_by))
            .set_last_modified_by(
                (!self.last_modified_by.is_empty()).then_some(self.last_modified_by),
            );

        for (key, value) in self.dimension_params {
            builder = builder.dimension_params(key, value);
        }

        Ok(builder)
    }

    async fn execute(
        self,
        state: &SessionState,
        auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>> {
        validate_superposition_params(&self.dimension_params, auth)?;

        let output = self
            .into_input(org_id, workspace_id)?
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to list contexts from Superposition",
                )
            })?;

        Ok(list_contexts_to_response(&output)
            .without_disallowed_keys(&state.conf.superposition_proxy))
    }
}

#[async_trait(?Send)]
impl SuperpositionProxyFlow for ListDefaultConfigsQuery {
    type InputBuilder = ListDefaultConfigsInputBuilder;
    type Response = PaginatedListResponse<DefaultConfigResponse>;

    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
        // Superposition cannot filter by an exact key list, so fetch every config (narrowed by
        // the caller's `name` substring, if any) and filter here.
        Ok(ListDefaultConfigsInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .all(true)
            .set_name(self.name))
    }

    async fn execute(
        self,
        state: &SessionState,
        _auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>> {
        let output = self
            .into_input(org_id, workspace_id)?
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to list default configs from Superposition",
                )
            })?;

        // Fetched with `all`, so this is a single page and the count is recomputed after filtering.
        let page = list_default_configs_to_response(&output)
            .without_disallowed_keys(&state.conf.superposition_proxy);
        Ok(PaginatedListResponse {
            total_items: i32::try_from(page.data.len()).unwrap_or(i32::MAX),
            ..page
        })
    }
}

#[async_trait(?Send)]
impl SuperpositionProxyFlow for ListDimensionsQuery {
    type InputBuilder = ListDimensionsInputBuilder;
    type Response = PaginatedListResponse<DimensionResponse>;

    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
        Ok(ListDimensionsInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .set_count(self.count)
            .set_page(self.page)
            .set_all(self.all))
    }

    async fn execute(
        self,
        state: &SessionState,
        _auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>> {
        let output = self
            .into_input(org_id, workspace_id)?
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to list dimensions from Superposition",
                )
            })?;

        Ok(list_dimensions_to_response(&output))
    }
}

/// `GetDimension` request: the name of the dimension to fetch.
#[derive(Debug, Clone)]
pub struct GetDimensionRequest(pub String);

#[async_trait(?Send)]
impl SuperpositionProxyFlow for GetDimensionRequest {
    type InputBuilder = GetDimensionInputBuilder;
    type Response = DimensionResponse;

    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
        Ok(GetDimensionInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .dimension(self.0))
    }

    async fn execute(
        self,
        state: &SessionState,
        _auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>> {
        let output = self
            .into_input(org_id, workspace_id)?
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to get dimension from Superposition",
                )
            })?;

        Ok(get_dimension_output_to_struct(&output))
    }
}

/// `GetDefaultConfig` request: the key of the default config to fetch.
#[derive(Debug, Clone)]
pub struct GetDefaultConfigRequest(pub String);

#[async_trait(?Send)]
impl SuperpositionProxyFlow for GetDefaultConfigRequest {
    type InputBuilder = GetDefaultConfigInputBuilder;
    type Response = DefaultConfigResponse;

    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
        Ok(GetDefaultConfigInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .key(self.0))
    }

    async fn execute(
        self,
        state: &SessionState,
        _auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>> {
        state.conf.superposition_proxy.ensure_allowed(&self.0)?;

        let output = self
            .into_input(org_id, workspace_id)?
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to get default config from Superposition",
                )
            })?;

        Ok(get_default_config_output_to_struct(&output))
    }
}

#[async_trait(?Send)]
impl SuperpositionProxyFlow for ContextPutRequest {
    type InputBuilder = CreateContextInputBuilder;
    type Response = ContextResponse;

    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
        let context_put = context_put_from_request(&self)
            .map_err(|error| map_superposition_err(error, "Invalid context request body"))?;

        Ok(CreateContextInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .request(context_put))
    }

    async fn execute(
        self,
        state: &SessionState,
        auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>> {
        let input = self.into_input(org_id, workspace_id)?;

        // Read context dims off the built request for auth-scoped validation.
        let context_json = input
            .get_request()
            .as_ref()
            .map(|context_put| doc_map_to_json(context_put.context()))
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
        validate_superposition_context_body(&context_json, auth)?;
        if let Some(context_put) = input.get_request() {
            state
                .conf
                .superposition_proxy
                .ensure_all_allowed(context_put.r#override().keys())?;
        }

        let output = input
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to create context in Superposition",
                )
            })?;

        // Superposition merges into an existing context's override, so the response can carry
        // hidden keys that were already set on it.
        Ok(create_context_output_to_struct(&output)
            .without_disallowed_keys(&state.conf.superposition_proxy))
    }
}

/// Fetch a context, check its dimensions are within the caller's scope, and return the
/// hidden keys of its override (those off the allowlist). Superposition's override update
/// replaces the whole override, so these must be carried into every update to stay untouched.
async fn get_scoped_context_hidden_overrides(
    state: &SessionState,
    auth: &UserFromToken,
    org_id: &str,
    workspace_id: &str,
    context_id: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, error_stack::Report<errors::ApiErrorResponse>>
{
    let context = GetContextInputBuilder::default()
        .org_id(org_id)
        .workspace_id(workspace_id)
        .id(context_id)
        .send_with(state.superposition_service.superposition_sdk_client())
        .await
        .map_err(|sdk_error| {
            map_superposition_err(
                error_stack::report!(map_sdk_error(sdk_error)),
                "Failed to get context from Superposition",
            )
        })?;
    validate_superposition_context_body(&doc_map_to_json(context.value()), auth)?;

    let mut hidden_overrides = match doc_map_to_json(context.r#override()) {
        serde_json::Value::Object(r#override) => r#override,
        _ => serde_json::Map::new(),
    };
    let allowlist = &state.conf.superposition_proxy;
    hidden_overrides.retain(|key, _| !allowlist.is_allowed_key(key));
    Ok(hidden_overrides)
}

/// Change reason recorded when a delete only removes a context's allowed keys and no
/// caller-supplied reason applies.
const REMOVED_ALLOWED_OVERRIDES_REASON: &str = "Removed allowlisted overrides";

/// Build an override update that replaces the override of the context with `context_id`.
fn update_override_input(
    org_id: String,
    workspace_id: String,
    context_id: String,
    r#override: serde_json::Map<String, serde_json::Value>,
    description: Option<String>,
    change_reason: String,
) -> Result<UpdateOverrideInputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
    let request = update_override_request(context_id, r#override, description, change_reason)?;

    Ok(UpdateOverrideInputBuilder::default()
        .org_id(org_id)
        .workspace_id(workspace_id)
        .request(request))
}

/// Build the request body that replaces the override of the context with `context_id`.
fn update_override_request(
    context_id: String,
    r#override: serde_json::Map<String, serde_json::Value>,
    description: Option<String>,
    change_reason: String,
) -> Result<UpdateContextOverrideRequest, error_stack::Report<errors::ApiErrorResponse>> {
    r#override
        .into_iter()
        .fold(
            UpdateContextOverrideRequest::builder()
                .context(ContextIdentifier::Id(context_id))
                .set_description(description)
                .change_reason(change_reason),
            |builder, (key, value)| builder.r#override(key, value_to_document(value)),
        )
        .build()
        .map_err(|error| {
            map_superposition_err(
                error_stack::report!(SuperpositionError::ClientError(format!(
                    "Failed to build override update: {error:?}"
                ))),
                "Invalid override update request",
            )
        })
}

/// Identifies the context an override update applies to.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ContextIdRef {
    pub id: String,
}

/// `UpdateOverride` request: replaces the allowed override keys of an existing context.
/// The context's hidden keys are preserved.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateContextOverrideBody {
    pub context: ContextIdRef,
    pub r#override: serde_json::Map<String, serde_json::Value>,
    pub description: Option<String>,
    pub change_reason: String,
}

#[async_trait(?Send)]
impl SuperpositionProxyFlow for UpdateContextOverrideBody {
    type InputBuilder = UpdateOverrideInputBuilder;
    type Response = ContextResponse;

    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
        update_override_input(
            org_id,
            workspace_id,
            self.context.id,
            self.r#override,
            self.description,
            self.change_reason,
        )
    }

    async fn execute(
        mut self,
        state: &SessionState,
        auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>> {
        state
            .conf
            .superposition_proxy
            .ensure_all_allowed(self.r#override.keys())?;
        let hidden_overrides = get_scoped_context_hidden_overrides(
            state,
            auth,
            &org_id,
            &workspace_id,
            &self.context.id,
        )
        .await?;
        self.r#override.extend(hidden_overrides);

        let output = self
            .into_input(org_id, workspace_id)?
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to update context override in Superposition",
                )
            })?;

        Ok(update_override_output_to_struct(&output)
            .without_disallowed_keys(&state.conf.superposition_proxy))
    }
}

/// `DeleteContext` request: the id of the context to delete.
#[derive(Debug, Clone)]
pub struct DeleteContextRequest(pub String);

#[async_trait(?Send)]
impl SuperpositionProxyFlow for DeleteContextRequest {
    type InputBuilder = DeleteContextInputBuilder;
    type Response = DeleteContextResponse;

    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
        Ok(DeleteContextInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .id(self.0))
    }

    /// A context that still carries hidden keys is not deleted: only its allowed keys are
    /// removed, which hides it from the proxy while leaving the hidden overrides in place.
    async fn execute(
        self,
        state: &SessionState,
        auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>> {
        let hidden_overrides =
            get_scoped_context_hidden_overrides(state, auth, &org_id, &workspace_id, &self.0)
                .await?;
        let response = DeleteContextResponse { id: self.0.clone() };

        if hidden_overrides.is_empty() {
            self.into_input(org_id, workspace_id)?
                .send_with(state.superposition_service.superposition_sdk_client())
                .await
                .map_err(|sdk_error| {
                    map_superposition_err(
                        error_stack::report!(map_sdk_error(sdk_error)),
                        "Failed to delete context in Superposition",
                    )
                })?;
        } else {
            update_override_input(
                org_id,
                workspace_id,
                self.0,
                hidden_overrides,
                None,
                REMOVED_ALLOWED_OVERRIDES_REASON.to_string(),
            )?
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to remove allowlisted overrides in Superposition",
                )
            })?;
        }

        Ok(response)
    }
}

/// One operation of a bulk-operations request, in Superposition's `ContextAction` JSON shape.
/// MOVE is not supported, so a request containing it fails to parse.
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BulkContextOperation {
    Put(ContextPutRequest),
    Replace(UpdateContextOverrideBody),
    Delete(String),
}

impl BulkContextOperation {
    /// The existing context this operation writes to, which needs a scope check.
    fn target_context_id(&self) -> Option<&str> {
        match self {
            Self::Replace(body) => Some(&body.context.id),
            Self::Delete(context_id) => Some(context_id),
            Self::Put(_) => None,
        }
    }

    /// Checks that need no upstream call, matching the single-context route for each action.
    fn validate(
        &self,
        auth: &UserFromToken,
        allowlist: &SuperpositionProxySettings,
    ) -> errors::RouterResult<()> {
        match self {
            Self::Put(request) => {
                let context_put = context_put_from_request(request).map_err(|error| {
                    map_superposition_err(error, "Invalid context request body")
                })?;
                validate_superposition_context_body(&doc_map_to_json(context_put.context()), auth)?;
                allowlist.ensure_all_allowed(context_put.r#override().keys())
            }
            Self::Replace(body) => allowlist.ensure_all_allowed(body.r#override.keys()),
            Self::Delete(_) => Ok(()),
        }
    }

    /// Keep the target context's hidden keys, as the single-context routes do: a REPLACE
    /// carries them along, and a DELETE of a context that has some becomes a REPLACE that
    /// removes only the allowed keys.
    fn preserve_hidden_keys(
        self,
        hidden_overrides: &HashMap<String, serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        let hidden = |context_id: &str| {
            hidden_overrides
                .get(context_id)
                .filter(|overrides| !overrides.is_empty())
                .cloned()
        };
        match self {
            Self::Replace(mut body) => {
                body.r#override
                    .extend(hidden(&body.context.id).unwrap_or_default());
                Self::Replace(body)
            }
            Self::Delete(context_id) => match hidden(&context_id) {
                Some(overrides) => Self::Replace(UpdateContextOverrideBody {
                    context: ContextIdRef { id: context_id },
                    r#override: overrides,
                    description: None,
                    change_reason: REMOVED_ALLOWED_OVERRIDES_REASON.to_string(),
                }),
                None => Self::Delete(context_id),
            },
            put @ Self::Put(_) => put,
        }
    }

    fn into_action(self) -> Result<ContextAction, error_stack::Report<errors::ApiErrorResponse>> {
        match self {
            Self::Put(request) => context_put_from_request(&request)
                .map(ContextAction::Put)
                .map_err(|error| map_superposition_err(error, "Invalid context request body")),
            Self::Replace(body) => update_override_request(
                body.context.id,
                body.r#override,
                body.description,
                body.change_reason,
            )
            .map(ContextAction::Replace),
            Self::Delete(context_id) => Ok(ContextAction::Delete(context_id)),
        }
    }
}

/// Names the failing operation in a batch error, keeping the original error kind.
trait AtOperation {
    fn at_operation(self, index: usize) -> Self;
}

impl<T> AtOperation for errors::RouterResult<T> {
    fn at_operation(self, index: usize) -> Self {
        self.map_err(|report| {
            let context = match report.current_context() {
                errors::ApiErrorResponse::InvalidRequestData { message } => {
                    errors::ApiErrorResponse::InvalidRequestData {
                        message: format!("operations[{index}]: {message}"),
                    }
                }
                errors::ApiErrorResponse::AccessForbidden { resource } => {
                    errors::ApiErrorResponse::AccessForbidden {
                        resource: format!("{resource} in operations[{index}]"),
                    }
                }
                _ => return report.attach_printable(format!("Failed at operations[{index}]")),
            };
            report.change_context(context)
        })
    }
}

/// `BulkOperation` request: context writes that Superposition applies in one transaction.
///
/// Every operation gets the checks of its single-context route before anything is sent, and a
/// DELETE of a context that holds hidden keys is sent as a REPLACE keeping only those keys.
#[derive(Clone, serde::Deserialize)]
pub struct BulkOperationsBody {
    pub operations: Vec<BulkContextOperation>,
}

#[async_trait(?Send)]
impl SuperpositionProxyFlow for BulkOperationsBody {
    type InputBuilder = BulkOperationInputBuilder;
    type Response = BulkOperationResponse;

    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
        let operations = self
            .operations
            .into_iter()
            .enumerate()
            .map(|(index, operation)| operation.into_action().at_operation(index))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(BulkOperationInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .set_operations(Some(operations)))
    }

    async fn execute(
        self,
        state: &SessionState,
        auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>> {
        let operation_count = self.operations.len();
        if !(1..=MAX_BULK_OPERATIONS).contains(&operation_count) {
            return Err(error_stack::report!(
                errors::ApiErrorResponse::InvalidRequestData {
                    message: format!(
                        "operations must hold between 1 and {MAX_BULK_OPERATIONS} entries, found {operation_count}"
                    ),
                }
            ));
        }

        // Reject on the cheap checks before making any upstream call.
        let allowlist = &state.conf.superposition_proxy;
        for (index, operation) in self.operations.iter().enumerate() {
            operation.validate(auth, allowlist).at_operation(index)?;
        }

        // One scope check per distinct context, shared by every operation on that context.
        let mut hidden_overrides = HashMap::new();
        for (index, operation) in self.operations.iter().enumerate() {
            if let Some(context_id) = operation.target_context_id() {
                if let Entry::Vacant(entry) = hidden_overrides.entry(context_id.to_owned()) {
                    entry.insert(
                        get_scoped_context_hidden_overrides(
                            state,
                            auth,
                            &org_id,
                            &workspace_id,
                            context_id,
                        )
                        .await
                        .at_operation(index)?,
                    );
                }
            }
        }

        // Responses report a DELETE as deleted even when it was sent as a REPLACE.
        let deleted_ids = self
            .operations
            .iter()
            .map(|operation| match operation {
                BulkContextOperation::Delete(context_id) => Some(context_id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();

        let operations = self
            .operations
            .into_iter()
            .map(|operation| operation.preserve_hidden_keys(&hidden_overrides))
            .collect();

        let output = Self { operations }
            .into_input(org_id, workspace_id)?
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to apply bulk context operations in Superposition",
                )
            })?;

        if output.output().len() != deleted_ids.len() {
            return Err(
                error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Superposition returned a different number of bulk results"),
            );
        }

        let output = output
            .output()
            .iter()
            .zip(deleted_ids)
            .map(|(result, deleted_id)| {
                let allowed = |context| {
                    context_response_to_struct(context).without_disallowed_keys(allowlist)
                };
                match (deleted_id, result) {
                    (Some(context_id), _) => Ok(BulkOperationResult::Delete(context_id)),
                    (None, ContextActionOut::Put(context)) => {
                        Ok(BulkOperationResult::Put(allowed(context)))
                    }
                    (None, ContextActionOut::Replace(context)) => {
                        Ok(BulkOperationResult::Replace(allowed(context)))
                    }
                    (None, result) => Err(error_stack::report!(
                        errors::ApiErrorResponse::InternalServerError
                    )
                    .attach_printable(format!(
                        "Unexpected bulk operation result from Superposition: {result:?}"
                    ))),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(BulkOperationResponse { output })
    }
}

/// `GetDetailedResolvedConfig` request: the context dimensions to resolve against.
#[derive(Debug, Clone)]
pub struct ResolveDetailedConfigRequest(pub serde_json::Map<String, serde_json::Value>);

#[async_trait(?Send)]
impl SuperpositionProxyFlow for ResolveDetailedConfigRequest {
    type InputBuilder = GetDetailedResolvedConfigInputBuilder;
    type Response = ResolveConfigResponse;

    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
        let mut builder = GetDetailedResolvedConfigInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id);

        for (dimension_key, dimension_value) in self.0 {
            builder = builder.context(dimension_key, value_to_document(dimension_value));
        }

        Ok(builder)
    }

    async fn execute(
        self,
        state: &SessionState,
        auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>> {
        let context_json = serde_json::Value::Object(self.0.clone());
        validate_superposition_context_body(&context_json, auth)?;

        let allowlist = &state.conf.superposition_proxy;
        // Superposition filters by key prefix; an allowed key as a prefix also matches longer
        // keys, so the result is narrowed to exact matches below.
        let output = self
            .into_input(org_id, workspace_id)?
            .set_prefix(Some(
                allowlist.allowed_config_keys.iter().cloned().collect(),
            ))
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to resolve config from Superposition",
                )
            })?;

        let resolved_entries = serde_json::from_value(document_to_value(output.config().clone()))
            .map_err(|err| {
            map_superposition_err(
                error_stack::report!(SuperpositionError::ClientError(err.to_string())),
                "Failed to parse resolved config from Superposition",
            )
        })?;
        Ok(ResolveConfigResponse(resolved_entries).without_disallowed_keys(allowlist))
    }
}

/// `GetResolvedConfigExplanation` request: config key plus context dimensions.
#[derive(Debug, Clone)]
pub struct ResolveConfigExplanationRequest {
    pub key: String,
    pub context: serde_json::Map<String, serde_json::Value>,
}

#[async_trait(?Send)]
impl SuperpositionProxyFlow for ResolveConfigExplanationRequest {
    type InputBuilder = GetResolvedConfigExplanationInputBuilder;
    type Response = ResolveConfigExplanationResponse;

    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
        let mut builder = GetResolvedConfigExplanationInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .key(self.key);

        for (dimension_key, dimension_value) in self.context {
            builder = builder.context(dimension_key, value_to_document(dimension_value));
        }

        Ok(builder)
    }

    async fn execute(
        self,
        state: &SessionState,
        auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>> {
        state.conf.superposition_proxy.ensure_allowed(&self.key)?;
        let context_json = serde_json::Value::Object(self.context.clone());
        validate_superposition_context_body(&context_json, auth)?;

        let output = self
            .into_input(org_id, workspace_id)?
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to resolve config explanation from Superposition",
                )
            })?;

        Ok(resolve_config_explanation_to_response(&output))
    }
}

#[async_trait(?Send)]
impl SuperpositionProxyFlow for ListAuditLogsQuery {
    type InputBuilder = ListAuditLogsInputBuilder;
    type Response = PaginatedListResponse<AuditLogResponse>;

    fn into_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::InputBuilder, error_stack::Report<errors::ApiErrorResponse>> {
        let action = (!self.action.is_empty()).then(|| {
            self.action
                .iter()
                .map(|a| AuditAction::from(a.as_str()))
                .collect()
        });

        let mut builder = ListAuditLogsInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .set_count(self.count)
            .set_page(self.page)
            .set_all(self.all)
            .set_from_date(self.from_date)
            .set_to_date(self.to_date)
            .set_username(self.username)
            .set_sort_by(self.sort_by)
            .set_tables((!self.table.is_empty()).then_some(self.table))
            .set_action(action);

        for (key, value) in self.dimension_params {
            builder = builder.dimension_params(key, value);
        }

        Ok(builder)
    }

    async fn execute(
        self,
        state: &SessionState,
        auth: &UserFromToken,
        org_id: String,
        workspace_id: String,
    ) -> Result<Self::Response, error_stack::Report<errors::ApiErrorResponse>> {
        validate_superposition_params(&self.dimension_params, auth)?;

        let output = self
            .into_input(org_id, workspace_id)?
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to list audit logs from Superposition",
                )
            })?;

        Ok(list_audit_logs_to_response(&output)
            .without_disallowed_keys(&state.conf.superposition_proxy))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Allowlist holding `allowed.a` and `allowed.b`.
    fn test_allowlist() -> SuperpositionProxySettings {
        SuperpositionProxySettings {
            allowed_config_keys: ["allowed.a", "allowed.b"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }

    fn audit_log(
        table_name: &str,
        original_data: Option<serde_json::Value>,
        new_data: Option<serde_json::Value>,
    ) -> AuditLogResponse {
        AuditLogResponse {
            id: "log_1".to_string(),
            table_name: table_name.to_string(),
            user_name: "user".to_string(),
            timestamp: "2026-10-21T00:00:00Z".to_string(),
            action: "UPDATE".to_string(),
            original_data,
            new_data,
            query: "UPDATE contexts SET override = $1".to_string(),
        }
    }

    #[test]
    fn only_exact_allowlisted_keys_are_allowed() {
        let allowlist = test_allowlist();
        assert!(allowlist.is_allowed_key("allowed.a"));
        assert!(!allowlist.is_allowed_key("allowed"));
        assert!(!allowlist.is_allowed_key("allowed.a.extra"));
        assert!(!allowlist.is_allowed_key("ALLOWED.A"));

        let empty = SuperpositionProxySettings {
            allowed_config_keys: std::collections::HashSet::new(),
        };
        assert!(!empty.is_allowed_key("allowed.a"));
    }

    #[test]
    fn override_with_hidden_key_is_rejected() {
        let allowlist = test_allowlist();
        let keys = ["allowed.a".to_string(), "payments.requires_cvv".to_string()];
        assert!(allowlist.ensure_all_allowed(&keys).is_err());

        let keys = ["allowed.a".to_string(), "allowed.b".to_string()];
        assert!(allowlist.ensure_all_allowed(&keys).is_ok());
    }

    #[test]
    fn override_is_stripped_to_allowed_keys() {
        let allowlist = test_allowlist();
        let mut r#override = json!({ "allowed.a": 1, "hidden.b": 2 });
        assert!(r#override.hide_disallowed_keys(&allowlist));
        assert_eq!(r#override, json!({ "allowed.a": 1 }));

        let mut r#override = json!({ "hidden.b": 2 });
        assert!(!r#override.hide_disallowed_keys(&allowlist));
    }

    #[test]
    fn audit_logs_are_filtered_by_table_and_key() {
        let allowlist = test_allowlist();
        let visible =
            |mut log: AuditLogResponse| log.hide_disallowed_keys(&allowlist).then_some(log);

        // Contexts: hidden keys are stripped, the SQL is cleared, and hidden-only rows dropped.
        let log = visible(audit_log(
            "contexts",
            Some(json!({ "override": { "hidden.b": 1 } })),
            Some(json!({ "override": { "allowed.a": 1, "hidden.b": 2 } })),
        ))
        .expect("context with an allowed key is visible");
        assert_eq!(log.original_data, Some(json!({ "override": {} })));
        assert_eq!(
            log.new_data,
            Some(json!({ "override": { "allowed.a": 1 } }))
        );
        assert!(log.query.is_empty());
        let hidden_context = json!({ "override": { "hidden.b": 2 } });
        assert!(visible(audit_log("contexts", None, Some(hidden_context))).is_none());

        // Default configs: shown only for allowed keys.
        let allowed_config = json!({ "key": "allowed.a" });
        assert!(visible(audit_log("default_configs", None, Some(allowed_config))).is_some());
        let hidden_config = json!({ "key": "hidden.b" });
        assert!(visible(audit_log("default_configs", None, Some(hidden_config))).is_none());

        // Dimensions are always shown; every other table is dropped.
        assert!(visible(audit_log("dimensions", None, None)).is_some());
        assert!(visible(audit_log("secrets", None, Some(json!({})))).is_none());
    }

    #[test]
    fn bulk_body_matches_superposition_shape() {
        let body: BulkOperationsBody = serde_json::from_value(json!({
            "operations": [
                {
                    "REPLACE": {
                        "context": { "id": "ctx_1" },
                        "override": { "allowed.a": "value" },
                        "change_reason": "no longer needed",
                    }
                },
                { "DELETE": "ctx_1" },
            ]
        }))
        .expect("body deserializes");

        let targets = body
            .operations
            .iter()
            .map(BulkContextOperation::target_context_id)
            .collect::<Vec<_>>();
        assert_eq!(targets, [Some("ctx_1"), Some("ctx_1")]);

        let actions = body
            .operations
            .into_iter()
            .map(BulkContextOperation::into_action)
            .collect::<Result<Vec<_>, _>>()
            .expect("actions build");
        let replace = actions[0].as_replace().expect("first is a replace");
        assert_eq!(replace.change_reason(), "no longer needed");
        assert_eq!(actions[1].as_delete().map(String::as_str), Ok("ctx_1"));
    }

    #[test]
    fn bulk_move_is_rejected() {
        let body = serde_json::from_value::<BulkOperationsBody>(json!({
            "operations": [{ "MOVE": { "id": "ctx_1", "request": {} } }]
        }));
        assert!(body.is_err());
    }

    #[test]
    fn bulk_operations_preserve_hidden_keys() {
        let hidden_overrides = HashMap::from([
            (
                "mixed".to_string(),
                json!({ "hidden.b": 2 })
                    .as_object()
                    .cloned()
                    .unwrap_or_default(),
            ),
            ("allowed_only".to_string(), serde_json::Map::new()),
        ]);
        let replace = |context_id: &str| {
            BulkContextOperation::Replace(UpdateContextOverrideBody {
                context: ContextIdRef {
                    id: context_id.to_string(),
                },
                r#override: json!({ "allowed.a": 1 })
                    .as_object()
                    .cloned()
                    .unwrap_or_default(),
                description: None,
                change_reason: "update".to_string(),
            })
        };
        let preserved =
            |operation: BulkContextOperation| operation.preserve_hidden_keys(&hidden_overrides);

        // A REPLACE carries the context's hidden keys along.
        let BulkContextOperation::Replace(body) = preserved(replace("mixed")) else {
            panic!("REPLACE stays a REPLACE");
        };
        assert_eq!(
            serde_json::Value::Object(body.r#override),
            json!({ "allowed.a": 1, "hidden.b": 2 })
        );
        assert_eq!(body.change_reason, "update");

        // A DELETE of a context with hidden keys removes only the allowed keys.
        let BulkContextOperation::Replace(body) =
            preserved(BulkContextOperation::Delete("mixed".to_string()))
        else {
            panic!("DELETE of a mixed context becomes a REPLACE");
        };
        assert_eq!(body.context.id, "mixed");
        assert_eq!(
            serde_json::Value::Object(body.r#override),
            json!({ "hidden.b": 2 })
        );
        assert_eq!(body.change_reason, REMOVED_ALLOWED_OVERRIDES_REASON);

        // A DELETE of a context with only allowed keys stays a DELETE.
        assert!(matches!(
            preserved(BulkContextOperation::Delete("allowed_only".to_string())),
            BulkContextOperation::Delete(context_id) if context_id == "allowed_only"
        ));
    }
}
