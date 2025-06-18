use actix_multipart::form::{self, bytes, text};
use api_models::payment_methods as pm_api;
use csv::Reader;
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::{api, merchant_context};
use masking::PeekInterface;
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
    merchant_context: &merchant_context::MerchantContext,
    mca_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    controller: &dyn pm::PaymentMethodsController,
) -> PmMigrationResult<Vec<pm_api::PaymentMethodMigrationResponse>> {
    let mut result = Vec::new();
    for record in payment_methods {
        let req = pm_api::PaymentMethodMigrate::try_from((
            record.clone(),
            merchant_id.clone(),
            mca_id.clone(),
        ))
        .map_err(|err| errors::ApiErrorResponse::InvalidRequestData {
            message: format!("error: {:?}", err),
        })
        .attach_printable("record deserialization failed");
        let res = match req {
            Ok(migrate_request) => {
                let res = migrate_payment_method(
                    state,
                    migrate_request,
                    merchant_id,
                    merchant_context,
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

    pub merchant_id: text::Text<common_utils::id_type::MerchantId>,

    pub merchant_connector_id:
        text::Text<Option<common_utils::id_type::MerchantConnectorAccountId>>,
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
pub fn get_payment_method_records(
    form: PaymentMethodsMigrateForm,
) -> Result<
    (
        common_utils::id_type::MerchantId,
        Vec<pm_api::PaymentMethodRecord>,
        Option<common_utils::id_type::MerchantConnectorAccountId>,
    ),
    errors::ApiErrorResponse,
> {
    match parse_csv(form.file.data.to_bytes()) {
        Ok(records) => {
            let merchant_id = form.merchant_id.clone();
            let mca_id = form.merchant_connector_id.clone();
            Ok((merchant_id.clone(), records, mca_id))
        }
        Err(e) => Err(errors::ApiErrorResponse::PreconditionFailed {
            message: e.to_string(),
        }),
    }
}

#[instrument(skip_all)]
pub fn validate_card_expiry(
    card_exp_month: &masking::Secret<String>,
    card_exp_year: &masking::Secret<String>,
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
    pub network_token_migrated: Option<bool>,
    pub connector_mandate_details_migrated: Option<bool>,
    pub network_transaction_migrated: Option<bool>,
}

#[derive(Debug)]
pub struct RecordMigrationStatusBuilder {
    pub card_migrated: Option<bool>,
    pub network_token_migrated: Option<bool>,
    pub connector_mandate_details_migrated: Option<bool>,
    pub network_transaction_migrated: Option<bool>,
}

impl RecordMigrationStatusBuilder {
    pub fn new() -> Self {
        Self {
            card_migrated: None,
            network_token_migrated: None,
            connector_mandate_details_migrated: None,
            network_transaction_migrated: None,
        }
    }

    pub fn card_migrated(&mut self, card_migrated: bool) {
        self.card_migrated = Some(card_migrated);
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
