use actix_multipart::form::{bytes::Bytes, text::Text, MultipartForm};
use api_models::payment_methods::{PaymentMethodMigrationResponse, PaymentMethodRecord};
use csv::Reader;
use error_stack::ResultExt;
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
    mca_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
) -> errors::RouterResponse<Vec<PaymentMethodMigrationResponse>> {
    let mut result = Vec::new();
    for record in payment_methods {
        let req = api::PaymentMethodMigrate::try_from((
            record.clone(),
            merchant_id.clone(),
            mca_id.clone(),
        ))
        .map_err(|err| errors::ApiErrorResponse::InvalidRequestData {
            message: format!("error: {:?}", err),
        })
        .attach_printable("record deserialization failed");
        match req {
            Ok(_) => (),
            Err(e) => {
                result.push(PaymentMethodMigrationResponse::from((
                    Err(e.to_string()),
                    record,
                )));
                continue;
            }
        };
        let res = migrate_payment_method(
            state.clone(),
            req?,
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

    pub merchant_id: Text<common_utils::id_type::MerchantId>,

    pub merchant_connector_id: Text<Option<common_utils::id_type::MerchantConnectorAccountId>>,
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
) -> Result<
    (
        common_utils::id_type::MerchantId,
        Vec<PaymentMethodRecord>,
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
