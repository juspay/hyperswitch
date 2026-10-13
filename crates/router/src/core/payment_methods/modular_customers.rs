//! Customer creation and permanent card storage for modular batch migration.
//! The legacy pipeline is independent.
//! Customer lists go to the OLTP-enabled modular service (primary-backed reads), not OLAP.

use std::collections::HashMap;

use api_models::payment_methods::{MigrationStatus, PaymentMethodRecord};
use common_utils::{
    date_time, id_type,
    request::{Headers, Method, Request, RequestContent},
};
use error_stack::ResultExt;
use hyperswitch_interfaces::api_client::ApiClientWrapper;
use hyperswitch_masking::{Maskable, PeekInterface, Secret, StrongSecret};
use payment_methods::client::migration::{MigrationCardInput, MigrationCardInputError};
use serde::{Deserialize, Serialize};

mod cards;

use crate::{
    core::{
        api_keys,
        api_locking::{LockAction, LockingInput},
        errors::{self, RouterResult, StorageErrorExt},
    },
    routes::{lock_utils::ApiIdentifier, SessionState},
    types::{domain, transformers::ForeignFrom},
};

// Use the same credit/debit inference as ordinary payment flows.
fn supported_card_subtype(
    explicit: Option<common_enums::PaymentMethodType>,
    bin_card_type: Option<String>,
) -> bool {
    let details = bin_card_type.map(|card_type| {
        api_models::payments::AdditionalPaymentData::Card(Box::new(
            api_models::payments::AdditionalCardInfo {
                card_type: Some(card_type),
                ..Default::default()
            },
        ))
    });
    matches!(
        Option::<common_enums::PaymentMethodType>::foreign_from((
            explicit,
            details.as_ref(),
            Some(common_enums::PaymentMethod::Card),
        )),
        Some(common_enums::PaymentMethodType::Credit | common_enums::PaymentMethodType::Debit)
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CustomerError {
    InvalidInput,
    InvalidCredential,
    ExpiredCredential,
    InvalidProfile,
    AmbiguousReference,
    InvalidResponse,
    ServiceUnavailable,
    UncertainCreation,
    LockUnavailable,
    JournalUnavailable,
    PayloadConflict,
}

impl CustomerError {
    fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "migration_customer_input_invalid",
            Self::InvalidCredential => "migration_customer_key_rejected",
            Self::ExpiredCredential => "migration_customer_key_expired",
            Self::InvalidProfile => "migration_customer_profile_invalid",
            Self::AmbiguousReference => "migration_customer_reference_ambiguous",
            Self::InvalidResponse => "migration_customer_response_invalid",
            Self::ServiceUnavailable => "migration_customer_service_unavailable",
            Self::UncertainCreation => "migration_customer_creation_uncertain",
            Self::LockUnavailable => "migration_customer_lock_unavailable",
            Self::JournalUnavailable => "migration_customer_journal_unavailable",
            Self::PayloadConflict => "migration_customer_payload_conflict",
        }
    }

    fn api_error(self) -> errors::ApiErrorResponse {
        errors::ApiErrorResponse::InvalidRequestData {
            message: self.code().to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct CustomerRequest {
    merchant_reference_id: id_type::CustomerId,
    name: Secret<String>,
    email: common_utils::pii::Email,
}

#[derive(Clone, Debug, Deserialize)]
struct CustomerResponse {
    #[serde(deserialize_with = "deserialize_modular_customer_id")]
    id: id_type::GlobalCustomerId,
    merchant_reference_id: Option<id_type::CustomerId>,
    name: Option<Secret<String>>,
    email: Option<common_utils::pii::Email>,
}

fn deserialize_modular_customer_id<'de, D>(
    deserializer: D,
) -> Result<id_type::GlobalCustomerId, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // The shared type deliberately accepts legacy IDs during v1/v2 coexistence.
    // This boundary requires the generated modular ID, not the merchant reference.
    let value = String::deserialize(deserializer)?;
    if !is_modular_id(&value, "cus") {
        return Err(serde::de::Error::custom("Invalid modular customer ID"));
    }
    Ok(id_type::GlobalCustomerId::new_unchecked(value))
}

fn is_modular_id(value: &str, entity: &str) -> bool {
    let mut parts = value.split('_');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(cell), Some(prefix), Some(identifier), None) if prefix == entity => {
            id_type::CellId::from_string(cell).is_ok()
                && identifier.len() == 32
                && identifier.bytes().all(|byte| byte.is_ascii_hexdigit())
        }
        _ => false,
    }
}

