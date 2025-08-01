use std::collections::HashMap;

use async_trait::async_trait;
use common_utils::{errors::CustomResult, id_type::CustomerId};
use error_stack::ResultExt;
use storage_impl::{
    redis::kv_store::{kv_wrapper, KvOperation, KvResult, PartitionKey},
    kv_router_store::KVRouterStore,
    DatabaseStore,
};
use storage_impl::redis_manipulation::*;
use router_env::{instrument, tracing};
use diesel_models::kv;

use crate::db::errors;

#[derive(Debug,Clone)]
pub struct TokenStatus {
    pub response_code: String,
    pub status: String,
}

#[async_trait]
pub trait RedisTokenMap {
    async fn insert_token_for_customer(
        &self,
        customer_id: CustomerId,
        token_id: &str,
        response_code: &str,
        status: &str,
    ) -> CustomResult<(), errors::StorageError>;

    async fn get_all_tokens_for_customer(
        &self,
        customer_id: CustomerId,
    ) -> CustomResult<HashMap<String, TokenStatus>, errors::StorageError>;
}

#[async_trait]
impl<T: DatabaseStore + Send + Sync> RedisTokenMap for KVRouterStore<T> {
    #[instrument(skip_all)]
    async fn insert_token_for_customer(
        &self,
        customer_id: CustomerId,
        token_id: &str,
        response_code: &str,
        status: &str,
    ) -> CustomResult<(), errors::StorageError> {
        let key = PartitionKey::ConnectorCustomerId {
            id: &customer_id,
        };

        let token_data = diesel_models::TokenData {
            id: token_id.to_string(),
            response_code: response_code.to_string(),
            status: status.to_string(),
        };

        let field = format!("token_{}", token_id);

        let redis_entry = kv::TypedSql {
            op: kv::DBOperation::Insert {
                insertable: Box::new(kv::Insertable::None),
            },
        };
        let serialized_token_data= serde_json::to_string(&token_data)
            .change_context(errors::StorageError::SerializationFailed)
            .attach_printable("Failed to serialize token data")?;   

        

        match Box::pin(kv_wrapper::<diesel_models::TokenData, _, diesel_models::TokenData>(
            self,
            KvOperation::Hset((&field, serialized_token_data), redis_entry),
            key,
        ))
        .await
        .change_context(errors::StorageError::KVError)?
        .try_into_hset()
        {
        Ok(()) => Ok(()),
        Err(error) => Err(error.change_context(errors::StorageError::KVError)),
        }
        // .try_into_hset()
        // {
        //     Ok(redis_interface::HsetReply::KeySet) | Ok(redis_interface::HsetReply::KeyOverwritten) => Ok(()),
        //     Err(error) => Err(error.change_context(errors::StorageError::KVError)),
        // }
        
        
        // .try_into_hsetnx()
        // {
        //     Ok(redis_interface::HsetnxReply::KeySet) => Ok(()),
        //     Ok(redis_interface::HsetnxReply::KeyNotSet) => {
        //         Err(errors::StorageError::DuplicateValue {
        //             entity: "token",
        //             key: Some(field),
        //         }
        //         .into())
        //     }
        //     Err(error) => Err(error.change_context(errors::StorageError::KVError)),
        // }
    }

    #[instrument(skip_all)]
    async fn get_all_tokens_for_customer(
        &self,
        customer_id: CustomerId,
    ) -> CustomResult<HashMap<String, TokenStatus>, errors::StorageError> {
        
        let key = PartitionKey::ConnectorCustomerId {
            id: &customer_id,
        };

        let result = Box::pin(kv_wrapper::<diesel_models::TokenData, _, diesel_models::TokenData>(
            self,
            KvOperation::Scan("token_*"),
            key,
        ))
        .await
        .change_context(errors::StorageError::KVError)
        .attach_printable("Failed fetching tokens from Redis")?;

        

        match result {
            KvResult::Scan(tokens) => {
                let token_map = tokens
                    .into_iter()
                    .enumerate()
                    .map(|(i, token_data)| {
                        (
                            format!("token_{}", token_data.id),
                            TokenStatus {
                                response_code: token_data.response_code,
                                status: token_data.status,
                            },
                        )
                    })
                    .collect();
                Ok(token_map)
            }
            _ => Ok(HashMap::new()),
        }
    }
}

pub async fn get_inactive_tokens_for_customer<T: RedisTokenMap>(
    store: &T,
    customer_id: CustomerId,
) -> CustomResult<HashMap<String, TokenStatus>, errors::StorageError> {
    let all_tokens = store.get_all_tokens_for_customer(customer_id).await?;

    let inactive_tokens = all_tokens
        .into_iter()
        .filter(|(_token_id, token_status)| token_status.status.to_lowercase() != "active")
        .collect();

    Ok(inactive_tokens)
}

pub async fn find_best_psp_token_using_customer_id<T: RedisTokenMap>(
    store: &T,
    customer_id: CustomerId,
) -> CustomResult<Option<(String, TokenStatus)>, errors::StorageError> {
    let all_tokens = store.get_all_tokens_for_customer(customer_id).await?;

    // Priority 1: response_code == "0"
    if let Some(entry) = all_tokens
        .iter()
        .find(|(_, token_status)| token_status.response_code == "0")
    {
        return Ok(Some((entry.0.clone(), entry.1.clone())));
    }

    // Priority 2: response_code is null or empty
    if let Some(entry) = all_tokens.iter().find(|(_, token_status)| {
        let code = token_status.response_code.trim();
        code.is_empty() || code.eq_ignore_ascii_case("null")
    }) {
        return Ok(Some((entry.0.clone(), entry.1.clone())));
    }

    // Priority 3: No ideal match — use fallback to choose best among all
    if let Some(best) = get_best_token_among_all(&all_tokens).await {
        println!("Fallback selected best token: {:?}", best.0);
        return Ok(Some(best));
    }

    // If nothing at all
    Ok(None)
}
