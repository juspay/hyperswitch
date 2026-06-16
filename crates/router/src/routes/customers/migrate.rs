use std::marker::PhantomData;

use actix_multipart::form::{bytes::Bytes as MultipartBytes, MultipartForm};
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::customers::migrate::{
    CustomerGlobalIdMigrationResponse, CustomerGlobalIdMigrationRowResult,
    CustomerGlobalIdMigrationStatus,
};
use common_enums::ApiVersion;
use common_utils::id_type;
use error_stack::{report, ResultExt};
use router_env::{instrument, logger, tracing, Flow};

use crate::{
    core::{api_locking, customers, errors},
    routes::{app::AppState, SessionState},
    services::{api, authentication as auth, ApplicationResponse},
};

pub const MAX_CUSTOMER_GLOBAL_ID_MIGRATION_FILE_SIZE: usize = 1024 * 1024;
pub const MAX_CUSTOMER_GLOBAL_ID_MIGRATION_RECORDS: usize = 500;
pub const MAX_CUSTOMER_GLOBAL_ID_MIGRATION_PARALLELISM: usize = 10;

type MigrationStepResult<T> = Result<T, Box<CustomerGlobalIdMigrationRowResult>>;

#[derive(Debug, MultipartForm)]
pub struct CustomerGlobalIdMigrationForm {
    #[multipart(limit = "1MB")]
    pub file: MultipartBytes,
}

#[instrument(skip_all, fields(flow = ?Flow::CustomersGlobalIdMigration))]
pub async fn migrate_global_id(
    state: web::Data<AppState>,
    req: HttpRequest,
    MultipartForm(form): MultipartForm<CustomerGlobalIdMigrationForm>,
) -> HttpResponse {
    let flow = Flow::CustomersGlobalIdMigration;
    let csv_bytes = form.file.data.to_vec();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, _, _payload, _| {
            let csv_bytes = csv_bytes.clone();
            async move { migrate_customer_global_id(state, &csv_bytes).await }
        },
        &auth::V2AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

async fn migrate_customer_global_id(
    state: SessionState,
    csv_bytes: &[u8],
) -> errors::RouterResponse<CustomerGlobalIdMigrationResponse> {
    let rows = parse_customer_global_id_migration_csv(csv_bytes)?;
    let total_rows = rows.len();
    let mut results = Vec::with_capacity(total_rows);
    let mut rows = rows.into_iter();

    loop {
        let row_chunk = rows
            .by_ref()
            .take(MAX_CUSTOMER_GLOBAL_ID_MIGRATION_PARALLELISM)
            .collect::<Vec<_>>();

        if row_chunk.is_empty() {
            break;
        }

        let chunk_results = futures::future::join_all(row_chunk.into_iter().map(|row| async {
            let result = match row {
                Ok(row) => process_fetched_row(row.fetch(&state).await, &state).await,
                Err(result) => *result,
            };
            log_row_result(&result);
            result
        }))
        .await;

        results.extend(chunk_results);
    }

    let updated_count = results
        .iter()
        .filter(|result| {
            matches!(
                result.status,
                CustomerGlobalIdMigrationStatus::UpdatedNullId
                    | CustomerGlobalIdMigrationStatus::UpdatedNonGlobalId
            )
        })
        .count();
    let skipped_count = results
        .iter()
        .filter(|result| {
            matches!(
                result.status,
                CustomerGlobalIdMigrationStatus::AlreadyGlobalId
                    | CustomerGlobalIdMigrationStatus::SkippedNonV1
            )
        })
        .count();
    let failed_count = total_rows - updated_count - skipped_count;

    Ok(ApplicationResponse::Json(
        CustomerGlobalIdMigrationResponse {
            total_rows,
            updated_count,
            skipped_count,
            failed_count,
            results,
        },
    ))
}

fn parse_customer_global_id_migration_csv(
    csv_bytes: &[u8],
) -> errors::RouterResult<Vec<MigrationStepResult<MigrationRow<Parsed>>>> {
    if csv_bytes.len() > MAX_CUSTOMER_GLOBAL_ID_MIGRATION_FILE_SIZE {
        Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "CSV file exceeds maximum size of {} bytes",
                MAX_CUSTOMER_GLOBAL_ID_MIGRATION_FILE_SIZE
            ),
        }))
    } else {
        let mut reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(csv_bytes);
        let headers =
            reader
                .headers()
                .change_context(errors::ApiErrorResponse::InvalidRequestData {
                    message: "CSV must include header: merchant_id,customer_id".to_string(),
                })?;

        if headers.len() != 2
            || headers.get(0) != Some("merchant_id")
            || headers.get(1) != Some("customer_id")
        {
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "CSV must include header: merchant_id,customer_id".to_string(),
            }))
        } else {
            reader
                .records()
                .enumerate()
                .map(|(index, record)| {
                    if index >= MAX_CUSTOMER_GLOBAL_ID_MIGRATION_RECORDS {
                        Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                            message: format!(
                                "CSV file exceeds maximum record count of {}",
                                MAX_CUSTOMER_GLOBAL_ID_MIGRATION_RECORDS
                            ),
                        }))
                    } else {
                        let row_number = index + 2;
                        let row = record
                            .map_err(|error| {
                                Box::new(invalid_csv_row_result(
                                    row_number,
                                    None,
                                    None,
                                    format!("Invalid CSV row: {error}"),
                                ))
                            })
                            .and_then(|record| {
                                parse_customer_global_id_migration_record(row_number, &record)
                            });
                        Ok(row)
                    }
                })
                .collect()
        }
    }
}

