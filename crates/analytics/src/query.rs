use std::marker::PhantomData;

use api_models::{
    analytics::{
        self as analytics_api,
        api_event::ApiEventDimensions,
        disputes::DisputeDimensions,
        payments::{PaymentDimensions, PaymentDistributions},
        refunds::{RefundDimensions, RefundType},
        sdk_events::{SdkEventDimensions, SdkEventNames},
        Granularity,
    },
    enums::{
        AttemptStatus, AuthenticationType, Connector, Currency, DisputeStage, PaymentMethod,
        PaymentMethodType,
    },
    refunds::RefundStatus,
};
use common_utils::errors::{CustomResult, ParsingError};
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use router_env::{logger, Flow};

use super::types::{AnalyticsCollection, AnalyticsDataSource, LoadRow, TableEngine};
use crate::types::QueryExecutionError;
pub type QueryResult<T> = error_stack::Result<T, QueryBuildingError>;
pub trait QueryFilter<T>
where
    T: AnalyticsDataSource,
    AnalyticsCollection: ToSql<T>,
{
    fn set_filter_clause(&self, builder: &mut QueryBuilder<T>) -> QueryResult<()>;
}

pub trait GroupByClause<T>
where
    T: AnalyticsDataSource,
    AnalyticsCollection: ToSql<T>,
{
    fn set_group_by_clause(&self, builder: &mut QueryBuilder<T>) -> QueryResult<()>;
}

pub trait SeriesBucket {
    type SeriesType;
    type GranularityLevel;

    fn get_lowest_common_granularity_level(&self) -> Self::GranularityLevel;

    fn get_bucket_size(&self) -> u8;

    fn clip_to_start(
        &self,
        value: Self::SeriesType,
    ) -> error_stack::Result<Self::SeriesType, PostProcessingError>;

    fn clip_to_end(
        &self,
        value: Self::SeriesType,
    ) -> error_stack::Result<Self::SeriesType, PostProcessingError>;
}

impl<T> QueryFilter<T> for analytics_api::TimeRange
where
    T: AnalyticsDataSource,
    time::PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
{
    fn set_filter_clause(&self, builder: &mut QueryBuilder<T>) -> QueryResult<()> {
        builder.add_custom_filter_clause("created_at", self.start_time, FilterTypes::Gte)?;
        if let Some(end) = self.end_time {
            builder.add_custom_filter_clause("created_at", end, FilterTypes::Lte)?;
        }
        Ok(())
    }
}

impl GroupByClause<super::SqlxClient> for Granularity {
    fn set_group_by_clause(
        &self,
        builder: &mut QueryBuilder<super::SqlxClient>,
    ) -> QueryResult<()> {
        let trunc_scale = self.get_lowest_common_granularity_level();

        let granularity_bucket_scale = match self {
            Self::OneMin => None,
            Self::FiveMin | Self::FifteenMin | Self::ThirtyMin => Some("minute"),
            Self::OneHour | Self::OneDay => None,
        };

        let granularity_divisor = self.get_bucket_size();

        builder
            .add_group_by_clause(format!("DATE_TRUNC('{trunc_scale}', created_at)"))
            .attach_printable("Error adding time prune group by")?;
        if let Some(scale) = granularity_bucket_scale {
            builder
                .add_group_by_clause(format!(
                    "FLOOR(DATE_PART('{scale}', created_at)/{granularity_divisor})"
                ))
                .attach_printable("Error adding time binning group by")?;
        }
        Ok(())
    }
}

impl GroupByClause<super::ClickhouseClient> for Granularity {
    fn set_group_by_clause(
        &self,
        builder: &mut QueryBuilder<super::ClickhouseClient>,
    ) -> QueryResult<()> {
        let interval = match self {
            Self::OneMin => "toStartOfMinute(created_at)",
            Self::FiveMin => "toStartOfFiveMinutes(created_at)",
            Self::FifteenMin => "toStartOfFifteenMinutes(created_at)",
            Self::ThirtyMin => "toStartOfInterval(created_at, INTERVAL 30 minute)",
            Self::OneHour => "toStartOfHour(created_at)",
            Self::OneDay => "toStartOfDay(created_at)",
        };

        builder
            .add_group_by_clause(interval)
            .attach_printable("Error adding interval group by")
    }
}

#[derive(strum::Display)]
#[strum(serialize_all = "lowercase")]
pub enum TimeGranularityLevel {
    Minute,
    Hour,
    Day,
}

