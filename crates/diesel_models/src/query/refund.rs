use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};
use error_stack::ResultExt;

use super::generics;
#[cfg(feature = "v1")]
use crate::schema::refund::dsl;
#[cfg(feature = "v2")]
use crate::schema_v2::refund::dsl;
use crate::{
    errors, kv,
    refund::{Refund, RefundNew, RefundUpdate, RefundUpdateInternal},
    DatabaseConnectionWithContext, StorageResult,
};

impl RefundNew {
    pub async fn insert(self, conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Refund> {
        Box::pin(generics::generic_insert(conn, self)).await
    }

    pub async fn generate_drainer_insert_query(
        self,
        conn: &mut DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<kv::SerializableQuery> {
        kv::generate_insert_query(conn, self)
            .await
            .attach_printable("Failed to generate insert query for refund")
    }
}

#[cfg(feature = "v1")]
impl Refund {
    pub async fn update(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
        refund: RefundUpdate,
    ) -> StorageResult<Self> {
        if self.external_surcharge_refund_details.is_some() {
            return self.update_surcharge_refund(conn, refund).await;
        }

        let processor_merchant_id = self
            .processor_merchant_id
            .clone()
            .unwrap_or_else(|| self.merchant_id.clone());
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::refund_id
                .eq(self.refund_id.to_owned())
                .and(dsl::processor_merchant_id.eq(processor_merchant_id)),
            RefundUpdateInternal::from(refund),
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

    // Fallback function for stagger release - updates by merchant_id when processor_merchant_id is NULL
    pub async fn update_by_merchant_id(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
        refund: RefundUpdate,
    ) -> StorageResult<Self> {
        if self.external_surcharge_refund_details.is_some() {
            return self.update_surcharge_refund(conn, refund).await;
        }

        let processor_merchant_id = self
            .processor_merchant_id
            .clone()
            .unwrap_or_else(|| self.merchant_id.clone());
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::refund_id
                .eq(self.refund_id.to_owned())
                .and(dsl::merchant_id.eq(processor_merchant_id)),
            RefundUpdateInternal::from(refund),
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

    // This is required to be changed for KV.
    pub async fn find_by_processor_merchant_id_refund_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        refund_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::refund_id.eq(refund_id.to_owned())),
        )
        .await
    }

    // Fallback function for stagger release - queries by merchant_id when processor_merchant_id is NULL
    pub async fn find_by_merchant_id_refund_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        refund_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::refund_id.eq(refund_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_processor_merchant_id_connector_refund_id_connector(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        connector_refund_id: &str,
        connector: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::connector_refund_id.eq(connector_refund_id.to_owned()))
                .and(dsl::connector.eq(connector.to_owned())),
        )
        .await
    }

