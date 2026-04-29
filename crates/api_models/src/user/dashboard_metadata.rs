use common_enums::{CountryAlpha2, MerchantProductType};
use common_types::primitive_wrappers::SafeString;
use common_utils::{id_type, pii};
use hyperswitch_masking::Secret;
use strum::EnumString;

#[cfg(feature = "v1")]
use crate::{enums, payments};

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
    #[cfg(feature = "v1")]
    PaymentViews(SavedViewOperation),
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum SavedViewOperation {
    Create(CreateSavedViewRequest),
    Update(UpdateSavedViewRequest),
    Delete(DeleteSavedViewRequest),
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
    #[cfg(feature = "v1")]
    PaymentViews,
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
    #[cfg(feature = "v1")]
    PaymentViews(Option<Vec<SavedViewResponse>>),
}

// === Saved Views API Types ===

#[cfg(feature = "v1")]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "entity", content = "filters")]
#[serde(rename_all = "snake_case")]
pub enum SavedViewFiltersV1 {
    PaymentViews(PaymentListFilterConstraintsV1),
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PaymentListFilterConstraintsV1 {
    pub payment_id: Option<id_type::PaymentId>,
    pub profile_id: Option<id_type::ProfileId>,
    pub customer_id: Option<id_type::CustomerId>,
    #[serde(default = "common_utils::consts::default_payments_list_limit")]
    pub limit: u32,
    pub offset: Option<u32>,
    pub amount_filter: Option<payments::AmountFilter>,
    #[serde(flatten)]
    pub time_range: Option<common_utils::types::TimeRange>,
    pub connector: Option<Vec<enums::Connector>>,
    pub currency: Option<Vec<enums::Currency>>,
    pub status: Option<Vec<enums::IntentStatus>>,
    pub payment_method: Option<Vec<enums::PaymentMethod>>,
    pub payment_method_type: Option<Vec<enums::PaymentMethodType>>,
    pub authentication_type: Option<Vec<enums::AuthenticationType>>,
    pub merchant_connector_id: Option<Vec<id_type::MerchantConnectorAccountId>>,
    #[serde(default)]
    pub order: payments::Order,
    pub card_network: Option<Vec<enums::CardNetwork>>,
    pub merchant_order_reference_id: Option<String>,
    pub card_discovery: Option<Vec<enums::CardDiscovery>>,
    pub customer_email: Option<pii::Email>,
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum SavedViewFilters {
    V1(SavedViewFiltersV1),
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SavedViewResponse {
    pub view_id: String,
    pub view_name: String,
    pub data: SavedViewFilters,
    pub created_at: String,
    pub updated_at: String,
}
/// POST /user/views
#[cfg(feature = "v1")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CreateSavedViewRequest {
    pub view_name: String,
    #[serde(flatten)]
    pub data: SavedViewFilters,
}

/// PUT /user/views
#[cfg(feature = "v1")]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UpdateSavedViewRequest {
    pub view_id: String,
    pub view_name: Option<String>,
    #[serde(flatten)]
    pub data: SavedViewFilters,
}

/// DELETE /user/views
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DeleteSavedViewRequest {
    pub view_id: String,
}
