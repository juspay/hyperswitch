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
    RefundViews(Box<api::RefundViewOperation>),
    #[cfg(feature = "v1")]
    DisputeViews(Box<api::DisputeViewOperation>),
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
pub struct SavedViewV1 {
    pub view_id: String,
    pub view_name: String,
    pub filters: api::PaymentListFilterConstraintsV1,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PaymentViewsValue {
    pub views: Vec<SavedViewV1>,
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct VersionedView<T> {
    pub view_id: String,
    pub view_name: String,
    pub filters: T,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct VersionedViewsValue<T> {
    pub views: Vec<VersionedView<T>>,
}

#[cfg(feature = "v1")]
pub trait IntoApiFilters {
    type Api;
    fn into_api(self) -> Self::Api;
}

#[cfg(feature = "v1")]
pub trait IntoStoredFilters {
    type Stored: serde::Serialize + serde::de::DeserializeOwned;
    fn into_stored(self) -> Self::Stored;
}

#[cfg(feature = "v1")]
impl<T: IntoApiFilters> VersionedView<T> {
    pub fn into_response(self) -> api::ViewResponse<T::Api> {
        api::ViewResponse {
            view_id: self.view_id,
            view_name: self.view_name,
            data: self.filters.into_api(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum PaymentAdvancedViewFilters {
    V1(api::PaymentAdvancedViewFilterConstraints),
}

#[cfg(feature = "v1")]
pub type PaymentAdvancedView = VersionedView<PaymentAdvancedViewFilters>;

#[cfg(feature = "v1")]
pub type PaymentAdvancedViewsValue = VersionedViewsValue<PaymentAdvancedViewFilters>;

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum RefundViewFilters {
    V1(api::RefundViewFilterConstraints),
}

#[cfg(feature = "v1")]
pub type RefundView = VersionedView<RefundViewFilters>;

#[cfg(feature = "v1")]
pub type RefundViewsValue = VersionedViewsValue<RefundViewFilters>;

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum DisputeViewFilters {
    V1(api::DisputeViewFilterConstraints),
}

#[cfg(feature = "v1")]
pub type DisputeView = VersionedView<DisputeViewFilters>;

#[cfg(feature = "v1")]
pub type DisputeViewsValue = VersionedViewsValue<DisputeViewFilters>;

#[cfg(feature = "v1")]
impl IntoApiFilters for PaymentAdvancedViewFilters {
    type Api = api::PaymentAdvancedViewFilters;

    fn into_api(self) -> Self::Api {
        match self {
            Self::V1(filters) => api::PaymentAdvancedViewFilters::V1(
                api::PaymentAdvancedViewFiltersV1::PaymentViews(filters),
            ),
        }
    }
}

#[cfg(feature = "v1")]
impl IntoApiFilters for RefundViewFilters {
    type Api = api::RefundViewFilters;

    fn into_api(self) -> Self::Api {
        match self {
            Self::V1(filters) => {
                api::RefundViewFilters::V1(api::RefundViewFiltersV1::RefundViews(filters))
            }
        }
    }
}

#[cfg(feature = "v1")]
impl IntoApiFilters for DisputeViewFilters {
    type Api = api::DisputeViewFilters;

    fn into_api(self) -> Self::Api {
        match self {
            Self::V1(filters) => {
                api::DisputeViewFilters::V1(api::DisputeViewFiltersV1::DisputeViews(filters))
            }
        }
    }
}

#[cfg(feature = "v1")]
impl IntoStoredFilters for api::PaymentAdvancedViewFilters {
    type Stored = PaymentAdvancedViewFilters;

    fn into_stored(self) -> Self::Stored {
        match self {
            Self::V1(api::PaymentAdvancedViewFiltersV1::PaymentViews(filters)) => {
                PaymentAdvancedViewFilters::V1(filters)
            }
        }
    }
}

#[cfg(feature = "v1")]
impl IntoStoredFilters for api::RefundViewFilters {
    type Stored = RefundViewFilters;

    fn into_stored(self) -> Self::Stored {
        match self {
            Self::V1(api::RefundViewFiltersV1::RefundViews(filters)) => {
                RefundViewFilters::V1(filters)
            }
        }
    }
}

#[cfg(feature = "v1")]
impl IntoStoredFilters for api::DisputeViewFilters {
    type Stored = DisputeViewFilters;

    fn into_stored(self) -> Self::Stored {
        match self {
            Self::V1(api::DisputeViewFiltersV1::DisputeViews(filters)) => {
                DisputeViewFilters::V1(filters)
            }
        }
    }
}

#[cfg(feature = "v1")]
impl From<PaymentAdvancedView> for api::PaymentAdvancedViewResponse {
    fn from(v: PaymentAdvancedView) -> Self {
        v.into_response()
    }
}

#[cfg(feature = "v1")]
impl From<RefundView> for api::RefundViewResponse {
    fn from(v: RefundView) -> Self {
        v.into_response()
    }
}

#[cfg(feature = "v1")]
impl From<DisputeView> for api::DisputeViewResponse {
    fn from(v: DisputeView) -> Self {
        v.into_response()
    }
}