impl SeriesBucket for Granularity {
    type SeriesType = time::PrimitiveDateTime;

    type GranularityLevel = TimeGranularityLevel;

    fn get_lowest_common_granularity_level(&self) -> Self::GranularityLevel {
        match self {
            Self::OneMin => TimeGranularityLevel::Minute,
            Self::FiveMin | Self::FifteenMin | Self::ThirtyMin | Self::OneHour => {
                TimeGranularityLevel::Hour
            }
            Self::OneDay => TimeGranularityLevel::Day,
        }
    }

    fn get_bucket_size(&self) -> u8 {
        match self {
            Self::OneMin => 60,
            Self::FiveMin => 5,
            Self::FifteenMin => 15,
            Self::ThirtyMin => 30,
            Self::OneHour => 60,
            Self::OneDay => 24,
        }
    }

    fn clip_to_start(
        &self,
        value: Self::SeriesType,
    ) -> error_stack::Result<Self::SeriesType, PostProcessingError> {
        let clip_start = |value: u8, modulo: u8| -> u8 { value - value % modulo };

        let clipped_time = match (
            self.get_lowest_common_granularity_level(),
            self.get_bucket_size(),
        ) {
            (TimeGranularityLevel::Minute, i) => time::Time::MIDNIGHT
                .replace_second(clip_start(value.second(), i))
                .and_then(|t| t.replace_minute(value.minute()))
                .and_then(|t| t.replace_hour(value.hour())),
            (TimeGranularityLevel::Hour, i) => time::Time::MIDNIGHT
                .replace_minute(clip_start(value.minute(), i))
                .and_then(|t| t.replace_hour(value.hour())),
            (TimeGranularityLevel::Day, i) => {
                time::Time::MIDNIGHT.replace_hour(clip_start(value.hour(), i))
            }
        }
        .change_context(PostProcessingError::BucketClipping)?;

        Ok(value.replace_time(clipped_time))
    }

