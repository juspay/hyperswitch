//! Types.

use serde::Deserialize;
use strum::{Display, EnumString};
pub use tracing::{
    field::{Field, Visit},
    Level, Value,
};

/// Category and tag of log event.
///
/// Don't hesitate to add your variant if it is missing here.
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

    /// Compatibility Layer Request
    CompatibilityLayerRequest,
}

/// API Flow
#[derive(Debug, Display, Clone, PartialEq, Eq)]
pub enum Flow {
    /// Health check
    HealthCheck,
    /// Deep health Check
    DeepHealthCheck,
    /// Organization create flow
    OrganizationCreate,
    /// Organization retrieve flow
    OrganizationRetrieve,
    /// Organization update flow
    OrganizationUpdate,
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
    /// Merchant Transfer Keys
    MerchantTransferKey,
    /// ConfigKey create flow.
    ConfigKeyCreate,
    /// ConfigKey fetch flow.
    ConfigKeyFetch,
    /// Enable platform account flow.
    EnablePlatformAccount,
    /// ConfigKey Update flow.
    ConfigKeyUpdate,
    /// ConfigKey Delete flow.
    ConfigKeyDelete,
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
    /// Payment methods migrate flow.
    PaymentMethodsMigrate,
    /// Payment methods list flow.
    PaymentMethodsList,
    /// Payment method save flow
    PaymentMethodSave,
    /// Customer payment methods list flow.
    CustomerPaymentMethodsList,
    /// List Customers for a merchant
    CustomersList,
    /// Retrieve countries and currencies for connector and payment method
    ListCountriesCurrencies,
    /// Payment method create collect link flow.
    PaymentMethodCollectLink,
    /// Payment methods retrieve flow.
    PaymentMethodsRetrieve,
    /// Payment methods update flow.
    PaymentMethodsUpdate,
    /// Payment methods delete flow.
    PaymentMethodsDelete,
    /// Default Payment method flow.
    DefaultPaymentMethodsSet,
    /// Payments create flow.
    PaymentsCreate,
    /// Payments Retrieve flow.
    PaymentsRetrieve,
    /// Payments Retrieve force sync flow.
    PaymentsRetrieveForceSync,
    /// Payments Retrieve using merchant reference id
    PaymentsRetrieveUsingMerchantReferenceId,
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
    /// Payments filters flow
    PaymentsFilters,
    /// Payments aggregates flow
    PaymentsAggregate,
    /// Payments Create Intent flow
    PaymentsCreateIntent,
    /// Payments Get Intent flow
    PaymentsGetIntent,
    /// Payments Update Intent flow
    PaymentsUpdateIntent,
    /// Payments confirm intent flow
    PaymentsConfirmIntent,
    /// Payments create and confirm intent flow
    PaymentsCreateAndConfirmIntent,
    #[cfg(feature = "payouts")]
    /// Payouts create flow
    PayoutsCreate,
    #[cfg(feature = "payouts")]
    /// Payouts retrieve flow.
    PayoutsRetrieve,
    #[cfg(feature = "payouts")]
    /// Payouts update flow.
    PayoutsUpdate,
    /// Payouts confirm flow.
    PayoutsConfirm,
    #[cfg(feature = "payouts")]
    /// Payouts cancel flow.
    PayoutsCancel,
    #[cfg(feature = "payouts")]
    /// Payouts fulfill flow.
    PayoutsFulfill,
    #[cfg(feature = "payouts")]
    /// Payouts list flow.
    PayoutsList,
    #[cfg(feature = "payouts")]
    /// Payouts filter flow.
    PayoutsFilter,
    /// Payouts accounts flow.
    PayoutsAccounts,
    /// Payout link initiate flow
    PayoutLinkInitiate,
    /// Payments Redirect flow
    PaymentsRedirect,
    /// Payemnts Complete Authorize Flow
    PaymentsCompleteAuthorize,
    /// Refunds create flow.
    RefundsCreate,
    /// Refunds retrieve flow.
    RefundsRetrieve,
    /// Refunds retrieve force sync flow.
    RefundsRetrieveForceSync,
    /// Refunds update flow.
    RefundsUpdate,
    /// Refunds list flow.
    RefundsList,
    /// Refunds filters flow
    RefundsFilters,
    /// Refunds aggregates flow
    RefundsAggregate,
    // Retrieve forex flow.
    RetrieveForexFlow,
    /// Toggles recon service for a merchant.
    ReconMerchantUpdate,
    /// Recon token request flow.
    ReconTokenRequest,
    /// Initial request for recon service.
    ReconServiceRequest,
    /// Recon token verification flow
    ReconVerifyToken,
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
    /// Toggle dynamic routing
    ToggleDynamicRouting,
    /// Update dynamic routing config
    UpdateDynamicRoutingConfigs,
    /// Add record to blocklist
    AddToBlocklist,
    /// Delete record from blocklist
    DeleteFromBlocklist,
    /// List entries from blocklist
    ListBlocklist,
    /// Toggle blocklist for merchant
    ToggleBlocklistGuard,
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
    /// Dispute Filters flow
    DisputesFilters,
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
    /// Delete Dispute Evidence flow
    DeleteDisputeEvidence,
    /// Disputes aggregate flow
    DisputesAggregate,
    /// Retrieve Dispute Evidence flow
    RetrieveDisputeEvidence,
    /// Invalidate cache flow
    CacheInvalidate,
    /// Payment Link Retrieve flow
    PaymentLinkRetrieve,
    /// payment Link Initiate flow
    PaymentLinkInitiate,
    /// payment Link Initiate flow
    PaymentSecureLinkInitiate,
    /// Payment Link List flow
    PaymentLinkList,
    /// Payment Link Status
    PaymentLinkStatus,
    /// Create a profile
    ProfileCreate,
    /// Update a profile
    ProfileUpdate,
    /// Retrieve a profile
    ProfileRetrieve,
    /// Delete a profile
    ProfileDelete,
    /// List all the profiles for a merchant
    ProfileList,
    /// Different verification flows
    Verification,
    /// Rust locker migration
    RustLockerMigration,
    /// Gsm Rule Creation flow
    GsmRuleCreate,
    /// Gsm Rule Retrieve flow
    GsmRuleRetrieve,
    /// Gsm Rule Update flow
    GsmRuleUpdate,
    /// Apple pay certificates migration
    ApplePayCertificatesMigration,
    /// Gsm Rule Delete flow
    GsmRuleDelete,
    /// User Sign Up
    UserSignUp,
    /// User Sign Up
    UserSignUpWithMerchantId,
    /// User Sign In
    UserSignIn,
    /// User transfer key
    UserTransferKey,
    /// User connect account
    UserConnectAccount,
    /// Upsert Decision Manager Config
    DecisionManagerUpsertConfig,
    /// Delete Decision Manager Config
    DecisionManagerDeleteConfig,
    /// Retrieve Decision Manager Config
    DecisionManagerRetrieveConfig,
    /// Manual payment fulfillment acknowledgement
    FrmFulfillment,
    /// Get connectors feature matrix
    FeatureMatrix,
    /// Change password flow
    ChangePassword,
    /// Signout flow
    Signout,
    /// Set Dashboard Metadata flow
    SetDashboardMetadata,
    /// Get Multiple Dashboard Metadata flow
    GetMultipleDashboardMetadata,
    /// Payment Connector Verify
    VerifyPaymentConnector,
    /// Internal user signup
    InternalUserSignup,
    /// Create tenant level user
    TenantUserCreate,
    /// Switch org
    SwitchOrg,
    /// Switch merchant v2
    SwitchMerchantV2,
    /// Switch profile
    SwitchProfile,
    /// Get permission info
    GetAuthorizationInfo,
    /// Get Roles info
    GetRolesInfo,
    /// Get Parent Group Info
    GetParentGroupInfo,
    /// List roles v2
    ListRolesV2,
    /// List invitable roles at entity level
    ListInvitableRolesAtEntityLevel,
    /// List updatable roles at entity level
    ListUpdatableRolesAtEntityLevel,
    /// Get role
    GetRole,
    /// Get parent info for role
    GetRoleV2,
    /// Get role from token
    GetRoleFromToken,
    /// Get resources and groups for role from token
    GetRoleFromTokenV2,
    /// Update user role
    UpdateUserRole,
    /// Create merchant account for user in a org
    UserMerchantAccountCreate,
    /// Create Org in a given tenancy
    UserOrgMerchantCreate,
    /// Generate Sample Data
    GenerateSampleData,
    /// Delete Sample Data
    DeleteSampleData,
    /// Get details of a user
    GetUserDetails,
    /// Get details of a user role in a merchant account
    GetUserRoleDetails,
    /// PaymentMethodAuth Link token create
    PmAuthLinkTokenCreate,
    /// PaymentMethodAuth Exchange token create
    PmAuthExchangeToken,
    /// Get reset password link
    ForgotPassword,
    /// Reset password using link
    ResetPassword,
    /// Force set or force change password
    RotatePassword,
    /// Invite multiple users
    InviteMultipleUser,
    /// Reinvite user
    ReInviteUser,
    /// Accept invite from email
    AcceptInviteFromEmail,
    /// Delete user role
    DeleteUserRole,
    /// Incremental Authorization flow
    PaymentsIncrementalAuthorization,
    /// Get action URL for connector onboarding
    GetActionUrl,
    /// Sync connector onboarding status
    SyncOnboardingStatus,
    /// Reset tracking id
    ResetTrackingId,
    /// Verify email Token
    VerifyEmail,
    /// Send verify email
    VerifyEmailRequest,
    /// Update user account details
    UpdateUserAccountDetails,
    /// Accept user invitation using entities
    AcceptInvitationsV2,
    /// Accept user invitation using entities before user login
    AcceptInvitationsPreAuth,
    /// Initiate external authentication for a payment
    PaymentsExternalAuthentication,
    /// Authorize the payment after external 3ds authentication
    PaymentsAuthorize,
    /// Create Role
    CreateRole,
    /// Update Role
    UpdateRole,
    /// User email flow start
    UserFromEmail,
    /// Begin TOTP
    TotpBegin,
    /// Reset TOTP
    TotpReset,
    /// Verify TOTP
    TotpVerify,
    /// Update TOTP secret
    TotpUpdate,
    /// Verify Access Code
    RecoveryCodeVerify,
    /// Generate or Regenerate recovery codes
    RecoveryCodesGenerate,
    /// Terminate two factor authentication
    TerminateTwoFactorAuth,
    /// Check 2FA status
    TwoFactorAuthStatus,
    /// Create user authentication method
    CreateUserAuthenticationMethod,
    /// Update user authentication method
    UpdateUserAuthenticationMethod,
    /// List user authentication methods
    ListUserAuthenticationMethods,
    /// Get sso auth url
    GetSsoAuthUrl,
    /// Signin with SSO
    SignInWithSso,
    /// Auth Select
    AuthSelect,
    /// List Orgs for user
    ListOrgForUser,
    /// List Merchants for user in org
    ListMerchantsForUserInOrg,
    /// List Profile for user in org and merchant
    ListProfileForUserInOrgAndMerchant,
    /// List Users in Org
    ListUsersInLineage,
    /// List invitations for user
    ListInvitationsForUser,
    /// Get theme using lineage
    GetThemeUsingLineage,
    /// Get theme using theme id
    GetThemeUsingThemeId,
    /// Upload file to theme storage
    UploadFileToThemeStorage,
    /// Create theme
    CreateTheme,
    /// Update theme
    UpdateTheme,
    /// Delete theme
    DeleteTheme,
    /// List initial webhook delivery attempts
    WebhookEventInitialDeliveryAttemptList,
    /// List delivery attempts for a webhook event
    WebhookEventDeliveryAttemptList,
    /// Manually retry the delivery for a webhook event
    WebhookEventDeliveryRetry,
    /// Retrieve status of the Poll
    RetrievePollStatus,
    /// Toggles the extended card info feature in profile level
    ToggleExtendedCardInfo,
    /// Toggles the extended card info feature in profile level
    ToggleConnectorAgnosticMit,
    /// Get the extended card info associated to a payment_id
    GetExtendedCardInfo,
    /// Manually update the refund details like status, error code, error message etc.
    RefundsManualUpdate,
    /// Manually update the payment details like status, error code, error message etc.
    PaymentsManualUpdate,
    /// Dynamic Tax Calcultion
    SessionUpdateTaxCalculation,
    /// Payments post session tokens flow
    PaymentsPostSessionTokens,
    /// Payments start redirection flow
    PaymentStartRedirection,
    /// Volume split on the routing type
    VolumeSplitOnRoutingType,
    /// Relay flow
    Relay,
    /// Relay retrieve flow
    RelayRetrieve,
    /// Incoming Relay Webhook Receive
    IncomingRelayWebhookReceive,
    /// Payment Method Session Create
    PaymentMethodSessionCreate,
    /// Payment Method Session Retrieve
    PaymentMethodSessionRetrieve,
    /// Update a saved payment method using the payment methods session
    PaymentMethodSessionUpdateSavedPaymentMethod,
}

/// Trait for providing generic behaviour to flow metric
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
