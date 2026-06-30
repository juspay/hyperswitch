use std::{collections::HashMap, marker::PhantomData};

use ::payment_methods::controller::PaymentMethodsController;
use actix_multipart::form::{bytes::Bytes as MultipartBytes, MultipartForm};
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::payment_methods::{
    MigrationStatus, ModularPaymentMethodMigrationRecord, ModularPaymentMethodMigrationResponse,
    ModularPaymentMethodMigrationRowResult, PaymentMethodId,
};
use common_enums::{enums, ApiVersion};
use common_utils::id_type;
use error_stack::{report, ResultExt};
use futures::future;
use hyperswitch_domain_models::platform;
use router_env::{instrument, logger, tracing, Flow};

use crate::{
    core::{api_locking, errors, payment_methods::cards},
    routes::{app::AppState, SessionState},
    services::{api, authentication as auth, ApplicationResponse},
};

pub const MAX_FINGERPRINT_ID_MIGRATION_PARALLELISM: usize = 10;
const MAX_FINGERPRINT_ID_MIGRATION_ROWS: usize = 500;

type MigrationStepResult<T> = Result<T, Box<ModularPaymentMethodMigrationRowResult>>;

#[derive(Debug, MultipartForm)]
pub struct FingerprintIdMigrationForm {
    #[multipart(limit = "1MB")]
    pub file: MultipartBytes,
}

#[instrument(skip_all, fields(flow = ?Flow::ModularPaymentMethodsMigrate))]
pub async fn modular_migrate_payment_methods(
    state: web::Data<AppState>,
    req: HttpRequest,
    MultipartForm(form): MultipartForm<FingerprintIdMigrationForm>,
) -> HttpResponse {
    let flow = Flow::ModularPaymentMethodsMigrate;
    let csv_bytes = form.file.data.to_vec();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, _, _payload, _| {
            let csv_bytes = csv_bytes.clone();
            async move { migrate_fingerprint_ids(state, &csv_bytes).await }
        },
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

async fn migrate_fingerprint_ids(
    state: SessionState,
    csv_bytes: &[u8],
) -> errors::RouterResponse<ModularPaymentMethodMigrationResponse> {
    let rows = parse_csv(csv_bytes)?;
    let total_rows = rows.len();
    let results = rows.hydrate_merchants(&state).await.migrate(&state).await;

    let successful_count = results
        .iter()
        .filter(|result| matches!(result.migration_status, MigrationStatus::Success))
        .count();
    let failed_count = total_rows - successful_count;

    Ok(ApplicationResponse::Json(
        ModularPaymentMethodMigrationResponse {
            total_rows,
            successful_count,
            failed_count,
            results,
        },
    ))
}

fn parse_csv(csv_bytes: &[u8]) -> errors::RouterResult<MigrationBatch<Parsed>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(csv_bytes);
    let headers = reader
        .headers()
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "CSV must include header: merchant_id,payment_method_id".to_string(),
        })?
        .clone();

    let has_valid_headers = headers.len() == 2
        && headers.get(0) == Some("merchant_id")
        && headers.get(1) == Some("payment_method_id");

    if !has_valid_headers {
        Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "CSV must include header: merchant_id,payment_method_id".to_string(),
        }))
    } else {
        let rows = reader
            .records()
            .enumerate()
            .map(|(index, record)| {
                let row_number = index + 2;
                record
                    .map_err(|error| {
                        Box::new(invalid_row_result(
                            row_number,
                            None,
                            None,
                            format!("Invalid CSV row: {error}"),
                        ))
                    })
                    .and_then(|record| parse_record(row_number, &headers, &record))
            })
            .collect::<Vec<_>>();

        match (rows.is_empty(), rows.len() > MAX_FINGERPRINT_ID_MIGRATION_ROWS) {
            (true, _) => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "CSV must contain at least one payment method row".to_string(),
            })),
            (_, true) => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: format!(
                    "CSV can contain at most {MAX_FINGERPRINT_ID_MIGRATION_ROWS} payment method rows"
                ),
            })),
            (false, false) => Ok(MigrationBatch::new(rows)),
        }
    }
}

