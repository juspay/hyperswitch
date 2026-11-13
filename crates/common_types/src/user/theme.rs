//! Types related to themes

use common_utils::types::Url;
use serde::{Deserialize, Serialize};

use crate::primitive_wrappers::SafeString;

/// Struct for holding the theme settings for email
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmailThemeConfig {
    /// The entity name to be used in the email
    pub entity_name: SafeString,

    /// The URL of the entity logo to be used in the email
    pub entity_logo_url: Url,

    /// The primary color to be used in the email
    pub primary_color: SafeString,

    /// The foreground color to be used in the email
    pub foreground_color: SafeString,

    /// The background color to be used in the email
    pub background_color: SafeString,
}

impl Default for EmailThemeConfig {
    fn default() -> Self {
        Self {
            entity_name: SafeString::default(),
            #[allow(clippy::expect_used)]
            entity_logo_url: Url::wrap(
                url::Url::parse("https://app.hyperswitch.io/email-assets/HyperswitchLogo.png")
                    .expect("Failed to parse default email entity logo URL"),
            ),
            primary_color: SafeString::default(),
            foreground_color: SafeString::default(),
            background_color: SafeString::default(),
        }
    }
}
