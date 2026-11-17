//! Bounded direct discovery. Completeness means certified scope plus stable observed full rows;
//! it is always provisional without a writer-exclusion window and grants no mutation authority.

use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use super::{
    raw_json::{fingerprint, Json},
    types::*,
    Name,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReadRequest {
    Page {
        resource: Resource,
        number: u32,
        count: u32,
    },
    Workspace,
    VersionHead,
}

#[derive(Clone)]
pub(super) struct RawResponse {
    pub status: u16,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub content_encoding: Option<String>,
    pub content_range: bool,
    pub age: Option<String>,
    pub body: Vec<u8>,
}

/// Closed internal interface: no filters, URL forwarding, cached serving reads or mutations.
pub(super) trait ReadSource {
    async fn read(
        &self,
        request: ReadRequest,
        timeout: Duration,
        body_cap: usize,
    ) -> InventoryResult<RawResponse>;
}

struct Budget<'a> {
    limits: &'a InventoryLimits,
    deadline: tokio::time::Instant,
    requests: u32,
    bytes: usize,
    items: u32,
}

impl<'a> Budget<'a> {
    fn new(limits: &'a InventoryLimits) -> Self {
        Self {
            limits,
            deadline: tokio::time::Instant::now() + limits.total_timeout,
            requests: 0,
            bytes: 0,
            items: 0,
        }
    }
    fn time(&self) -> InventoryResult<()> {
        if tokio::time::Instant::now() >= self.deadline {
            Err(failure(InventoryErrorKind::Deadline))
        } else {
            Ok(())
        }
    }
    fn items(&mut self, count: usize) -> InventoryResult<()> {
        self.items = self
            .items
            .checked_add(u32::try_from(count).map_err(|_| failure(InventoryErrorKind::ItemLimit))?)
            .ok_or_else(|| failure(InventoryErrorKind::ItemLimit))?;
        if self.items > self.limits.max_total_items {
            return Err(failure(InventoryErrorKind::ItemLimit));
        }
        Ok(())
    }
    async fn fetch(
        &mut self,
        source: &impl ReadSource,
        request: ReadRequest,
    ) -> InventoryResult<Json> {
        use InventoryErrorKind as K;
        self.time()?;
        if self.requests >= self.limits.max_requests {
            return Err(failure(K::RequestLimit));
        }
        let cap = self
            .limits
            .max_response_bytes
            .min(self.limits.max_total_bytes.saturating_sub(self.bytes));
        if cap == 0 {
            return Err(failure(K::ByteLimit));
        }
        self.requests += 1;
        let deadline = self
            .deadline
            .min(tokio::time::Instant::now() + self.limits.request_timeout);
        let response = tokio::time::timeout_at(
            deadline,
            source.read(
                request,
                deadline.saturating_duration_since(tokio::time::Instant::now()),
                cap,
            ),
        )
        .await
        .map_err(|_| failure(K::Deadline))??;
        self.time()?;
        match response.status {
            200 => (),
            401 | 403 => return Err(failure(K::PermissionDenied)),
            206 => return Err(failure(K::PartialResponse)),
            300..=399 => return Err(failure(K::Redirect)),
            _ => return Err(failure(K::HttpStatus)),
        }
        if response.body.len() > cap {
            return Err(failure(K::ByteLimit));
        }
        self.bytes += response.body.len();
        if response
            .content_length
            .is_some_and(|n| usize::try_from(n).ok() != Some(response.body.len()))
            || response.content_range
        {
            return Err(failure(K::PartialResponse));
        }
        if !response.content_type.as_deref().is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"))
        }) || response
            .content_encoding
            .as_deref()
            .is_some_and(|v| !v.eq_ignore_ascii_case("identity"))
        {
            return Err(failure(K::MalformedResponse));
        }
        if response
            .age
            .as_deref()
            .is_some_and(|v| v.parse::<u64>().ok() != Some(0))
        {
            return Err(failure(K::PartialResponse));
        }
        let body = Json::parse(&response.body)?;
        self.time()?;
        Ok(body)
    }
}