fn parse_record(
    row_number: usize,
    headers: &csv::StringRecord,
    record: &csv::StringRecord,
) -> MigrationStepResult<MigrationRow<Parsed>> {
    let raw_payment_method_id = record.get(1).map(str::to_owned);

    match record.len() == 2 {
        true => record
            .deserialize::<ModularPaymentMethodMigrationRecord>(Some(headers))
            .map(|record| {
                MigrationRow::new(row_number, record.merchant_id, record.payment_method_id)
            })
            .map_err(|_| {
                Box::new(invalid_row_result(
                    row_number,
                    None,
                    raw_payment_method_id,
                    "CSV row must contain valid merchant_id and payment_method_id values"
                        .to_string(),
                ))
            }),
        false => Err(Box::new(invalid_row_result(
            row_number,
            None,
            raw_payment_method_id,
            "CSV row must contain valid merchant_id and payment_method_id values".to_string(),
        ))),
    }
}

struct Parsed;
struct MerchantsHydrated;
struct PaymentMethodsFetched;
struct ReadyForVaultRetrieve;
struct VaultRetrieved;
struct EligibilityVerified;

struct MigrationBatch<State> {
    rows: Vec<MigrationStepResult<MigrationRow<State>>>,
    merchant_contexts: HashMap<id_type::MerchantId, platform::Platform>,
    merchant_context_errors: HashMap<id_type::MerchantId, String>,
    customer_cache: HashMap<(String, String), Result<String, String>>,
    state: PhantomData<State>,
}

struct MerchantContextBatch {
    contexts: HashMap<id_type::MerchantId, platform::Platform>,
    errors: HashMap<id_type::MerchantId, String>,
}

struct CompletedMigrationChunk {
    results: Vec<ModularPaymentMethodMigrationRowResult>,
    customer_cache: HashMap<(String, String), Result<String, String>>,
}

enum CustomerCacheCandidate<'a> {
    Ready {
        row: &'a MigrationRow<EligibilityVerified>,
        customer_id: &'a id_type::CustomerId,
        key: (String, String),
    },
    Skipped {
        row_number: usize,
        merchant_id: Option<id_type::MerchantId>,
        payment_method_id: Option<String>,
        reason: String,
    },
}

fn customer_cache_candidate(
    row: &MigrationStepResult<MigrationRow<EligibilityVerified>>,
) -> CustomerCacheCandidate<'_> {
    match row {
        Err(result) => CustomerCacheCandidate::Skipped {
            row_number: result.row_number,
            merchant_id: result.merchant_id.clone(),
            payment_method_id: result.payment_method_id.clone(),
            reason: result.error_message.clone().unwrap_or_else(|| {
                "Row already failed before customer cache hydration".to_string()
            }),
        },
        Ok(row) => match row.customer_id.as_ref() {
            None => CustomerCacheCandidate::Skipped {
                row_number: row.row_number,
                merchant_id: Some(row.merchant_id.clone()),
                payment_method_id: Some(row.payment_method_id.payment_method_id.clone()),
                reason: "Payment method does not have a customer_id".to_string(),
            },
            Some(customer_id) => CustomerCacheCandidate::Ready {
                row,
                customer_id,
                key: customer_cache_key(&row.merchant_id, customer_id),
            },
        },
    }
}

impl MigrationBatch<Parsed> {
    fn new(rows: Vec<MigrationStepResult<MigrationRow<Parsed>>>) -> Self {
        Self {
            rows,
            merchant_contexts: HashMap::new(),
            merchant_context_errors: HashMap::new(),
            customer_cache: HashMap::new(),
            state: PhantomData,
        }
    }

    async fn hydrate_merchants(self, state: &SessionState) -> MigrationBatch<MerchantsHydrated> {
        let merchant_ids = self.valid_merchant_ids();
        let merchant_contexts = get_merchant_contexts_for_batch(state, merchant_ids).await;
        self.transition_with_contexts::<MerchantsHydrated>(merchant_contexts)
    }
}

