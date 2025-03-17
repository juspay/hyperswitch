use std::sync::Arc;

use common_utils::{errors::CustomResult, id_type, types::keymanager::KeyManagerState};
use hyperswitch_domain_models::{
    merchant_account::MerchantAccount, merchant_key_store::MerchantKeyStore,
    payment_methods::PaymentMethod,
};
use hyperswitch_interfaces::secrets_interface::secret_state::RawSecret;
use storage_impl::errors;

use crate::{core::settings, db::StorageInterface};

#[derive(Clone)]
pub struct PaymentMethodsState {
    pub store: Box<dyn StorageInterface>,
    pub conf: Arc<settings::Settings<RawSecret>>,
    pub key_store: Option<MerchantKeyStore>,
    pub base_url: String,
    pub customer_id: Option<id_type::CustomerId>,
    pub merchant_id: Option<id_type::MerchantId>,
    pub limit: Option<i64>,
    pub key_manager_state: KeyManagerState,
}
impl From<&PaymentMethodsState> for KeyManagerState {
    fn from(state: &PaymentMethodsState) -> Self {
        state.key_manager_state.clone()
    }
}
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
impl PaymentMethodsState {
    pub async fn find_payment_method(
        &self,
        key_store: &MerchantKeyStore,
        merchant_account: &MerchantAccount,
        payment_method_id: String,
    ) -> CustomResult<PaymentMethod, errors::StorageError> {
        let db = &*self.store;
        let key_manager_state = &(self.key_manager_state).clone();

        match db
            .find_payment_method(
                key_manager_state,
                key_store,
                &payment_method_id,
                merchant_account.storage_scheme,
            )
            .await
        {
            Err(err) if err.current_context().is_db_not_found() => {
                db.find_payment_method_by_locker_id(
                    key_manager_state,
                    key_store,
                    &payment_method_id,
                    merchant_account.storage_scheme,
                )
                .await
            }
            Ok(pm) => Ok(pm),
            Err(err) => Err(err),
        }
    }
}