fn name(value: &Json) -> InventoryResult<Name> {
    value
        .string()?
        .try_into()
        .map_err(|_| failure(InventoryErrorKind::MalformedResponse))
}
fn optional_name(row: &Json, key: &str) -> InventoryResult<Option<Name>> {
    row.optional(key)?
        .filter(|v| !v.null())
        .map(name)
        .transpose()
}
fn integer(value: &Json) -> InventoryResult<u32> {
    let value = value.unsigned()?;
    if value > 2_147_483_647 {
        return Err(failure(InventoryErrorKind::MalformedResponse));
    }
    u32::try_from(value).map_err(|_| failure(InventoryErrorKind::MalformedResponse))
}
fn strings(row: &Json, keys: &[&str]) -> InventoryResult<()> {
    for key in keys {
        row.get(key)?.string()?;
    }
    Ok(())
}
fn optional_strings(row: &Json, keys: &[&str]) -> InventoryResult<()> {
    for key in keys {
        if let Some(value) = row.optional(key)?.filter(|v| !v.null()) {
            value.string()?;
        }
    }
    Ok(())
}
fn dates(row: &Json, keys: &[&str]) -> InventoryResult<()> {
    for key in keys {
        date(row.get(key)?)?;
    }
    Ok(())
}
fn date(value: &Json) -> InventoryResult<()> {
    superposition_sdk::primitives::DateTime::from_str(
        &value.string()?,
        superposition_sdk::primitives::DateTimeFormat::DateTime,
    )
    .map_err(|_| failure(InventoryErrorKind::MalformedResponse))?;
    Ok(())
}
fn audit(row: &Json, modified: &str) -> InventoryResult<()> {
    strings(
        row,
        &[
            "description",
            "change_reason",
            "created_by",
            "last_modified_by",
        ],
    )?;
    dates(row, &["created_at", modified])
}

fn condition(value: &Json) -> InventoryResult<ObservedCondition> {
    let mut equality = BTreeMap::new();
    for (key, value) in value.object()? {
        if Name::try_from(key.clone()).is_err() || !value.scalar() || value.null() {
            return Ok(ObservedCondition::QuarantinedUnsupportedPredicate);
        }
        let scalar = if value.raw().starts_with('"') {
            Scalar::String(value.string()?)
        } else if matches!(value.raw(), "true" | "false") {
            Scalar::Bool(value.boolean()?)
        } else {
            Scalar::Number(value.raw().to_owned())
        };
        equality.insert(key.clone(), scalar);
    }
    Ok(ObservedCondition::Equality(equality))
}