impl MigrationBatch<MerchantsHydrated> {
    async fn migrate(self, state: &SessionState) -> Vec<ModularPaymentMethodMigrationRowResult> {
        let merchant_contexts = self.merchant_contexts;
        let merchant_context_errors = self.merchant_context_errors;
        let mut customer_cache = self.customer_cache;
        let mut rows = self.rows.into_iter();
        let mut results = Vec::new();

        loop {
            let row_chunk = rows
                .by_ref()
                .take(MAX_FINGERPRINT_ID_MIGRATION_PARALLELISM)
                .collect::<Vec<_>>();

            if row_chunk.is_empty() {
                break;
            }

            let completed_chunk = Self {
                rows: row_chunk,
                merchant_contexts: merchant_contexts.clone(),
                merchant_context_errors: merchant_context_errors.clone(),
                customer_cache,
                state: PhantomData::<MerchantsHydrated>,
            }
            .fetch_payment_methods(state)
            .await
            .prepare_rows()
            .retrieve_vault_data(state)
            .await
            .cache_customers(state)
            .await
            .migrate_chunk(state)
            .await;

            customer_cache = completed_chunk.customer_cache;
            results.extend(completed_chunk.results);
        }

        results.sort_by_key(|result| result.row_number);
        results
    }

    async fn fetch_payment_methods(
        self,
        state: &SessionState,
    ) -> MigrationBatch<PaymentMethodsFetched> {
        let mut grouped_rows: HashMap<id_type::MerchantId, Vec<MigrationRow<MerchantsHydrated>>> =
            HashMap::new();
        let mut results = Vec::new();

        for row in self.rows {
            match row {
                Ok(row) => grouped_rows
                    .entry(row.merchant_id.clone())
                    .or_default()
                    .push(row),
                Err(result) => results.push(Err(result)),
            }
        }

        let mut fetched_rows = Vec::new();

        for (merchant_id, rows) in grouped_rows {
            match self.merchant_contexts.get(&merchant_id) {
                None => {
                    let message = self
                        .merchant_context_errors
                        .get(&merchant_id)
                        .cloned()
                        .unwrap_or_else(|| {
                            "Merchant account or key store not found for merchant_id".to_string()
                        });
                    fetched_rows.extend(
                        rows.into_iter()
                            .map(|row| Err(Box::new(row.failed(message.clone())))),
                    );
                }
                Some(platform) => {
                    let mut payment_method_ids = Vec::new();
                    for row in &rows {
                        if !payment_method_ids.contains(&row.payment_method_id.payment_method_id) {
                            payment_method_ids
                                .push(row.payment_method_id.payment_method_id.clone());
                        }
                    }

                    match state
                        .store
                        .find_payment_methods_by_merchant_id_payment_method_ids(
                            platform.get_provider().get_key_store(),
                            &merchant_id,
                            &payment_method_ids,
                            platform.get_provider().get_account().storage_scheme,
                        )
                        .await
                    {
                        Ok(payment_methods) => {
                            let payment_method_map = payment_methods
                                .into_iter()
                                .map(|payment_method| {
                                    (payment_method.payment_method_id.clone(), payment_method)
                                })
                                .collect::<HashMap<_, _>>();

                            fetched_rows.extend(rows.into_iter().map(|row| {
                                row.attach_payment_method_or_validate_mismatch(&payment_method_map)
                            }));
                        }
                        Err(error) => {
                            fetched_rows.extend(rows.into_iter().map(|row| {
                                Err(Box::new(row.result(
                                    MigrationStatus::Failed,
                                    Some(format!(
                                        "Failed to fetch payment method records: {error:?}"
                                    )),
                                )))
                            }));
                        }
                    }
                }
            }
        }

        fetched_rows.extend(results);

        MigrationBatch {
            rows: fetched_rows,
            merchant_contexts: self.merchant_contexts,
            merchant_context_errors: self.merchant_context_errors,
            customer_cache: self.customer_cache,
            state: PhantomData,
        }
    }
}

