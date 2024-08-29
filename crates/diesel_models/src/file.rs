use common_utils::custom_serde;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use masking::{Deserialize, Serialize};

use crate::schema::file_metadata;

#[derive(Clone, Debug, Deserialize, Insertable, Serialize, router_derive::DebugAsDisplay)]
#[diesel(table_name = file_metadata)]
#[serde(deny_unknown_fields)]
pub struct FileMetadataNew {
    pub file_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub file_name: Option<String>,
    pub file_size: i32,
    pub file_type: String,
    pub provider_file_id: Option<String>,
    pub file_upload_provider: Option<common_enums::FileUploadProvider>,
    pub available: bool,
    pub connector_label: Option<String>,
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Identifiable, Queryable, Selectable)]
#[diesel(table_name = file_metadata, primary_key(file_id, merchant_id), check_for_backend(diesel::pg::Pg))]
pub struct FileMetadata {
    #[serde(skip_serializing)]
    pub file_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub file_name: Option<String>,
    pub file_size: i32,
    pub file_type: String,
    pub provider_file_id: Option<String>,
    pub file_upload_provider: Option<common_enums::FileUploadProvider>,
    pub available: bool,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    pub connector_label: Option<String>,
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

#[derive(Debug)]
pub enum FileMetadataUpdate {
    Update {
        provider_file_id: Option<String>,
        file_upload_provider: Option<common_enums::FileUploadProvider>,
        available: bool,
        profile_id: Option<common_utils::id_type::ProfileId>,
        merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = file_metadata)]
pub struct FileMetadataUpdateInternal {
    provider_file_id: Option<String>,
    file_upload_provider: Option<common_enums::FileUploadProvider>,
    available: bool,
    profile_id: Option<common_utils::id_type::ProfileId>,
    merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

impl From<FileMetadataUpdate> for FileMetadataUpdateInternal {
    fn from(merchant_account_update: FileMetadataUpdate) -> Self {
        match merchant_account_update {
            FileMetadataUpdate::Update {
                provider_file_id,
                file_upload_provider,
                available,
                profile_id,
                merchant_connector_id,
            } => Self {
                provider_file_id,
                file_upload_provider,
                available,
                profile_id,
                merchant_connector_id,
            },
        }
    }
}
