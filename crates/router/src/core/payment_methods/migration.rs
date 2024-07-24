use actix_multipart::form::{bytes::Bytes, MultipartForm};
use api_models::payment_methods::{PaymentMethodMigrationResponse, PaymentMethodRecord};
use csv::Reader;
use rdkafka::message::ToBytes;

use crate::{
    core::{errors, payment_methods::cards::migrate_payment_method},
    routes, services,
    types::{api, domain},
};

pub async fn migrate_payment_methods(
    state: routes::SessionState,
    payment_methods: Vec<PaymentMethodRecord>,
    merchant_id: &common_utils::id_type::MerchantId,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<Vec<PaymentMethodMigrationResponse>> {
    let mut result = Vec::new();
    for record in payment_methods {
        let res = migrate_payment_method(
            state.clone(),
            api::PaymentMethodMigrate::from(record.clone()),
            merchant_id,
            merchant_account,
            key_store,
        )
        .await;
        result.push(PaymentMethodMigrationResponse::from((
            match res {
                Ok(services::api::ApplicationResponse::Json(response)) => Ok(response),
                Err(e) => Err(e.to_string()),
                _ => Err("Failed to migrate payment method".to_string()),
            },
            record,
        )));
    }
    Ok(services::api::ApplicationResponse::Json(result))
}

#[derive(Debug, MultipartForm)]
pub struct PaymentMethodsMigrateForm {
    #[multipart(limit = "1MB")]
    pub file: Bytes,
}

fn parse_csv(data: &[u8]) -> csv::Result<Vec<PaymentMethodRecord>> {
    let mut csv_reader = Reader::from_reader(data);
    let mut records = Vec::new();
    let mut id_counter = 0;
    for result in csv_reader.deserialize() {
        let mut record: PaymentMethodRecord = result?;
        id_counter += 1;
        record.line_number = Some(id_counter);
        records.push(record);
    }
    Ok(records)
}
pub fn get_payment_method_records(
    form: PaymentMethodsMigrateForm,
) -> Result<(common_utils::id_type::MerchantId, Vec<PaymentMethodRecord>), errors::ApiErrorResponse>
{
    match parse_csv(form.file.data.to_bytes()) {
        Ok(records) => {
            if let Some(first_record) = records.first() {
                if records
                    .iter()
                    .all(|merchant_id| merchant_id.merchant_id == first_record.merchant_id)
                {
                    Ok((first_record.merchant_id.clone(), records))
                } else {
                    Err(errors::ApiErrorResponse::PreconditionFailed {
                        message: "Only one merchant id can be updated at a time".to_string(),
                    })
                }
            } else {
                Err(errors::ApiErrorResponse::PreconditionFailed {
                    message: "No records found".to_string(),
                })
            }
        }
        Err(e) => Err(errors::ApiErrorResponse::PreconditionFailed {
            message: e.to_string(),
        }),
    }
}
