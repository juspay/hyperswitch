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
pub struct SavedViewV1 {
    pub view_id: String,
    pub view_name: String,
    pub filters: api::PaymentListFilterConstraintsV1,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SavedViewV2 {
    pub view_id: String,
    pub view_name: String,
    pub filters: api::PaymentListFilterConstraintsV2,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum SavedView {
    V1(SavedViewV1),
    V2(SavedViewV2),
}

#[cfg(feature = "v1")]
impl<'de> Deserialize<'de> for SavedView {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value.get("version").and_then(serde_json::Value::as_str) {
            None | Some("v1") => SavedViewV1::deserialize(value)
                .map(Self::V1)
                .map_err(serde::de::Error::custom),
            Some("v2") => SavedViewV2::deserialize(value)
                .map(Self::V2)
                .map_err(serde::de::Error::custom),
            Some(version) => Err(serde::de::Error::custom(format!(
                "unsupported saved view version: {version}"
            ))),
        }
    }
}

#[cfg(feature = "v1")]
impl SavedView {
    pub fn is_same_version(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::V1(_), Self::V1(_)) | (Self::V2(_), Self::V2(_))
        )
    }

    pub fn matches_filters_version(&self, filters: &api::SavedViewFilters) -> bool {
        matches!(
            (self, filters),
            (Self::V1(_), api::SavedViewFilters::V1(_))
                | (Self::V2(_), api::SavedViewFilters::V2(_))
        )
    }

    pub fn view_id(&self) -> &str {
        match self {
            Self::V1(view) => &view.view_id,
            Self::V2(view) => &view.view_id,
        }
    }

    pub fn view_name(&self) -> &str {
        match self {
            Self::V1(view) => &view.view_name,
            Self::V2(view) => &view.view_name,
        }
    }

    pub fn set_view_name(&mut self, view_name: String) {
        match self {
            Self::V1(view) => view.view_name = view_name,
            Self::V2(view) => view.view_name = view_name,
        }
    }

    pub fn set_updated_at(&mut self, updated_at: String) {
        match self {
            Self::V1(view) => view.updated_at = updated_at,
            Self::V2(view) => view.updated_at = updated_at,
        }
    }

    pub fn into_response(self) -> api::SavedViewResponse {
        match self {
            Self::V1(view) => api::SavedViewResponse {
                view_id: view.view_id,
                view_name: view.view_name,
                data: api::SavedViewFilters::V1(api::SavedViewFiltersV1::PaymentViews(
                    view.filters,
                )),
                created_at: view.created_at,
                updated_at: view.updated_at,
            },
            Self::V2(view) => api::SavedViewResponse {
                view_id: view.view_id,
                view_name: view.view_name,
                data: api::SavedViewFilters::V2(api::SavedViewFiltersV2::PaymentViews(
                    view.filters,
                )),
                created_at: view.created_at,
                updated_at: view.updated_at,
            },
        }
    }
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PaymentViewsValue {
    pub views: Vec<SavedView>,
}
