use common_utils::ext_traits::AsyncExt;
use error_stack::{self, ResultExt};
use futures::join;
use router_derive::Setter;
use storage_models::{
    self, address, connector_response, enums as storage_enums, payment_attempt, payment_intent,
};

use crate::{
    core::errors::{self, StorageErrorExt},
    db::StorageInterface,
};

pub enum PaymentIntentCaller {
    Insert(payment_intent::PaymentIntentNew),
}

pub enum PaymentAttemptCaller {
    Insert(payment_attempt::PaymentAttemptNew),
    Query {
        attempt_id: String,
        merchant_id: String,
    },
    Update {
        current_payment_attempt: payment_attempt::PaymentAttempt,
        payment_attempt_update: payment_attempt::PaymentAttemptUpdate,
    },
}

pub enum ConnectorResponseCaller {
    Insert(connector_response::ConnectorResponseNew),
    Query {
        payment_id: String,
        merchant_id: String,
        attempt_id: String,
    },
    Update {
        current_connector_response: connector_response::ConnectorResponse,
        connector_response_update: connector_response::ConnectorResponseUpdate,
    },
}

pub enum AddressCaller {
    Insert(address::AddressNew),
    Query {
        address_id: String,
    },
    Update {
        address_id: String,
        address_update: address::AddressUpdate,
    },
}

#[async_trait::async_trait]
pub trait GetQuery {
    type ResultItem;
    async fn get_query(
        self,
        db: &dyn StorageInterface,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> Result<Self::ResultItem, error_stack::Report<errors::ApiErrorResponse>>;
}

#[async_trait::async_trait]
impl GetQuery for PaymentIntentCaller {
    type ResultItem = payment_intent::PaymentIntent;
    async fn get_query(
        self,
        db: &dyn StorageInterface,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> Result<Self::ResultItem, error_stack::Report<errors::ApiErrorResponse>> {
        match self {
            Self::Insert(payment_intent_new) => {
                let payment_id = payment_intent_new.payment_id.clone();
                db.insert_payment_intent(payment_intent_new, storage_scheme)
                    .await
                    .map_err(|error| {
                        error.to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                            payment_id,
                        })
                    })
            }
        }
    }
}

#[async_trait::async_trait]
impl GetQuery for PaymentAttemptCaller {
    type ResultItem = payment_attempt::PaymentAttempt;
    async fn get_query(
        self,
        db: &dyn StorageInterface,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> Result<Self::ResultItem, error_stack::Report<errors::ApiErrorResponse>> {
        match self {
            Self::Insert(payment_attempt_new) => {
                let payment_id = payment_attempt_new.payment_id.clone();
                db.insert_payment_attempt(payment_attempt_new, storage_scheme)
                    .await
                    .map_err(|error| {
                        error.to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                            payment_id,
                        })
                    })
            }
            Self::Query {
                attempt_id,
                merchant_id,
            } => db
                .find_payment_attempt_by_merchant_id_attempt_id(
                    &merchant_id,
                    &attempt_id,
                    storage_scheme,
                )
                .await
                .map_err(|error| {
                    error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
                }),
            Self::Update {
                current_payment_attempt,
                payment_attempt_update,
            } => db
                .update_payment_attempt(
                    current_payment_attempt,
                    payment_attempt_update,
                    storage_scheme,
                )
                .await
                .map_err(|error| {
                    error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
                }),
        }
    }
}

#[async_trait::async_trait]
impl GetQuery for ConnectorResponseCaller {
    type ResultItem = connector_response::ConnectorResponse;
    async fn get_query(
        self,
        db: &dyn StorageInterface,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> Result<Self::ResultItem, error_stack::Report<errors::ApiErrorResponse>> {
        match self {
            Self::Insert(connector_response_new) => {
                let payment_id = connector_response_new.payment_id.clone();
                db.insert_connector_response(connector_response_new, storage_scheme)
                    .await
                    .map_err(|error| {
                        error.to_duplicate_response(errors::ApiErrorResponse::DuplicatePayment {
                            payment_id,
                        })
                    })
            }
            Self::Query {
                attempt_id,
                merchant_id,
                payment_id,
            } => db
                .find_connector_response_by_payment_id_merchant_id_attempt_id(
                    &payment_id,
                    &merchant_id,
                    &attempt_id,
                    storage_scheme,
                )
                .await
                .map_err(|error| {
                    error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
                }),
            Self::Update {
                current_connector_response,
                connector_response_update,
            } => db
                .update_connector_response(
                    current_connector_response,
                    connector_response_update,
                    storage_scheme,
                )
                .await
                .map_err(|error| {
                    error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
                }),
        }
    }
}

#[async_trait::async_trait]
impl GetQuery for AddressCaller {
    type ResultItem = address::Address;
    async fn get_query(
        self,
        db: &dyn StorageInterface,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> Result<Self::ResultItem, error_stack::Report<errors::ApiErrorResponse>> {
        match self {
            Self::Insert(address_new) => db
                .insert_address(address_new)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error when inserting the address"),
            Self::Query { address_id } => db
                .find_address(&address_id)
                .await
                .change_context(errors::ApiErrorResponse::AddressNotFound),
            Self::Update {
                address_id,
                address_update,
            } => db
                .update_address(address_id, address_update)
                .await
                .map_err(|error| {
                    error.to_not_found_response(errors::ApiErrorResponse::AddressNotFound)
                }),
        }
    }
}

#[derive(Setter)]
pub struct DbCalls {
    pub payment_intent: Option<PaymentIntentCaller>,
    pub payment_attempt: PaymentAttemptCaller,
    pub connector_response: ConnectorResponseCaller,
    pub billing_address: Option<AddressCaller>,
    pub shipping_address: Option<AddressCaller>,
}

pub struct DbCallResults {
    pub payment_intent: Result<
        Option<payment_intent::PaymentIntent>,
        error_stack::Report<errors::ApiErrorResponse>,
    >,
    pub payment_attempt:
        Result<payment_attempt::PaymentAttempt, error_stack::Report<errors::ApiErrorResponse>>,
    pub connector_response: Result<
        connector_response::ConnectorResponse,
        error_stack::Report<errors::ApiErrorResponse>,
    >,
    pub billing_address:
        Result<Option<address::Address>, error_stack::Report<errors::ApiErrorResponse>>,
    pub shipping_address:
        Result<Option<address::Address>, error_stack::Report<errors::ApiErrorResponse>>,
}

pub async fn make_db_calls(
    db_calls: DbCalls,
    db: &dyn StorageInterface,
    storage_scheme: storage_enums::MerchantStorageScheme,
) -> DbCallResults {
    let pi_fut = db_calls
        .payment_intent
        .async_map(|payment_intent_caller| payment_intent_caller.get_query(db, storage_scheme));

    let pa_fut = db_calls.payment_attempt.get_query(db, storage_scheme);
    let cr_fut = db_calls.connector_response.get_query(db, storage_scheme);
    let sa_fut = db_calls
        .shipping_address
        .async_map(|shipping_address_caller| shipping_address_caller.get_query(db, storage_scheme));

    let ba_fut = db_calls
        .billing_address
        .async_map(|billing_address_caller| billing_address_caller.get_query(db, storage_scheme));

    let (maybe_pi, pa, cr, maybe_sa, maybe_ba) = join!(pi_fut, pa_fut, cr_fut, sa_fut, ba_fut);

    DbCallResults {
        payment_intent: maybe_pi.transpose(),
        payment_attempt: pa,
        connector_response: cr,
        billing_address: maybe_ba.transpose(),
        shipping_address: maybe_sa.transpose(),
    }
}