impl MigrationBatch<PaymentMethodsFetched> {
    fn prepare_rows(self) -> MigrationBatch<ReadyForVaultRetrieve> {
        MigrationBatch {
            rows: self
                .rows
                .into_iter()
                .map(|row| {
                    row.and_then(|row| {
                        row.prepare_for_retrieve().and_then(
                            MigrationRow::<ReadyForVaultRetrieve>::validate_record_version,
                        )
                    })
                })
                .collect(),
            merchant_contexts: self.merchant_contexts,
            merchant_context_errors: self.merchant_context_errors,
            customer_cache: self.customer_cache,
            state: PhantomData,
        }
    }
}

impl MigrationBatch<ReadyForVaultRetrieve> {
    async fn retrieve_vault_data(
        self,
        state: &SessionState,
    ) -> MigrationBatch<EligibilityVerified> {
        let merchant_contexts = &self.merchant_contexts;
        let rows = future::join_all(self.rows.into_iter().map(|row| async move {
            match row {
                Ok(row) => match merchant_contexts.get(&row.merchant_id) {
                    Some(platform) => match row.retrieve_from_vault(state, platform).await {
                        Ok(row) => row.validate_vaulting_data(),
                        Err(result) => Err(result),
                    },
                    None => Err(Box::new(
                        row.failed("Merchant account or key store not found for merchant_id"),
                    )),
                },
                Err(result) => Err(result),
            }
        }))
        .await;

        MigrationBatch {
            rows,
            merchant_contexts: self.merchant_contexts,
            merchant_context_errors: self.merchant_context_errors,
            customer_cache: self.customer_cache,
            state: PhantomData,
        }
    }
}

impl MigrationBatch<EligibilityVerified> {
    async fn cache_customers(self, state: &SessionState) -> Self {
        let mut customer_cache = self.customer_cache;

        for row in &self.rows {
            match customer_cache_candidate(row) {
                CustomerCacheCandidate::Ready {
                    row,
                    customer_id,
                    key,
                } if !customer_cache.contains_key(&key) => {
                    let result =
                        match self.merchant_contexts.get(&row.merchant_id) {
                            Some(platform) => match state
                                .store
                                .find_customer_by_customer_id_merchant_id(
                                    customer_id,
                                    &row.merchant_id,
                                    platform.get_provider().get_key_store(),
                                    platform.get_provider().get_account().storage_scheme,
                                )
                                .await
                            {
                                Ok(customer) => {
                                    customer.get_global_customer_id().clone().ok_or_else(|| {
                                        "Customer global id is required for fingerprint migration"
                                            .to_string()
                                    })
                                }
                                Err(error) => {
                                    Err(format!("Failed to fetch customer record: {error:?}"))
                                }
                            },
                            None => Err("Merchant account or key store not found for merchant_id"
                                .to_string()),
                        };

                    customer_cache.insert(key, result);
                }
                CustomerCacheCandidate::Ready { row, key, .. } => {
                    logger::debug!(
                        row_number = row.row_number,
                        merchant_id = ?row.merchant_id,
                        payment_method_id = row.payment_method_id.payment_method_id,
                        customer_cache_key = ?key,
                        "customer cache hydration skipped because customer was already cached"
                    );
                }
                CustomerCacheCandidate::Skipped {
                    row_number,
                    merchant_id,
                    payment_method_id,
                    reason,
                } => {
                    logger::warn!(
                        row_number,
                        merchant_id = ?merchant_id,
                        payment_method_id = ?payment_method_id,
                        reason,
                        "customer cache hydration skipped for migration row"
                    );
                }
            }
        }

        Self {
            rows: self.rows,
            merchant_contexts: self.merchant_contexts,
            merchant_context_errors: self.merchant_context_errors,
            customer_cache,
            state: PhantomData,
        }
    }
}

impl MigrationBatch<EligibilityVerified> {
    async fn migrate_chunk(self, state: &SessionState) -> CompletedMigrationChunk {
        let results = future::join_all(self.rows.into_iter().map(|row| async {
            let result = match row {
                Ok(row) => {
                    process_row(row, state, &self.merchant_contexts, &self.customer_cache).await
                }
                Err(result) => *result,
            };
            log_row_result(&result);
            result
        }))
        .await;

        CompletedMigrationChunk {
            results,
            customer_cache: self.customer_cache,
        }
    }
}