fn context(row: Json) -> InventoryResult<ObservedRow<ContextProjection>> {
    audit(&row, "last_modified_at")?;
    row.get("weight")?.string()?;
    if row.get("override")?.object()?.is_empty() {
        return Err(failure(InventoryErrorKind::MalformedResponse));
    }
    let projection = ContextProjection {
        id: name(row.get("id")?)?,
        override_id: name(row.get("override_id")?)?,
        condition: condition(row.get("value")?)?,
    };
    Ok(ObservedRow {
        raw: row,
        projection,
    })
}
fn default(row: Json) -> InventoryResult<ObservedRow<DefaultProjection>> {
    audit(&row, "last_modified_at")?;
    row.get("value")?;
    row.get("schema")?.object()?;
    optional_strings(
        &row,
        &[
            "value_validation_function_name",
            "value_compute_function_name",
        ],
    )?;
    let projection = DefaultProjection {
        key: name(row.get("key")?)?,
    };
    Ok(ObservedRow {
        raw: row,
        projection,
    })
}
fn dimension(row: Json) -> InventoryResult<ObservedRow<DimensionProjection>> {
    audit(&row, "last_modified_at")?;
    row.get("schema")?.object()?;
    // SDK 0.119 wire unions are objects. The serving conversion's "REGULAR" string is lossy.
    let tag = row.get("dimension_type")?.object()?;
    if tag.len() != 1 {
        return Err(failure(InventoryErrorKind::MalformedResponse));
    }
    let (key, value) = tag
        .first_key_value()
        .ok_or_else(|| failure(InventoryErrorKind::MalformedResponse))?;
    let kind = match key.as_str() {
        "REGULAR" => {
            value.object()?;
            DimensionKind::Regular
        }
        "LOCAL_COHORT" => DimensionKind::LocalCohort(value.string()?),
        "REMOTE_COHORT" => DimensionKind::RemoteCohort(value.string()?),
        _ => return Err(failure(InventoryErrorKind::MalformedResponse)),
    };
    for dependencies in row.get("dependency_graph")?.object()?.values() {
        for dependency in dependencies.array()? {
            dependency.string()?;
        }
    }
    optional_strings(
        &row,
        &[
            "value_validation_function_name",
            "value_compute_function_name",
        ],
    )?;
    let projection = DimensionProjection {
        name: name(row.get("dimension")?)?,
        position: row
            .get("position")?
            .raw()
            .parse()
            .map_err(|_| failure(InventoryErrorKind::MalformedResponse))?,
        mandatory: row.get("mandatory")?.boolean()?,
        kind,
    };
    Ok(ObservedRow {
        raw: row,
        projection,
    })
}
fn experiment(row: Json) -> InventoryResult<ObservedRow<ExperimentProjection>> {
    audit(&row, "last_modified")?;
    strings(&row, &["name", "experiment_type"])?;
    use superposition_sdk::types::ExperimentType;
    if !matches!(
        ExperimentType::from(row.get("experiment_type")?.string()?.as_str()),
        ExperimentType::Default | ExperimentType::DeleteOverrides
    ) {
        return Err(failure(InventoryErrorKind::MalformedResponse));
    }
    let status = row.get("status")?.string()?;
    use superposition_sdk::types::ExperimentStatusType as Status;
    if !matches!(
        Status::from(status.as_str()),
        Status::Created
            | Status::Inprogress
            | Status::Paused
            | Status::Concluded
            | Status::Discarded
    ) {
        return Err(failure(InventoryErrorKind::MalformedResponse));
    }
    if integer(row.get("traffic_percentage")?)? > 100 {
        return Err(failure(InventoryErrorKind::MalformedResponse));
    }
    for key in row.get("override_keys")?.array()? {
        name(key)?;
    }
    optional_strings(
        &row,
        &[
            "chosen_variant",
            "started_by",
            "metrics_url",
            "experiment_group_id",
        ],
    )?;
    if let Some(started_at) = row.optional("started_at")?.filter(|v| !v.null()) {
        date(started_at)?;
    }
    let mut variants = Vec::new();
    let mut ids = BTreeSet::new();
    for variant in row.get("variants")?.array()? {
        let id = name(variant.get("id")?)?;
        if !ids.insert(id.clone()) {
            return Err(failure(InventoryErrorKind::DuplicateIdentity));
        }
        if !matches!(
            variant.get("variant_type")?.string()?.as_str(),
            "CONTROL" | "EXPERIMENTAL"
        ) {
            return Err(failure(InventoryErrorKind::MalformedResponse));
        }
        variant.get("overrides")?.object()?;
        variants.push(VariantReference {
            id,
            context_id: optional_name(variant, "context_id")?,
            override_id: optional_name(variant, "override_id")?,
        });
    }
    if variants.is_empty() {
        return Err(failure(InventoryErrorKind::MalformedResponse));
    }
    let projection = ExperimentProjection {
        id: name(row.get("id")?)?,
        status,
        condition: condition(row.get("context")?)?,
        variants,
        group_id: optional_name(&row, "experiment_group_id")?,
    };
    Ok(ObservedRow {
        raw: row,
        projection,
    })
}
fn group(row: Json) -> InventoryResult<ObservedRow<ExperimentGroupProjection>> {
    audit(&row, "last_modified_at")?;
    strings(&row, &["name", "context_hash", "group_type"])?;
    if !matches!(
        row.get("group_type")?.string()?.as_str(),
        "USER_CREATED" | "SYSTEM_GENERATED"
    ) {
        return Err(failure(InventoryErrorKind::MalformedResponse));
    }
    let buckets = row
        .get("buckets")?
        .array()?
        .iter()
        .map(|value| {
            if value.null() {
                Ok(None)
            } else {
                Ok(Some(BucketReference {
                    experiment_id: name(value.get("experiment_id")?)?,
                    variant_id: name(value.get("variant_id")?)?,
                }))
            }
        })
        .collect::<InventoryResult<Vec<_>>>()?;
    if integer(row.get("traffic_percentage")?)? > 100 {
        return Err(failure(InventoryErrorKind::MalformedResponse));
    }
    let members = row
        .get("member_experiment_ids")?
        .array()?
        .iter()
        .map(name)
        .collect::<InventoryResult<Vec<_>>>()?;
    if members.iter().collect::<BTreeSet<_>>().len() != members.len() {
        return Err(failure(InventoryErrorKind::DuplicateIdentity));
    }
    let projection = ExperimentGroupProjection {
        id: name(row.get("id")?)?,
        condition: condition(row.get("context")?)?,
        member_experiment_ids: members,
        buckets,
    };
    Ok(ObservedRow {
        raw: row,
        projection,
    })
}
fn workspace(
    row: Json,
    target: &CertifiedReadTarget,
) -> InventoryResult<ObservedRow<WorkspaceProjection>> {
    strings(
        &row,
        &[
            "organisation_name",
            "workspace_schema_name",
            "workspace_status",
            "workspace_admin_email",
            "created_by",
            "last_modified_by",
        ],
    )?;
    dates(&row, &["created_at", "last_modified_at"])?;
    use superposition_sdk::types::WorkspaceStatus;
    if !matches!(
        WorkspaceStatus::from(row.get("workspace_status")?.string()?.as_str()),
        WorkspaceStatus::Enabled | WorkspaceStatus::Disabled
    ) {
        return Err(failure(InventoryErrorKind::MalformedResponse));
    }
    for flag in [
        "allow_experiment_self_approval",
        "auto_populate_control",
        "enable_context_validation",
        "enable_change_reason_validation",
    ] {
        row.get(flag)?.boolean()?;
    }
    row.get("metrics")?;
    if let Some(dimensions) = row.optional("mandatory_dimensions")?.filter(|v| !v.null()) {
        for dimension in dimensions.array()? {
            name(dimension)?;
        }
    }
    if let Some(lock) = row.optional("workspace_lock")?.filter(|v| !v.null()) {
        strings(lock, &["lock_id", "operation", "locked_by"])?;
        dates(lock, &["acquired_at", "expires_at"])?;
    }
    let projection = WorkspaceProjection {
        organisation: name(row.get("organisation_id")?)?,
        workspace: name(row.get("workspace_name")?)?,
        config_version: optional_name(&row, "config_version")?,
    };
    if projection.organisation != target.identity.organisation
        || projection.workspace != target.identity.workspace
    {
        return Err(failure(InventoryErrorKind::WorkspaceMismatch));
    }
    Ok(ObservedRow {
        raw: row,
        projection,
    })
}

