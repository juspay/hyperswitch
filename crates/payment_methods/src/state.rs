#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use common_utils::errors::CustomResult;
use common_utils::types::keymanager::KeyManagerState;
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use hyperswitch_domain_models::{
    errors, merchant_account::MerchantAccount, payment_methods::PaymentMethod,
};
use hyperswitch_domain_models::{
    merchant_key_store::MerchantKeyStore, payment_methods::PaymentMethodInterface,
};
use storage_impl::{kv_router_store::KVRouterStore, DatabaseStore, MockDb, RouterStore};

#[async_trait::async_trait]
pub trait PaymentMethodsStorageInterface:
    Send + Sync + dyn_clone::DynClone + PaymentMethodInterface + 'static
{
}
dyn_clone::clone_trait_object!(PaymentMethodsStorageInterface);

#[async_trait::async_trait]
impl PaymentMethodsStorageInterface for MockDb {}

#[async_trait::async_trait]
impl<T: DatabaseStore + 'static> PaymentMethodsStorageInterface for RouterStore<T> {}

#[async_trait::async_trait]
impl<T: DatabaseStore + 'static> PaymentMethodsStorageInterface for KVRouterStore<T> {}

#[derive(Clone)]
pub struct PaymentMethodsState {
    pub store: Box<dyn PaymentMethodsStorageInterface>,
    pub key_store: Option<MerchantKeyStore>,
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