#[derive(Clone, Debug)]
struct PreparedInput {
    line_number: Option<i64>,
    profile_id: id_type::ProfileId,
    customer: CustomerRequest,
    card: Result<MigrationCardInput, MigrationCardInputError>,
}

/// Modular responses are separate from the existing legacy response contract.
#[derive(Debug, Serialize)]
pub struct ModularMigrationResult {
    line_number: Option<i64>,
    customer_creation_flow: &'static str,
    merchant_reference_id: id_type::CustomerId,
    customer_id: Option<id_type::GlobalCustomerId>,
    profile_id: id_type::ProfileId,
    payment_method_id: Option<String>,
    migration_operation_id: Option<String>,
    customer_created: bool,
    customer_migration_status: MigrationStatus,
    migration_status: MigrationStatus,
    migration_error: Option<&'static str>,
    errors: Vec<MigrationError>,
    card_migrated: bool,
    payment_method_migrated: bool,
}

#[derive(Debug, Serialize)]
// Stable codes and retry hints let the tool report partial failures without sensitive values.
struct MigrationError {
    stage: &'static str,
    code: &'static str,
    retryable: bool,
    outcome_uncertain: bool,
    fields: Vec<&'static str>,
}

impl ModularMigrationResult {
    fn initial(input: &PreparedInput) -> Self {
        Self {
            line_number: input.line_number,
            customer_creation_flow: "modular",
            merchant_reference_id: input.customer.merchant_reference_id.clone(),
            customer_id: None,
            profile_id: input.profile_id.clone(),
            payment_method_id: None,
            migration_operation_id: None,
            customer_created: false,
            customer_migration_status: MigrationStatus::Failed,
            migration_status: MigrationStatus::Failed,
            migration_error: None,
            errors: Vec::new(),
            card_migrated: false,
            payment_method_migrated: false,
        }
    }
    fn fail(&mut self, error: MigrationError) {
        self.migration_error = Some(error.code);
        self.errors.push(error);
    }
}

#[derive(Debug, Serialize)]
pub struct MigrationCapabilities {
    contract_version: u8,
    legacy_migration: bool,
    modular_customer_creation: bool,
    modular_card_storage: bool,
}

impl common_utils::events::ApiEventMetric for MigrationCapabilities {}

// Advertise the compiled contract, not upstream service health or merchant readiness.
pub fn capabilities() -> MigrationCapabilities {
    MigrationCapabilities {
        contract_version: 1,
        legacy_migration: true,
        modular_customer_creation: true,
        modular_card_storage: true,
    }
}

fn validate_key(
    key_merchant: &id_type::MerchantId,
    expires_at: Option<time::PrimitiveDateTime>,
    merchant_id: &id_type::MerchantId,
    now: time::PrimitiveDateTime,
) -> Result<(), CustomerError> {
    // Customer creation must use a live, expiring key owned by this batch merchant.
    if key_merchant != merchant_id {
        return Err(CustomerError::InvalidCredential);
    }
    match expires_at {
        Some(expiry) if expiry > now => Ok(()),
        Some(_) => Err(CustomerError::ExpiredCredential),
        None => Err(CustomerError::InvalidCredential),
    }
}

fn customer_request(record: &PaymentMethodRecord) -> Result<CustomerRequest, CustomerError> {
    let name = record
        .name
        .as_ref()
        .map(|name| name.peek().trim())
        .filter(|name| !name.is_empty() && name.chars().count() <= 255)
        .ok_or(CustomerError::InvalidInput)?;
    let email = record.email.clone().ok_or(CustomerError::InvalidInput)?;
    if record.customer_id.get_string_repr().trim().is_empty() {
        return Err(CustomerError::InvalidInput);
    }
    // The CSV customer_id remains the merchant reference; the service generates the real ID.
    Ok(CustomerRequest {
        merchant_reference_id: record.customer_id.clone(),
        name: Secret::new(name.to_string()),
        email,
    })
}

