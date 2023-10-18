pub use diesel_models::{
    connector_response::{
        ConnectorResponse, ConnectorResponseNew, ConnectorResponseUpdate,
        ConnectorResponseUpdateInternal,
    },
    enums::MerchantStorageScheme,
};

pub trait ConnectorResponseExt {
    fn make_new_connector_response(
        payment_id: String,
        merchant_id: String,
        attempt_id: String,
        connector: Option<String>,
        storage_scheme: String,
    ) -> ConnectorResponseNew;
}

impl ConnectorResponseExt for ConnectorResponse {
    fn make_new_connector_response(
        payment_id: String,
        merchant_id: String,
        attempt_id: String,
        connector: Option<String>,
        storage_scheme: String,
    ) -> ConnectorResponseNew {
        let now = common_utils::date_time::now();
        ConnectorResponseNew {
            payment_id,
            merchant_id,
            attempt_id,
            created_at: now,
            modified_at: now,
            connector_name: connector,
            connector_transaction_id: None,
            authentication_data: None,
            encoded_data: None,
            updated_by: storage_scheme,
        }
    }
}
