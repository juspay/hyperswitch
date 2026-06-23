use common_enums::BatchBlocklistJobStatus;
use common_utils::id_type;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::schema::batch_blocklist_jobs;

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Deserialize, Serialize)]
#[diesel(table_name = batch_blocklist_jobs, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct BatchBlocklistJob {
    pub id: String,
    pub merchant_id: id_type::MerchantId,
    pub status: BatchBlocklistJobStatus,
    pub total_rows: i32,
    pub succeeded_rows: i32,
    pub failed_rows: i32,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable, Deserialize, Serialize)]
#[diesel(table_name = batch_blocklist_jobs)]
pub struct BatchBlocklistJobNew {
    pub id: String,
    pub merchant_id: id_type::MerchantId,
    pub status: BatchBlocklistJobStatus,
    pub total_rows: i32,
    pub succeeded_rows: i32,
    pub failed_rows: i32,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = batch_blocklist_jobs)]
pub struct BatchBlocklistJobUpdate {
    pub status: Option<BatchBlocklistJobStatus>,
    pub succeeded_rows: Option<i32>,
    pub failed_rows: Option<i32>,
    pub updated_at: PrimitiveDateTime,
}
