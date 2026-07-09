use api_models::user::dashboard_metadata as api;
use diesel_models::enums::DashboardMetadata as DBEnum;
use hyperswitch_masking::Secret;
use serde::Deserialize;
use time::PrimitiveDateTime;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum MetaData {
    ProductionAgreement(ProductionAgreementValue),
    SetupProcessor(api::SetupProcessor),
    ConfigureEndpoint(bool),
    SetupComplete(bool),
    FirstProcessorConnected(api::ProcessorConnected),
    SecondProcessorConnected(api::ProcessorConnected),
    ConfiguredRouting(api::ConfiguredRouting),
    TestPayment(api::TestPayment),
    IntegrationMethod(api::IntegrationMethod),
    ConfigurationType(api::ConfigurationType),
    IntegrationCompleted(bool),
    StripeConnected(api::ProcessorConnected),
    PaypalConnected(api::ProcessorConnected),
    SPRoutingConfigured(api::ConfiguredRouting),
    Feedback(api::Feedback),
    ProdIntent(api::ProdIntent),
    SPTestPayment(bool),
    DownloadWoocom(bool),
    ConfigureWoocom(bool),
    SetupWoocomWebhook(bool),
    IsMultipleConfiguration(bool),
    IsChangePasswordRequired(bool),
    OnboardingSurvey(api::OnboardingSurvey),
    ReconStatus(api::ReconStatus),
    #[cfg(feature = "v1")]
    PaymentViews(Box<api::SavedViewOperation>),
}

impl From<&MetaData> for DBEnum {
    fn from(value: &MetaData) -> Self {
        match value {
            MetaData::ProductionAgreement(_) => Self::ProductionAgreement,
            MetaData::SetupProcessor(_) => Self::SetupProcessor,
            MetaData::ConfigureEndpoint(_) => Self::ConfigureEndpoint,
            MetaData::SetupComplete(_) => Self::SetupComplete,
            MetaData::FirstProcessorConnected(_) => Self::FirstProcessorConnected,
            MetaData::SecondProcessorConnected(_) => Self::SecondProcessorConnected,
            MetaData::ConfiguredRouting(_) => Self::ConfiguredRouting,
            MetaData::TestPayment(_) => Self::TestPayment,
            MetaData::IntegrationMethod(_) => Self::IntegrationMethod,
            MetaData::ConfigurationType(_) => Self::ConfigurationType,
            MetaData::IntegrationCompleted(_) => Self::IntegrationCompleted,
            MetaData::StripeConnected(_) => Self::StripeConnected,
            MetaData::PaypalConnected(_) => Self::PaypalConnected,
            MetaData::SPRoutingConfigured(_) => Self::SpRoutingConfigured,
            MetaData::Feedback(_) => Self::Feedback,
            MetaData::ProdIntent(_) => Self::ProdIntent,
            MetaData::SPTestPayment(_) => Self::SpTestPayment,
            MetaData::DownloadWoocom(_) => Self::DownloadWoocom,
            MetaData::ConfigureWoocom(_) => Self::ConfigureWoocom,
            MetaData::SetupWoocomWebhook(_) => Self::SetupWoocomWebhook,
            MetaData::IsMultipleConfiguration(_) => Self::IsMultipleConfiguration,
            MetaData::IsChangePasswordRequired(_) => Self::IsChangePasswordRequired,
            MetaData::OnboardingSurvey(_) => Self::OnboardingSurvey,
            MetaData::ReconStatus(_) => Self::ReconStatus,
            #[cfg(feature = "v1")]
            MetaData::PaymentViews(_) => Self::PaymentViews,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProductionAgreementValue {
    pub version: String,
    pub ip_address: Secret<String, common_utils::pii::IpAddress>,
    pub timestamp: PrimitiveDateTime,
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "version", content = "filters", rename_all = "snake_case")]
pub enum SavedViewFilter {
    V1(Box<api::PaymentListFilterConstraintsV1>),
    V2(Box<api::PaymentListFilterConstraintsV2>),
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize)]
pub struct SavedView {
    pub view_id: String,
    pub view_name: String,
    #[serde(flatten)]
    pub filters: SavedViewFilter,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(feature = "v1")]
impl<'de> Deserialize<'de> for SavedView {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        #[derive(serde::Deserialize)]
        struct SavedViewData {
            view_id: String,
            view_name: String,
            filters: serde_json::Value,
            created_at: String,
            updated_at: String,
        }

        let version = value
            .get("version")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("v1")
            .to_owned();
        let view = SavedViewData::deserialize(value).map_err(serde::de::Error::custom)?;
        let filters = match version.as_str() {
            "v1" => serde_json::from_value(view.filters)
                .map(Box::new)
                .map(SavedViewFilter::V1)
                .map_err(serde::de::Error::custom)?,
            "v2" => serde_json::from_value(view.filters)
                .map(Box::new)
                .map(SavedViewFilter::V2)
                .map_err(serde::de::Error::custom)?,
            _ => {
                return Err(serde::de::Error::custom(format!(
                    "unsupported saved view version: {version}"
                )));
            }
        };

        Ok(Self {
            view_id: view.view_id,
            view_name: view.view_name,
            filters,
            created_at: view.created_at,
            updated_at: view.updated_at,
        })
    }
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PaymentViewsValue {
    pub views: Vec<SavedView>,
}