/// Known partial/filter signals never become unknown metadata that the caller might overlook.
fn envelope(body: &Json, page: u32, count: u32) -> InventoryResult<(u32, u32, &[Json])> {
    for key in [
        "prefix",
        "exclude_prefix",
        "filter",
        "filters",
        "dimension_params",
        "scope",
        "status",
        "from_date",
        "to_date",
    ] {
        if body.optional(key)?.is_some() {
            return Err(failure(InventoryErrorKind::FilteredResponse));
        }
    }
    for key in ["partial", "filtered", "truncated"] {
        if body
            .optional(key)?
            .is_some_and(|v| v.boolean().ok() != Some(false))
        {
            return Err(failure(InventoryErrorKind::PartialResponse));
        }
    }
    for (key, expected) in [("page", page), ("count", count)] {
        if let Some(actual) = body.optional(key)? {
            if integer(actual)? != expected {
                return Err(failure(InventoryErrorKind::PaginationMismatch));
            }
        }
    }
    if body
        .optional("all")?
        .is_some_and(|v| v.boolean().ok() != Some(false))
    {
        return Err(failure(InventoryErrorKind::PaginationMismatch));
    }
    Ok((
        integer(body.get("total_items")?)?,
        integer(body.get("total_pages")?)?,
        body.get("data")?.array()?,
    ))
}

