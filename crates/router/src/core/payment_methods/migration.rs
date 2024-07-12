use std::collections::HashMap;

use actix_multipart::form::{bytes::Bytes, text::Text, MultipartForm};
use api_models::payment_methods::{
    MigrateCardDetail, PaymentsMandateReference, PaymentsMandateReferenceRecord,
};
use common_utils::id_type;
use csv::Reader;
use masking::Secret;
use rdkafka::message::ToBytes;

use crate::{
    core::{errors, payment_methods::cards::migrate_payment_method},
    routes, services,
    types::{api, api::routing::api_enums, domain},
};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PaymentMethodRecord {
    pub customer_id: common_utils::id_type::CustomerId,
    pub name: Option<Secret<String>>,
    pub email: Option<common_utils::pii::Email>,
    pub phone: Option<Secret<String>>,
    pub phone_country_code: Option<String>,
    pub merchant_id: String,
    pub payment_method: Option<api_enums::PaymentMethod>,
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
    pub nick_name: Secret<String>,
    pub payment_instrument_id: String,
    pub card_number_masked: Secret<String>,
    pub card_expiry_month: Secret<String>,
    pub card_expiry_year: Secret<String>,
    pub card_scheme: Option<String>,
    pub original_transaction_id: String,
    pub billing_address_zip: Secret<String>,
    pub billing_address_state: Secret<String>,
    pub billing_address_first_name: Secret<String>,
    pub billing_address_last_name: Secret<String>,
    pub billing_address_city: String,
    pub billing_address_country: Option<api_enums::CountryAlpha2>,
    pub billing_address_line1: Secret<String>,
    pub billing_address_line2: Option<Secret<String>>,
    pub billing_address_line3: Option<Secret<String>>,
    pub raw_card_number: Option<Secret<String>>,
    pub merchant_connector_id: String,
    pub original_transaction_amount: Option<i64>,
    pub original_transaction_currency: Option<common_enums::Currency>,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct PaymentMethodMigrationResponse {
    pub payment_method_id: Option<String>,
    pub payment_method: Option<api_enums::PaymentMethod>,
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
    pub customer_id: Option<id_type::CustomerId>,
    pub migration_status: MigrationStatus,
    pub migration_error: Option<String>,
    pub card_number_masked: Option<Secret<String>>,
}

#[derive(Debug, Default, serde::Serialize)]
pub enum MigrationStatus {
    Success,
    #[default]
    Failed,
}

// errors::RouterResponse<api::PaymentMethodResponse>
type PaymentMethodMigrationResponseType = (
    errors::RouterResponse<api::PaymentMethodResponse>,
    PaymentMethodRecord,
);
impl From<PaymentMethodMigrationResponseType> for PaymentMethodMigrationResponse {
    fn from(
        (response, record): PaymentMethodMigrationResponseType,
    ) -> PaymentMethodMigrationResponse {
        match response {
            Ok(services::api::ApplicationResponse::Json(res)) => PaymentMethodMigrationResponse {
                payment_method_id: Some(res.payment_method_id),
                payment_method: res.payment_method,
                payment_method_type: res.payment_method_type,
                customer_id: res.customer_id,
                migration_status: MigrationStatus::Success,
                migration_error: None,
                card_number_masked: Some(record.card_number_masked),
            },
            Err(e) => PaymentMethodMigrationResponse {
                customer_id: Some(record.customer_id),
                migration_status: MigrationStatus::Failed,
                migration_error: Some(e.to_string()),
                card_number_masked: Some(record.card_number_masked),
                ..PaymentMethodMigrationResponse::default()
            },
            _ => PaymentMethodMigrationResponse {
                customer_id: Some(record.customer_id),
                migration_status: MigrationStatus::Failed,
                migration_error: Some("Failed to migrate payment method".to_string()),
                card_number_masked: Some(record.card_number_masked),
                ..PaymentMethodMigrationResponse::default()
            },
        }
    }
}

impl From<PaymentMethodRecord> for api::PaymentMethodMigrate {
    fn from(record: PaymentMethodRecord) -> api::PaymentMethodMigrate {
        let mut mandate_reference = HashMap::new();
        mandate_reference.insert(
            record.merchant_connector_id,
            PaymentsMandateReferenceRecord {
                connector_mandate_id: record.payment_instrument_id,
                payment_method_type: record.payment_method_type,
                original_payment_authorized_amount: record.original_transaction_amount,
                original_payment_authorized_currency: record.original_transaction_currency,
            },
        );
        api::PaymentMethodMigrate {
            merchant_id: record.merchant_id,
            customer_id: Some(record.customer_id),
            card: Some(MigrateCardDetail {
                card_number: record.raw_card_number.unwrap_or(record.card_number_masked),
                card_exp_month: record.card_expiry_month,
                card_exp_year: record.card_expiry_year,
                card_holder_name: record.name,
                card_network: None,
                card_type: None,
                card_issuer: None,
                card_issuing_country: None,
                nick_name: Some(record.nick_name),
            }),
            payment_method: record.payment_method.into(),
            payment_method_type: record.payment_method_type.into(),
            payment_method_issuer: None,
            billing: Some(api::Address {
                address: Some(api::AddressDetails {
                    city: Some(record.billing_address_city),
                    country: record.billing_address_country,
                    line1: Some(record.billing_address_line1),
                    line2: record.billing_address_line2,
                    state: Some(record.billing_address_state),
                    line3: record.billing_address_line3,
                    zip: Some(record.billing_address_zip),
                    first_name: Some(record.billing_address_first_name),
                    last_name: Some(record.billing_address_last_name),
                }),
                phone: Some(api::PhoneDetails {
                    number: record.phone,
                    country_code: record.phone_country_code,
                }),
                email: record.email.into(),
            }),
            connector_mandate_details: Some(PaymentsMandateReference(mandate_reference)),
            metadata: None,
            payment_method_issuer_code: None,
            card_network: None,
            bank_transfer: None,
            wallet: None,
            payment_method_data: None,
            network_transaction_id: record.original_transaction_id.into(),
        }
    }
}

impl From<PaymentMethodRecord> for api::CustomerRequest {
    fn from(record: PaymentMethodRecord) -> api::CustomerRequest {
        api::CustomerRequest {
            customer_id: Some(record.customer_id),
            merchant_id: record.merchant_id,
            name: record.name,
            email: record.email,
            phone: record.phone,
            description: None,
            phone_country_code: record.phone_country_code,
            address: Some(api::AddressDetails {
                city: Some(record.billing_address_city),
                country: record.billing_address_country,
                line1: Some(record.billing_address_line1),
                line2: record.billing_address_line2,
                state: Some(record.billing_address_state),
                line3: record.billing_address_line3,
                zip: Some(record.billing_address_zip),
                first_name: Some(record.billing_address_first_name),
                last_name: Some(record.billing_address_last_name),
            }),
            metadata: None,
        }
    }
}

pub async fn migrate_payment_methods(
    state: routes::SessionState,
    payment_methods: Vec<PaymentMethodRecord>,
    merchant_id: &str,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<Vec<PaymentMethodMigrationResponse>> {
    let mut result = Vec::new();
    for record in payment_methods {
        let res = migrate_payment_method(
            state.clone(),
            record.clone().into(),
            merchant_id,
            merchant_account,
            key_store,
        )
        .await;
        result.push((res, record).into());
    }
    return Ok(services::api::ApplicationResponse::Json(result));
}

#[derive(Debug, MultipartForm)]
pub struct PaymentMethodsMigrateForm {
    #[multipart(limit = "1MB")]
    pub file: Bytes,
    pub merchant_id: Text<String>,
    pub merchant_connector_id: Option<Text<String>>,
}

fn parse_csv(data: &[u8]) -> csv::Result<Vec<PaymentMethodRecord>> {
    let mut csv_reader = Reader::from_reader(data);
    let mut records = Vec::new();
    for result in csv_reader.deserialize() {
        let record: PaymentMethodRecord = result?;
        records.push(record);
    }
    Ok(records)
}
pub fn get_payment_method_records(
    form: PaymentMethodsMigrateForm,
) -> Result<Vec<PaymentMethodRecord>, errors::ApiErrorResponse> {
    let merchant_connector_id = form.merchant_connector_id.map(|e| e.clone());
    match parse_csv(form.file.data.to_bytes()) {
        Ok(parsed_records) => Ok(parsed_records
            .iter()
            .map(|r| PaymentMethodRecord {
                merchant_id: form.merchant_id.clone(),
                merchant_connector_id: merchant_connector_id
                    .clone()
                    .unwrap_or(r.merchant_connector_id.clone()),
                ..r.clone()
            })
            .collect()),
        Err(e) => Err(errors::ApiErrorResponse::PreconditionFailed {
            message: e.to_string(),
        }
        .into()),
    }
}
