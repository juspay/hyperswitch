use common_utils::types::keymanager::KeyManagerState;
use diesel_models;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::behaviour::{Conversion, ReverseConversion};
use storage_impl::MockDb;

use super::domain;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::kafka_store::KafkaStore,
    services::Store,
};

#[async_trait::async_trait]
pub trait ThreeDSDecisionRuleInterface {
    async fn insert_three_ds_decision_rule(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        three_ds_decision_rule: hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
    ) -> CustomResult<
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        errors::StorageError,
    >;

    async fn update_three_ds_decision_rule(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        update: hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRuleUpdate,
    ) -> CustomResult<
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        errors::StorageError,
    >;

    async fn find_three_ds_decision_rule_by_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        rule_id: &common_utils::id_type::ThreeDSDecisionRuleId,
    ) -> CustomResult<
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        errors::StorageError,
    >;

    async fn delete_three_ds_decision_rule(
        &self,
        rule_id: &common_utils::id_type::ThreeDSDecisionRuleId,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl ThreeDSDecisionRuleInterface for Store {
    async fn insert_three_ds_decision_rule(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        three_ds_decision_rule: hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
    ) -> CustomResult<
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        errors::StorageError,
    > {
        let conn = connection::pg_connection_write(self).await?;
        three_ds_decision_rule
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn update_three_ds_decision_rule(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        update: hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRuleUpdate,
    ) -> CustomResult<
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        errors::StorageError,
    > {
        let conn = connection::pg_connection_write(self).await?;
        Conversion::convert(current_state)
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .update(
                &conn,
                diesel_models::three_ds_decision_rule::ThreeDSDecisionRuleUpdateInternal::from(
                    diesel_models::three_ds_decision_rule::ThreeDSDecisionRuleUpdate::from(update),
                ),
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_three_ds_decision_rule_by_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        rule_id: &common_utils::id_type::ThreeDSDecisionRuleId,
    ) -> CustomResult<
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        errors::StorageError,
    > {
        let conn = connection::pg_connection_read(self).await?;
        diesel_models::three_ds_decision_rule::ThreeDSDecisionRule::find_by_id(&conn, rule_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn delete_three_ds_decision_rule(
        &self,
        rule_id: &common_utils::id_type::ThreeDSDecisionRuleId,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        match diesel_models::three_ds_decision_rule::ThreeDSDecisionRule::find_by_id(&conn, rule_id)
            .await
        {
            Ok(rule) => rule
                .update(
                    &conn,
                    diesel_models::three_ds_decision_rule::ThreeDSDecisionRuleUpdateInternal {
                        rule: None,
                        name: None,
                        description: None,
                        modified_at: common_utils::date_time::now(),
                        active: Some(false),
                    },
                )
                .await
                .map(|_| true)
                .map_err(|error| report!(errors::StorageError::from(error))),
            Err(error) => Err(report!(errors::StorageError::from(error))),
        }
    }
}

#[async_trait::async_trait]
impl ThreeDSDecisionRuleInterface for MockDb {
    async fn insert_three_ds_decision_rule(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &domain::MerchantKeyStore,
        _three_ds_decision_rule: hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
    ) -> CustomResult<
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        errors::StorageError,
    > {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_three_ds_decision_rule(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &domain::MerchantKeyStore,
        _current_state: hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        _update: hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRuleUpdate,
    ) -> CustomResult<
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        errors::StorageError,
    > {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_three_ds_decision_rule_by_id(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &domain::MerchantKeyStore,
        _rule_id: &common_utils::id_type::ThreeDSDecisionRuleId,
    ) -> CustomResult<
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        errors::StorageError,
    > {
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_three_ds_decision_rule(
        &self,
        _rule_id: &common_utils::id_type::ThreeDSDecisionRuleId,
    ) -> CustomResult<bool, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[async_trait::async_trait]
impl ThreeDSDecisionRuleInterface for KafkaStore {
    async fn insert_three_ds_decision_rule(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        three_ds_decision_rule: hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
    ) -> CustomResult<
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        errors::StorageError,
    > {
        self.diesel_store
            .insert_three_ds_decision_rule(
                key_manager_state,
                merchant_key_store,
                three_ds_decision_rule,
            )
            .await
    }

    async fn update_three_ds_decision_rule(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        update: hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRuleUpdate,
    ) -> CustomResult<
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        errors::StorageError,
    > {
        self.diesel_store
            .update_three_ds_decision_rule(
                key_manager_state,
                merchant_key_store,
                current_state,
                update,
            )
            .await
    }

    async fn find_three_ds_decision_rule_by_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        rule_id: &common_utils::id_type::ThreeDSDecisionRuleId,
    ) -> CustomResult<
        hyperswitch_domain_models::three_ds_decision_rule::ThreeDSDecisionRule,
        errors::StorageError,
    > {
        self.diesel_store
            .find_three_ds_decision_rule_by_id(key_manager_state, merchant_key_store, rule_id)
            .await
    }

    async fn delete_three_ds_decision_rule(
        &self,
        rule_id: &common_utils::id_type::ThreeDSDecisionRuleId,
    ) -> CustomResult<bool, errors::StorageError> {
        self.diesel_store
            .delete_three_ds_decision_rule(rule_id)
            .await
    }
}