fn validate_customer_details(
    records: &[PaymentMethodRecord],
    merchant_id: &id_type::MerchantId,
) -> Result<Vec<CustomerRequest>, CustomerError> {
    // Reject inconsistent name/email across cards for the same customer before any writes.
    let mut details: HashMap<id_type::CustomerId, CustomerRequest> = HashMap::new();
    let mut requests = Vec::with_capacity(records.len());
    for record in records {
        if record
            .merchant_id
            .as_ref()
            .is_some_and(|id| id != merchant_id)
        {
            return Err(CustomerError::InvalidInput);
        }
        let customer = customer_request(record)?;
        if let Some(previous) = details.get(&customer.merchant_reference_id) {
            if previous.name.peek() != customer.name.peek() || previous.email != customer.email {
                return Err(CustomerError::InvalidInput);
            }
        } else {
            details.insert(customer.merchant_reference_id.clone(), customer.clone());
        }
        requests.push(customer);
    }
    Ok(requests)
}

fn connector_ids(
    record: &PaymentMethodRecord,
    fallback: Option<&Vec<id_type::MerchantConnectorAccountId>>,
) -> Result<Vec<id_type::MerchantConnectorAccountId>, CustomerError> {
    match (
        &record.merchant_connector_id,
        &record.merchant_connector_ids,
    ) {
        (Some(_), Some(_)) => Err(CustomerError::InvalidProfile),
        (Some(id), None) => Ok(vec![id.clone()]),
        (None, Some(ids)) => {
            payment_methods::core::migration::MerchantConnectorValidator::parse_comma_separated_ids(
                ids,
            )
            .map_err(|_| CustomerError::InvalidProfile)
        }
        (None, None) => fallback.cloned().ok_or(CustomerError::InvalidProfile),
    }
    .and_then(|ids| {
        if ids.is_empty() {
            Err(CustomerError::InvalidProfile)
        } else {
            Ok(ids)
        }
    })
}

fn single_profile(
    bindings: &[(id_type::MerchantId, id_type::ProfileId)],
    merchant_id: &id_type::MerchantId,
) -> Result<id_type::ProfileId, CustomerError> {
    // Every mapped MCA on a row must belong to this merchant and resolve to one profile.
    let first = bindings.first().ok_or(CustomerError::InvalidProfile)?;
    if bindings
        .iter()
        .any(|(merchant, profile)| merchant != merchant_id || profile != &first.1)
    {
        return Err(CustomerError::InvalidProfile);
    }
    Ok(first.1.clone())
}

fn validate_customer(
    response: CustomerResponse,
    reference: &id_type::CustomerId,
) -> Result<CustomerResponse, CustomerError> {
    if response.merchant_reference_id.as_ref() != Some(reference)
        || response
            .name
            .as_ref()
            .is_some_and(|name| name.peek() == common_utils::pii::REDACTED)
    {
        return Err(CustomerError::InvalidResponse);
    }
    Ok(response)
}

fn unique_customer(
    mut customers: Vec<CustomerResponse>,
    reference: &id_type::CustomerId,
) -> Result<Option<CustomerResponse>, CustomerError> {
    // Duplicate references are ambiguous; never choose an arbitrary generated customer ID.
    match customers.len() {
        0 => Ok(None),
        1 => customers
            .pop()
            .map(|customer| validate_customer(customer, reference))
            .transpose(),
        _ => Err(CustomerError::AmbiguousReference),
    }
}

#[async_trait::async_trait]
trait CustomerBackend: Sync {
    async fn lock(&self, reference: &id_type::CustomerId) -> Result<(), CustomerError>;
    async fn unlock(&self, reference: &id_type::CustomerId) -> Result<(), CustomerError>;
    async fn list(&self, input: &PreparedInput) -> Result<Vec<CustomerResponse>, CustomerError>;
    async fn create(&self, input: &PreparedInput) -> Result<CustomerResponse, CustomerError>;
    async fn attempt(
        &self,
        input: &PreparedInput,
    ) -> Result<Option<CustomerAttempt>, CustomerError>;
    async fn remember(
        &self,
        input: &PreparedInput,
        attempt: CustomerAttempt,
    ) -> Result<(), CustomerError>;
    async fn forget_rejected(&self, input: &PreparedInput) -> Result<(), CustomerError>;
}

