use api_models::apple_pay_certificates_migration;
use common_utils::{errors::CustomResult, type_name, types::keymanager::Identifier};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};

use super::{
    errors::{self, StorageErrorExt},
    payments::helpers,
};
use crate::{
    routes::SessionState,
    services::{self, logger},
    types::{domain::types as domain_types, storage},
};

#[cfg(feature = "v1")]
pub async fn apple_pay_certificates_migration(
    state: SessionState,
    req: &apple_pay_certificates_migration::ApplePayCertificatesMigrationRequest,
) -> CustomResult<
    services::ApplicationResponse<
        apple_pay_certificates_migration::ApplePayCertificatesMigrationResponse,
    >,
    errors::ApiErrorResponse,
> {
    let db = state.store.as_ref();

    let merchant_id_list = &req.merchant_ids;

    let mut migration_successful_merchant_ids = vec![];
    let mut migration_failed_merchant_ids = vec![];
    for merchant_id in merchant_id_list {
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

        let mut mca_to_update = vec![];

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
                let encrypted_apple_pay_metadata = domain_types::crypto_operation(
                    &(&state).into(),
                    type_name!(storage::MerchantConnectorAccount),
                    domain_types::CryptoOperation::Encrypt(Secret::new(
                        serde_json::to_value(apple_pay_metadata)
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to serialize apple pay metadata as JSON")?,
                    )),
                    Identifier::Merchant(merchant_id.clone()),
                    key_store.key.get_inner().peek(),
                )
                .await
                .and_then(|val| val.try_into_operation())
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to encrypt connector apple pay metadata")?;

                let updated_mca =
                    storage::MerchantConnectorAccountUpdate::ConnectorWalletDetailsUpdate {
                        connector_wallets_details: encrypted_apple_pay_metadata,
                    };

                mca_to_update.push((connector_account, updated_mca.into()));
            }
        }

        let merchant_connector_accounts_update = db
            .update_multiple_merchant_connector_accounts(mca_to_update)
            .await;

        match merchant_connector_accounts_update {
            Ok(_) => {
                logger::debug!(
                    "Merchant connector accounts updated for merchant id {merchant_id:?}"
                );
                migration_successful_merchant_ids.push(merchant_id.clone());
            }
            Err(error) => {
                logger::debug!(
                    "Merchant connector accounts update failed with error {error} for merchant id {merchant_id:?}");
                migration_failed_merchant_ids.push(merchant_id.clone());
            }
        };
    }

    Ok(services::api::ApplicationResponse::Json(
        apple_pay_certificates_migration::ApplePayCertificatesMigrationResponse {
            migration_successful: migration_successful_merchant_ids,
            migration_failed: migration_failed_merchant_ids,
        },
    ))
}
