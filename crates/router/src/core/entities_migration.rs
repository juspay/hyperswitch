use api_models::entities_migration::{
    EntitiesMigrationRequest, EntitiesMigrationResponse, EntityMigrationResult,
    EntityMigrationStatus,
};
use common_utils::{
    ext_traits::{Encode, StringExt},
    id_type,
};
use router_env::logger;

use super::{errors, payment_methods::vault};
use crate::{
    routes::SessionState, services::ApplicationResponse, types::payment_methods as pm_types,
};

const ENTITIES_MIGRATION_PARALLELISM: usize = 10;

pub async fn entities_migration(
    state: SessionState,
    req: &EntitiesMigrationRequest,
) -> errors::RouterResponse<EntitiesMigrationResponse> {
    let total = req.merchant_ids.len();
    let mut results = Vec::with_capacity(total);
    let mut merchant_ids = req.merchant_ids.clone().into_iter();

    loop {
        let chunk = merchant_ids
            .by_ref()
            .take(ENTITIES_MIGRATION_PARALLELISM)
            .collect::<Vec<_>>();

        if chunk.is_empty() {
            break;
        }

        let chunk_results = futures::future::join_all(
            chunk
                .into_iter()
                .map(|merchant_id| create_entity(&state, merchant_id)),
        )
        .await;

        results.extend(chunk_results);
    }

    let succeeded = results
        .iter()
        .filter(|result| matches!(result.status, EntityMigrationStatus::Success))
        .count();
    let failed = total - succeeded;

    Ok(ApplicationResponse::Json(EntitiesMigrationResponse {
        total,
        succeeded,
        failed,
        results,
    }))
}

async fn create_entity(
    state: &SessionState,
    merchant_id: id_type::MerchantId,
) -> EntityMigrationResult {
    let entity_id = merchant_id.get_string_repr().to_owned();

    let payload = match (pm_types::EntityCreateRequest {
        entity_id: merchant_id.clone(),
    })
    .encode_to_vec()
    {
        Ok(payload) => payload,
        Err(error) => {
            logger::error!(?error, entity_id, "Failed to encode entity create request");
            return EntityMigrationResult {
                merchant_id,
                status: EntityMigrationStatus::Error,
                created_at: None,
                error_code: None,
                error_message: Some("Failed to encode entity create request".to_string()),
            };
        }
    };

    match vault::call_to_vault::<pm_types::EntityCreate>(state, payload, None, None).await {
        Ok(response) => {
            let parsed: Result<pm_types::EntityCreateResponse, _> =
                response.parse_struct("EntityCreateResponse");
            match parsed {
                Ok(response) => {
                    logger::info!(entity_id, "Entity created in locker");
                    EntityMigrationResult {
                        merchant_id,
                        status: EntityMigrationStatus::Success,
                        created_at: Some(response.created_at),
                        error_code: None,
                        error_message: None,
                    }
                }
                Err(error) => {
                    logger::error!(?error, entity_id, "Failed to parse entity create response");
                    EntityMigrationResult {
                        merchant_id,
                        status: EntityMigrationStatus::Error,
                        created_at: None,
                        error_code: None,
                        error_message: Some(format!("Failed to parse locker response: {error:?}")),
                    }
                }
            }
        }
        Err(error) => {
            logger::error!(?error, entity_id, "Failed to create entity in locker");
            EntityMigrationResult {
                merchant_id,
                status: EntityMigrationStatus::Error,
                created_at: None,
                error_code: None,
                error_message: Some(format!("{error:?}")),
            }
        }
    }
}
