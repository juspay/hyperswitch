use common_utils::{errors::CustomResult, id_type, types::keymanager::KeyManagerState};
use hyperswitch_domain_models::{
    errors,
    merchant_account::MerchantAccount,
    merchant_key_store::MerchantKeyStore,
    payment_methods::{PaymentMethod, PaymentMethodInterface},
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
pub struct PaymentMethodsClient {
    pub state: Box<dyn PaymentMethodsStorageInterface>,
    pub key_store: Option<MerchantKeyStore>,
    pub customer_id: Option<id_type::CustomerId>,
    pub merchant_id: Option<id_type::MerchantId>,
    pub limit: Option<i64>,
    pub key_manager_state: KeyManagerState,
}
impl From<&PaymentMethodsClient> for KeyManagerState {
    fn from(state: &PaymentMethodsClient) -> Self {
        state.key_manager_state.clone()
    }
}
impl PaymentMethodsClient {
    pub async fn find_payment_method(
        &self,
        key_store: &MerchantKeyStore,
        merchant_account: &MerchantAccount,
        payment_method_id: String,
    ) -> CustomResult<PaymentMethod, errors::StorageError> {
        let db = &*self.state;
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