#[derive(Clone, Serialize, Deserialize)]
struct CustomerAttempt {
    customer_id: Option<id_type::GlobalCustomerId>,
}

fn validate_customer_payload(
    customer: &CustomerResponse,
    input: &PreparedInput,
) -> Result<(), CustomerError> {
    if customer.name.as_ref().map(|name| name.peek()) != Some(input.customer.name.peek())
        || customer.email.as_ref() != Some(&input.customer.email)
    {
        return Err(CustomerError::PayloadConflict);
    }
    Ok(())
}

async fn resolve_customer<B: CustomerBackend>(
    backend: &B,
    input: &PreparedInput,
) -> Result<(CustomerResponse, bool), CustomerError> {
    let reference = &input.customer.merchant_reference_id;
    backend.lock(reference).await?;
    let mut creation_attempted = false;
    let outcome = async {
        let attempt = backend.attempt(input).await?;
        // Reuse a confirmed customer before considering a new create, including resumed rows.
        if let Some(customer) = unique_customer(backend.list(input).await?, reference)? {
            if attempt
                .as_ref()
                .and_then(|attempt| attempt.customer_id.as_ref())
                .is_some_and(|id| id != &customer.id)
            {
                return Err(CustomerError::AmbiguousReference);
            }
            validate_customer_payload(&customer, input)?;
            backend
                .remember(
                    input,
                    CustomerAttempt {
                        customer_id: Some(customer.id.clone()),
                    },
                )
                .await?;
            return Ok((customer, false));
        }
        if attempt.is_some() {
            // A missing primary-backed read does not prove an older POST failed.
            return Err(CustomerError::UncertainCreation);
        }
        backend
            .remember(input, CustomerAttempt { customer_id: None })
            .await?;
        // Reconcile the create result with a primary-backed list, including lost responses.
        // The shared HTTP client retains its normal connection-close retry behavior.
        creation_attempted = true;
        let created = backend.create(input).await;
        if matches!(
            created,
            Err(CustomerError::InvalidCredential | CustomerError::ExpiredCredential)
        ) {
            // Customer route authentication rejects these before its create handler.
            backend.forget_rejected(input).await?;
            creation_attempted = false;
            return created.map(|customer| (customer, false));
        }
        if let Ok(created) = &created {
            // Preserve the generated ID even if the confirmation list is delayed (for example,
            // a KV write waiting for the drainer). A later lookup must resolve that same ID.
            validate_customer(created.clone(), reference)
                .map_err(|_| CustomerError::UncertainCreation)?;
            validate_customer_payload(created, input)
                .map_err(|_| CustomerError::UncertainCreation)?;
            backend
                .remember(
                    input,
                    CustomerAttempt {
                        customer_id: Some(created.id.clone()),
                    },
                )
                .await?;
        }
        let confirmed = unique_customer(
            backend
                .list(input)
                .await
                .map_err(|_| CustomerError::UncertainCreation)?,
            reference,
        )?;
        match (created, confirmed) {
            (Ok(created), Some(confirmed)) => {
                let created = validate_customer(created, reference)
                    .map_err(|_| CustomerError::UncertainCreation)?;
                if created.id != confirmed.id {
                    return Err(CustomerError::UncertainCreation);
                }
                validate_customer_payload(&confirmed, input)?;
                backend
                    .remember(
                        input,
                        CustomerAttempt {
                            customer_id: Some(confirmed.id.clone()),
                        },
                    )
                    .await?;
                Ok((confirmed, true))
            }
            (Err(_), Some(confirmed)) => {
                validate_customer_payload(&confirmed, input)?;
                backend
                    .remember(
                        input,
                        CustomerAttempt {
                            customer_id: Some(confirmed.id.clone()),
                        },
                    )
                    .await?;
                Ok((confirmed, false))
            }
            _ => Err(CustomerError::UncertainCreation),
        }
    }
    .await;
    let outcome = outcome.map_err(|error| {
        if creation_attempted && error == CustomerError::JournalUnavailable {
            CustomerError::UncertainCreation
        } else {
            error
        }
    });
    // Failed post-write confirmation keeps the lease: the upstream may still commit.
    // Cancellation likewise leaves the Redis lease in place, preventing immediate replay.
    if outcome.is_ok() || !creation_attempted {
        backend.unlock(reference).await?;
    }
    outcome
}

