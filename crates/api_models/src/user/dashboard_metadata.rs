use common_enums::{CountryAlpha2, MerchantProductType};
use common_types::primitive_wrappers::SafeString;
use common_utils::{id_type, pii};
use masking::Secret;
use strum::EnumString;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum SetMetaDataRequest {
    ProductionAgreement(ProductionAgreementRequest),
    SetupProcessor(SetupProcessor),
    ConfigureEndpoint,
    SetupComplete,
    FirstProcessorConnected(ProcessorConnected),
    SecondProcessorConnected(ProcessorConnected),
    ConfiguredRouting(ConfiguredRouting),
    TestPayment(TestPayment),
    IntegrationMethod(IntegrationMethod),
    ConfigurationType(ConfigurationType),
    IntegrationCompleted,
    SPRoutingConfigured(ConfiguredRouting),
    Feedback(Feedback),
    ProdIntent(ProdIntent),
    SPTestPayment,
    DownloadWoocom,
    ConfigureWoocom,
    SetupWoocomWebhook,
    IsMultipleConfiguration,
    #[serde(skip)]
    IsChangePasswordRequired,
    OnboardingSurvey(OnboardingSurvey),
    ReconStatus(ReconStatus),
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ProductionAgreementRequest {
    pub version: String,
    #[serde(skip_deserializing)]
    pub ip_address: Option<Secret<String, pii::IpAddress>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SetupProcessor {
    pub connector_id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ProcessorConnected {
    pub processor_id: id_type::MerchantConnectorAccountId,
    pub processor_name: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OnboardingSurvey {
    pub designation: Option<SafeString>,
    pub about_business: Option<SafeString>,
    pub business_website: Option<SafeString>,
    pub hyperswitch_req: Option<SafeString>,
    pub major_markets: Option<Vec<SafeString>>,
    pub business_size: Option<SafeString>,
    pub required_features: Option<Vec<SafeString>>,
    pub required_processors: Option<Vec<SafeString>>,
    pub planned_live_date: Option<SafeString>,
    pub miscellaneous: Option<SafeString>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ConfiguredRouting {
    pub routing_id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TestPayment {
    pub payment_id: id_type::PaymentId,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct IntegrationMethod {
    pub integration_type: String,
}
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub enum ConfigurationType {
    Single,
    Multiple,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Feedback {
    pub email: pii::Email,
    pub description: Option<SafeString>,
    pub rating: Option<i32>,
    pub category: Option<SafeString>,
}
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ProdIntent {
    pub legal_business_name: Option<SafeString>,
    pub business_label: Option<SafeString>,
    pub business_location: Option<CountryAlpha2>,
    pub display_name: Option<SafeString>,
    pub poc_email: Option<pii::Email>,
    pub business_type: Option<SafeString>,
    pub business_identifier: Option<SafeString>,
    pub business_website: Option<SafeString>,
    pub poc_name: Option<Secret<SafeString>>,
    pub poc_contact: Option<Secret<SafeString>>,
    pub comments: Option<SafeString>,
    pub is_completed: bool,
    #[serde(default)]
    pub product_type: MerchantProductType,
    pub business_country_name: Option<SafeString>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ReconStatus {
    pub is_order_data_set: bool,
    pub is_processor_data_set: bool,
}

#[derive(Debug, serde::Deserialize, EnumString, serde::Serialize)]
pub enum GetMetaDataRequest {
    ProductionAgreement,
    SetupProcessor,
    ConfigureEndpoint,
    SetupComplete,
    FirstProcessorConnected,
    SecondProcessorConnected,
    ConfiguredRouting,
    TestPayment,
    IntegrationMethod,
    ConfigurationType,
    IntegrationCompleted,
    StripeConnected,
    PaypalConnected,
    SPRoutingConfigured,
    Feedback,
    ProdIntent,
    SPTestPayment,
    DownloadWoocom,
    ConfigureWoocom,
    SetupWoocomWebhook,
    IsMultipleConfiguration,
    IsChangePasswordRequired,
    OnboardingSurvey,
    ReconStatus,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct GetMultipleMetaDataPayload {
    pub results: Vec<GetMetaDataRequest>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetMultipleMetaDataRequest {
    pub keys: String,
}

#[derive(Debug, serde::Serialize)]
pub enum GetMetaDataResponse {
    ProductionAgreement(bool),
    SetupProcessor(Option<SetupProcessor>),
    ConfigureEndpoint(bool),
    SetupComplete(bool),
    FirstProcessorConnected(Option<ProcessorConnected>),
    SecondProcessorConnected(Option<ProcessorConnected>),
    ConfiguredRouting(Option<ConfiguredRouting>),
    TestPayment(Option<TestPayment>),
    IntegrationMethod(Option<IntegrationMethod>),
    ConfigurationType(Option<ConfigurationType>),
    IntegrationCompleted(bool),
    StripeConnected(Option<ProcessorConnected>),
    PaypalConnected(Option<ProcessorConnected>),
    SPRoutingConfigured(Option<ConfiguredRouting>),
    Feedback(Option<Feedback>),
    ProdIntent(Option<ProdIntent>),
    SPTestPayment(bool),
    DownloadWoocom(bool),
    ConfigureWoocom(bool),
    SetupWoocomWebhook(bool),
    IsMultipleConfiguration(bool),
    IsChangePasswordRequired(bool),
    OnboardingSurvey(Option<OnboardingSurvey>),
    ReconStatus(Option<ReconStatus>),
}

// === Saved Views API Types ===

/// Entity types that support saved views
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SavedViewEntity {
    Payments,
    Refunds,
    Customers,
    Disputes,
    Payouts,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "entity", content = "filters")]
#[serde(rename_all = "snake_case")]
pub enum SavedViewFilters {
    #[cfg(feature = "v1")]
    Payments(crate::payments::PaymentListFilterConstraints),
    Refunds(crate::refunds::RefundListRequest),
    #[cfg(feature = "payouts")]
    Payouts(crate::payouts::PayoutListFilterConstraints),
    Disputes(crate::disputes::DisputeListGetConstraints),
    Customers(crate::customers::CustomerListRequestWithConstraints),
}

impl SavedViewFilters {
    pub fn get_entity(&self) -> SavedViewEntity {
        match self {
            #[cfg(feature = "v1")]
            Self::Payments(_) => SavedViewEntity::Payments,
            Self::Refunds(_) => SavedViewEntity::Refunds,
            #[cfg(feature = "payouts")]
            Self::Payouts(_) => SavedViewEntity::Payouts,
            Self::Disputes(_) => SavedViewEntity::Disputes,
            Self::Customers(_) => SavedViewEntity::Customers,
        }
    }
}

/// A single saved view
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SavedView {
    pub view_name: String,
    #[serde(flatten)]
    pub data: SavedViewFilters,
    pub created_at: String,
    pub updated_at: String,
}

/// Wrapper for the JSONB `filters` column
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SavedViewsData {
    pub views: Vec<SavedView>,
}

/// POST /user/views
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CreateSavedViewRequest {
    pub view_name: String,
    #[serde(flatten)]
    pub data: SavedViewFilters,
}

impl CreateSavedViewRequest {
    pub fn validate(&self) -> Result<(), common_utils::errors::ValidationError> {
        if self.view_name.trim().is_empty() {
            return Err(
                common_utils::errors::ValidationError::MissingRequiredField {
                    field_name: "view_name".to_string(),
                },
            );
        }
        Ok(())
    }
}

/// PUT /user/views
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct UpdateSavedViewRequest {
    pub view_name: String,
    #[serde(flatten)]
    pub data: SavedViewFilters,
}

/// GET /user/views?entity=payments
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ListSavedViewsRequest {
    pub entity: SavedViewEntity,
}

/// DELETE /user/views
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DeleteSavedViewRequest {
    pub entity: SavedViewEntity,
    pub view_name: String,
}

/// Response for saved view CRUD operations
#[derive(Debug, serde::Serialize)]
pub struct SavedViewResponse {
    pub count: usize,
    pub views: Vec<SavedView>,
}
