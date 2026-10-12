use euclid::frontend::dir::DirKeyKind;
#[cfg(feature = "payouts")]
use euclid::frontend::dir::PayoutDirKeyKind;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct WebhookStatusConfig {
    pub value: String,
    pub event_type: common_enums::EventType,
}

#[derive(Serialize, Clone)]
pub struct WebhookEventClassConfig {
    pub event_class: common_enums::EventClass,
    pub api_field: &'static str,
    pub statuses: Vec<WebhookStatusConfig>,
}

#[derive(Serialize, Clone)]
pub struct Details<'a> {
    pub description: Option<&'a str>,
    pub kind: DirKeyKind,
}

#[cfg(feature = "payouts")]
#[derive(Serialize, Clone)]
pub struct PayoutDetails<'a> {
    pub description: Option<&'a str>,
    pub kind: PayoutDirKeyKind,
}