struct ServiceBackend<'a> {
    state: &'a SessionState,
    merchant_id: &'a id_type::MerchantId,
    api_key: &'a StrongSecret<String>,
    expires_at: Option<time::PrimitiveDateTime>,
    hash_key: &'a [u8; 32],
}

fn customer_journal_key(
    tenant: &str,
    merchant: &id_type::MerchantId,
    reference: &id_type::CustomerId,
    hash_key: &[u8; 32],
) -> String {
    // Persist only a keyed reference digest and generated ID, never customer details or keys.
    let identity = format!(
        "customer-migration-v1\0{tenant}\0{}\0{}",
        merchant.get_string_repr(),
        reference.get_string_repr()
    );
    format!(
        "MODULAR_CUSTOMER_MIGRATION_{}",
        blake3::keyed_hash(hash_key, identity.as_bytes()).to_hex()
    )
}

impl ServiceBackend<'_> {
    fn journal_key(&self, reference: &id_type::CustomerId) -> String {
        customer_journal_key(
            self.state.tenant.tenant_id.get_string_repr(),
            self.merchant_id,
            reference,
            self.hash_key,
        )
    }
    fn lock_action(&self, reference: &id_type::CustomerId) -> LockAction {
        let identity = format!(
            "{}\0{}\0{}",
            self.state.tenant.tenant_id.get_string_repr(),
            self.merchant_id.get_string_repr(),
            reference.get_string_repr()
        );
        LockAction::Hold {
            input: LockingInput {
                unique_locking_key: format!(
                    "modular_migration_{}",
                    blake3::hash(identity.as_bytes()).to_hex()
                ),
                api_identifier: ApiIdentifier::Customers,
                override_lock_retries: None,
            },
        }
    }

    async fn call<T: serde::de::DeserializeOwned>(
        &self,
        input: &PreparedInput,
        create: bool,
    ) -> Result<T, CustomerError> {
        let request = customer_http_request(
            self.state
                .conf
                .micro_services
                .payment_methods_base_url
                .as_ref(),
            &self.state.conf.micro_services.payment_methods_prefix.0,
            &input.profile_id,
            self.api_key,
            self.state.tenant.tenant_id.get_string_repr(),
            &input.customer,
            create,
        )?;
        let mut request = request;
        if let Some(request_id) = ApiClientWrapper::get_request_id(self.state) {
            request.headers.insert((
                self.state.conf.trace_header.header_name.clone(),
                Maskable::Normal(request_id.to_string()),
            ));
        }
        let response = self
            .state
            .get_api_client()
            .send_request(
                self.state,
                request,
                Some(common_utils::consts::REQUEST_TIME_OUT),
                false,
            )
            .await
            .map_err(|_| CustomerError::ServiceUnavailable)?;
        if matches!(response.status().as_u16(), 401 | 403) {
            return Err(CustomerError::InvalidCredential);
        }
        if !response.status().is_success() {
            return Err(CustomerError::ServiceUnavailable);
        }
        response
            .json()
            .await
            .map_err(|_| CustomerError::InvalidResponse)
    }
}

