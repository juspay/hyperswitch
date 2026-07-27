use actix_web::{HttpRequest, HttpResponse};
pub use api_models::superposition_proxy::{
    AuditLogResponse, ContextResponse, DefaultConfigResponse, DimensionResponse,
    PaginatedListResponse, ResolveConfigResponse,
};
use async_trait::async_trait;
use common_utils::events::ApiEventMetric;
use external_services::superposition::{
    context_put_from_request, create_context_output_to_struct, doc_map_to_json, document_to_value,
    list_audit_logs_to_response, list_contexts_to_response, list_default_configs_to_response,
    list_dimensions_to_response, map_sdk_error, parse_datetime, value_to_document, AuditAction,
    ContextFilterSortOn, ContextPutRequest, CreateContextInputBuilder, DateTime,
    DimensionMatchStrategy, GetDetailedResolvedConfigInputBuilder, ListAuditLogsInputBuilder,
    ListContextsInputBuilder, ListDefaultConfigsInputBuilder, ListDimensionsInputBuilder, SortBy,
    SuperpositionError,
};

use crate::{
    consts::user_role::{ROLE_ID_MERCHANT_ADMIN, ROLE_ID_PROFILE_ADMIN},
    core::errors::{self, RouterResponse},
    services::{authentication::UserFromToken, ApplicationResponse},
    SessionState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
enum ScopingDimension {
    OrganizationId,
    MerchantId,
    ProfileId,
    ProviderMerchantId,
    ProcessorMerchantId,
}

impl ScopingDimension {
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

fn validate_superposition_params(
    params: &[(String, String)],
    auth: &UserFromToken,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let unauthorized = || {
        error_stack::report!(errors::ApiErrorResponse::AccessForbidden {
            resource: "superposition".to_string(),
        })
    };
    for (key, value) in params {
        if let Some(dimension) = ScopingDimension::from_dimension_param(key) {
            if value != dimension.expected_value(auth) {
                return Err(unauthorized());
            }
        }
    }
    Ok(())
}

fn require_superposition_context(
    params: &[(String, String)],
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let has_scoping_dimension = params
        .iter()
        .any(|(k, _)| ScopingDimension::from_dimension_param(k).is_some());
    if !has_scoping_dimension {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::InvalidRequestData {
                message: "at least one dimension filter (organization_id, provider_merchant_id, processor_merchant_id, merchant_id, or profile_id) is required".to_string(),
            }
        ));
    }
    Ok(())
}

fn validate_superposition_context_body(
    context: &serde_json::Value,
    auth: &UserFromToken,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let Some(context_obj) = context.as_object() else {
        return Ok(());
    };
    let has_scoping_dim = context_obj
        .keys()
        .any(|k| ScopingDimension::from_context_key(k).is_some());
    if !has_scoping_dim {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::InvalidRequestData {
                message: "context must contain at least one of: organization_id, profile_id, provider_merchant_id, processor_merchant_id".to_string(),
            }
        ));
    }
    let is_merchant_admin_role = auth.role_id == ROLE_ID_MERCHANT_ADMIN;
    let is_profile_admin_role = auth.role_id == ROLE_ID_PROFILE_ADMIN;
    let has_merchant_level_dim = context_obj.contains_key("merchant_id")
        || context_obj.contains_key("profile_id")
        || context_obj.contains_key("processor_merchant_id")
        || context_obj.contains_key("provider_merchant_id");
    if is_merchant_admin_role
        && context_obj.contains_key("organization_id")
        && !has_merchant_level_dim
    {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::AccessForbidden {
                resource: "superposition".to_string(),
            }
        ));
    }
    // Profile admin: body must carry profile_id (no org-only/merchant-only contexts).
    if is_profile_admin_role && !context_obj.contains_key("profile_id") {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::AccessForbidden {
                resource: "superposition".to_string(),
            }
        ));
    }
    let params = context_obj
        .iter()
        .filter_map(|(k, v)| {
            v.as_str()
                .map(|s| (format!("dimension[{k}]"), s.to_owned()))
        })
        .collect::<Vec<_>>();
    validate_superposition_params(&params, auth)
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

/// Extract the `x-org-id` and `x-workspace` headers required by every proxy
/// endpoint, returning a `400` response if either is missing.
pub fn extract_proxy_headers(req: &HttpRequest) -> Result<(String, String), HttpResponse> {
    let org_id = req
        .headers()
        .get("x-org-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": { "message": "missing required header: x-org-id" }
            }))
        })?;

    let workspace_id = req
        .headers()
        .get("x-workspace")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": { "message": "missing required header: x-workspace" }
            }))
        })?;

    Ok((org_id, workspace_id))
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

/// Typed `ListDefaultConfigs` query params.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ListDefaultConfigsQuery {
    pub count: Option<i32>,
    pub page: Option<i32>,
    pub all: Option<bool>,
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
pub async fn handle_proxy_flow<R: SuperpositionProxyFlow>(
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
        require_superposition_context(&self.dimension_params)?;
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

        Ok(list_contexts_to_response(&output))
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
        Ok(ListDefaultConfigsInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .set_count(self.count)
            .set_page(self.page)
            .set_all(self.all)
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

        Ok(list_default_configs_to_response(&output))
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

        let output = input
            .send_with(state.superposition_service.superposition_sdk_client())
            .await
            .map_err(|sdk_error| {
                map_superposition_err(
                    error_stack::report!(map_sdk_error(sdk_error)),
                    "Failed to create context in Superposition",
                )
            })?;

        Ok(create_context_output_to_struct(&output))
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

        let output = self
            .into_input(org_id, workspace_id)?
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

        Ok(ResolveConfigResponse(resolved_entries))
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
                    "Failed to list audit logs from Superposition",
                )
            })?;

        Ok(list_audit_logs_to_response(&output))
    }
}
