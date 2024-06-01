use api_models::apple_pay_certificates_migration::ApplePayCertificatesMigrationResponse;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};

use super::{
    errors::{self, StorageErrorExt},
    payments::helpers,
};
use crate::{
    routes::AppState,
    services::{self, logger},
    types::{domain::types as domain_types, storage},
};

pub async fn apple_pay_certificates_migration(
    state: AppState,
    merchant_id: &str,
) -> CustomResult<
    services::ApplicationResponse<ApplePayCertificatesMigrationResponse>,
    errors::ApiErrorResponse,
> {
    let db = state.store.as_ref();

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let merchant_connector_accounts = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            merchant_id,
            true,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

    for connector_account in merchant_connector_accounts {
        let connector_apple_pay_metadata =
            helpers::get_applepay_metadata(connector_account.clone().metadata)
                .map_err(|error| {
                    logger::error!(
                "Apple pay metadata parsing failed for {:?} in certificates migrations api {:?}",
                connector_account.clone().connector_name,
                error
            )
                })
                .ok();

        if let Some(apple_pay_metadata) = connector_apple_pay_metadata {
            let encrypted_apple_pay_metadata = domain_types::encrypt(
                Secret::new(
                    serde_json::to_value(apple_pay_metadata)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to serialize apple pay metadata as JSON")?,
                ),
                key_store.key.get_inner().peek(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt connector apple pay metadata")?;

            let updated_mca =
                storage::MerchantConnectorAccountUpdate::ConnectorWalletDeatilsUpdate {
                    connector_wallets_details: Some(encrypted_apple_pay_metadata),
                };
            db.update_merchant_connector_account(
                connector_account.clone(),
                updated_mca.into(),
                &key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!(
                    "Failed while updating MerchantConnectorAccount: id: {:?}",
                    connector_account.merchant_connector_id
                )
            })?;
        }
    }

    Ok(services::api::ApplicationResponse::Json(
        ApplePayCertificatesMigrationResponse {
            status_code: "200".to_string(),
            status_message: "Apple pay certificate migration completed".to_string(),
        },
    ))
}
