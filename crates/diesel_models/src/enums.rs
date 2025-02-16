#[doc(hidden)]
pub mod diesel_exports {
    pub use super::{
        DbApiVersion as ApiVersion, DbAttemptStatus as AttemptStatus,
        DbAuthenticationType as AuthenticationType, DbBlocklistDataKind as BlocklistDataKind,
        DbCaptureMethod as CaptureMethod, DbCaptureStatus as CaptureStatus,
        DbCardDiscovery as CardDiscovery, DbConnectorStatus as ConnectorStatus,
        DbConnectorType as ConnectorType, DbCountryAlpha2 as CountryAlpha2, DbCurrency as Currency,
        DbDashboardMetadata as DashboardMetadata, DbDeleteStatus as DeleteStatus,
        DbDisputeStage as DisputeStage, DbDisputeStatus as DisputeStatus,
        DbEventClass as EventClass, DbEventObjectType as EventObjectType, DbEventType as EventType,
        DbFraudCheckStatus as FraudCheckStatus, DbFraudCheckType as FraudCheckType,
        DbFutureUsage as FutureUsage, DbGenericLinkType as GenericLinkType,
        DbIntentStatus as IntentStatus, DbMandateStatus as MandateStatus,
        DbMandateType as MandateType, DbMerchantStorageScheme as MerchantStorageScheme,
        DbOrderFulfillmentTimeOrigin as OrderFulfillmentTimeOrigin,
        DbPaymentMethodIssuerCode as PaymentMethodIssuerCode, DbPaymentSource as PaymentSource,
        DbPaymentType as PaymentType, DbPayoutStatus as PayoutStatus, DbPayoutType as PayoutType,
        DbProcessTrackerStatus as ProcessTrackerStatus, DbReconStatus as ReconStatus,
        DbRefundStatus as RefundStatus, DbRefundType as RefundType, DbRelayStatus as RelayStatus,
        DbRelayType as RelayType,
        DbRequestIncrementalAuthorization as RequestIncrementalAuthorization,
        DbRoleScope as RoleScope, DbRoutingAlgorithmKind as RoutingAlgorithmKind,
        DbScaExemptionType as ScaExemptionType,
        DbSuccessBasedRoutingConclusiveState as SuccessBasedRoutingConclusiveState,
        DbTotpStatus as TotpStatus, DbTransactionType as TransactionType,
        DbUserRoleVersion as UserRoleVersion, DbUserStatus as UserStatus,
        DbWebhookDeliveryAttempt as WebhookDeliveryAttempt,
    };
}
pub use common_enums::*;
use common_utils::pii;
use diesel::{deserialize::FromSqlRow, expression::AsExpression, sql_types::Jsonb};
use router_derive::diesel_enum;
use time::PrimitiveDateTime;

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RoutingAlgorithmKind {
    Single,
    Priority,
    VolumeSplit,
    Advanced,
    Dynamic,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EventObjectType {
    PaymentDetails,
    RefundDetails,
    DisputeDetails,
    MandateDetails,
    PayoutDetails,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ProcessTrackerStatus {
    // Picked by the producer
    Processing,
    // State when the task is added
    New,
    // Send to retry
    Pending,
    // Picked by consumer
    ProcessStarted,
    // Finished by consumer
    Finish,
    // Review the task
    Review,
}

// Refund
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[diesel_enum(storage_type = "db_enum")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RefundType {
    InstantRefund,
    #[default]
    RegularRefund,
    RetryRefund,
}

// Mandate
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Default,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MandateType {
    SingleUse,
    #[default]
    MultiUse,
}

#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
pub struct MandateDetails {
    pub update_mandate_id: Option<String>,
}

common_utils::impl_to_sql_from_sql_json!(MandateDetails);

#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
pub enum MandateDataType {
    SingleUse(MandateAmountData),
    MultiUse(Option<MandateAmountData>),
}

common_utils::impl_to_sql_from_sql_json!(MandateDataType);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct MandateAmountData {
    pub amount: common_utils::types::MinorUnit,
    pub currency: Currency,
    pub start_date: Option<PrimitiveDateTime>,
    pub end_date: Option<PrimitiveDateTime>,
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FraudCheckType {
    PreFrm,
    PostFrm,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
pub enum FraudCheckLastStep {
    #[default]
    Processing,
    CheckoutOrSale,
    TransactionOrRecordRefund,
    Fulfillment,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum UserStatus {
    Active,
    #[default]
    InvitationSent,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[router_derive::diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DashboardMetadata {
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
    SpRoutingConfigured,
    Feedback,
    ProdIntent,
    SpTestPayment,
    DownloadWoocom,
    ConfigureWoocom,
    SetupWoocomWebhook,
    IsMultipleConfiguration,
    IsChangePasswordRequired,
    OnboardingSurvey,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TotpStatus {
    Set,
    InProgress,
    #[default]
    NotSet,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::EnumString,
    strum::Display,
)]
#[diesel_enum(storage_type = "db_enum")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum UserRoleVersion {
    #[default]
    V1,
    V2,
}
