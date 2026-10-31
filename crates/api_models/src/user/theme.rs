use actix_multipart::form::{bytes::Bytes, text::Text, MultipartForm};
use common_enums::EntityType;
use common_types::{primitive_wrappers::SafeString, user::theme::EmailThemeConfig};
use common_utils::{
    id_type,
    types::{user::ThemeLineage, Url},
};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct GetThemeResponse {
    pub theme_id: String,
    pub theme_name: String,
    pub entity_type: EntityType,
    pub tenant_id: id_type::TenantId,
    pub org_id: Option<id_type::OrganizationId>,
    pub merchant_id: Option<id_type::MerchantId>,
    pub profile_id: Option<id_type::ProfileId>,
    pub email_config: EmailThemeConfig,
    pub theme_data: ThemeData,
}

#[derive(Debug, Serialize)]
pub struct ThemeVersionResponse {
    pub theme_id: String,
    pub theme_config_version: String,
}

#[derive(Debug, MultipartForm)]
pub struct UploadFileAssetData {
    pub asset_name: Text<SafeString>,
    #[multipart(limit = "10MB")]
    pub asset_data: Bytes,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UploadFileRequest {
    pub asset_name: SafeString,
    pub asset_data: Secret<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateThemeRequest {
    pub lineage: ThemeLineage,
    pub theme_name: SafeString,
    pub theme_data: ThemeData,
    pub email_config: Option<EmailThemeConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateUserThemeRequest {
    pub entity_type: EntityType,
    pub theme_name: SafeString,
    pub theme_data: ThemeData,
    pub email_config: Option<EmailThemeConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateThemeRequest {
    pub theme_data: Option<ThemeData>,
    pub email_config: Option<EmailThemeConfig>,
}

// All the below structs are for the theme.json file,
// which will be used by frontend to style the dashboard.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ThemeData {
    settings: Settings,
    urls: Option<Urls>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Settings {
    colors: Colors,
    sidebar: Option<Sidebar>,
    typography: Option<Typography>,
    buttons: Buttons,
    borders: Option<Borders>,
    spacing: Option<Spacing>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Colors {
    primary: SafeString,
    secondary: Option<SafeString>,
    background: Option<SafeString>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Sidebar {
    primary: SafeString,
    text_color: Option<SafeString>,
    text_color_primary: Option<SafeString>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Typography {
    font_family: Option<SafeString>,
    font_size: Option<SafeString>,
    heading_font_size: Option<SafeString>,
    text_color: Option<SafeString>,
    link_color: Option<SafeString>,
    link_hover_color: Option<SafeString>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Buttons {
    primary: PrimaryButton,
    secondary: Option<SecondaryButton>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PrimaryButton {
    background_color: Option<SafeString>,
    text_color: Option<SafeString>,
    hover_background_color: SafeString,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SecondaryButton {
    background_color: Option<SafeString>,
    text_color: Option<SafeString>,
    hover_background_color: Option<SafeString>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Borders {
    default_radius: Option<SafeString>,
    border_color: Option<SafeString>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Spacing {
    padding: Option<SafeString>,
    margin: Option<SafeString>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Urls {
    favicon_url: Option<Url>,
    logo_url: Option<Url>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct EntityTypeQueryParam {
    pub entity_type: EntityType,
}