    fn clip_to_end(
        &self,
        value: Self::SeriesType,
    ) -> error_stack::Result<Self::SeriesType, PostProcessingError> {
        let clip_end = |value: u8, modulo: u8| -> u8 { value + modulo - 1 - value % modulo };

        let clipped_time = match (
            self.get_lowest_common_granularity_level(),
            self.get_bucket_size(),
        ) {
            (TimeGranularityLevel::Minute, i) => time::Time::MIDNIGHT
                .replace_second(clip_end(value.second(), i))
                .and_then(|t| t.replace_minute(value.minute()))
                .and_then(|t| t.replace_hour(value.hour())),
            (TimeGranularityLevel::Hour, i) => time::Time::MIDNIGHT
                .replace_minute(clip_end(value.minute(), i))
                .and_then(|t| t.replace_hour(value.hour())),
            (TimeGranularityLevel::Day, i) => {
                time::Time::MIDNIGHT.replace_hour(clip_end(value.hour(), i))
            }
        }
        .change_context(PostProcessingError::BucketClipping)
        .attach_printable_lazy(|| format!("Bucket Clip Error: {value}"))?;

        Ok(value.replace_time(clipped_time))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum QueryBuildingError {
    #[allow(dead_code)]
    #[error("Not Implemented: {0}")]
    NotImplemented(String),
    #[error("Failed to Serialize to SQL")]
    SqlSerializeError,
    #[error("Failed to build sql query: {0}")]
    InvalidQuery(&'static str),
}

#[derive(thiserror::Error, Debug)]
pub enum PostProcessingError {
    #[error("Error Clipping values to bucket sizes")]
    BucketClipping,
}

#[derive(Debug)]
pub enum Aggregate<R> {
    Count {
        field: Option<R>,
        alias: Option<&'static str>,
    },
    Sum {
        field: R,
        alias: Option<&'static str>,
    },
    Min {
        field: R,
        alias: Option<&'static str>,
    },
    Max {
        field: R,
        alias: Option<&'static str>,
    },
}

// Window functions in query
// ---
// Description -
// field: to_sql type value used as expr in aggregation
// partition_by: partition by fields in window
// order_by: order by fields and order (Ascending / Descending) in window
// alias: alias of window expr in query
// ---
// Usage -
// Window::Sum {
//     field: "count",
//     partition_by: Some(query_builder.transform_to_sql_values(&dimensions).switch()?),
//     order_by: Some(("value", Descending)),
//     alias: Some("total"),
// }
#[derive(Debug)]
pub enum Window<R> {
    Sum {
        field: R,
        partition_by: Option<String>,
        order_by: Option<(String, Order)>,
        alias: Option<&'static str>,
    },
    RowNumber {
        field: R,
        partition_by: Option<String>,
        order_by: Option<(String, Order)>,
        alias: Option<&'static str>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Order {
    Ascending,
    Descending,
}

impl std::fmt::Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ascending => write!(f, "asc"),
            Self::Descending => write!(f, "desc"),
        }
    }
}

// Select TopN values for a group based on a metric
// ---
// Description -
// columns: Columns in group to select TopN values for
// count: N in TopN
// order_column: metric used to sort and limit TopN
// order: sort order of metric (Ascending / Descending)
// ---
// Usage -
// Use via add_top_n_clause fn of query_builder
// add_top_n_clause(
//     &dimensions,
//     distribution.distribution_cardinality.into(),
//     "count",
//     Order::Descending,
// )
#[derive(Debug)]
pub struct TopN {
    pub columns: String,
    pub count: u64,
    pub order_column: String,
    pub order: Order,
}

#[derive(Debug)]
pub struct QueryBuilder<T>
where
    T: AnalyticsDataSource,
    AnalyticsCollection: ToSql<T>,
{
    columns: Vec<String>,
    filters: Vec<(String, FilterTypes, String)>,
    group_by: Vec<String>,
    having: Option<Vec<(String, FilterTypes, String)>>,
    outer_select: Vec<String>,
    top_n: Option<TopN>,
    table: AnalyticsCollection,
    distinct: bool,
    db_type: PhantomData<T>,
    table_engine: TableEngine,
}

pub trait ToSql<T: AnalyticsDataSource> {
    fn to_sql(&self, table_engine: &TableEngine) -> error_stack::Result<String, ParsingError>;
}

/// Implement `ToSql` on arrays of types that impl `ToString`.
macro_rules! impl_to_sql_for_to_string {
    ($($type:ty),+) => {
        $(
            impl<T: AnalyticsDataSource> ToSql<T> for $type {
                fn to_sql(&self, _table_engine: &TableEngine) -> error_stack::Result<String, ParsingError> {
                    Ok(self.to_string())
                }
            }
        )+
     };
}

impl_to_sql_for_to_string!(
    String,
    &str,
    &PaymentDimensions,
    &RefundDimensions,
    PaymentDimensions,
    &PaymentDistributions,
    RefundDimensions,
    PaymentMethod,
    PaymentMethodType,
    AuthenticationType,
    Connector,
    AttemptStatus,
    RefundStatus,
    storage_enums::RefundStatus,
    Currency,
    RefundType,
    Flow,
    &String,
    &bool,
    &u64,
    u64,
    Order
);

impl_to_sql_for_to_string!(&SdkEventDimensions, SdkEventDimensions, SdkEventNames);

impl_to_sql_for_to_string!(&ApiEventDimensions, ApiEventDimensions);

impl_to_sql_for_to_string!(&DisputeDimensions, DisputeDimensions, DisputeStage);

#[derive(Debug)]
pub enum FilterTypes {
    Equal,
    NotEqual,
    EqualBool,
    In,
    Gte,
    Lte,
    Gt,
    Like,
    NotLike,
    IsNotNull,
}

pub fn filter_type_to_sql(l: &String, op: &FilterTypes, r: &String) -> String {
    match op {
        FilterTypes::EqualBool => format!("{l} = {r}"),
        FilterTypes::Equal => format!("{l} = '{r}'"),
        FilterTypes::NotEqual => format!("{l} != '{r}'"),
        FilterTypes::In => format!("{l} IN ({r})"),
        FilterTypes::Gte => format!("{l} >= '{r}'"),
        FilterTypes::Gt => format!("{l} > {r}"),
        FilterTypes::Lte => format!("{l} <= '{r}'"),
        FilterTypes::Like => format!("{l} LIKE '%{r}%'"),
        FilterTypes::NotLike => format!("{l} NOT LIKE '%{r}%'"),
        FilterTypes::IsNotNull => format!("{l} IS NOT NULL"),
    }
}

impl<T> QueryBuilder<T>
where
    T: AnalyticsDataSource,
    AnalyticsCollection: ToSql<T>,
{
    pub fn new(table: AnalyticsCollection) -> Self {
        Self {
            columns: Default::default(),
            filters: Default::default(),
            group_by: Default::default(),
            having: Default::default(),
            outer_select: Default::default(),
            top_n: Default::default(),
            table,
            distinct: Default::default(),
            db_type: Default::default(),
            table_engine: T::get_table_engine(table),
        }
    }

    pub fn add_select_column(&mut self, column: impl ToSql<T>) -> QueryResult<()> {
        self.columns.push(
            column
                .to_sql(&self.table_engine)
                .change_context(QueryBuildingError::SqlSerializeError)
                .attach_printable("Error serializing select column")?,
        );
        Ok(())
    }

    pub fn transform_to_sql_values(&mut self, values: &[impl ToSql<T>]) -> QueryResult<String> {
        let res = values
            .iter()
            .map(|i| i.to_sql(&self.table_engine))
            .collect::<error_stack::Result<Vec<String>, ParsingError>>()
            .change_context(QueryBuildingError::SqlSerializeError)
            .attach_printable("Error serializing range filter value")?
            .join(", ");
        Ok(res)
    }

    pub fn add_top_n_clause(
        &mut self,
        columns: &[impl ToSql<T>],
        count: u64,
        order_column: impl ToSql<T>,
        order: Order,
    ) -> QueryResult<()>
    where
        Window<&'static str>: ToSql<T>,
    {
        let partition_by_columns = self.transform_to_sql_values(columns)?;
        let order_by_column = order_column
            .to_sql(&self.table_engine)
            .change_context(QueryBuildingError::SqlSerializeError)
            .attach_printable("Error serializing select column")?;

        self.add_outer_select_column(Window::RowNumber {
            field: "",
            partition_by: Some(partition_by_columns.clone()),
            order_by: Some((order_by_column.clone(), order)),
            alias: Some("top_n"),
        })?;

        self.top_n = Some(TopN {
            columns: partition_by_columns,
            count,
            order_column: order_by_column,
            order,
        });
        Ok(())
    }

    pub fn set_distinct(&mut self) {
        self.distinct = true
    }

    pub fn add_filter_clause(
        &mut self,
        key: impl ToSql<T>,
        value: impl ToSql<T>,
    ) -> QueryResult<()> {
        self.add_custom_filter_clause(key, value, FilterTypes::Equal)
    }

    pub fn add_bool_filter_clause(
        &mut self,
        key: impl ToSql<T>,
        value: impl ToSql<T>,
    ) -> QueryResult<()> {
        self.add_custom_filter_clause(key, value, FilterTypes::EqualBool)
    }

    pub fn add_custom_filter_clause(
        &mut self,
        lhs: impl ToSql<T>,
        rhs: impl ToSql<T>,
        comparison: FilterTypes,
    ) -> QueryResult<()> {
        self.filters.push((
            lhs.to_sql(&self.table_engine)
                .change_context(QueryBuildingError::SqlSerializeError)
                .attach_printable("Error serializing filter key")?,
            comparison,
            rhs.to_sql(&self.table_engine)
                .change_context(QueryBuildingError::SqlSerializeError)
                .attach_printable("Error serializing filter value")?,
        ));
        Ok(())
    }

    pub fn add_filter_in_range_clause(
        &mut self,
        key: impl ToSql<T>,
        values: &[impl ToSql<T>],
    ) -> QueryResult<()> {
        let list = values
            .iter()
            .map(|i| {
                // trimming whitespaces from the filter values received in request, to prevent a possibility of an SQL injection
                i.to_sql(&self.table_engine).map(|s| {
                    let trimmed_str = s.replace(' ', "");
                    format!("'{trimmed_str}'")
                })
            })
            .collect::<error_stack::Result<Vec<String>, ParsingError>>()
            .change_context(QueryBuildingError::SqlSerializeError)
            .attach_printable("Error serializing range filter value")?
            .join(", ");
        self.add_custom_filter_clause(key, list, FilterTypes::In)
    }

    pub fn add_group_by_clause(&mut self, column: impl ToSql<T>) -> QueryResult<()> {
        self.group_by.push(
            column
                .to_sql(&self.table_engine)
                .change_context(QueryBuildingError::SqlSerializeError)
                .attach_printable("Error serializing group by field")?,
        );
        Ok(())
    }

    pub fn add_granularity_in_mins(&mut self, granularity: &Granularity) -> QueryResult<()> {
        let interval = match granularity {
            Granularity::OneMin => "1",
            Granularity::FiveMin => "5",
            Granularity::FifteenMin => "15",
            Granularity::ThirtyMin => "30",
            Granularity::OneHour => "60",
            Granularity::OneDay => "1440",
        };
        let _ = self.add_select_column(format!(
            "toStartOfInterval(created_at, INTERVAL {interval} MINUTE) as time_bucket"
        ));
        Ok(())
    }

    fn get_filter_clause(&self) -> String {
        self.filters
            .iter()
            .map(|(l, op, r)| filter_type_to_sql(l, op, r))
            .collect::<Vec<String>>()
            .join(" AND ")
    }

    fn get_select_clause(&self) -> String {
        self.columns.join(", ")
    }

    fn get_group_by_clause(&self) -> String {
        self.group_by.join(", ")
    }

    fn get_outer_select_clause(&self) -> String {
        self.outer_select.join(", ")
    }

    pub fn add_having_clause<R>(
        &mut self,
        aggregate: Aggregate<R>,
        filter_type: FilterTypes,
        value: impl ToSql<T>,
    ) -> QueryResult<()>
    where
        Aggregate<R>: ToSql<T>,
    {
        let aggregate = aggregate
            .to_sql(&self.table_engine)
            .change_context(QueryBuildingError::SqlSerializeError)
            .attach_printable("Error serializing having aggregate")?;
        let value = value
            .to_sql(&self.table_engine)
            .change_context(QueryBuildingError::SqlSerializeError)
            .attach_printable("Error serializing having value")?;
        let entry = (aggregate, filter_type, value);
        if let Some(having) = &mut self.having {
            having.push(entry);
        } else {
            self.having = Some(vec![entry]);
        }
        Ok(())
    }

    pub fn add_outer_select_column(&mut self, column: impl ToSql<T>) -> QueryResult<()> {
        self.outer_select.push(
            column
                .to_sql(&self.table_engine)
                .change_context(QueryBuildingError::SqlSerializeError)
                .attach_printable("Error serializing outer select column")?,
        );
        Ok(())
    }

    pub fn get_filter_type_clause(&self) -> Option<String> {
        self.having.as_ref().map(|vec| {
            vec.iter()
                .map(|(l, op, r)| filter_type_to_sql(l, op, r))
                .collect::<Vec<String>>()
                .join(" AND ")
        })
    }

    pub fn build_query(&mut self) -> QueryResult<String>
    where
        Aggregate<&'static str>: ToSql<T>,
        Window<&'static str>: ToSql<T>,
    {
        if self.columns.is_empty() {
            Err(QueryBuildingError::InvalidQuery(
                "No select fields provided",
            ))?;
        }
        let mut query = String::from("SELECT ");

        if self.distinct {
            query.push_str("DISTINCT ");
        }

        query.push_str(&self.get_select_clause());

        query.push_str(" FROM ");

        query.push_str(
            &self
                .table
                .to_sql(&self.table_engine)
                .change_context(QueryBuildingError::SqlSerializeError)
                .attach_printable("Error serializing table value")?,
        );

        if !self.filters.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&self.get_filter_clause());
        }

