use std::collections::{HashMap, HashSet};

use actix_multipart::form::{bytes::Bytes, text::Text, MultipartForm};
use api_models::payment_methods::{
    PaymentMethodsBatchRecord, PaymentMethodsBatchRetrieveResponse, PaymentMethodsData,
};
use common_utils::{ext_traits::ValueExt, id_type};
use error_stack::ResultExt;
use masking::ExposeInterface;
use router_env::logger;

use crate::{
    core::errors::{self, RouterResult},
    routes,
    types::domain,
};

#[derive(Debug, MultipartForm)]
pub struct PaymentMethodsBatchRetrieveForm {
    #[multipart(limit = "1MB")]
    pub file: Bytes,
    pub merchant_id: Text<id_type::MerchantId>,
}

pub fn parse_csv(data: &[u8]) -> csv::Result<Vec<PaymentMethodsBatchRecord>> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(data);
    let mut records = Vec::new();
    let mut id_counter = 0;
    for (line_number, result) in csv_reader
        .deserialize::<PaymentMethodsBatchRecord>()
        .enumerate()
    {
        match result {
            Ok(mut record) => {
                id_counter += 1;
                record.line_number = Some(id_counter);
                records.push(record);
            }
            Err(error) => {
                logger::error!("Error parsing line {}: {}", line_number + 1, error);
            }
        }
    }
    Ok(records)
}

pub fn get_payment_method_batch_records(
    form: PaymentMethodsBatchRetrieveForm,
) -> Result<(id_type::MerchantId, Vec<PaymentMethodsBatchRecord>), errors::ApiErrorResponse> {
    match parse_csv(form.file.data.as_ref()) {
        Ok(records) => {
            logger::info!("Parsed a total of {} records", records.len());
            Ok((form.merchant_id.0, records))
        }
        Err(error) => {
            logger::error!("Failed to parse CSV: {:?}", error);
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: error.to_string(),
            })
        }
    }
}

pub async fn retrieve_payment_method_data(
    state: &routes::SessionState,
    merchant_id: &id_type::MerchantId,
    platform: &domain::Platform,
    records: Vec<PaymentMethodsBatchRecord>,
) -> RouterResult<Vec<PaymentMethodsBatchRetrieveResponse>> {
    let storage_scheme = platform.get_provider().get_account().storage_scheme;
    let mut seen_ids = HashSet::new();
    let mut unique_ids = Vec::new();
    for record in &records {
        if seen_ids.insert(record.payment_method_id.clone()) {
            unique_ids.push(record.payment_method_id.clone());
        }
    }

    let payment_methods = if unique_ids.is_empty() {
        Vec::new()
    } else {
        state
            .store
            .find_payment_methods_by_merchant_id_payment_method_ids(
                platform.get_provider().get_key_store(),
                merchant_id,
                &unique_ids,
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?
    };

    let pm_map = payment_methods
        .into_iter()
        .map(|pm| (pm.payment_method_id.clone(), pm))
        .collect::<HashMap<_, _>>();

    let responses = records
        .into_iter()
        .map(|record| {
            if let Some(payment_method) = pm_map.get(&record.payment_method_id) {
                let mut error_message = None;
                let payment_method_data = if let Some(raw_payment_method_data) =
                    payment_method.payment_method_data.clone()
                {
                    let value = raw_payment_method_data.into_inner().expose();
                    match value.parse_value::<PaymentMethodsData>("PaymentMethodsData") {
                        Ok(data) => Some(data),
                        Err(err) => {
                            logger::error!("Failed to deserialize payment_method_data: {:?}", err);
                            error_message =
                                Some("Failed to deserialize payment_method_data".to_string());
                            None
                        }
                    }
                } else {
                    error_message = Some("payment_method_data not found".to_string());
                    None
                };

                PaymentMethodsBatchRetrieveResponse {
                    payment_method_id: record.payment_method_id,
                    payment_method_type: payment_method.get_payment_method_type(),
                    payment_method_subtype: payment_method.get_payment_method_subtype(),
                    payment_method_data,
                    error_message,
                    line_number: record.line_number,
                }
            } else {
                PaymentMethodsBatchRetrieveResponse {
                    payment_method_id: record.payment_method_id,
                    payment_method_type: None,
                    payment_method_subtype: None,
                    payment_method_data: None,
                    error_message: Some("Payment method not found".to_string()),
                    line_number: record.line_number,
                }
            }
        })
        .collect();

    Ok(responses)
}
