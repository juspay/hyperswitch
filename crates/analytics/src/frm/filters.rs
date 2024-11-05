use api_models::analytics::{
    frm::{FrmDimensions, FrmTransactionType},
    Granularity, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use diesel_models::enums::FraudCheckStatus;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, ToSql, Window},
    types::{
        AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, FiltersError, FiltersResult,
        LoadRow,
    },
};
pub trait FrmFilterAnalytics: LoadRow<FrmFilterRow> {}

pub async fn get_frm_filter_for_dimension<T>(
    dimension: FrmDimensions,
    merchant_id: &common_utils::id_type::MerchantId,
    time_range: &TimeRange,
    pool: &T,
) -> FiltersResult<Vec<FrmFilterRow>>
where
    T: AnalyticsDataSource + FrmFilterAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::FraudCheck);

    query_builder.add_select_column(dimension).switch()?;
    time_range
        .set_filter_clause(&mut query_builder)
        .attach_printable("Error filtering time range")
        .switch()?;

    query_builder
        .add_filter_clause("merchant_id", merchant_id)
        .switch()?;

    query_builder.set_distinct();

    query_builder
        .execute_query::<FrmFilterRow, _>(pool)
        .await
        .change_context(FiltersError::QueryBuildingError)?
        .change_context(FiltersError::QueryExecutionFailure)
}

#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct FrmFilterRow {
    pub frm_status: Option<DBEnumWrapper<FraudCheckStatus>>,
    pub frm_transaction_type: Option<DBEnumWrapper<FrmTransactionType>>,
    pub frm_name: Option<String>,
}
