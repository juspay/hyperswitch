use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, logger, tracing};

use super::generics;
use crate::{
    connector_response::{
        ConnectorResponse, ConnectorResponseNew, ConnectorResponseUpdate,
        ConnectorResponseUpdateInternal,
    },
    errors,
    payment_attempt::{PaymentAttempt, PaymentAttemptUpdate, PaymentAttemptUpdateInternal},
    schema::{connector_response::dsl, payment_attempt::dsl as pa_dsl},
    PgPooledConn, StorageResult,
};

impl ConnectorResponseNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<ConnectorResponse> {
        let payment_attempt_update = PaymentAttemptUpdate::ConnectorResponse {
            authentication_data: self.authentication_data,
            encoded_data: self.encoded_data,
            connector_transaction_id: self.connector_transaction_id,
            connector: self.connector_name,
            updated_by: self.updated_by,
        };

        let payment_attempt: PaymentAttempt =
            generics::generic_update_with_unique_predicate_get_result::<
                <PaymentAttempt as HasTable>::Table,
                _,
                _,
                _,
            >(
                conn,
                pa_dsl::attempt_id
                    .eq(self.attempt_id.to_owned())
                    .and(pa_dsl::merchant_id.eq(self.merchant_id.to_owned())),
                PaymentAttemptUpdateInternal::from(payment_attempt_update),
            )
            .await?;

        Ok(ConnectorResponse {
            id: 0i32,
            payment_id: payment_attempt.payment_id,
            merchant_id: payment_attempt.merchant_id,
            attempt_id: payment_attempt.attempt_id,
            created_at: payment_attempt.created_at,
            modified_at: payment_attempt.modified_at,
            connector_name: payment_attempt.connector,
            connector_transaction_id: payment_attempt.connector_transaction_id,
            authentication_data: payment_attempt.authentication_data,
            encoded_data: payment_attempt.encoded_data,
            updated_by: payment_attempt.updated_by,
        })
    }
}

impl ConnectorResponse {
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        connector_response: ConnectorResponseUpdate,
    ) -> StorageResult<Self> {
        let payment_attempt_update = match connector_response {
            ConnectorResponseUpdate::ResponseUpdate {
                ref connector_transaction_id,
                ref authentication_data,
                ref encoded_data,
                ref connector_name,
                ref updated_by,
            } => PaymentAttemptUpdate::ConnectorResponse {
                authentication_data: authentication_data.to_owned(),
                encoded_data: encoded_data.to_owned(),
                connector_transaction_id: connector_transaction_id.to_owned(),
                connector: connector_name.to_owned(),
                updated_by: updated_by.to_owned(),
            },
            ConnectorResponseUpdate::ErrorUpdate {
                ref connector_name,
                ref updated_by,
            } => PaymentAttemptUpdate::ConnectorResponse {
                authentication_data: None,
                encoded_data: None,
                connector_transaction_id: None,
                connector: connector_name.clone(),
                updated_by: updated_by.to_owned(),
            },
        };

        match generics::generic_update_with_unique_predicate_get_result::<
            <PaymentAttempt as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            pa_dsl::attempt_id
                .eq(self.attempt_id.to_owned())
                .and(pa_dsl::merchant_id.eq(self.merchant_id.to_owned())),
            PaymentAttemptUpdateInternal::from(payment_attempt_update),
        )
        .await
        {
            Ok::<PaymentAttempt, _>(payment_attempt) => Ok(Self {
                id: 0i32,
                payment_id: payment_attempt.payment_id,
                merchant_id: payment_attempt.merchant_id,
                attempt_id: payment_attempt.attempt_id,
                created_at: payment_attempt.created_at,
                modified_at: payment_attempt.modified_at,
                connector_name: payment_attempt.connector,
                connector_transaction_id: payment_attempt.connector_transaction_id,
                authentication_data: payment_attempt.authentication_data,
                encoded_data: payment_attempt.encoded_data,
                updated_by: payment_attempt.updated_by,
            }),
            Err(err) => {
                logger::error!(
                    "Error while updating payment attempt in connector_response flow {:?}",
                    err
                );
                match err.current_context() {
                    errors::DatabaseError::NotFound => {
                        match generics::generic_update_with_unique_predicate_get_result::<
                            <Self as HasTable>::Table,
                            _,
                            _,
                            _,
                        >(
                            conn,
                            dsl::merchant_id
                                .eq(self.merchant_id.clone())
                                .and(dsl::payment_id.eq(self.payment_id.clone()))
                                .and(dsl::attempt_id.eq(self.attempt_id.clone())),
                            ConnectorResponseUpdateInternal::from(connector_response),
                        )
                        .await
                        {
                            Err(error) => match error.current_context() {
                                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                                _ => Err(error),
                            },
                            result => result,
                        }
                    }
                    _ => Err(err),
                }
            }
        }
    }

    #[instrument(skip(conn))]
    pub async fn find_by_payment_id_merchant_id_attempt_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
        attempt_id: &str,
    ) -> StorageResult<Self> {
        match generics::generic_find_one::<<PaymentAttempt as HasTable>::Table, _, _>(
            conn,
            pa_dsl::payment_id.eq(payment_id.to_owned()).and(
                pa_dsl::merchant_id
                    .eq(merchant_id.to_owned())
                    .and(pa_dsl::attempt_id.eq(attempt_id.to_owned())),
            ),
        )
        .await
        {
            Ok::<PaymentAttempt, _>(payment_attempt) => Ok(Self {
                id: 0i32,
                payment_id: payment_attempt.payment_id,
                merchant_id: payment_attempt.merchant_id,
                attempt_id: payment_attempt.attempt_id,
                created_at: payment_attempt.created_at,
                modified_at: payment_attempt.modified_at,
                connector_name: payment_attempt.connector,
                connector_transaction_id: payment_attempt.connector_transaction_id,
                authentication_data: payment_attempt.authentication_data,
                encoded_data: payment_attempt.encoded_data,
                updated_by: payment_attempt.updated_by,
            }),
            Err(err) => match err.current_context() {
                errors::DatabaseError::NotFound => {
                    generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
                        conn,
                        dsl::merchant_id.eq(merchant_id.to_owned()).and(
                            dsl::payment_id
                                .eq(payment_id.to_owned())
                                .and(dsl::attempt_id.eq(attempt_id.to_owned())),
                        ),
                    )
                    .await
                }
                _ => Err(err),
            },
        }
    }
}
