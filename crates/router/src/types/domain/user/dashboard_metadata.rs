use api_models::user::dashboard_metadata as api;
use diesel_models::enums::DashboardMetadata as DBEnum;
use hyperswitch_masking::Secret;
use time::PrimitiveDateTime;

#[cfg(feature = "v1")]
use crate::types::transformers::ForeignFrom;

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
#[serde(deny_unknown_fields)]
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
    V1(Box<SavedViewV1>),
    V2(Box<SavedViewV2>),
}

#[cfg(feature = "v1")]
impl<'de> serde::Deserialize<'de> for SavedView {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(tag = "version", rename_all = "snake_case")]
        enum VersionedSavedView {
            V1(Box<SavedViewV1>),
            V2(Box<SavedViewV2>),
        }

        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum SavedViewData {
            Versioned(VersionedSavedView),
            LegacyV1(Box<SavedViewV1>),
        }

        match <SavedViewData as serde::Deserialize>::deserialize(deserializer)? {
            SavedViewData::Versioned(VersionedSavedView::V1(view))
            | SavedViewData::LegacyV1(view) => Ok(Self::V1(view)),
            SavedViewData::Versioned(VersionedSavedView::V2(view)) => Ok(Self::V2(view)),
        }
    }
}

#[cfg(feature = "v1")]
impl ForeignFrom<SavedView> for api::SavedViewResponse {
    fn foreign_from(from: SavedView) -> Self {
        match from {
            SavedView::V1(view) => Self {
                view_id: view.view_id,
                view_name: view.view_name,
                data: api::SavedViewFilters::V1(Box::new(api::SavedViewFiltersV1::PaymentViews(
                    view.filters,
                ))),
                created_at: view.created_at,
                updated_at: view.updated_at,
            },
            SavedView::V2(view) => Self {
                view_id: view.view_id,
                view_name: view.view_name,
                data: api::SavedViewFilters::V2(Box::new(api::SavedViewFiltersV2::PaymentViews(
                    view.filters,
                ))),
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
