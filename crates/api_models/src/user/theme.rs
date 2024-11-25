use actix_multipart::form::{bytes::Bytes, json::Json, text::Text, MultipartForm};
use common_enums::EntityType;
use common_utils::{id_type, types::theme::ThemeLineage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct GetThemeResponse {
    pub theme_id: String,
    pub theme_name: String,
    pub entity_type: EntityType,
    pub tenant_id: String,
    pub org_id: Option<id_type::OrganizationId>,
    pub merchant_id: Option<id_type::MerchantId>,
    pub profile_id: Option<id_type::ProfileId>,
    pub theme_data: ThemeData,
}

#[derive(Debug, MultipartForm)]
pub struct RawUploadFileRequest {
    #[multipart]
    pub lineage: Json<ThemeLineage>,
    #[multipart]
    pub asset_name: Text<String>,
    #[multipart(limit = "100MB")]
    pub asset_data: Bytes,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UploadFileRequest {
    pub lineage: ThemeLineage,
    pub asset_name: String,
    pub asset_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateThemeRequest {
    pub lineage: ThemeLineage,
    pub theme_name: String,
    pub theme_data: ThemeData,
}

// All the below structs are for the theme.json file,
// which will be used by frontend to style the dashboard.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ThemeData {
    pub settings: Settings,
    pub urls: Option<Urls>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub colors: Colors,
    pub typography: Option<Typography>,
    pub buttons: Buttons,
    pub borders: Option<Borders>,
    pub spacing: Option<Spacing>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Colors {
    pub primary: String,
    pub sidebar: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Typography {
    pub font_family: Option<String>,
    pub font_size: Option<String>,
    pub heading_font_size: Option<String>,
    pub text_color: Option<String>,
    pub link_color: Option<String>,
    pub link_hover_color: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Buttons {
    pub primary: PrimaryButton,
    pub secondary: Option<SecondaryButton>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PrimaryButton {
    pub background_color: Option<String>,
    pub text_color: Option<String>,
    pub hover_background_color: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SecondaryButton {
    pub background_color: Option<String>,
    pub text_color: Option<String>,
    pub hover_background_color: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Borders {
    pub default_radius: Option<String>,
    pub border_color: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Spacing {
    pub padding: Option<String>,
    pub margin: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Urls {
    pub favicon_url: Option<String>,
    pub logo_url: Option<String>,
}
