use actix_multipart::form::{self, bytes, text};
use api_models::payment_methods as pm_api;
#[cfg(feature = "v1")]
use common_enums::{enums, ApiVersion};
use csv::Reader;
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::{api, platform};
use hyperswitch_masking::PeekInterface;
use rdkafka::message::ToBytes;
use router_env::{instrument, tracing};

use crate::core::errors;
#[cfg(feature = "v1")]
use crate::{controller as pm, state};
pub mod payment_methods;
pub use payment_methods::migrate_payment_method;

#[cfg(feature = "v1")]
type PmMigrationResult<T> =
    errors::CustomResult<api::ApplicationResponse<T>, errors::ApiErrorResponse>;

#[cfg(feature = "v1")]
pub async fn migrate_payment_methods(
    state: &state::PaymentMethodsState,
    payment_methods: Vec<pm_api::PaymentMethodRecord>,
    merchant_id: &common_utils::id_type::MerchantId,
    platform: &platform::Platform,
    mca_ids: Option<Vec<common_utils::id_type::MerchantConnectorAccountId>>,
    controller: &dyn pm::PaymentMethodsController,
) -> PmMigrationResult<Vec<pm_api::PaymentMethodMigrationResponse>> {
    let mut result = Vec::with_capacity(payment_methods.len());

    for record in payment_methods {
        let req = pm_api::PaymentMethodMigrate::try_from((
            &record,
            merchant_id.clone(),
            mca_ids.as_ref(),
        ))
        .map_err(|err| errors::ApiErrorResponse::InvalidRequestData {
            message: format!("error: {err:?}"),
        })
        .attach_printable("record deserialization failed");

        let res = match req {
            Ok(migrate_request) => {
                let res = migrate_payment_method(
                    state,
                    migrate_request,
                    merchant_id,
                    platform,
                    controller,
                )
                .await;
                match res {
                    Ok(api::ApplicationResponse::Json(response)) => Ok(response),
                    Err(e) => Err(e.to_string()),
                    _ => Err("Failed to migrate payment method".to_string()),
                }
            }
            Err(e) => Err(e.to_string()),
        };

        result.push(pm_api::PaymentMethodMigrationResponse::from((res, record)));
    }
    Ok(api::ApplicationResponse::Json(result))
}

#[cfg(feature = "v1")]
pub async fn modular_migrate_payment_methods(
    state: &state::PaymentMethodsState,
    payment_method_ids: Vec<pm_api::PaymentMethodId>,
    merchant_id: &common_utils::id_type::MerchantId,
    platform: &platform::Platform,
    controller: &dyn pm::PaymentMethodsController,
) -> PmMigrationResult<pm_api::ModularPaymentMethodMigrationResponse> {
    let mut successfully_migrated = Vec::new();
    let mut failed_migrations = Vec::new();

    for pm_id_record in payment_method_ids {
        let pm_id = pm_id_record.payment_method_id.clone();

        match migrate_single_payment_method(state, &pm_id, merchant_id, platform, controller).await
        {
            Ok(()) => {
                router_env::logger::info!("Successfully migrated payment method: {}", pm_id);
                successfully_migrated.push(pm_id_record.payment_method_id);
            }
            Err(err) => {
                router_env::logger::error!("Failed to migrate payment method {}: {:?}", pm_id, err);
                let failed_migration = pm_api::FailedMigration {
                    failed_record: pm_id_record.payment_method_id,
                    error_message: err.to_string(),
                };
                failed_migrations.push(failed_migration);
            }
        }
    }

    Ok(api::ApplicationResponse::Json(
        pm_api::ModularPaymentMethodMigrationResponse {
            successfully_migrated,
            failed_migrations,
        },
    ))
}

