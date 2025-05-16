pub mod helpers;
pub mod transformers;

use crate::SessionState;
use error_stack::ResultExt;

use crate::types::transformers::ForeignTryFrom;
use api_models::enums as api_enums;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use api_models::routing::DynamicRoutingAlgoAccessor;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use common_utils::ext_traits::AsyncExt;
use diesel_models::enums as storage_enums;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use external_services::grpc_client::dynamic_routing::{
    contract_routing_client::ContractBasedDynamicRouting,
    success_rate_client::SuccessBasedDynamicRouting,
};
use hyperswitch_domain_models::{merchant_account, merchant_connector_account, merchant_key_store};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use router_env::logger;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use storage_impl::redis::cache;

#[cfg(feature = "payouts")]
use super::payouts;
#[cfg(feature = "v2")]
use crate::{core::admin, utils::ValueExt};
use common_utils::errors::CustomResult;
pub use hyperswitch_routing::{core_logic::*, errors, state};

pub enum TransactionData<'a> {
    Payment(PaymentsDslInput<'a>),
    #[cfg(feature = "payouts")]
    Payout(&'a payouts::PayoutData),
}

#[derive(Clone)]
pub struct MerchantConnectorHandler<'a> {
    pub state: &'a SessionState,
}

#[derive(Clone)]
pub struct MerchantAccountHandler<'a> {
    pub state: &'a SessionState,
}

#[async_trait::async_trait]
impl state::MerchantConnectorInterface for MerchantConnectorHandler<'_> {
    async fn filter_merchant_connectors(
        &self,
        key_store: &merchant_key_store::MerchantKeyStore,
        transaction_type: &api_enums::TransactionType,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<Vec<api_models::admin::MerchantConnectorResponse>, errors::RoutingError> {
        let mut merchant_connector_accounts = self
            .state
            .store
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                &self.state.into(),
                &key_store.merchant_id,
                false,
                key_store,
            )
            .await
            .change_context(errors::RoutingError::KgraphCacheRefreshFailed)?;
        match transaction_type {
            api_enums::TransactionType::Payment => {
                merchant_connector_accounts.retain(|mca| {
                    mca.connector_type != storage_enums::ConnectorType::PaymentVas
                        && mca.connector_type != storage_enums::ConnectorType::PaymentMethodAuth
                        && mca.connector_type != storage_enums::ConnectorType::PayoutProcessor
                        && mca.connector_type
                            != storage_enums::ConnectorType::AuthenticationProcessor
                });
            }
            #[cfg(feature = "payouts")]
            api_enums::TransactionType::Payout => {
                merchant_connector_accounts.retain(|mca| {
                    mca.connector_type == storage_enums::ConnectorType::PayoutProcessor
                });
            }
        };

        let connector_type = match transaction_type {
            api_enums::TransactionType::Payment => common_enums::ConnectorType::PaymentProcessor,
            #[cfg(feature = "payouts")]
            api_enums::TransactionType::Payout => common_enums::ConnectorType::PayoutProcessor,
        };

        let merchant_connector_accounts = merchant_connector_accounts
            .filter_based_on_profile_and_connector_type(profile_id, connector_type);

        merchant_connector_accounts
            .into_iter()
            .map(api_models::admin::MerchantConnectorResponse::foreign_try_from)
            .collect::<Result<Vec<_>, _>>()
            .change_context(errors::RoutingError::KgraphCacheRefreshFailed)
    }

    async fn get_disabled_merchant_connector_accounts(
        &self,
        key_store: &merchant_key_store::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<merchant_connector_account::MerchantConnectorAccounts, errors::ApiErrorResponse>
    {
        self.state
            .store
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                &self.state.into(),
                merchant_id,
                true,
                key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: merchant_id.get_string_repr().to_owned(),
            })
            .attach_printable("unable to retrieve merchant connectors")
    }
}

#[async_trait::async_trait]
impl state::MerchantAccountInterface for MerchantAccountHandler<'_> {
    async fn update_specific_fields_in_merchant(
        &self,
        key_store: &merchant_key_store::MerchantKeyStore,
        merchant_account_update: merchant_account::MerchantAccountUpdate,
    ) -> CustomResult<merchant_account::MerchantAccount, errors::ApiErrorResponse> {
        self.state
            .store
            .update_specific_fields_in_merchant(
                &self.state.into(),
                &key_store.merchant_id,
                merchant_account_update,
                key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update routing algorithm ref in merchant account")
    }
}
