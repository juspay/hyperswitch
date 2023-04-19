use common_utils::custom_serde;
use diesel::{Identifiable, Insertable, Queryable};
use masking::{Deserialize, Serialize};

use crate::{enums as storage_enums, schema::file_metadata};

#[derive(Clone, Debug, Deserialize, Insertable, Serialize, router_derive::DebugAsDisplay)]
#[diesel(table_name = file_metadata)]
#[serde(deny_unknown_fields)]
pub struct FileNew {
    pub file_id: String,
    pub merchant_id: String,
    pub file_name: Option<String>,
    pub file_size: i32,
    pub file_type: String,
    pub provider_file_id: String,
    pub file_upload_provider: storage_enums::FileUploadProvider,
    pub available: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, Identifiable, Queryable)]
#[diesel(table_name = file_metadata)]
pub struct File {
    #[serde(skip_serializing)]
    pub id: i32,
    pub file_id: String,
    pub merchant_id: String,
    pub file_name: Option<String>,
    pub file_size: i32,
    pub file_type: String,
    pub provider_file_id: String,
    pub file_upload_provider: storage_enums::FileUploadProvider,
    pub available: bool,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
}