fn customer_http_request(
    base_url: &url::Url,
    prefix: &str,
    profile: &id_type::ProfileId,
    api_key: &StrongSecret<String>,
    tenant: &str,
    customer: &CustomerRequest,
    create: bool,
) -> Result<Request, CustomerError> {
    let path = if create {
        format!("/{}/customers", prefix.trim_matches('/'))
    } else {
        format!("/{}/customers/list", prefix.trim_matches('/'))
    };
    let mut url = base_url
        .join(&path)
        .map_err(|_| CustomerError::ServiceUnavailable)?;
    if !create {
        // Read at most two matching references so duplicate customers can be detected.
        url.query_pairs_mut()
            .append_pair(
                "customer_id",
                customer.merchant_reference_id.get_string_repr(),
            )
            .append_pair("limit", "2")
            .append_pair("offset", "0");
    }
    let mut request = Request::new(
        if create { Method::Post } else { Method::Get },
        url.as_str(),
    );
    // Keep customer API merchant-key authentication, with the row profile and current tenant.
    let mut headers = Headers::new();
    headers.insert((
        "Authorization".to_string(),
        Maskable::Masked(Secret::new(format!("api-key={}", api_key.peek()))),
    ));
    headers.insert((
        "x-profile-id".to_string(),
        Maskable::Normal(profile.get_string_repr().to_string()),
    ));
    headers.insert((
        "x-tenant-id".to_string(),
        Maskable::Normal(tenant.to_string()),
    ));
    headers.insert((
        "Content-Type".to_string(),
        Maskable::Normal("application/json".to_string()),
    ));
    request.headers = headers;
    if create {
        request.set_body(RequestContent::Json(Box::new(customer.clone())));
    }
    Ok(request)
}

#[async_trait::async_trait]
impl CustomerBackend for ServiceBackend<'_> {
    async fn attempt(
        &self,
        input: &PreparedInput,
    ) -> Result<Option<CustomerAttempt>, CustomerError> {
        let redis = self
            .state
            .store
            .get_redis_conn()
            .map_err(|_| CustomerError::JournalUnavailable)?;
        let value: Option<String> = redis
            .get_key(&redis_interface::RedisKey::from(
                self.journal_key(&input.customer.merchant_reference_id),
            ))
            .await
            .map_err(|_| CustomerError::JournalUnavailable)?;
        value
            .map(|value| {
                let attempt: CustomerAttempt =
                    serde_json::from_str(&value).map_err(|_| CustomerError::JournalUnavailable)?;
                if attempt
                    .customer_id
                    .as_ref()
                    .is_some_and(|id| !is_modular_id(id.get_string_repr(), "cus"))
                {
                    return Err(CustomerError::JournalUnavailable);
                }
                Ok(attempt)
            })
            .transpose()
    }
    async fn remember(
        &self,
        input: &PreparedInput,
        attempt: CustomerAttempt,
    ) -> Result<(), CustomerError> {
        let redis = self
            .state
            .store
            .get_redis_conn()
            .map_err(|_| CustomerError::JournalUnavailable)?;
        let value =
            serde_json::to_string(&attempt).map_err(|_| CustomerError::JournalUnavailable)?;
        // Keep pending IDs across lock expiry so resumes reconcile older create attempts.
        redis
            .set_key_without_modifying_ttl(
                &redis_interface::RedisKey::from(
                    self.journal_key(&input.customer.merchant_reference_id),
                ),
                value,
            )
            .await
            .map_err(|_| CustomerError::JournalUnavailable)
    }
    async fn forget_rejected(&self, input: &PreparedInput) -> Result<(), CustomerError> {
        let redis = self
            .state
            .store
            .get_redis_conn()
            .map_err(|_| CustomerError::JournalUnavailable)?;
        match redis
            .delete_key(&redis_interface::RedisKey::from(
                self.journal_key(&input.customer.merchant_reference_id),
            ))
            .await
            .map_err(|_| CustomerError::JournalUnavailable)?
        {
            redis_interface::types::DelReply::KeyDeleted => Ok(()),
            redis_interface::types::DelReply::KeyNotDeleted => {
                Err(CustomerError::JournalUnavailable)
            }
        }
    }
    async fn lock(&self, reference: &id_type::CustomerId) -> Result<(), CustomerError> {
        validate_key(
            self.merchant_id,
            self.expires_at,
            self.merchant_id,
            date_time::now(),
        )?;
        self.lock_action(reference)
            .perform_locking_action(self.state, self.merchant_id.clone())
            .await
            .map_err(|_| CustomerError::LockUnavailable)
    }
    async fn unlock(&self, reference: &id_type::CustomerId) -> Result<(), CustomerError> {
        self.lock_action(reference)
            .free_lock_action(self.state, self.merchant_id.clone())
            .await
            .map_err(|_| CustomerError::LockUnavailable)
    }
    async fn list(&self, input: &PreparedInput) -> Result<Vec<CustomerResponse>, CustomerError> {
        self.call(input, false).await
    }
    async fn create(&self, input: &PreparedInput) -> Result<CustomerResponse, CustomerError> {
        self.call(input, true).await
    }
}