async fn pages(
    source: &impl ReadSource,
    budget: &mut Budget<'_>,
    target: &CertifiedReadTarget,
    resource: Resource,
) -> InventoryResult<(Vec<Json>, Vec<RawPage>)> {
    use InventoryErrorKind as K;
    let contract = target
        .pages
        .get(&resource)
        .ok_or_else(|| failure(K::InvalidContract))?;
    let count = budget.limits.page_size;
    let mut expected = None;
    let mut rows = Vec::new();
    let mut pages = Vec::new();
    let mut ids = BTreeSet::new();
    let id_field = match resource {
        Resource::Defaults => "key",
        Resource::Dimensions => "dimension",
        _ => "id",
    };
    for number in 1..=budget.limits.max_pages_per_resource {
        let body = budget
            .fetch(
                source,
                ReadRequest::Page {
                    resource,
                    number,
                    count,
                },
            )
            .await?;
        let (total, total_pages, data) = envelope(&body, number, count)?;
        if total > budget.limits.max_items_per_resource {
            return Err(failure(K::ItemLimit));
        }
        let required_pages = if total == 0 {
            contract.empty_total_pages
        } else {
            total.div_ceil(count)
        };
        if total_pages != required_pages
            || expected.is_some_and(|values| values != (total, total_pages))
        {
            return Err(failure(K::PaginationMismatch));
        }
        if total_pages.max(1) > budget.limits.max_pages_per_resource {
            return Err(failure(K::PageLimit));
        }
        expected = Some((total, total_pages));
        let offset = (number - 1)
            .checked_mul(count)
            .ok_or_else(|| failure(K::PageLimit))?;
        let required_items = count.min(total.saturating_sub(offset));
        if usize::try_from(required_items).ok() != Some(data.len()) {
            return Err(failure(K::PaginationMismatch));
        }
        budget.items(data.len())?;
        for row in data {
            if !ids.insert(name(row.get(id_field)?)?) {
                return Err(failure(K::DuplicateIdentity));
            }
            rows.push(row.clone());
        }
        pages.push(RawPage {
            resource,
            page: number,
            body,
        });
        if number == total_pages.max(1) {
            if usize::try_from(total).ok() != Some(rows.len()) {
                return Err(failure(K::PaginationMismatch));
            }
            return Ok((rows, pages));
        }
    }
    Err(failure(K::PageLimit))
}

async fn version(
    source: &impl ReadSource,
    budget: &mut Budget<'_>,
    target: &CertifiedReadTarget,
) -> InventoryResult<(Option<Name>, RawPage)> {
    let body = budget.fetch(source, ReadRequest::VersionHead).await?;
    let (total, total_pages, rows) = envelope(&body, 1, 1)?;
    let expected_pages = if total == 0 {
        target.empty_version_total_pages
    } else {
        total
    };
    if total_pages != expected_pages || rows.len() != usize::from(total > 0) {
        return Err(failure(InventoryErrorKind::PaginationMismatch));
    }
    budget.items(rows.len())?;
    let id = rows
        .first()
        .map(|row| {
            row.get("config")?.object()?;
            row.get("description")?.string()?;
            dates(row, &["created_at"])?;
            if let Some(tags) = row.optional("tags")?.filter(|v| !v.null()) {
                for tag in tags.array()? {
                    tag.string()?;
                }
            }
            name(row.get("id")?)
        })
        .transpose()?;
    Ok((
        id,
        RawPage {
            resource: Resource::VersionHead,
            page: 1,
            body,
        },
    ))
}