#[cfg(feature = "v1")]
async fn migrate_single_payment_method(
    state: &state::PaymentMethodsState,
    payment_method_id: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    platform: &platform::Platform,
    controller: &dyn pm::PaymentMethodsController,
) -> errors::PmResult<()> {
    // Step 1: Fetch payment method record
    let db = &*state.store;
    let payment_method = fetch_payment_method_record(state, payment_method_id, platform).await?;

    router_env::logger::info!(
        "Fetched payment method record for ID : {}",
        payment_method_id
    );

    // Step 2: Validate merchant_id
    validate_merchant_id(&payment_method, merchant_id)?;

    let customer_id = payment_method.customer_id.as_ref().ok_or(
        errors::ApiErrorResponse::InvalidRequestData {
            message: "Payment method must have a customer_id".to_string(),
        },
    )?;

    let customer_obj = db
        .find_customer_by_customer_id_merchant_id(
            customer_id,
            merchant_id,
            platform.get_provider().get_key_store(),
            platform.get_provider().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch customer record")?;

    let customer_key = customer_obj.get_global_customer_id();

    let locker_id =
        payment_method
            .locker_id
            .as_ref()
            .ok_or(errors::ApiErrorResponse::InvalidRequestData {
                message: "Payment method must have a locker_id".to_string(),
            })?;

    let fingerprint_id = payment_method.locker_fingerprint_id.as_ref().ok_or(
        errors::ApiErrorResponse::InvalidRequestData {
            message: "Payment method must have a fingerprint".to_string(),
        },
    )?;

    let is_v2_pm = payment_method.version == ApiVersion::V2
        && payment_method.payment_method == Some(enums::PaymentMethod::Card);

    let vault_id = hyperswitch_domain_models::payment_methods::VaultId::generate(locker_id.clone());

    let vaulting_data = controller
        .retrieve_payment_method_from_vault(&vault_id, merchant_id, customer_id, is_v2_pm)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve payment method from vault")?;

    // Step 4: Check if payment method is bank_debit or wallet or card (only for v2)
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
    ) && payment_method.version == ApiVersion::V2;

    common_utils::fp_utils::when(!is_bank_debit && !is_wallet && !is_card, || {
        router_env::logger::info!(
            "Skipping migration for payment method {}: not bank_debit, wallet, or card",
            payment_method_id
        );
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "Payment method {} is not bank_debit, wallet, or card, skipping migration",
                payment_method_id
            ),
        })
    })?;

    // Step 5: Create new record in vault with new entity_id(merchant_id)

    controller
        .store_payment_method_in_vault(&merchant_id.clone(), &vault_id, &vaulting_data)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to store payment method in vault with new entity_id")?;

    // Step 6 & 7: If bank_debit and wallet,, handle fingerprint data migration with new customer key
    if is_bank_debit && is_wallet {
        let fingerprint_data = vaulting_data.to_fingerprint_data();
        controller
            .get_fingerprint_id_from_vault(customer_key, &fingerprint_data, fingerprint_id.clone())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to create fingerprint record in vault")?;
    }

    Ok(())
}

#[cfg(feature = "v1")]
async fn fetch_payment_method_record(
    state: &state::PaymentMethodsState,
    payment_method_id: &str,
    platform: &platform::Platform,
) -> errors::PmResult<hyperswitch_domain_models::payment_methods::PaymentMethod> {
    let key_store = platform.get_provider().get_key_store();
    let merchant_account = platform.get_provider().get_account();

    state
        .find_payment_method(key_store, merchant_account, payment_method_id.to_string())
        .await
        .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
        .attach_printable("Failed to find payment method record")
}

#[cfg(feature = "v1")]
fn validate_merchant_id(
    payment_method: &hyperswitch_domain_models::payment_methods::PaymentMethod,
    expected_merchant_id: &common_utils::id_type::MerchantId,
) -> errors::PmResult<()> {
    common_utils::fp_utils::when(payment_method.merchant_id != *expected_merchant_id, || {
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "Merchant ID mismatch: expected {}, found {}",
                expected_merchant_id.get_string_repr(),
                payment_method.merchant_id.get_string_repr()
            ),
        })
    })?;
    Ok(())
}

#[derive(Debug, form::MultipartForm)]
pub struct PaymentMethodsMigrateForm {
    #[multipart(limit = "1MB")]
    pub file: bytes::Bytes,

    pub merchant_id: text::Text<common_utils::id_type::MerchantId>,

    pub merchant_connector_id:
        Option<text::Text<common_utils::id_type::MerchantConnectorAccountId>>,

    pub merchant_connector_ids: Option<text::Text<String>>,
}

#[derive(Debug, form::MultipartForm)]
pub struct ModularPaymentMethodsMigrateForm {
    #[multipart(limit = "1MB")]
    pub file: bytes::Bytes,

    pub merchant_id: text::Text<common_utils::id_type::MerchantId>,
}

pub struct MerchantConnectorValidator;