pub async fn prepare(
    state: &SessionState,
    platform: &domain::Platform,
    records: Vec<PaymentMethodRecord>,
    fallback_mcas: Option<Vec<id_type::MerchantConnectorAccountId>>,
    api_key: StrongSecret<String>,
) -> RouterResult<Vec<ModularMigrationResult>> {
    let merchant = platform.get_provider().get_account().get_id();
    // Verify the supplied key locally before sending customer requests to another service.
    let hash_key = state.conf.api_keys.get_inner().get_hash_key()?;
    let hashed =
        api_keys::PlaintextApiKey::from(api_key.peek().as_str()).keyed_hash(hash_key.peek());
    let stored_key = state
        .store
        .find_api_key_by_hash_optional(hashed.into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .ok_or_else(|| CustomerError::InvalidCredential.api_error())?;
    validate_key(
        &stored_key.merchant_id,
        stored_key.expires_at,
        merchant,
        date_time::now(),
    )
    .map_err(CustomerError::api_error)?;

    // Bound the whole resolution below the lease lifetime; late/uncertain POSTs keep the lease.
    let lease = u64::from(state.conf.lock_settings.redis_lock_expiry_seconds);
    let minimum_budget = common_utils::consts::REQUEST_TIME_OUT * 3 + 5;
    if lease <= minimum_budget {
        return Err(error_stack::report!(
            CustomerError::LockUnavailable.api_error()
        ));
    }
    let mut profiles = HashMap::new();
    let mut inputs = Vec::with_capacity(records.len());
    // Validate the entire batch before the first customer POST.
    let customers =
        validate_customer_details(&records, merchant).map_err(CustomerError::api_error)?;
    for (record, customer) in records.into_iter().zip(customers) {
        // MCAs select the profile and enforce ownership; they are not stored as card tokens.
        let mut bindings = Vec::new();
        for mca_id in
            connector_ids(&record, fallback_mcas.as_ref()).map_err(CustomerError::api_error)?
        {
            let binding = if let Some(binding) = profiles.get(&mca_id) {
                binding
            } else {
                let mca = state
                    .store
                    .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                        merchant,
                        &mca_id,
                        platform.get_provider().get_key_store(),
                    )
                    .await
                    .to_not_found_response(
                        errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                            id: mca_id.get_string_repr().to_string(),
                        },
                    )?;
                profiles
                    .entry(mca_id)
                    .or_insert((mca.merchant_id, mca.profile_id))
            };
            bindings.push(binding.clone());
        }
        let profile_id = single_profile(&bindings, merchant).map_err(CustomerError::api_error)?;
        state
            .store
            .find_business_profile_by_merchant_id_profile_id(
                platform.get_provider().get_key_store(),
                merchant,
                &profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_string(),
            })?;
        let mut card = MigrationCardInput::try_from(&record);
        if let Ok(input) = &card {
            if input.subtype.is_none() {
                // The modular list/payment contract requires a known subtype. Reuse its
                // BIN inference rather than guessing credit or creating an unlistable card.
                let bin_type = state
                    .store
                    .get_card_info(&input.card.card_number.get_card_isin())
                    .await
                    .ok()
                    .flatten()
                    .and_then(|info| info.card_type);
                if !supported_card_subtype(None, bin_type) {
                    card = Err(MigrationCardInputError {
                        code: "migration_card_subtype_unavailable",
                        fields: vec!["payment_method_type"],
                    });
                }
            }
        }
        inputs.push(PreparedInput {
            line_number: record.line_number,
            profile_id,
            customer,
            card,
        });
    }
    let backend = ServiceBackend {
        state,
        merchant_id: merchant,
        api_key: &api_key,
        expires_at: stored_key.expires_at,
        hash_key: hash_key.peek(),
    };
    let card_backend = cards::ServiceCardBackend { state, merchant };
    Ok(resolve_batch(
        &backend,
        &card_backend,
        inputs,
        minimum_budget,
        state.tenant.tenant_id.get_string_repr(),
        merchant,
        hash_key.peek(),
    )
    .await)
}