impl<State> MigrationBatch<State> {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn valid_merchant_ids(&self) -> Vec<id_type::MerchantId> {
        let mut merchant_ids = Vec::new();
        for row in self.rows.iter().flatten() {
            if !merchant_ids.contains(&row.merchant_id) {
                merchant_ids.push(row.merchant_id.clone());
            }
        }
        merchant_ids
    }

    fn transition_with_contexts<NextState>(
        self,
        merchant_context_batch: MerchantContextBatch,
    ) -> MigrationBatch<NextState> {
        MigrationBatch {
            rows: self
                .rows
                .into_iter()
                .map(|row| row.map(MigrationRow::transition))
                .collect(),
            merchant_contexts: merchant_context_batch.contexts,
            merchant_context_errors: merchant_context_batch.errors,
            customer_cache: self.customer_cache,
            state: PhantomData,
        }
    }
}

struct MigrationRow<State> {
    row_number: usize,
    merchant_id: id_type::MerchantId,
    payment_method_id: PaymentMethodId,
    payment_method: Option<hyperswitch_domain_models::payment_methods::PaymentMethod>,
    customer_id: Option<id_type::CustomerId>,
    vault_id: Option<hyperswitch_domain_models::payment_methods::VaultId>,
    vaulting_data: Option<hyperswitch_domain_models::vault::PaymentMethodVaultingData>,
    old_fingerprint_id: Option<String>,
    is_v2_pm: bool,
    state: PhantomData<State>,
}

impl MigrationRow<Parsed> {
    fn new(
        row_number: usize,
        merchant_id: id_type::MerchantId,
        payment_method_id: PaymentMethodId,
    ) -> Self {
        Self {
            row_number,
            merchant_id,
            payment_method_id,
            payment_method: None,
            customer_id: None,
            vault_id: None,
            vaulting_data: None,
            old_fingerprint_id: None,
            is_v2_pm: false,
            state: PhantomData,
        }
    }
}

impl MigrationRow<MerchantsHydrated> {
    fn attach_payment_method_or_validate_mismatch(
        self,
        payment_method_map: &HashMap<
            String,
            hyperswitch_domain_models::payment_methods::PaymentMethod,
        >,
    ) -> MigrationStepResult<MigrationRow<PaymentMethodsFetched>> {
        match payment_method_map.get(&self.payment_method_id.payment_method_id) {
            Some(payment_method) => self.attach_payment_method(payment_method.clone()),
            None => Err(Box::new(self.failed(
                "Payment method not found for merchant_id or belongs to another merchant",
            ))),
        }
    }

    fn attach_payment_method(
        self,
        payment_method: hyperswitch_domain_models::payment_methods::PaymentMethod,
    ) -> MigrationStepResult<MigrationRow<PaymentMethodsFetched>> {
        let mut next = self.transition::<PaymentMethodsFetched>();
        next.old_fingerprint_id = payment_method.locker_fingerprint_id.clone();
        next.payment_method = Some(payment_method);
        Ok(next)
    }
}

impl MigrationRow<PaymentMethodsFetched> {
    fn prepare_for_retrieve(self) -> MigrationStepResult<MigrationRow<ReadyForVaultRetrieve>> {
        let payment_method = self
            .payment_method
            .as_ref()
            .ok_or_else(|| Box::new(self.failed("Payment method record was not available")))?;
        let customer_id = payment_method
            .customer_id
            .clone()
            .ok_or_else(|| Box::new(self.failed("Payment method must have a customer_id")))?;
        let locker_id = payment_method
            .locker_id
            .clone()
            .ok_or_else(|| Box::new(self.failed("Payment method must have a locker_id")))?;
        let old_fingerprint_id = payment_method
            .locker_fingerprint_id
            .clone()
            .ok_or_else(|| Box::new(self.failed("Payment method must have a fingerprint")))?;

        let mut next = self.transition::<ReadyForVaultRetrieve>();
        next.customer_id = Some(customer_id);
        next.vault_id =
            Some(hyperswitch_domain_models::payment_methods::VaultId::generate(locker_id));
        next.old_fingerprint_id = Some(old_fingerprint_id);
        Ok(next)
    }
}