impl MerchantConnectorValidator {
    pub fn parse_comma_separated_ids(
        ids_string: &str,
    ) -> Result<Vec<common_utils::id_type::MerchantConnectorAccountId>, errors::ApiErrorResponse>
    {
        // Estimate capacity based on comma count
        let capacity = ids_string.matches(',').count() + 1;
        let mut result = Vec::with_capacity(capacity);

        for id in ids_string.split(',') {
            let trimmed_id = id.trim();
            if !trimmed_id.is_empty() {
                let mca_id =
                    common_utils::id_type::MerchantConnectorAccountId::wrap(trimmed_id.to_string())
                        .map_err(|_| errors::ApiErrorResponse::InvalidRequestData {
                            message: format!("Invalid merchant_connector_account_id: {trimmed_id}"),
                        })?;
                result.push(mca_id);
            }
        }

        Ok(result)
    }

    fn validate_form_csv_conflicts(
        records: &[pm_api::PaymentMethodRecord],
        form_has_single_id: bool,
        form_has_multiple_ids: bool,
    ) -> Result<(), errors::ApiErrorResponse> {
        if form_has_single_id {
            // If form has merchant_connector_id, CSV records should not have merchant_connector_ids
            for (index, record) in records.iter().enumerate() {
                if record.merchant_connector_ids.is_some() {
                    return Err(errors::ApiErrorResponse::InvalidRequestData {
                        message: format!(
                            "Record at line {} has merchant_connector_ids but form has merchant_connector_id. Only one should be provided",
                            index + 1
                        ),
                    });
                }
            }
        }

        if form_has_multiple_ids {
            // If form has merchant_connector_ids, CSV records should not have merchant_connector_id
            for (index, record) in records.iter().enumerate() {
                if record.merchant_connector_id.is_some() {
                    return Err(errors::ApiErrorResponse::InvalidRequestData {
                        message: format!(
                            "Record at line {} has merchant_connector_id but form has merchant_connector_ids. Only one should be provided",
                            index + 1
                        ),
                    });
                }
            }
        }

        Ok(())
    }
}

type MigrationValidationResult = Result<
    (
        common_utils::id_type::MerchantId,
        Vec<pm_api::PaymentMethodRecord>,
        Option<Vec<common_utils::id_type::MerchantConnectorAccountId>>,
    ),
    errors::ApiErrorResponse,
>;

impl PaymentMethodsMigrateForm {
    pub fn validate_and_get_payment_method_records(self) -> MigrationValidationResult {
        // Step 1: Validate form-level conflicts
        let form_has_single_id = self.merchant_connector_id.is_some();
        let form_has_multiple_ids = self.merchant_connector_ids.is_some();

        if form_has_single_id && form_has_multiple_ids {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "Both merchant_connector_id and merchant_connector_ids cannot be provided"
                    .to_string(),
            });
        }

        // Ensure at least one is provided
        if !form_has_single_id && !form_has_multiple_ids {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "Either merchant_connector_id or merchant_connector_ids must be provided"
                    .to_string(),
            });
        }

        // Step 2: Parse CSV
        let records = parse_csv(self.file.data.to_bytes()).map_err(|e| {
            errors::ApiErrorResponse::PreconditionFailed {
                message: e.to_string(),
            }
        })?;

        // Step 3: Validate CSV vs Form conflicts
        MerchantConnectorValidator::validate_form_csv_conflicts(
            &records,
            form_has_single_id,
            form_has_multiple_ids,
        )?;

        // Step 4: Prepare the merchant connector account IDs for return
        let mca_ids = if let Some(ref single_id) = self.merchant_connector_id {
            Some(vec![(**single_id).clone()])
        } else if let Some(ref ids_string) = self.merchant_connector_ids {
            let parsed_ids = MerchantConnectorValidator::parse_comma_separated_ids(ids_string)?;
            if parsed_ids.is_empty() {
                None
            } else {
                Some(parsed_ids)
            }
        } else {
            None
        };

        // Step 5: Return the updated structure
        Ok((self.merchant_id.clone(), records, mca_ids))
    }
}

type ModularMigrationValidationResult = Result<
    (
        common_utils::id_type::MerchantId,
        Vec<pm_api::PaymentMethodId>,
    ),
    errors::ApiErrorResponse,
>;

impl ModularPaymentMethodsMigrateForm {
    pub fn get_payment_method_ids(self) -> ModularMigrationValidationResult {
        let records = parse_csv_new(self.file.data.to_bytes()).map_err(|e| {
            errors::ApiErrorResponse::PreconditionFailed {
                message: e.to_string(),
            }
        })?;

        Ok((self.merchant_id.clone(), records))
    }
}