    // Fallback function for stagger release - queries by merchant_id when processor_merchant_id is NULL
    pub async fn find_by_merchant_id_connector_refund_id_connector(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        connector_refund_id: &str,
        connector: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::connector_refund_id.eq(connector_refund_id.to_owned()))
                .and(dsl::connector.eq(connector.to_owned())),
        )
        .await
    }

    pub async fn find_by_internal_reference_id_processor_merchant_id(
        conn: &DatabaseConnectionWithContext<'_>,
        internal_reference_id: &str,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::internal_reference_id.eq(internal_reference_id.to_owned())),
        )
        .await
    }

    // Fallback function for stagger release - queries by merchant_id when processor_merchant_id is NULL
    pub async fn find_by_internal_reference_id_merchant_id(
        conn: &DatabaseConnectionWithContext<'_>,
        internal_reference_id: &str,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(processor_merchant_id.to_owned())
                .and(dsl::internal_reference_id.eq(internal_reference_id.to_owned())),
        )
        .await
    }

    pub async fn find_by_processor_merchant_id_connector_transaction_id(
        conn: &DatabaseConnectionWithContext<'_>,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        connector_transaction_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::connector_transaction_id.eq(connector_transaction_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }

    pub async fn find_by_payment_id_processor_merchant_id(
        conn: &DatabaseConnectionWithContext<'_>,
        payment_id: &common_utils::id_type::PaymentId,
        processor_merchant_id: &common_utils::id_type::MerchantId,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::processor_merchant_id
                .eq(processor_merchant_id.to_owned())
                .or(dsl::processor_merchant_id
                    .is_null()
                    .and(dsl::merchant_id.eq(processor_merchant_id.to_owned())))
                .and(dsl::payment_id.eq(payment_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }

    pub fn is_refund_applied(&self) -> bool {
        matches!(
            self.refund_status,
            common_enums::RefundStatus::ManualReview
                | common_enums::RefundStatus::Pending
                | common_enums::RefundStatus::Success
        )
    }
}

#[cfg(feature = "v2")]
impl Refund {
    pub async fn update_with_id(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
        refund: RefundUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.id.to_owned(),
            RefundUpdateInternal::from(refund),
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

    pub async fn find_by_global_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: &common_utils::id_type::GlobalRefundId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
        )
        .await
    }

    pub async fn find_by_merchant_id_connector_transaction_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_transaction_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_transaction_id.eq(connector_transaction_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }
}

impl RefundUpdate {
    #[cfg(feature = "v1")]
    pub async fn generate_drainer_update_query(
        self,
        conn: &mut DatabaseConnectionWithContext<'_>,
        refund_id: String,
        processor_merchant_id: common_utils::id_type::MerchantId,
    ) -> StorageResult<kv::SerializableQuery> {
        kv::generate_update_query_with_predicate::<<Refund as HasTable>::Table, _, _>(
            conn,
            dsl::refund_id
                .eq(refund_id)
                .and(dsl::processor_merchant_id.eq(processor_merchant_id)),
            RefundUpdateInternal::from(self),
        )
        .await
        .attach_printable("Failed to generate update query for refund")
    }

    #[cfg(feature = "v2")]
    pub async fn generate_drainer_update_query(
        self,
        conn: &mut DatabaseConnectionWithContext<'_>,
        id: common_utils::id_type::GlobalRefundId,
    ) -> StorageResult<kv::SerializableQuery> {
        kv::generate_update_query_by_id::<<Refund as HasTable>::Table, _, _>(
            conn,
            id,
            RefundUpdateInternal::from(self),
        )
        .await
        .attach_printable("Failed to generate update query for refund")
    }
}

/// Claim refund progress in Rust. The payment row serializes Init claims across partial refunds.
#[cfg(feature = "v1")]
impl Refund {
    pub async fn compare_and_swap_surcharge(
        &self,
        conn: &DatabaseConnectionWithContext<'_>,
        details: common_types::refunds::ExternalSurchargeRefundDetails,
        surcharge: Option<common_utils::types::MinorUnit>,
        net: Option<common_utils::types::MinorUnit>,
        rejected: bool,
    ) -> StorageResult<Option<Self>> {
        use common_types::refunds::SurchargeRefundStatus;
        use diesel::{OptionalExtension, QueryDsl};
        use generics::db_metrics::DatabaseOperation;

        conn.transaction_async(move |conn| async move {
            if details.surcharge_refunds_status == SurchargeRefundStatus::InitInFlight {
                use crate::schema::payment_intent::dsl as pi;
                let owner = self.processor_merchant_id.as_ref().unwrap_or(&self.merchant_id);
                let lock = pi::payment_intent
                    .filter(pi::payment_id.eq(self.payment_id.clone()))
                    .filter(pi::processor_merchant_id.eq(owner.clone()).or(
                        pi::processor_merchant_id.is_null().and(pi::merchant_id.eq(owner.clone()))
                    ))
                    .select(pi::payment_id)
                    .for_update();
                conn.track_get_result::<crate::payment_intent::PaymentIntent, _, common_utils::id_type::PaymentId>(
                    DatabaseOperation::FindOne, lock,
                ).await?;
                let previous = dsl::refund
                    .filter(dsl::payment_id.eq(self.payment_id.clone()))
                    .filter(dsl::processor_merchant_id.eq(owner.clone()).or(
                        dsl::processor_merchant_id.is_null().and(dsl::merchant_id.eq(owner.clone()))
                    ));
                let previous = conn.track_load::<Self, _, Self>(DatabaseOperation::Filter, previous).await?;
                if previous.iter().any(|refund| refund.refund_id != self.refund_id
                    && refund.external_surcharge_refund_details.as_ref().is_some_and(|d| matches!(
                        d.surcharge_refunds_status,
                        SurchargeRefundStatus::InitInFlight | SurchargeRefundStatus::Initialized | SurchargeRefundStatus::ReconciliationRequired
                    )))
                {
                    return Ok(None);
                }
            }
            let query = diesel::update(
                dsl::refund.filter(
                    dsl::refund_id.eq(self.refund_id.clone())
                        .and(dsl::merchant_id.eq(self.merchant_id.clone()))
                        .and(dsl::refund_status.eq(self.refund_status))
                        .and(dsl::external_surcharge_refund_details.eq(self.external_surcharge_refund_details.clone())),
                ),
            ).set((
                dsl::external_surcharge_refund_details.eq(Some(details)),
                dsl::refund_surcharge_amount.eq(surcharge.or(self.refund_surcharge_amount)),
                dsl::net_refund_amount.eq(net.or(self.net_refund_amount)),
                dsl::refund_status.eq(if rejected { crate::enums::RefundStatus::Failure } else { self.refund_status }),
                dsl::modified_at.eq(common_utils::date_time::now()),
            ));
            conn.track_get_result::<Self, _, Self>(DatabaseOperation::UpdateOne, query).await.optional()
        }).await.change_context(errors::DatabaseError::Others)
    }
}

#[cfg(feature = "v1")]
impl Refund {
    /// Apply processor updates without losing successful refunds or surcharge recovery state.
    async fn update_surcharge_refund(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
        refund: RefundUpdate,
    ) -> StorageResult<Self> {
        use common_types::refunds::SurchargeRefundStatus;
        use diesel::QueryDsl;
        use generics::db_metrics::DatabaseOperation;
        conn.transaction_async(move |conn| async move {
            let predicate = dsl::refund_id
                .eq(self.refund_id.clone())
                .and(dsl::merchant_id.eq(self.merchant_id.clone()));
            let current = conn
                .track_get_result::<Self, _, Self>(
                    DatabaseOperation::FindOne,
                    dsl::refund.filter(predicate.clone()).for_update(),
                )
                .await?;
            let mut update = RefundUpdateInternal::from(refund);
            if current.refund_status == crate::enums::RefundStatus::Success {
                update.refund_status = Some(crate::enums::RefundStatus::Success);
            }
            let mut details = current.external_surcharge_refund_details;
            if matches!(
                update.refund_status,
                Some(
                    crate::enums::RefundStatus::Failure
                        | crate::enums::RefundStatus::TransactionFailure
                )
            ) {
                if let Some(details) = details.as_mut() {
                    if matches!(
                        details.surcharge_refunds_status,
                        SurchargeRefundStatus::Initialized
                            | SurchargeRefundStatus::ProcessorInFlight
                    ) {
                        details.surcharge_refunds_status =
                            SurchargeRefundStatus::ReconciliationRequired;
                    }
                }
            }
            let query = diesel::update(dsl::refund.filter(predicate))
                .set((update, dsl::external_surcharge_refund_details.eq(details)));
            conn.track_get_result::<Self, _, Self>(DatabaseOperation::UpdateOne, query)
                .await
        })
        .await
        .change_context(errors::DatabaseError::Others)
    }
}

#[cfg(feature = "v1")]
impl RefundNew {
    /// Store the refund and its recovery task together, without database triggers.
    pub async fn insert_with_surcharge_task(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
        application_source: common_enums::ApplicationSource,
    ) -> StorageResult<Refund> {
        use generics::db_metrics::DatabaseOperation;

        use crate::process_tracker::{ProcessTracker, ProcessTrackerNew};
        let task = ProcessTrackerNew::new(
            format!("SURCHARGE_REFUND_{}", self.internal_reference_id),
            "SURCHARGE_REFUND",
            crate::enums::ProcessTrackerRunner::RefundWorkflowRouter,
            ["REFUND"],
            crate::refund::RefundCoreWorkflow {
                refund_internal_reference_id: self.internal_reference_id.clone(),
                connector_transaction_id: self.connector_transaction_id.clone(),
                merchant_id: self.merchant_id.clone(),
                processor_merchant_id: self.processor_merchant_id.clone(),
                payment_id: self.payment_id.clone(),
                processor_transaction_data: self.processor_transaction_data.clone(),
            },
            None,
            common_utils::date_time::now() + time::Duration::minutes(1),
            common_types::consts::API_VERSION,
            application_source,
        )?;
        conn.transaction_async(move |conn| async move {
            let refund = conn
                .track_get_result::<Refund, _, Refund>(
                    DatabaseOperation::Insert,
                    diesel::insert_into(dsl::refund).values(self),
                )
                .await?;
            conn.track_get_result::<ProcessTracker, _, ProcessTracker>(
                DatabaseOperation::Insert,
                diesel::insert_into(crate::schema::process_tracker::table).values(task),
            )
            .await?;
            Ok::<_, diesel::result::Error>(refund)
        })
        .await
        .change_context(errors::DatabaseError::Others)
    }
}