impl MigrationRow<ReadyForVaultRetrieve> {
    fn validate_record_version(mut self) -> MigrationStepResult<Self> {
        let payment_method = self
            .payment_method
            .as_ref()
            .ok_or_else(|| Box::new(self.failed("Payment method record was not available")))?;

        match payment_method.version {
            ApiVersion::V1 => {
                self.is_v2_pm = false;
                Ok(self)
            }
            ApiVersion::V2 if payment_method.payment_method == Some(enums::PaymentMethod::Card) => {
                self.is_v2_pm = true;
                Ok(self)
            }
            ApiVersion::V2 => Err(Box::new(
                self.failed("v2 records can only be card payment methods"),
            )),
        }
    }

    async fn retrieve_from_vault(
        self,
        state: &SessionState,
        platform: &platform::Platform,
    ) -> MigrationStepResult<MigrationRow<VaultRetrieved>> {
        let customer_id = self
            .customer_id
            .as_ref()
            .ok_or_else(|| Box::new(self.failed("Payment method must have a customer_id")))?;
        let vault_id = self
            .vault_id
            .as_ref()
            .ok_or_else(|| Box::new(self.failed("Payment method must have a locker_id")))?;
        let controller = cards::PmCards {
            state,
            provider: platform.get_provider(),
        };

        match controller
            .retrieve_payment_method_from_vault(
                vault_id,
                &self.merchant_id,
                customer_id,
                self.is_v2_pm,
            )
            .await
        {
            Ok(vaulting_data) => {
                let mut next = self.transition::<VaultRetrieved>();
                next.vaulting_data = Some(vaulting_data);
                Ok(next)
            }
            Err(error) => Err(Box::new(self.failed(format!(
                "Failed to retrieve payment method from vault: {error:?}"
            )))),
        }
    }
}

impl MigrationRow<VaultRetrieved> {
    fn validate_vaulting_data(self) -> MigrationStepResult<MigrationRow<EligibilityVerified>> {
        let payment_method = self
            .payment_method
            .as_ref()
            .ok_or_else(|| Box::new(self.failed("Payment method record was not available")))?;
        let vaulting_data = self
            .vaulting_data
            .as_ref()
            .ok_or_else(|| Box::new(self.failed("Vaulting data was not available")))?;

        let is_bank_debit = matches!(
            vaulting_data,
            hyperswitch_domain_models::vault::PaymentMethodVaultingData::BankDebit(_)
        );
        let is_wallet = matches!(
            vaulting_data,
            hyperswitch_domain_models::vault::PaymentMethodVaultingData::Wallet(_)
        );
        let is_card = matches!(
            vaulting_data,
            hyperswitch_domain_models::vault::PaymentMethodVaultingData::Card(_)
        );

        match payment_method.version {
            ApiVersion::V1 if is_bank_debit || is_wallet => Ok(self.transition()),
            ApiVersion::V1 => Err(Box::new(
                self.failed("v1 records can only be wallet or bank debit"),
            )),
            ApiVersion::V2 if is_card => Ok(self.transition()),
            ApiVersion::V2 => Err(Box::new(
                self.failed("v2 records can only be card payment methods"),
            )),
        }
    }
}

impl MigrationRow<EligibilityVerified> {
    async fn migrate(
        self,
        state: &SessionState,
        platform: &platform::Platform,
        customer_cache: &HashMap<(String, String), Result<String, String>>,
    ) -> ModularPaymentMethodMigrationRowResult {
        match self
            .execute_vault_migration(state, platform, customer_cache)
            .await
        {
            Ok(new_fingerprint_id) => self.success_result(Some(new_fingerprint_id)),
            Err(result) => *result,
        }
    }