fn parse_csv(data: &[u8]) -> csv::Result<Vec<pm_api::PaymentMethodRecord>> {
    let mut csv_reader = Reader::from_reader(data);
    let mut records = Vec::new();
    let mut id_counter = 0;
    for result in csv_reader.deserialize() {
        let mut record: pm_api::PaymentMethodRecord = result?;
        id_counter += 1;
        record.line_number = Some(id_counter);
        records.push(record);
    }
    Ok(records)
}

fn parse_csv_new(data: &[u8]) -> csv::Result<Vec<pm_api::PaymentMethodId>> {
    let mut csv_reader = Reader::from_reader(data);
    let mut records = Vec::new();
    for result in csv_reader.deserialize() {
        let record: pm_api::PaymentMethodId = result?;
        records.push(record);
    }
    Ok(records)
}

#[instrument(skip_all)]
pub fn validate_card_expiry(
    card_exp_month: &hyperswitch_masking::Secret<String>,
    card_exp_year: &hyperswitch_masking::Secret<String>,
) -> errors::CustomResult<(), errors::ApiErrorResponse> {
    let exp_month = card_exp_month
        .peek()
        .to_string()
        .parse::<u8>()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "card_exp_month",
        })?;
    ::cards::CardExpirationMonth::try_from(exp_month).change_context(
        errors::ApiErrorResponse::PreconditionFailed {
            message: "Invalid Expiry Month".to_string(),
        },
    )?;

    let year_str = card_exp_year.peek().to_string();

    validate_card_exp_year(year_str).change_context(
        errors::ApiErrorResponse::PreconditionFailed {
            message: "Invalid Expiry Year".to_string(),
        },
    )?;

    Ok(())
}

fn validate_card_exp_year(year: String) -> Result<(), errors::ValidationError> {
    let year_str = year.to_string();
    if year_str.len() == 2 || year_str.len() == 4 {
        year_str
            .parse::<u16>()
            .map_err(|_| errors::ValidationError::InvalidValue {
                message: "card_exp_year".to_string(),
            })?;
        Ok(())
    } else {
        Err(errors::ValidationError::InvalidValue {
            message: "invalid card expiration year".to_string(),
        })
    }
}

#[derive(Debug)]
pub struct RecordMigrationStatus {
    pub card_migrated: Option<bool>,
    pub payment_method_migrated: Option<bool>,
    pub network_token_migrated: Option<bool>,
    pub connector_mandate_details_migrated: Option<bool>,
    pub network_transaction_migrated: Option<bool>,
}

#[derive(Debug)]
pub struct RecordMigrationStatusBuilder {
    pub card_migrated: Option<bool>,
    pub payment_method_migrated: Option<bool>,
    pub network_token_migrated: Option<bool>,
    pub connector_mandate_details_migrated: Option<bool>,
    pub network_transaction_migrated: Option<bool>,
}

impl RecordMigrationStatusBuilder {
    pub fn new() -> Self {
        Self {
            card_migrated: None,
            payment_method_migrated: None,
            network_token_migrated: None,
            connector_mandate_details_migrated: None,
            network_transaction_migrated: None,
        }
    }

    pub fn card_migrated(&mut self, card_migrated: bool) {
        self.card_migrated = Some(card_migrated);
    }

    pub fn payment_method_migrated(&mut self, payment_method_migrated: bool) {
        self.payment_method_migrated = Some(payment_method_migrated);
    }

    pub fn network_token_migrated(&mut self, network_token_migrated: Option<bool>) {
        self.network_token_migrated = network_token_migrated;
    }

    pub fn connector_mandate_details_migrated(
        &mut self,
        connector_mandate_details_migrated: Option<bool>,
    ) {
        self.connector_mandate_details_migrated = connector_mandate_details_migrated;
    }

    pub fn network_transaction_id_migrated(&mut self, network_transaction_migrated: Option<bool>) {
        self.network_transaction_migrated = network_transaction_migrated;
    }

    pub fn build(self) -> RecordMigrationStatus {
        RecordMigrationStatus {
            card_migrated: self.card_migrated,
            payment_method_migrated: self.payment_method_migrated,
            network_token_migrated: self.network_token_migrated,
            connector_mandate_details_migrated: self.connector_mandate_details_migrated,
            network_transaction_migrated: self.network_transaction_migrated,
        }
    }
}

impl Default for RecordMigrationStatusBuilder {
    fn default() -> Self {
        Self::new()
    }
}
