use diesel_models::hyperswitch_ai_interaction as storage;
use error_stack::report;
use router_env::{instrument, tracing};

use super::MockDb;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};

#[async_trait::async_trait]
pub trait HyperswitchAiInteractionInterface {
    async fn insert_hyperswitch_ai_interaction(
        &self,
        hyperswitch_ai_interaction: storage::HyperswitchAiInteractionNew,
    ) -> CustomResult<storage::HyperswitchAiInteraction, errors::StorageError>;

    async fn list_hyperswitch_ai_interactions(
        &self,
        merchant_id: Option<common_utils::id_type::MerchantId>,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::HyperswitchAiInteraction>, errors::StorageError>;
}

#[async_trait::async_trait]
impl HyperswitchAiInteractionInterface for Store {
    #[instrument(skip_all)]
    async fn insert_hyperswitch_ai_interaction(
        &self,
        hyperswitch_ai_interaction: storage::HyperswitchAiInteractionNew,
    ) -> CustomResult<storage::HyperswitchAiInteraction, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        hyperswitch_ai_interaction
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_hyperswitch_ai_interactions(
        &self,
        merchant_id: Option<common_utils::id_type::MerchantId>,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::HyperswitchAiInteraction>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::HyperswitchAiInteraction::filter_by_optional_merchant_id(
            &conn,
            merchant_id.as_ref(),
            limit,
            offset,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl HyperswitchAiInteractionInterface for MockDb {
    async fn insert_hyperswitch_ai_interaction(
        &self,
        hyperswitch_ai_interaction: storage::HyperswitchAiInteractionNew,
    ) -> CustomResult<storage::HyperswitchAiInteraction, errors::StorageError> {
        let mut hyperswitch_ai_interactions = self.hyperswitch_ai_interactions.lock().await;
        let hyperswitch_ai_interaction = storage::HyperswitchAiInteraction {
            id: hyperswitch_ai_interaction.id,
            session_id: hyperswitch_ai_interaction.session_id,
            user_id: hyperswitch_ai_interaction.user_id,
            merchant_id: hyperswitch_ai_interaction.merchant_id,
            profile_id: hyperswitch_ai_interaction.profile_id,
            org_id: hyperswitch_ai_interaction.org_id,
            role_id: hyperswitch_ai_interaction.role_id,
            user_query: hyperswitch_ai_interaction.user_query,
            response: hyperswitch_ai_interaction.response,
            database_query: hyperswitch_ai_interaction.database_query,
            interaction_status: hyperswitch_ai_interaction.interaction_status,
            created_at: hyperswitch_ai_interaction.created_at,
        };
        hyperswitch_ai_interactions.push(hyperswitch_ai_interaction.clone());
        Ok(hyperswitch_ai_interaction)
    }

    async fn list_hyperswitch_ai_interactions(
        &self,
        merchant_id: Option<common_utils::id_type::MerchantId>,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<storage::HyperswitchAiInteraction>, errors::StorageError> {
        let hyperswitch_ai_interactions = self.hyperswitch_ai_interactions.lock().await;

        let offset_usize = offset.try_into().unwrap_or_else(|_| {
            common_utils::consts::DEFAULT_LIST_OFFSET
                .try_into()
                .unwrap_or(usize::MIN)
        });

        let limit_usize = limit.try_into().unwrap_or_else(|_| {
            common_utils::consts::DEFAULT_LIST_LIMIT
                .try_into()
                .unwrap_or(usize::MAX)
        });

        let filtered_interactions: Vec<storage::HyperswitchAiInteraction> =
            hyperswitch_ai_interactions
                .iter()
                .filter(
                    |interaction| match (merchant_id.as_ref(), &interaction.merchant_id) {
                        (Some(merchant_id), Some(interaction_merchant_id)) => {
                            interaction_merchant_id == &merchant_id.get_string_repr().to_owned()
                        }
                        (None, _) => true,
                        _ => false,
                    },
                )
                .skip(offset_usize)
                .take(limit_usize)
                .cloned()
                .collect();
        Ok(filtered_interactions)
    }
}