    fn customer_key(
        &self,
        customer_cache: &HashMap<(String, String), Result<String, String>>,
    ) -> MigrationStepResult<String> {
        let customer_id = self
            .customer_id
            .as_ref()
            .ok_or_else(|| Box::new(self.failed("Payment method must have a customer_id")))?;
        let key = customer_cache_key(&self.merchant_id, customer_id);

        match customer_cache.get(&key) {
            Some(Ok(customer_key)) => Ok(customer_key.clone()),
            Some(Err(error)) => Err(Box::new(self.failed(error.clone()))),
            None => Err(Box::new(self.failed(
                "Customer record was not available for fingerprint migration",
            ))),
        }
    }

    async fn execute_vault_migration(
        &self,
        state: &SessionState,
        platform: &platform::Platform,
        customer_cache: &HashMap<(String, String), Result<String, String>>,
    ) -> MigrationStepResult<String> {
        let vault_id = self
            .vault_id
            .as_ref()
            .ok_or_else(|| Box::new(self.failed("Payment method must have a locker_id")))?;
        let vaulting_data = self
            .vaulting_data
            .as_ref()
            .ok_or_else(|| Box::new(self.failed("Vaulting data was not available")))?;
        let old_fingerprint_id = self
            .old_fingerprint_id
            .clone()
            .ok_or_else(|| Box::new(self.failed("Payment method must have a fingerprint")))?;
        let controller = cards::PmCards {
            state,
            provider: platform.get_provider(),
        };
        let fingerprint_data = vaulting_data.to_fingerprint_data();
        let customer_key = Some(self.customer_key(customer_cache)?);

        let new_fingerprint_id = controller
            .get_fingerprint_id_from_vault(&customer_key, &fingerprint_data, old_fingerprint_id)
            .await
            .map_err(|error| {
                Box::new(self.failed(format!(
                    "Failed to create fingerprint record in vault: {error:?}"
                )))
            })?;

        controller
            .store_payment_method_in_vault(&self.merchant_id, vault_id, vaulting_data)
            .await
            .map_err(|error| {
                Box::new(self.failed(format!(
                    "Failed to store payment method in vault with new entity_id: {error:?}"
                )))
            })?;

        Ok(new_fingerprint_id)
    }
}

impl<State> MigrationRow<State> {
    fn transition<NextState>(self) -> MigrationRow<NextState> {
        MigrationRow {
            row_number: self.row_number,
            merchant_id: self.merchant_id,
            payment_method_id: self.payment_method_id,
            payment_method: self.payment_method,
            customer_id: self.customer_id,
            vault_id: self.vault_id,
            vaulting_data: self.vaulting_data,
            old_fingerprint_id: self.old_fingerprint_id,
            is_v2_pm: self.is_v2_pm,
            state: PhantomData,
        }
    }

    fn result(
        &self,
        migration_status: MigrationStatus,
        error_message: Option<String>,
    ) -> ModularPaymentMethodMigrationRowResult {
        ModularPaymentMethodMigrationRowResult {
            row_number: self.row_number,
            merchant_id: Some(self.merchant_id.clone()),
            payment_method_id: Some(self.payment_method_id.payment_method_id.clone()),
            old_fingerprint_id: self.old_fingerprint_id.clone(),
            new_fingerprint_id: None,
            migration_status,
            error_message,
        }
    }

    fn failed(&self, message: impl Into<String>) -> ModularPaymentMethodMigrationRowResult {
        self.result(MigrationStatus::Failed, Some(message.into()))
    }

    fn success_result(
        &self,
        new_fingerprint_id: Option<String>,
    ) -> ModularPaymentMethodMigrationRowResult {
        ModularPaymentMethodMigrationRowResult {
            row_number: self.row_number,
            merchant_id: Some(self.merchant_id.clone()),
            payment_method_id: Some(self.payment_method_id.payment_method_id.clone()),
            old_fingerprint_id: self.old_fingerprint_id.clone(),
            new_fingerprint_id,
            migration_status: MigrationStatus::Success,
            error_message: None,
        }
    }
}

