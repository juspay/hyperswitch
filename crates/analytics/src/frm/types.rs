use api_models::analytics::frm::{FrmDimensions, FrmFilters};
use error_stack::ResultExt;

use crate::{
    query::{QueryBuilder, QueryFilter, QueryResult, ToSql},
    types::{AnalyticsCollection, AnalyticsDataSource},
};

impl<T> QueryFilter<T> for FrmFilters
where
    T: AnalyticsDataSource,
    AnalyticsCollection: ToSql<T>,
{
    fn set_filter_clause(&self, builder: &mut QueryBuilder<T>) -> QueryResult<()> {
        if !self.frm_status.is_empty() {
            builder
                .add_filter_in_range_clause(FrmDimensions::FrmStatus, &self.frm_status)
                .attach_printable("Error adding frm status filter")?;
        }

        if !self.frm_name.is_empty() {
            builder
                .add_filter_in_range_clause(FrmDimensions::FrmName, &self.frm_name)
                .attach_printable("Error adding frm name filter")?;
        }

        if !self.frm_transaction_type.is_empty() {
            builder
                .add_filter_in_range_clause(
                    FrmDimensions::FrmTransactionType,
                    &self.frm_transaction_type,
                )
                .attach_printable("Error adding frm transaction type filter")?;
        }

        Ok(())
    }
}
