use api_models::user::dashboard_metadata as api;
use diesel_models::enums::DashboardMetadata as DBEnum;
use hyperswitch_masking::Secret;
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
    #[cfg(feature = "v1")]
    PaymentAdvancedViews(Box<api::PaymentAdvancedViewOperation>),
    #[cfg(feature = "v1")]
    RefundViews(Box<api::SavedViewOperation>),
    #[cfg(feature = "v1")]
    DisputeViews(Box<api::SavedViewOperation>),
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
            #[cfg(feature = "v1")]
            MetaData::PaymentAdvancedViews(_) => Self::PaymentAdvancedViews,
            #[cfg(feature = "v1")]
            MetaData::RefundViews(_) => Self::RefundViews,
            #[cfg(feature = "v1")]
            MetaData::DisputeViews(_) => Self::DisputeViews,
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
pub struct SavedView<T> {
    pub view_id: String,
    pub view_name: String,
    pub filters: T,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SavedViewsValue<T> {
    pub views: Vec<SavedView<T>>,
}

#[cfg(feature = "v1")]
pub type PaymentViewsValue = SavedViewsValue<api::PaymentListFilterConstraintsV1>;

/// Stored refund-view filters, tagged with the version that wrote them so a future
/// filter shape can be told apart from `V1` without needing a new metadata key.
#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum RefundViewFilters {
    V1(api::RefundViewFilterConstraintsV1),
}

/// Stored dispute-view filters, tagged with the version that wrote them.
#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum DisputeViewFilters {
    V1(api::DisputeViewFilterConstraintsV1),
}

#[cfg(feature = "v1")]
pub type RefundViewsValue = SavedViewsValue<RefundViewFilters>;

#[cfg(feature = "v1")]
pub type DisputeViewsValue = SavedViewsValue<DisputeViewFilters>;

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum PaymentAdvancedViewFilters {
    V1(api::PaymentAdvancedViewFilterConstraints),
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PaymentAdvancedView {
    pub view_id: String,
    pub view_name: String,
    pub filters: PaymentAdvancedViewFilters,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PaymentAdvancedViewsValue {
    pub views: Vec<PaymentAdvancedView>,
}

#[cfg(feature = "v1")]
impl From<SavedView<RefundViewFilters>> for api::SavedViewResponse {
    fn from(v: SavedView<RefundViewFilters>) -> Self {
        let data = match v.filters {
            RefundViewFilters::V1(filters) => {
                api::SavedViewFilters::V1(api::SavedViewFiltersV1::RefundViews(filters))
            }
        };
        Self {
            view_id: v.view_id,
            view_name: v.view_name,
            data,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[cfg(feature = "v1")]
impl From<SavedView<DisputeViewFilters>> for api::SavedViewResponse {
    fn from(v: SavedView<DisputeViewFilters>) -> Self {
        let data = match v.filters {
            DisputeViewFilters::V1(filters) => {
                api::SavedViewFilters::V1(api::SavedViewFiltersV1::DisputeViews(filters))
            }
        };
        Self {
            view_id: v.view_id,
            view_name: v.view_name,
            data,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[cfg(feature = "v1")]
impl From<PaymentAdvancedView> for api::PaymentAdvancedViewResponse {
    fn from(v: PaymentAdvancedView) -> Self {
        let data = match v.filters {
            PaymentAdvancedViewFilters::V1(filters) => api::PaymentAdvancedViewFilters::V1(
                api::PaymentAdvancedViewFiltersV1::PaymentViews(filters),
            ),
        };
        Self {
            view_id: v.view_id,
            view_name: v.view_name,
            data,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}