async fn process_row(
    row: MigrationRow<EligibilityVerified>,
    state: &SessionState,
    merchant_contexts: &HashMap<id_type::MerchantId, platform::Platform>,
    customer_cache: &HashMap<(String, String), Result<String, String>>,
) -> ModularPaymentMethodMigrationRowResult {
    match merchant_contexts.get(&row.merchant_id) {
        None => row.failed("Merchant account or key store not found for merchant_id"),
        Some(platform) => row.migrate(state, platform, customer_cache).await,
    }
}

fn customer_cache_key(
    merchant_id: &id_type::MerchantId,
    customer_id: &id_type::CustomerId,
) -> (String, String) {
    (
        merchant_id.get_string_repr().to_owned(),
        customer_id.get_string_repr().to_owned(),
    )
}

fn invalid_row_result(
    row_number: usize,
    merchant_id: Option<id_type::MerchantId>,
    payment_method_id: Option<String>,
    message: String,
) -> ModularPaymentMethodMigrationRowResult {
    ModularPaymentMethodMigrationRowResult {
        row_number,
        merchant_id,
        payment_method_id,
        old_fingerprint_id: None,
        new_fingerprint_id: None,
        migration_status: MigrationStatus::Failed,
        error_message: Some(message),
    }
}

fn log_row_result(result: &ModularPaymentMethodMigrationRowResult) {
    logger::info!(
        row_number = result.row_number,
        merchant_id = ?result.merchant_id,
        payment_method_id = ?result.payment_method_id,
        status = ?result.migration_status,
        error_message = ?result.error_message,
        "fingerprint id migration row processed"
    );
}

async fn get_merchant_contexts_for_batch(
    state: &SessionState,
    merchant_ids: Vec<id_type::MerchantId>,
) -> MerchantContextBatch {
    match state
        .store
        .list_multiple_key_stores(
            merchant_ids.clone(),
            &state.store.get_master_key().to_vec().into(),
        )
        .await
    {
        Err(error) => {
            logger::error!("Failed to bulk fetch merchant key stores: {:?}", error);
            MerchantContextBatch::all_failed(
                merchant_ids,
                "Failed to fetch merchant key stores".to_string(),
            )
        }
        Ok(key_stores) => match state
            .store
            .list_multiple_merchant_accounts(merchant_ids.clone())
            .await
        {
            Err(error) => {
                logger::error!("Failed to bulk fetch merchant accounts: {:?}", error);
                MerchantContextBatch::all_failed(
                    merchant_ids,
                    "Failed to fetch merchant accounts".to_string(),
                )
            }
            Ok(merchant_accounts) => {
                let key_store_map = key_stores
                    .into_iter()
                    .map(|key_store| (key_store.merchant_id.clone(), key_store))
                    .collect::<HashMap<_, _>>();
                let merchant_account_map = merchant_accounts
                    .into_iter()
                    .map(|merchant_account| (merchant_account.get_id().clone(), merchant_account))
                    .collect::<HashMap<_, _>>();
                let mut contexts = HashMap::new();
                let mut errors = HashMap::new();

                for merchant_id in merchant_ids {
                    match (
                        key_store_map.get(&merchant_id),
                        merchant_account_map.get(&merchant_id),
                    ) {
                        (Some(key_store), Some(merchant_account)) => {
                            contexts.insert(
                                merchant_id,
                                platform::Platform::new(
                                    merchant_account.clone(),
                                    key_store.clone(),
                                    merchant_account.clone(),
                                    key_store.clone(),
                                    None,
                                ),
                            );
                        }
                        (None, _) => {
                            errors.insert(
                                merchant_id,
                                "Failed to fetch merchant key store".to_string(),
                            );
                        }
                        (_, None) => {
                            errors.insert(
                                merchant_id,
                                "Failed to fetch merchant account".to_string(),
                            );
                        }
                    }
                }

                MerchantContextBatch { contexts, errors }
            }
        },
    }
}

impl MerchantContextBatch {
    fn all_failed(merchant_ids: Vec<id_type::MerchantId>, message: String) -> Self {
        Self {
            contexts: HashMap::new(),
            errors: merchant_ids
                .into_iter()
                .map(|merchant_id| (merchant_id, message.clone()))
                .collect(),
        }
    }
}
