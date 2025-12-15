use actix_multipart::form::{bytes::Bytes, text::Text, MultipartForm};
use api_models::payment_methods::PaymentMethodsBatchRecord;
use common_utils::id_type;
use router_env::logger;

use crate::core::errors;

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