fn parse_customer_global_id_migration_record(
    row_number: usize,
    record: &csv::StringRecord,
) -> MigrationStepResult<MigrationRow<Parsed>> {
    let raw_merchant_id = record
        .get(0)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let raw_customer_id = record
        .get(1)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let merchant_id = raw_merchant_id
        .map(|value| id_type::MerchantId::wrap(value.to_owned()).ok())
        .unwrap_or(None);
    let customer_id = raw_customer_id
        .map(|value| id_type::CustomerId::wrap(value.to_owned()).ok())
        .unwrap_or(None);

    match (record.len() == 2, merchant_id, customer_id) {
        (true, Some(merchant_id), Some(customer_id)) => {
            Ok(MigrationRow::new(row_number, merchant_id, customer_id))
        }
        (_, merchant_id, customer_id) => Err(Box::new(invalid_csv_row_result(
            row_number,
            merchant_id,
            customer_id,
            "CSV row must contain valid merchant_id and customer_id values".to_string(),
        ))),
    }
}

fn invalid_csv_row_result(
    row_number: usize,
    merchant_id: Option<id_type::MerchantId>,
    customer_id: Option<id_type::CustomerId>,
    message: String,
) -> CustomerGlobalIdMigrationRowResult {
    CustomerGlobalIdMigrationRowResult {
        row_number,
        merchant_id,
        customer_id,
        status: CustomerGlobalIdMigrationStatus::InvalidCsvRow,
        old_id: None,
        new_id: None,
        message,
    }
}

struct Parsed;
struct Fetched;
struct V1Verified;
struct NeedsUpdate;
struct Generated;

struct MigrationRow<State> {
    row_number: usize,
    merchant_id: id_type::MerchantId,
    customer_id: id_type::CustomerId,
    old_id: Option<String>,
    version: Option<ApiVersion>,
    new_id: Option<id_type::GlobalCustomerId>,
    update_status: Option<CustomerGlobalIdMigrationStatus>,
    state: PhantomData<State>,
}

impl MigrationRow<Parsed> {
    fn new(
        row_number: usize,
        merchant_id: id_type::MerchantId,
        customer_id: id_type::CustomerId,
    ) -> Self {
        Self {
            row_number,
            merchant_id,
            customer_id,
            old_id: None,
            version: None,
            new_id: None,
            update_status: None,
            state: PhantomData,
        }
    }

    async fn fetch(self, state: &SessionState) -> MigrationStepResult<MigrationRow<Fetched>> {
        match state
            .store
            .find_customer_for_global_id_migration(&self.customer_id, &self.merchant_id)
            .await
        {
            Ok(row) => {
                let mut next = self.transition::<Fetched>();
                next.old_id = row.id;
                next.version = Some(row.version);
                Ok(next)
            }
            Err(error)
                if matches!(
                    error.current_context(),
                    errors::StorageError::ValueNotFound(_)
                ) =>
            {
                Err(Box::new(self.result(
                    CustomerGlobalIdMigrationStatus::NotFound,
                    None,
                    "Customer not found for merchant_id and customer_id".to_string(),
                )))
            }
            Err(error) => Err(Box::new(self.result(
                CustomerGlobalIdMigrationStatus::UpdateFailed,
                None,
                format!("Failed to fetch customer for migration: {error:?}"),
            ))),
        }
    }
}

