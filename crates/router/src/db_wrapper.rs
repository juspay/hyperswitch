use error_stack::{FutureExt, IntoReport, ResultExt};
use futures::{
    future::{join_all, try_join, try_join_all},
    join,
};
use router_derive::Setter;
use storage_models::enums::MerchantStorageScheme;

use crate::{
    core::errors::{self, RouterResult, StorageErrorExt},
    db::StorageInterface,
    types::storage::{self as storage_types},
};

pub enum PaymentAttemptDbCall {
    Query {
        merchant_id: String,
        payment_id: String,
    },
    Insert(storage_types::PaymentAttemptNew),
    Update {
        current_payment_attempt: storage_types::PaymentAttempt,
        updated_payment_attempt: storage_types::PaymentAttemptUpdate,
    },
}

impl PaymentAttemptDbCall {
    async fn get_db_call(
        self,
        db: &dyn StorageInterface,
        storage_scheme: MerchantStorageScheme,
    ) -> Result<storage_types::PaymentAttempt, error_stack::Report<errors::ApiErrorResponse>> {
        match self {
            PaymentAttemptDbCall::Query {
                merchant_id,
                payment_id,
            } => db
                .find_payment_attempt_by_payment_id_merchant_id(
                    &payment_id,
                    &merchant_id,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound),
            PaymentAttemptDbCall::Insert(payment_attempt_new) => {
                let payment_id = payment_attempt_new.payment_id.clone();
                db.insert_payment_attempt(payment_attempt_new, storage_scheme)
                    .await
                    .change_context(errors::ApiErrorResponse::DuplicatePayment { payment_id })
            }
            PaymentAttemptDbCall::Update {
                current_payment_attempt,
                updated_payment_attempt,
            } => db
                .update_payment_attempt(
                    current_payment_attempt,
                    updated_payment_attempt,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update payment attempt"),
        }
    }
}

pub enum PaymentIntentDbCall {
    Insert(storage_types::PaymentIntentNew),
}

impl PaymentIntentDbCall {
    async fn get_db_call(
        self,
        db: &dyn StorageInterface,
        storage_scheme: MerchantStorageScheme,
    ) -> Result<storage_types::PaymentIntent, error_stack::Report<errors::ApiErrorResponse>> {
        match self {
            PaymentIntentDbCall::Insert(payment_intent_new) => {
                let payment_id = payment_intent_new.payment_id.clone();
                db.insert_payment_intent(payment_intent_new, storage_scheme)
                    .await
                    .change_context(errors::ApiErrorResponse::DuplicatePayment { payment_id })
            }
        }
    }
}

pub enum ConnectorResponseDbCall {
    Query {
        merchant_id: String,
        payment_id: String,
        attempt_id: String,
    },
    Insert(storage_types::ConnectorResponseNew),
    Update(storage_types::ConnectorResponseUpdate),
}

pub enum AddressDbCall {
    Query { address_id: String },
    Insert(storage_types::AddressNew),
    Update(storage_types::AddressUpdate),
}

pub struct DbCall {
    payment_intent: PaymentIntentDbCall,
    payment_attempt: PaymentAttemptDbCall,
    connector_response: ConnectorResponseDbCall,
    shipping_address: Option<AddressDbCall>,
    billing_address: Option<AddressDbCall>,
}

pub enum EntityRequest {
    PaymentIntent {},
    PaymentAttempt {
        payment_id: String,
        merchant_id: String,
    },
    Address {
        address_id: String,
    },
    ConnectorResponse {
        payment_id: String,
        attempt_id: String,
        merchant_id: String,
    },
}

#[derive(Debug)]
pub enum Entity {
    PaymentIntent(
        Result<storage_types::PaymentIntent, error_stack::Report<errors::ApiErrorResponse>>,
    ),
    PaymentAttempt(
        Result<storage_types::PaymentAttempt, error_stack::Report<errors::ApiErrorResponse>>,
    ),
    Address(Result<storage_types::Address, error_stack::Report<errors::ApiErrorResponse>>),
    ConnectorResponse(
        Result<storage_types::ConnectorResponse, error_stack::Report<errors::ApiErrorResponse>>,
    ),
    None, //FIXME: for testing purposes only
}

#[derive(Setter)]
pub struct EntityResult {
    pub payment_intent:
        Result<storage_types::PaymentIntent, error_stack::Report<errors::ApiErrorResponse>>,
    pub payment_attempt:
        Result<storage_types::PaymentAttempt, error_stack::Report<errors::ApiErrorResponse>>,
    pub connector_response:
        Result<storage_types::ConnectorResponse, error_stack::Report<errors::ApiErrorResponse>>,
    pub billing_address:
        Result<Option<storage_types::Address>, error_stack::Report<errors::ApiErrorResponse>>,
    pub shipping_address:
        Result<Option<storage_types::Address>, error_stack::Report<errors::ApiErrorResponse>>,
}

// #[derive(Setter)]
// pub struct DbCallRequest<T, F: Fn() -> Result<T, errors::ApiErrorResponse>> {
//     pub payment_attempt: F,
//     pub connector_response:
//         Result<storage_types::ConnectorResponse, error_stack::Report<errors::ApiErrorResponse>>,
//     pub billing_address:
//         Result<Option<storage_types::Address>, error_stack::Report<errors::ApiErrorResponse>>,
//     pub shipping_address:
//         Result<Option<storage_types::Address>, error_stack::Report<errors::ApiErrorResponse>>,
//     pub mandate:
//         Result<Option<storage_types::Mandate>, error_stack::Report<errors::ApiErrorResponse>>,
// }

impl EntityResult {
    fn new() -> Self {
        Self {
            payment_intent: Err(error_stack::report!(
                errors::ApiErrorResponse::PaymentNotFound
            )),
            payment_attempt: Err(error_stack::report!(
                errors::ApiErrorResponse::PaymentNotFound
            )),
            connector_response: Err(error_stack::report!(
                errors::ApiErrorResponse::PaymentNotFound
            )),
            billing_address: Ok(None),
            shipping_address: Ok(None),
        }
    }
}

impl Default for EntityResult {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
pub trait QueryEntity {
    async fn query_entity(
        &self,
        db: &dyn StorageInterface,
        storage_scheme: MerchantStorageScheme,
    ) -> Entity;
}

#[async_trait::async_trait]
impl QueryEntity for EntityRequest {
    async fn query_entity(
        &self,
        db: &dyn StorageInterface,
        storage_scheme: MerchantStorageScheme,
    ) -> Entity {
        match self {
            EntityRequest::PaymentIntent {
                payment_id,
                merchant_id,
            } => Entity::PaymentIntent(
                db.find_payment_intent_by_payment_id_merchant_id(
                    payment_id,
                    merchant_id,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound),
            ),
            EntityRequest::PaymentAttempt {
                payment_id,
                merchant_id,
            } => Entity::PaymentAttempt(
                db.find_payment_attempt_by_payment_id_merchant_id(
                    payment_id,
                    merchant_id,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound),
            ),
            EntityRequest::Address { address_id } => Entity::Address(
                db.find_address(address_id)
                    .await
                    .change_context(errors::ApiErrorResponse::AddressNotFound), //FIXME: do not change context
            ),
            EntityRequest::ConnectorResponse {
                payment_id,
                attempt_id,
                merchant_id,
            } => Entity::ConnectorResponse(
                db.find_connector_response_by_payment_id_merchant_id_attempt_id(
                    &payment_id,
                    &merchant_id,
                    &attempt_id,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound),
            ),
        }
    }
}

pub async fn make_parallel_db_call(
    db: &dyn StorageInterface,
    db_calls: DbCall,
    storage_scheme: MerchantStorageScheme,
) -> EntityResult {
    let (payment_intent_res) = join!(
        db_calls.payment_attempt.get_db_call(db, storage_scheme),
        db_calls.payment_intent.get_db_call(db, storage_scheme)
    );

    let mut entities_result = EntityResult::new();

    for entity in combined_res {
        match entity {
            Entity::PaymentIntent(pi_res) => entities_result.set_payment_intent(pi_res),
            Entity::PaymentAttempt(pa_res) => entities_result.set_payment_attempt(pa_res),
            _ => &mut entities_result,
        };
    }

    entities_result
}
