use base64::Engine;
use error_stack::ResultExt;
use hyperswitch_domain_models::merchant_key_store::MerchantKeyStore;
use masking::ExposeInterface;

use common_utils::{
    keymanager::transfer_key_to_key_manager,
    types::keymanager::{EncryptionTransferRequest, Identifier},
};

use crate::{consts::BASE64_ENGINE, errors, SessionState};

pub async fn transfer_encryption_key(
    state: &SessionState,
) -> errors::CustomResult<usize, errors::ApiErrorResponse> {
    let db = &*state.store;
    let key_stores = db
        .get_all_key_stores(&db.get_master_key().to_vec().into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    send_request_to_key_service_for_merchant(state, key_stores).await
}

pub async fn send_request_to_key_service_for_merchant(
    state: &SessionState,
    keys: Vec<MerchantKeyStore>,
) -> errors::CustomResult<usize, errors::ApiErrorResponse> {
    futures::future::try_join_all(keys.into_iter().map(|key| async move {
        let key_encoded = BASE64_ENGINE.encode(key.key.clone().into_inner().expose());
        let req = EncryptionTransferRequest {
            identifier: Identifier::Merchant(key.merchant_id.clone()),
            key: key_encoded,
        };
        transfer_key_to_key_manager(&state.into(), req).await
    }))
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .map(|v| v.len())
}
