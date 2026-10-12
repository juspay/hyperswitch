use actix_multipart::form::{self, bytes, text};
use api_models::payment_methods as pm_api;
use common_utils::id_type;
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
    merchant_id: &id_type::MerchantId,
    platform: &platform::Platform,
    mca_ids: Option<Vec<id_type::MerchantConnectorAccountId>>,
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

#[derive(Debug, form::MultipartForm)]
pub struct PaymentMethodsMigrateForm {
    #[multipart(limit = "1MB")]
    pub file: bytes::Bytes,

    pub merchant_id: text::Text<id_type::MerchantId>,

    pub merchant_connector_id: Option<text::Text<id_type::MerchantConnectorAccountId>>,

    pub merchant_connector_ids: Option<text::Text<String>>,

    pub use_modular_customer_creation: Option<text::Text<bool>>,

    pub customer_api_key: Option<text::Text<hyperswitch_masking::StrongSecret<String>>>,
}

#[derive(Debug)]
pub enum CustomerMigrationOptions {
    Legacy,
    Modular {
        api_key: hyperswitch_masking::StrongSecret<String>,
    },
}

impl PaymentMethodsMigrateForm {
    pub fn validate_modular_csv_headers(&self) -> Result<(), errors::ApiErrorResponse> {
        validate_modular_csv_headers(self.file.data.to_bytes())
    }

    pub fn take_customer_migration_options(
        &mut self,
    ) -> Result<CustomerMigrationOptions, errors::ApiErrorResponse> {
        if !self
            .use_modular_customer_creation
            .as_ref()
            .is_some_and(|value| **value)
        {
            return Ok(CustomerMigrationOptions::Legacy);
        }
        let api_key = self
            .customer_api_key
            .take()
            .map(|value| value.0)
            .filter(|value| !value.peek().trim().is_empty())
            .ok_or_else(|| errors::ApiErrorResponse::InvalidRequestData {
                message: "customer_api_key is required for modular customer migration".to_string(),
            })?;
        Ok(CustomerMigrationOptions::Modular { api_key })
    }
}

fn validate_modular_csv_headers(data: &[u8]) -> Result<(), errors::ApiErrorResponse> {
    let invalid = || errors::ApiErrorResponse::InvalidRequestData {
        message: "migration_csv_headers_invalid: unknown or duplicate columns".to_string(),
    };
    // Derive the allow-list from the existing record, without modifying legacy CSV parsing.
    let record: pm_api::PaymentMethodRecord = serde_json::from_value(serde_json::json!({
        "customer_id": "migration_header_check"
    }))
    .map_err(|_| invalid())?;
    let fields = serde_json::to_value(record).map_err(|_| invalid())?;
    let fields = fields.as_object().ok_or_else(invalid)?;
    let mut reader = Reader::from_reader(data);
    let headers = reader.headers().map_err(|_| invalid())?;
    let mut seen = std::collections::HashSet::new();
    if headers
        .iter()
        .any(|header| !fields.contains_key(header) || !seen.insert(header))
    {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(all(test, feature = "v1"))]
mod customer_options_tests {
    use actix_web::{http::header, test, web, App, HttpResponse};

    use super::*;

    #[actix_web::test]
    async fn unknown_and_duplicate_modular_columns_are_rejected() {
        assert!(validate_modular_csv_headers(
            b"customer_id,raw_card_number\ncus_one,4242424242424242\n"
        )
        .is_ok());
        assert!(validate_modular_csv_headers(b"customer_id,card_cvc\ncus_one,123\n").is_err());
        assert!(
            validate_modular_csv_headers(b"customer_id,customer_id\ncus_one,cus_one\n").is_err()
        );
    }

    async fn inspect(
        form::MultipartForm(mut form): form::MultipartForm<PaymentMethodsMigrateForm>,
    ) -> HttpResponse {
        match form.take_customer_migration_options() {
            Ok(CustomerMigrationOptions::Legacy) => HttpResponse::Ok().body("legacy"),
            Ok(options @ CustomerMigrationOptions::Modular { .. }) => {
                HttpResponse::Ok().body(format!("{options:?}"))
            }
            Err(_) => HttpResponse::BadRequest().finish(),
        }
    }

    fn multipart(flag: Option<&str>, key: Option<&str>) -> String {
        let mut body = String::new();
        for (name, value) in [
            ("merchant_id", Some("merchant_test")),
            ("merchant_connector_ids", Some("mca_test")),
            ("use_modular_customer_creation", flag),
            ("customer_api_key", key),
        ] {
            if let Some(value) = value {
                body.push_str(&format!("--test-boundary\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"));
            }
        }
        body.push_str("--test-boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"cards.csv\"\r\nContent-Type: text/csv\r\n\r\ncustomer_id\ncustomer_one\n\r\n--test-boundary--\r\n");
        body
    }

    #[actix_web::test]
    async fn multipart_dispatch_preserves_legacy_and_masks_modular_key() {
        let app = test::init_service(App::new().route("/", web::post().to(inspect))).await;
        for (flag, key) in [
            (None, None),
            (Some("false"), None),
            (None, Some("unused_secret")),
            (Some("false"), Some("unused_secret")),
        ] {
            let request = test::TestRequest::post()
                .uri("/")
                .insert_header((
                    header::CONTENT_TYPE,
                    "multipart/form-data; boundary=test-boundary",
                ))
                .set_payload(multipart(flag, key))
                .to_request();
            let response = test::call_service(&app, request).await;
            assert!(response.status().is_success());
            assert_eq!(test::read_body(response).await, "legacy");
        }
        let request = test::TestRequest::post()
            .uri("/")
            .insert_header((
                header::CONTENT_TYPE,
                "multipart/form-data; boundary=test-boundary",
            ))
            .set_payload(multipart(Some("true"), Some("synthetic_customer_secret")))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert!(response.status().is_success());
        let body = test::read_body(response).await;
        let text = std::str::from_utf8(&body).expect("UTF8");
        assert!(text.contains("Modular"));
        assert!(!text.contains("synthetic_customer_secret"));
    }

    #[actix_web::test]
    async fn modular_key_is_required_and_invalid_flag_is_rejected() {
        let app = test::init_service(App::new().route("/", web::post().to(inspect))).await;
        for (flag, key) in [
            ("true", None),
            ("true", Some("  ")),
            ("invalid", Some("synthetic_customer_secret")),
        ] {
            let request = test::TestRequest::post()
                .uri("/")
                .insert_header((
                    header::CONTENT_TYPE,
                    "multipart/form-data; boundary=test-boundary",
                ))
                .set_payload(multipart(Some(flag), key))
                .to_request();
            assert_eq!(
                test::call_service(&app, request).await.status(),
                actix_web::http::StatusCode::BAD_REQUEST
            );
        }
    }
}

pub struct MerchantConnectorValidator;

impl MerchantConnectorValidator {
    pub fn parse_comma_separated_ids(
        ids_string: &str,
    ) -> Result<Vec<id_type::MerchantConnectorAccountId>, errors::ApiErrorResponse> {
        // Estimate capacity based on comma count
        let capacity = ids_string.matches(',').count() + 1;
        let mut result = Vec::with_capacity(capacity);

        for id in ids_string.split(',') {
            let trimmed_id = id.trim();
            if !trimmed_id.is_empty() {
                let mca_id = id_type::MerchantConnectorAccountId::wrap(trimmed_id.to_string())
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
        id_type::MerchantId,
        Vec<pm_api::PaymentMethodRecord>,
        Option<Vec<id_type::MerchantConnectorAccountId>>,
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
            field_name: "card_exp_month".into(),
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
