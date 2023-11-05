//!
//! Types.
//!

use serde::Deserialize;
use strum::{Display, EnumString};
pub use tracing::{
    field::{Field, Visit},
    Level, Value,
};

///
/// Category and tag of log event.
///
/// Don't hesitate to add your variant if it is missing here.
///

#[derive(Debug, Default, Deserialize, Clone, Display, EnumString)]
pub enum Tag {
    /// General.
    #[default]
    General,

    /// Redis: get.
    RedisGet,
    /// Redis: set.
    RedisSet,

    /// API: incoming web request.
    ApiIncomingRequest,
    /// API: outgoing web request.
    ApiOutgoingRequest,

    /// Data base: create.
    DbCreate,
    /// Data base: read.
    DbRead,
    /// Data base: updare.
    DbUpdate,
    /// Data base: delete.
    DbDelete,
    /// Begin Request
    BeginRequest,
    /// End Request
    EndRequest,

    /// Call initiated to connector.
    InitiatedToConnector,

    /// Event: general.
    Event,
}

/// API Flow
#[derive(Debug, Display, Clone, PartialEq, Eq)]
pub enum Flow {
    /// Merchants account create flow.
    MerchantsAccountCreate,
    /// Merchants account retrieve flow.
    MerchantsAccountRetrieve,
    /// Merchants account update flow.
    MerchantsAccountUpdate,
    /// Merchants account delete flow.
    MerchantsAccountDelete,
    /// Merchant Connectors create flow.
    MerchantConnectorsCreate,
    /// Merchant Connectors retrieve flow.
    MerchantConnectorsRetrieve,
    /// Merchant account list
    MerchantAccountList,
    /// Merchant Connectors update flow.
    MerchantConnectorsUpdate,
    /// Merchant Connectors delete flow.
    MerchantConnectorsDelete,
    /// Merchant Connectors list flow.
    MerchantConnectorsList,
    /// ConfigKey create flow.
    ConfigKeyCreate,
    /// ConfigKey fetch flow.
    ConfigKeyFetch,
    /// ConfigKey Update flow.
    ConfigKeyUpdate,
    /// Customers create flow.
    CustomersCreate,
    /// Customers retrieve flow.
    CustomersRetrieve,
    /// Customers update flow.
    CustomersUpdate,
    /// Customers delete flow.
    CustomersDelete,
    /// Customers get mandates flow.
    CustomersGetMandates,
    /// Create an Ephemeral Key.
    EphemeralKeyCreate,
    /// Delete an Ephemeral Key.
    EphemeralKeyDelete,
    /// Mandates retrieve flow.
    MandatesRetrieve,
    /// Mandates revoke flow.
    MandatesRevoke,
    /// Mandates list flow.
    MandatesList,
    /// Payment methods create flow.
    PaymentMethodsCreate,
    /// Payment methods list flow.
    PaymentMethodsList,
    /// Customer payment methods list flow.
    CustomerPaymentMethodsList,
    /// List Customers for a merchant
    CustomersList,
    /// Payment methods retrieve flow.
    PaymentMethodsRetrieve,
    /// Payment methods update flow.
    PaymentMethodsUpdate,
    /// Payment methods delete flow.
    PaymentMethodsDelete,
    /// Payments create flow.
    PaymentsCreate,
    /// Payments Retrieve flow.
    PaymentsRetrieve,
    /// Payments update flow.
    PaymentsUpdate,
    /// Payments confirm flow.
    PaymentsConfirm,
    /// Payments capture flow.
    PaymentsCapture,
    /// Payments cancel flow.
    PaymentsCancel,
    /// Payments approve flow.
    PaymentsApprove,
    /// Payments reject flow.
    PaymentsReject,
    /// Payments Session Token flow
    PaymentsSessionToken,
    /// Payments start flow.
    PaymentsStart,
    /// Payments list flow.
    PaymentsList,
    #[cfg(feature = "payouts")]
    /// Payouts create flow
    PayoutsCreate,
    #[cfg(feature = "payouts")]
    /// Payouts retrieve flow.
    PayoutsRetrieve,
    #[cfg(feature = "payouts")]
    /// Payouts update flow.
    PayoutsUpdate,
    #[cfg(feature = "payouts")]
    /// Payouts cancel flow.
    PayoutsCancel,
    #[cfg(feature = "payouts")]
    /// Payouts fulfill flow.
    PayoutsFulfill,
    /// Payouts accounts flow.
    PayoutsAccounts,
    /// Payments Redirect flow.
    PaymentsRedirect,
    /// Refunds create flow.
    RefundsCreate,
    /// Refunds retrieve flow.
    RefundsRetrieve,
    /// Refunds update flow.
    RefundsUpdate,
    /// Refunds list flow.
    RefundsList,
    /// Routing create flow,
    RoutingCreateConfig,
    /// Routing link config
    RoutingLinkConfig,
    /// Routing link config
    RoutingUnlinkConfig,
    /// Routing retrieve config
    RoutingRetrieveConfig,
    /// Routing retrieve active config
    RoutingRetrieveActiveConfig,
    /// Routing retrieve default config
    RoutingRetrieveDefaultConfig,
    /// Routing retrieve dictionary
    RoutingRetrieveDictionary,
    /// Routing update config
    RoutingUpdateConfig,
    /// Routing update default config
    RoutingUpdateDefaultConfig,
    /// Routing delete config
    RoutingDeleteConfig,
    /// Incoming Webhook Receive
    IncomingWebhookReceive,
    /// Validate payment method flow
    ValidatePaymentMethod,
    /// API Key create flow
    ApiKeyCreate,
    /// API Key retrieve flow
    ApiKeyRetrieve,
    /// API Key update flow
    ApiKeyUpdate,
    /// API Key revoke flow
    ApiKeyRevoke,
    /// API Key list flow
    ApiKeyList,
    /// Dispute Retrieve flow
    DisputesRetrieve,
    /// Dispute List flow
    DisputesList,
    /// Cards Info flow
    CardsInfo,
    /// Create File flow
    CreateFile,
    /// Delete File flow
    DeleteFile,
    /// Retrieve File flow
    RetrieveFile,
    /// Dispute Evidence submission flow
    DisputesEvidenceSubmit,
    /// Create Config Key flow
    CreateConfigKey,
    /// Attach Dispute Evidence flow
    AttachDisputeEvidence,
    /// Retrieve Dispute Evidence flow
    RetrieveDisputeEvidence,
    /// Invalidate cache flow
    CacheInvalidate,
    /// Payment Link Retrieve flow
    PaymentLinkRetrieve,
    /// payment Link Initiate flow
    PaymentLinkInitiate,
    /// Create a business profile
    BusinessProfileCreate,
    /// Update a business profile
    BusinessProfileUpdate,
    /// Retrieve a business profile
    BusinessProfileRetrieve,
    /// Delete a business profile
    BusinessProfileDelete,
    /// List all the business profiles for a merchant
    BusinessProfileList,
    /// Different verification flows
    Verification,
}

///
/// Trait for providing generic behaviour to flow metric
///
pub trait FlowMetric: ToString + std::fmt::Debug + Clone {}
impl FlowMetric for Flow {}

/// Category of log event.
#[derive(Debug)]
pub enum Category {
    /// Redis: general.
    Redis,
    /// API: general.
    Api,
    /// Database: general.
    Store,
    /// Event: general.
    Event,
    /// General: general.
    General,
}
