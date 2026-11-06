//! Types related to themes

use serde::{Deserialize, Serialize};

use crate::primitive_wrappers::SafeString;

/// Struct for holding the theme settings for email
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EmailThemeConfig {
    /// The entity name to be used in the email
    pub entity_name: SafeString,

    /// The URL of the entity logo to be used in the email
    pub entity_logo_url: SafeString,

    /// The primary color to be used in the email
    pub primary_color: SafeString,

    /// The foreground color to be used in the email
    pub foreground_color: SafeString,

    /// The background color to be used in the email
    pub background_color: SafeString,
}
