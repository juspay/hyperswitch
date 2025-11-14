#[cfg(feature = "v1")]
use common_utils::errors::CustomResult;
use common_utils::types::keymanager;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::merchant_account;
use hyperswitch_domain_models::{
    cards_info, customer, merchant_key_store, payment_methods as pm_domain,
};
use storage_impl::{errors, kv_router_store::KVRouterStore, DatabaseStore, MockDb, RouterStore};

#[async_trait::async_trait]
pub trait PaymentMethodsStorageInterface:
    Send
    + Sync
    + dyn_clone::DynClone
    + pm_domain::PaymentMethodInterface<Error = errors::StorageError>
    + cards_info::CardsInfoInterface<Error = errors::StorageError>
    + customer::CustomerInterface<Error = errors::StorageError>
    + 'static
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
    pub key_store: Option<merchant_key_store::MerchantKeyStore>,
    pub key_manager_state: keymanager::KeyManagerState,
}
impl From<&PaymentMethodsState> for keymanager::KeyManagerState {
    fn from(state: &PaymentMethodsState) -> Self {
        state.key_manager_state.clone()
    }
}
#[cfg(feature = "v1")]
impl PaymentMethodsState {
    pub async fn find_payment_method(
        &self,
        key_store: &merchant_key_store::MerchantKeyStore,
        merchant_account: &merchant_account::MerchantAccount,
        payment_method_id: String,
    ) -> CustomResult<pm_domain::PaymentMethod, errors::StorageError> {
        let db = &*self.store;

        match db
            .find_payment_method(
                key_store,
                &payment_method_id,
                merchant_account.storage_scheme,
            )
            .await
        {
            Err(err) if err.current_context().is_db_not_found() => {
                db.find_payment_method_by_locker_id(
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
