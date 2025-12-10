use common_enums::EntityType;
use common_utils::{
    date_time, id_type,
    types::user::{EmailThemeConfig, ThemeLineage},
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use router_derive::DebugAsDisplay;
use time::PrimitiveDateTime;

use crate::schema::themes;

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = themes, primary_key(theme_id), check_for_backend(diesel::pg::Pg))]
pub struct Theme {
    pub theme_id: String,
    pub tenant_id: id_type::TenantId,
    pub org_id: Option<id_type::OrganizationId>,
    pub merchant_id: Option<id_type::MerchantId>,
    pub profile_id: Option<id_type::ProfileId>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub entity_type: EntityType,
    pub theme_name: String,
    pub email_primary_color: String,
    pub email_foreground_color: String,
    pub email_background_color: String,
    pub email_entity_name: String,
    pub email_entity_logo_url: String,
    pub theme_config_version: String,
}

#[derive(Clone, Debug, Insertable, DebugAsDisplay)]
#[diesel(table_name = themes)]
pub struct ThemeNew {
    pub theme_id: String,
    pub tenant_id: id_type::TenantId,
    pub org_id: Option<id_type::OrganizationId>,
    pub merchant_id: Option<id_type::MerchantId>,
    pub profile_id: Option<id_type::ProfileId>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub entity_type: EntityType,
    pub theme_name: String,
    pub email_primary_color: String,
    pub email_foreground_color: String,
    pub email_background_color: String,
    pub email_entity_name: String,
    pub email_entity_logo_url: String,
    pub theme_config_version: String,
}

impl ThemeNew {
    pub fn new(
        theme_id: String,
        theme_name: String,
        lineage: ThemeLineage,
        email_config: EmailThemeConfig,
    ) -> Self {
        let now = date_time::now();
        let theme_config_version = now.assume_utc().unix_timestamp().to_string();
        Self {
            theme_id,
            theme_name,
            tenant_id: lineage.tenant_id().to_owned(),
            org_id: lineage.org_id().cloned(),
            merchant_id: lineage.merchant_id().cloned(),
            profile_id: lineage.profile_id().cloned(),
            entity_type: lineage.entity_type(),
            created_at: now,
            last_modified_at: now,
            email_primary_color: email_config.primary_color,
            email_foreground_color: email_config.foreground_color,
            email_background_color: email_config.background_color,
            email_entity_name: email_config.entity_name,
            email_entity_logo_url: email_config.entity_logo_url,
            theme_config_version: theme_config_version,
        }
    }
}

impl Theme {
    pub fn email_config(&self) -> EmailThemeConfig {
        EmailThemeConfig {
            primary_color: self.email_primary_color.clone(),
            foreground_color: self.email_foreground_color.clone(),
            background_color: self.email_background_color.clone(),
            entity_name: self.email_entity_name.clone(),
            entity_logo_url: self.email_entity_logo_url.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, AsChangeset, DebugAsDisplay)]
#[diesel(table_name = themes)]
pub struct ThemeUpdateInternal {
    pub email_primary_color: Option<String>,
    pub email_foreground_color: Option<String>,
    pub email_background_color: Option<String>,
    pub email_entity_name: Option<String>,
    pub email_entity_logo_url: Option<String>,
    pub last_modified_at: Option<PrimitiveDateTime>,
    pub theme_config_version: Option<String>,
}

#[derive(Clone)]
pub enum ThemeUpdate {
    EmailConfig { email_config: EmailThemeConfig },
    ThemeConfig,
}

impl From<ThemeUpdate> for ThemeUpdateInternal {
    fn from(value: ThemeUpdate) -> Self {
        let theme_config_version = date_time::now().assume_utc().unix_timestamp().to_string();
        match value {
            ThemeUpdate::EmailConfig { email_config } => Self {
                email_primary_color: Some(email_config.primary_color),
                email_foreground_color: Some(email_config.foreground_color),
                email_background_color: Some(email_config.background_color),
                email_entity_name: Some(email_config.entity_name),
                email_entity_logo_url: Some(email_config.entity_logo_url),
                last_modified_at: Some(date_time::now()),
                theme_config_version: None,
            },
            ThemeUpdate::ThemeConfig => Self {
                email_primary_color: None,
                email_foreground_color: None,
                email_background_color: None,
                email_entity_name: None,
                email_entity_logo_url: None,
                last_modified_at: Some(date_time::now()),
                theme_config_version: Some(theme_config_version),
            },
        }
    }
}