        if !self.group_by.is_empty() {
            query.push_str(" GROUP BY ");
            query.push_str(&self.get_group_by_clause());
            if let TableEngine::CollapsingMergeTree { sign } = self.table_engine {
                self.add_having_clause(
                    Aggregate::Count {
                        field: Some(sign),
                        alias: None,
                    },
                    FilterTypes::Gte,
                    "1",
                )?;
            }
        }

        if self.having.is_some() {
            if let Some(condition) = self.get_filter_type_clause() {
                query.push_str(" HAVING ");
                query.push_str(condition.as_str());
            }
        }

        if !self.outer_select.is_empty() {
            query.insert_str(
                0,
                format!("SELECT {} FROM (", &self.get_outer_select_clause()).as_str(),
            );
            query.push_str(") _");
        }

        if let Some(top_n) = &self.top_n {
            query.insert_str(0, "SELECT * FROM (");
            query.push_str(format!(") _ WHERE top_n <= {}", top_n.count).as_str());
        }

        println!("{}", query);

        Ok(query)
    }

    pub async fn execute_query<R, P>(
        &mut self,
        store: &P,
    ) -> CustomResult<CustomResult<Vec<R>, QueryExecutionError>, QueryBuildingError>
    where
        P: LoadRow<R> + AnalyticsDataSource,
        Aggregate<&'static str>: ToSql<T>,
        Window<&'static str>: ToSql<T>,
    {
        let query = self
            .build_query()
            .change_context(QueryBuildingError::SqlSerializeError)
            .attach_printable("Failed to execute query")?;
        logger::debug!(?query);
        Ok(store.load_results(query.as_str()).await)
    }
}
