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
            authentication_data: self.authentication_data.clone(),
            encoded_data: self.encoded_data.clone(),
            connector_transaction_id: self.connector_transaction_id.clone(),
            connector: self.connector_name.clone(),
            updated_by: self.updated_by.clone(),
        };

        let _payment_attempt: Result<PaymentAttempt, _> =
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
            .await
            .map_err(|err| {
                logger::error!(
                    "Error while updating payment attempt in connector_response flow {:?}",
                    err
                );
                err
            });

        generics::generic_insert(conn, self).await
    }
}

impl ConnectorResponse {
    #[instrument(skip(conn))]
    pub async fn update(
        self,
        conn: &PgPooledConn,
        connector_response: ConnectorResponseUpdate,
    ) -> StorageResult<Self> {
        let payment_attempt_update = match connector_response.clone() {
            ConnectorResponseUpdate::ResponseUpdate {
                connector_transaction_id,
                authentication_data,
                encoded_data,
                connector_name,
                updated_by,
            } => PaymentAttemptUpdate::ConnectorResponse {
                authentication_data,
                encoded_data,
                connector_transaction_id,
                connector: connector_name,
                updated_by,
            },
            ConnectorResponseUpdate::ErrorUpdate {
                connector_name,
                updated_by,
            } => PaymentAttemptUpdate::ConnectorResponse {
                authentication_data: None,
                encoded_data: None,
                connector_transaction_id: None,
                connector: connector_name,
                updated_by,
            },
        };

        let _payment_attempt: Result<PaymentAttempt, _> =
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
            .await
            .map_err(|err| {
                logger::error!(
                    "Error while updating payment attempt in connector_response flow {:?}",
                    err
                );
                err
            });

        let connector_response_result =
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
            };

        connector_response_result
    }

    #[instrument(skip(conn))]
    pub async fn find_by_payment_id_merchant_id_attempt_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
        attempt_id: &str,
    ) -> StorageResult<Self> {
        let connector_response: Self =
            generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
                conn,
                dsl::merchant_id.eq(merchant_id.to_owned()).and(
                    dsl::payment_id
                        .eq(payment_id.to_owned())
                        .and(dsl::attempt_id.eq(attempt_id.to_owned())),
                ),
            )
            .await?;

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
            Ok::<PaymentAttempt, _>(payment_attempt) => {
                if payment_attempt.authentication_data != connector_response.authentication_data {
                    logger::error!(
                        "Not Equal pa_authentication_data : {:?}, cr_authentication_data: {:?} ",
                        payment_attempt.authentication_data,
                        connector_response.authentication_data
                    );
                }

                if payment_attempt.encoded_data != connector_response.encoded_data {
                    logger::error!(
                        "Not Equal pa_encoded_data : {:?}, cr_encoded_data: {:?} ",
                        payment_attempt.encoded_data,
                        connector_response.encoded_data
                    );
                }

                if payment_attempt.connector_transaction_id
                    != connector_response.connector_transaction_id
                {
                    logger::error!(
                            "Not Equal pa_connector_transaction_id : {:?}, cr_connector_transaction_id: {:?} ",
                            payment_attempt.connector_transaction_id,
                            connector_response.connector_transaction_id
                        );
                }
                if payment_attempt.connector != connector_response.connector_name {
                    logger::error!(
                        "Not Equal pa_connector : {:?}, cr_connector_name: {:?} ",
                        payment_attempt.connector,
                        connector_response.connector_name
                    );
                }
            }
            Err(err) => {
                logger::error!(
                    "Error while finding payment attempt in connector_response flow {:?}",
                    err
                );
            }
        }

        Ok(connector_response)
    }
}