type CardStorageResult = Result<(cards::StoredCard, bool), cards::CardError>;

async fn resolve_batch<B: CustomerBackend, C: cards::CardBackend>(
    backend: &B,
    card_backend: &C,
    inputs: Vec<PreparedInput>,
    budget_seconds: u64,
    tenant: &str,
    merchant: &id_type::MerchantId,
    hash_key: &[u8; 32],
) -> Vec<ModularMigrationResult> {
    // Reuse customer/card outcomes within this batch while returning every source row.
    let mut resolved: HashMap<
        id_type::CustomerId,
        Result<(CustomerResponse, bool), CustomerError>,
    > = HashMap::new();
    let mut stored: HashMap<String, (String, CardStorageResult)> = HashMap::new();
    let mut results = Vec::with_capacity(inputs.len());
    for input in inputs {
        let mut row = ModularMigrationResult::initial(&input);
        let card = match &input.card {
            Ok(card) => card,
            Err(error) => {
                row.fail(MigrationError {
                    stage: "input",
                    code: error.code,
                    retryable: error.code == "migration_card_subtype_unavailable",
                    outcome_uncertain: false,
                    fields: error.fields.clone(),
                });
                results.push(row);
                continue;
            }
        };
        let reference = input.customer.merchant_reference_id.clone();
        let customer_result = if let Some(previous) = resolved.get(&reference) {
            previous.clone().map(|(customer, _)| (customer, false))
        } else {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(budget_seconds),
                resolve_customer(backend, &input),
            )
            .await
            .unwrap_or(Err(CustomerError::UncertainCreation));
            resolved.insert(reference.clone(), result.clone());
            result
        };
        let (customer, created) = match customer_result {
            Ok(result) => result,
            Err(error) => {
                row.fail(MigrationError {
                    stage: "customer",
                    code: error.code(),
                    retryable: matches!(
                        error,
                        CustomerError::ServiceUnavailable
                            | CustomerError::UncertainCreation
                            | CustomerError::LockUnavailable
                            | CustomerError::JournalUnavailable
                    ),
                    outcome_uncertain: error == CustomerError::UncertainCreation,
                    fields: Vec::new(),
                });
                results.push(row);
                continue;
            }
        };
        // Preserve customer success even when card storage later fails, for reports and resume.
        row.customer_id = Some(customer.id.clone());
        row.customer_created = created;
        row.customer_migration_status = MigrationStatus::Success;
        let card_result = match cards::identity(tenant, merchant, &customer.id, card, hash_key) {
            Err(error) => Err(error),
            Ok((key, digest)) => {
                row.migration_operation_id = Some(key.clone());
                if let Some((previous_digest, result)) = stored.get(&key) {
                    if previous_digest != &digest {
                        Err(cards::CardError::PayloadConflict)
                    } else {
                        result.clone().map(|(card, _)| (card, false))
                    }
                } else {
                    let result = tokio::time::timeout(
                        std::time::Duration::from_secs(budget_seconds),
                        cards::store(
                            card_backend,
                            &key,
                            &digest,
                            &input.profile_id,
                            &customer.id,
                            card,
                        ),
                    )
                    .await
                    .unwrap_or(Err(cards::CardError::UncertainStorage));
                    stored.insert(key, (digest, result.clone()));
                    result
                }
            }
        };
        match card_result {
            Ok((card, _)) => {
                row.payment_method_id = Some(card.payment_method_id);
                row.card_migrated = true;
                row.payment_method_migrated = true;
                row.migration_status = MigrationStatus::Success;
            }
            Err(error) => row.fail(MigrationError {
                stage: "card",
                code: error.code(),
                retryable: error.retryable(),
                outcome_uncertain: error.uncertain(),
                fields: Vec::new(),
            }),
        }
        results.push(row);
    }
    results
}