impl MigrationRow<Fetched> {
    fn verify_v1(self) -> MigrationStepResult<MigrationRow<V1Verified>> {
        match self.version {
            Some(ApiVersion::V1) => Ok(self.transition::<V1Verified>()),
            Some(_) => Err(Box::new(self.result(
                CustomerGlobalIdMigrationStatus::SkippedNonV1,
                None,
                "Customer is not a v1 row; skipping migration".to_string(),
            ))),
            None => Err(Box::new(self.result(
                CustomerGlobalIdMigrationStatus::UpdateFailed,
                None,
                "Customer version was not available for migration".to_string(),
            ))),
        }
    }
}

impl MigrationRow<V1Verified> {
    fn classify_current_id(self) -> MigrationStepResult<MigrationRow<NeedsUpdate>> {
        match self.old_id.as_deref() {
            Some(id) if customers::is_global_customer_id_format(id) => Err(Box::new(self.result(
                CustomerGlobalIdMigrationStatus::AlreadyGlobalId,
                None,
                "Customer id is already in global id format".to_string(),
            ))),
            None => {
                let mut next = self.transition::<NeedsUpdate>();
                next.update_status = Some(CustomerGlobalIdMigrationStatus::UpdatedNullId);
                Ok(next)
            }
            Some(_) => {
                let mut next = self.transition::<NeedsUpdate>();
                next.update_status = Some(CustomerGlobalIdMigrationStatus::UpdatedNonGlobalId);
                Ok(next)
            }
        }
    }
}

impl MigrationRow<NeedsUpdate> {
    fn generate(self, state: &SessionState) -> MigrationRow<Generated> {
        let new_id = id_type::GlobalCustomerId::generate(&state.conf.cell_information.id);
        let mut next = self.transition::<Generated>();
        next.new_id = Some(new_id);
        next
    }
}

impl MigrationRow<Generated> {
    async fn update(self, state: &SessionState) -> CustomerGlobalIdMigrationRowResult {
        match self.new_id.clone() {
            Some(new_id) => match state
                .store
                .update_customer_global_id_for_migration(
                    &self.customer_id,
                    &self.merchant_id,
                    new_id,
                )
                .await
            {
                Ok(_) => self.result(
                    self.update_status
                        .clone()
                        .unwrap_or(CustomerGlobalIdMigrationStatus::UpdateFailed),
                    self.new_id.as_ref(),
                    "Customer id updated successfully".to_string(),
                ),
                Err(error) => self.result(
                    CustomerGlobalIdMigrationStatus::UpdateFailed,
                    self.new_id.as_ref(),
                    format!("Failed to update customer id: {error:?}"),
                ),
            },
            None => self.result(
                CustomerGlobalIdMigrationStatus::UpdateFailed,
                None,
                "New customer global id was not generated".to_string(),
            ),
        }
    }
}

async fn process_fetched_row(
    row: MigrationStepResult<MigrationRow<Fetched>>,
    state: &SessionState,
) -> CustomerGlobalIdMigrationRowResult {
    match row {
        Ok(row) => match row
            .verify_v1()
            .and_then(MigrationRow::<V1Verified>::classify_current_id)
        {
            Ok(row) => row.generate(state).update(state).await,
            Err(result) => *result,
        },
        Err(result) => *result,
    }
}

impl<State> MigrationRow<State> {
    fn transition<NextState>(self) -> MigrationRow<NextState> {
        MigrationRow {
            row_number: self.row_number,
            merchant_id: self.merchant_id,
            customer_id: self.customer_id,
            old_id: self.old_id,
            version: self.version,
            new_id: self.new_id,
            update_status: self.update_status,
            state: PhantomData,
        }
    }

    fn result(
        &self,
        status: CustomerGlobalIdMigrationStatus,
        new_id: Option<&id_type::GlobalCustomerId>,
        message: String,
    ) -> CustomerGlobalIdMigrationRowResult {
        CustomerGlobalIdMigrationRowResult {
            row_number: self.row_number,
            merchant_id: Some(self.merchant_id.clone()),
            customer_id: Some(self.customer_id.clone()),
            status,
            old_id: self.old_id.clone(),
            new_id: new_id.map(|id| id.get_string_repr().to_owned()),
            message,
        }
    }
}

fn log_row_result(result: &CustomerGlobalIdMigrationRowResult) {
    logger::info!(
        row_number = result.row_number,
        merchant_id = ?result.merchant_id,
        customer_id = ?result.customer_id,
        old_id = ?result.old_id,
        new_id = ?result.new_id,
        status = ?result.status,
        message = result.message,
        "customer global id migration row processed"
    );
}
