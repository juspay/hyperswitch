use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::refund, services::logger};

use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::CustomResult;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use crate::{
    errors,
    schema::refund::dsl,
};
use error_stack::{IntoReport, ResultExt};

use crate::{connection::PgPooledConn};

#[derive(
    Clone, Debug, Eq, Identifiable, Queryable, PartialEq, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = refund)]
pub struct Refund {
    pub id: i32,
    pub internal_reference_id: String,
    pub refund_id: String, //merchant_reference id
    pub payment_id: String,
    pub merchant_id: String,
    pub connector_transaction_id: String,
    pub connector: String,
    pub connector_refund_id: Option<String>,
    pub external_reference_id: Option<String>,
    pub refund_type: storage_enums::RefundType,
    pub total_amount: i64,
    pub currency: storage_enums::Currency,
    pub refund_amount: i64,
    pub refund_status: storage_enums::RefundStatus,
    pub sent_to_gateway: bool,
    pub refund_error_message: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub refund_arn: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub updated_at: PrimitiveDateTime,
    pub description: Option<String>,
    pub attempt_id: String,
    pub refund_reason: Option<String>,
    pub refund_error_code: Option<String>,
}

#[cfg(feature = "kv_store")]
impl crate::utils::storage_partitioning::KvStorePartition for Refund {}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Insertable,
    router_derive::DebugAsDisplay,
    serde::Serialize,
    serde::Deserialize,
    router_derive::Setter,
)]
#[diesel(table_name = refund)]
pub struct RefundNew {
    pub refund_id: String,
    pub payment_id: String,
    pub merchant_id: String,
    pub internal_reference_id: String,
    pub external_reference_id: Option<String>,
    pub connector_transaction_id: String,
    pub connector: String,
    pub connector_refund_id: Option<String>,
    pub refund_type: storage_enums::RefundType,
    pub total_amount: i64,
    pub currency: storage_enums::Currency,
    pub refund_amount: i64,
    pub refund_status: storage_enums::RefundStatus,
    pub sent_to_gateway: bool,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub refund_arn: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub modified_at: Option<PrimitiveDateTime>,
    pub description: Option<String>,
    pub attempt_id: String,
    pub refund_reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RefundUpdate {
    Update {
        connector_refund_id: String,
        refund_status: storage_enums::RefundStatus,
        sent_to_gateway: bool,
        refund_error_message: Option<String>,
        refund_arn: String,
    },
    MetadataAndReasonUpdate {
        metadata: Option<pii::SecretSerdeValue>,
        reason: Option<String>,
    },
    StatusUpdate {
        connector_refund_id: Option<String>,
        sent_to_gateway: bool,
        refund_status: storage_enums::RefundStatus,
    },
    ErrorUpdate {
        refund_status: Option<storage_enums::RefundStatus>,
        refund_error_message: Option<String>,
        refund_error_code: Option<String>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = refund)]
pub struct RefundUpdateInternal {
    connector_refund_id: Option<String>,
    refund_status: Option<storage_enums::RefundStatus>,
    sent_to_gateway: Option<bool>,
    refund_error_message: Option<String>,
    refund_arn: Option<String>,
    metadata: Option<pii::SecretSerdeValue>,
    refund_reason: Option<String>,
    refund_error_code: Option<String>,
}

impl RefundUpdateInternal {
    pub fn create_refund(self, source: Refund) -> Refund {
        Refund {
            connector_refund_id: self.connector_refund_id,
            refund_status: self.refund_status.unwrap_or_default(),
            sent_to_gateway: self.sent_to_gateway.unwrap_or_default(),
            refund_error_message: self.refund_error_message,
            refund_arn: self.refund_arn,
            metadata: self.metadata,
            refund_reason: self.refund_reason,
            refund_error_code: self.refund_error_code,
            ..source
        }
    }
}

impl From<RefundUpdate> for RefundUpdateInternal {
    fn from(refund_update: RefundUpdate) -> Self {
        match refund_update {
            RefundUpdate::Update {
                connector_refund_id,
                refund_status,
                sent_to_gateway,
                refund_error_message,
                refund_arn,
            } => Self {
                connector_refund_id: Some(connector_refund_id),
                refund_status: Some(refund_status),
                sent_to_gateway: Some(sent_to_gateway),
                refund_error_message,
                refund_arn: Some(refund_arn),
                ..Default::default()
            },
            RefundUpdate::MetadataAndReasonUpdate { metadata, reason } => Self {
                metadata,
                refund_reason: reason,
                ..Default::default()
            },
            RefundUpdate::StatusUpdate {
                connector_refund_id,
                sent_to_gateway,
                refund_status,
            } => Self {
                connector_refund_id,
                sent_to_gateway: Some(sent_to_gateway),
                refund_status: Some(refund_status),
                ..Default::default()
            },
            RefundUpdate::ErrorUpdate {
                refund_status,
                refund_error_message,
                refund_error_code,
            } => Self {
                refund_status,
                refund_error_message,
                refund_error_code,
                ..Default::default()
            },
        }
    }
}

impl RefundUpdate {
    pub fn apply_changeset(self, source: Refund) -> Refund {
        let pa_update: RefundUpdateInternal = self.into();
        Refund {
            connector_refund_id: pa_update.connector_refund_id.or(source.connector_refund_id),
            refund_status: pa_update.refund_status.unwrap_or(source.refund_status),
            sent_to_gateway: pa_update.sent_to_gateway.unwrap_or(source.sent_to_gateway),
            refund_error_message: pa_update
                .refund_error_message
                .or(source.refund_error_message),
            refund_error_code: pa_update.refund_error_code.or(source.refund_error_code),
            refund_arn: pa_update.refund_arn.or(source.refund_arn),
            metadata: pa_update.metadata.or(source.metadata),
            refund_reason: pa_update.refund_reason.or(source.refund_reason),
            ..source
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RefundCoreWorkflow {
    pub refund_internal_reference_id: String,
    pub connector_transaction_id: String,
    pub merchant_id: String,
    pub payment_id: String,
}

#[async_trait::async_trait]
pub trait RefundDbExt: Sized {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::refunds::RefundListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError>;

    async fn filter_by_meta_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::refunds::TimeRange,
    ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::DatabaseError>;
}

#[async_trait::async_trait]
impl RefundDbExt for Refund {
    async fn filter_by_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::refunds::RefundListRequest,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<Self>, errors::DatabaseError> {
        let mut filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::modified_at.desc())
            .into_boxed();

        match &refund_list_details.payment_id {
            Some(pid) => {
                filter = filter.filter(dsl::payment_id.eq(pid.to_owned()));
            }
            None => {
                filter = filter.limit(limit).offset(offset);
            }
        };

        if let Some(time_range) = refund_list_details.time_range {
            filter = filter.filter(dsl::created_at.ge(time_range.start_time));

            if let Some(end_time) = time_range.end_time {
                filter = filter.filter(dsl::created_at.le(end_time));
            }
        }

        if let Some(connector) = refund_list_details.clone().connector {
            filter = filter.filter(dsl::connector.eq_any(connector));
        }

        if let Some(filter_currency) = &refund_list_details.currency {
            filter = filter.filter(dsl::currency.eq_any(filter_currency.clone()));
        }

        if let Some(filter_refund_status) = &refund_list_details.refund_status {
            filter = filter.filter(dsl::refund_status.eq_any(filter_refund_status.clone()));
        }

        logger::debug!(query = %diesel::debug_query::<diesel::pg::Pg, _>(&filter).to_string());

        filter
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::NotFound)
            .attach_printable_lazy(|| "Error filtering records by predicate")
    }

    async fn filter_by_meta_constraints(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_list_details: &api_models::refunds::TimeRange,
    ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::DatabaseError> {
        let start_time = refund_list_details.start_time;

        let end_time = refund_list_details
            .end_time
            .unwrap_or_else(common_utils::date_time::now);

        let filter = <Self as HasTable>::table()
            .filter(dsl::merchant_id.eq(merchant_id.to_owned()))
            .order(dsl::modified_at.desc())
            .filter(dsl::created_at.ge(start_time))
            .filter(dsl::created_at.le(end_time));

        let filter_connector: Vec<String> = filter
            .clone()
            .select(dsl::connector)
            .distinct()
            .order_by(dsl::connector.asc())
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error filtering records by connector")?;

        let filter_currency: Vec<storage_enums::Currency> = filter
            .clone()
            .select(dsl::currency)
            .distinct()
            .order_by(dsl::currency.asc())
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error filtering records by currency")?;

        let filter_status: Vec<storage_enums::RefundStatus> = filter
            .select(dsl::refund_status)
            .distinct()
            .order_by(dsl::refund_status.asc())
            .get_results_async(conn)
            .await
            .into_report()
            .change_context(errors::DatabaseError::Others)
            .attach_printable("Error filtering records by refund status")?;

        let meta = api_models::refunds::RefundListMetaData {
            connector: filter_connector,
            currency: filter_currency,
            status: filter_status,
        };

        Ok(meta)
    }
}