async fn pass(
    source: &impl ReadSource,
    budget: &mut Budget<'_>,
    target: &CertifiedReadTarget,
) -> InventoryResult<InventoryPass> {
    let mut collections = BTreeMap::new();
    let mut raw_pages = Vec::new();
    for resource in PAGED {
        let (rows, pages) = pages(source, budget, target, resource).await?;
        collections.insert(resource, rows);
        raw_pages.extend(pages);
    }
    let raw_workspace = budget.fetch(source, ReadRequest::Workspace).await?;
    budget.items(1)?;
    raw_pages.push(RawPage {
        resource: Resource::Workspace,
        page: 1,
        body: raw_workspace.clone(),
    });
    let mut take = |resource| {
        collections
            .remove(&resource)
            .ok_or_else(|| failure(InventoryErrorKind::InvalidContract))
    };
    let contexts = take(Resource::Contexts)?
        .into_iter()
        .map(|row| {
            let row = context(row)?;
            Ok((row.projection.id.clone(), row))
        })
        .collect::<InventoryResult<BTreeMap<_, _>>>()?;
    let defaults = take(Resource::Defaults)?
        .into_iter()
        .map(|row| {
            let row = default(row)?;
            Ok((row.projection.key.clone(), row))
        })
        .collect::<InventoryResult<_>>()?;
    let dimensions = take(Resource::Dimensions)?
        .into_iter()
        .map(|row| {
            let row = dimension(row)?;
            Ok((row.projection.name.clone(), row))
        })
        .collect::<InventoryResult<_>>()?;
    let experiments = take(Resource::Experiments)?
        .into_iter()
        .map(|row| {
            let row = experiment(row)?;
            Ok((row.projection.id.clone(), row))
        })
        .collect::<InventoryResult<_>>()?;
    let groups = take(Resource::ExperimentGroups)?
        .into_iter()
        .map(|row| {
            let row = group(row)?;
            Ok((row.projection.id.clone(), row))
        })
        .collect::<InventoryResult<_>>()?;
    let mut conditions = BTreeSet::new();
    let mut overrides = BTreeMap::new();
    for context in contexts.values() {
        if !conditions.insert(context.raw.get("value")?.canonical()?) {
            return Err(failure(InventoryErrorKind::AmbiguousCondition));
        }
        let value = context.raw.get("override")?.canonical()?;
        if overrides
            .get(&context.projection.override_id)
            .is_some_and(|before| before != &value)
        {
            return Err(failure(InventoryErrorKind::OverrideIdentityCollision));
        }
        overrides.insert(context.projection.override_id.clone(), value);
    }
    Ok(InventoryPass {
        contexts,
        defaults,
        dimensions,
        experiments,
        groups,
        workspace: workspace(raw_workspace, target)?,
        pages: raw_pages,
    })
}

fn canonical_pass(pass: &InventoryPass) -> InventoryResult<Vec<u8>> {
    // Preserve resource/page/row/array order. Only JSON object order and string escaping are
    // canonicalized. Numeric lexemes are kept; no guessed server ID is derived from this digest.
    let mut bytes = Vec::new();
    for page in &pass.pages {
        serde_json::to_writer(&mut bytes, &(page.resource, page.page))
            .map_err(|_| failure(InventoryErrorKind::MalformedJson))?;
        bytes.push(b'\n');
        bytes.extend(page.body.canonical()?);
        bytes.push(b'\n');
    }
    Ok(bytes)
}

pub(super) async fn discover(
    source: &impl ReadSource,
    target: &CertifiedReadTarget,
    limits: InventoryLimits,
) -> InventoryResult<CompleteInventory> {
    use InventoryErrorKind as K;
    target.validate()?;
    limits.validate()?;
    let mut budget = Budget::new(&limits);
    let (before_id, before) = version(source, &mut budget, target).await?;
    let first = pass(source, &mut budget, target).await?;
    let (middle_id, middle) = version(source, &mut budget, target).await?;
    if before_id != middle_id || before.body.canonical()? != middle.body.canonical()? {
        return Err(failure(K::VersionChanged));
    }
    let second = pass(source, &mut budget, target).await?;
    let (after_id, after) = version(source, &mut budget, target).await?;
    if before_id != after_id
        || before.body.canonical()? != after.body.canonical()?
        || first.workspace.projection.config_version != before_id
        || second.workspace.projection.config_version != before_id
    {
        return Err(failure(K::VersionChanged));
    }
    let canonical = canonical_pass(&first)?;
    if canonical != canonical_pass(&second)? {
        return Err(failure(K::InventoryChanged));
    }
    // Bind raw inventory to target, contract/permission evidence and full version observation.
    let mut bound = serde_json::to_vec(&(
        &target.identity,
        &target.evidence,
        &target.authorization_fingerprint,
    ))
    .map_err(|_| failure(K::MalformedJson))?;
    bound.extend(before.body.canonical()?);
    bound.push(b'\n');
    bound.extend(canonical);
    let proof = CompletenessProof {
        target: target.identity.clone(),
        evidence: target.evidence.clone(),
        inventory_hash: fingerprint("complete", &bound)?,
        version: before_id,
        requests: budget.requests,
        bytes: budget.bytes,
        observed_items: budget.items,
        identical_full_passes: 2,
        provisional_without_writer_exclusion: true,
    };
    budget.time()?;
    Ok(CompleteInventory {
        pass: first,
        proof,
        version_pages: vec![before, middle, after],
    })
}

#[cfg(test)]
#[path = "inventory_tests.rs"]
pub(super) mod tests;
