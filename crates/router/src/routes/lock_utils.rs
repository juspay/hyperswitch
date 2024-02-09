use router_env::Flow;

#[derive(Clone, Debug, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum ApiIdentifier {
    Payments,
    Refunds,
    Webhooks,
    MerchantAccount,
    MerchantConnector,
    Configs,
    Customers,
    Ephemeral,
    Health,
    Mandates,
    PaymentMethods,
    PaymentMethodAuth,
    Payouts,
    Disputes,
    CardsInfo,
    Files,
    Cache,
    Business,
    Verification,
    ApiKeys,
    PaymentLink,
    Routing,
    Blocklist,
    Forex,
    RustLockerMigration,
    Gsm,
    User,
    UserRole,
    ConnectorOnboarding,
    Recon,
}

impl From<Flow> for ApiIdentifier {
    fn from(flow: Flow) -> Self {
        match flow {
            Flow::MerchantsAccountCreate
            | Flow::MerchantsAccountRetrieve
            | Flow::MerchantsAccountUpdate
            | Flow::MerchantsAccountDelete
            | Flow::MerchantAccountList => Self::MerchantAccount,

            Flow::RoutingCreateConfig
            | Flow::RoutingLinkConfig
            | Flow::RoutingUnlinkConfig
            | Flow::RoutingRetrieveConfig
            | Flow::RoutingRetrieveActiveConfig
            | Flow::RoutingRetrieveDefaultConfig
            | Flow::RoutingRetrieveDictionary
            | Flow::RoutingUpdateConfig
            | Flow::RoutingUpdateDefaultConfig
            | Flow::RoutingDeleteConfig
            | Flow::DecisionManagerDeleteConfig
            | Flow::DecisionManagerRetrieveConfig
            | Flow::DecisionManagerUpsertConfig => Self::Routing,

            Flow::RetrieveForexFlow => Self::Forex,

            Flow::AddToBlocklist => Self::Blocklist,
            Flow::DeleteFromBlocklist => Self::Blocklist,
            Flow::ListBlocklist => Self::Blocklist,

            Flow::MerchantConnectorsCreate
            | Flow::MerchantConnectorsRetrieve
            | Flow::MerchantConnectorsUpdate
            | Flow::MerchantConnectorsDelete
            | Flow::MerchantConnectorsList => Self::MerchantConnector,

            Flow::ConfigKeyCreate
            | Flow::ConfigKeyFetch
            | Flow::ConfigKeyUpdate
            | Flow::CreateConfigKey => Self::Configs,

            Flow::CustomersCreate
            | Flow::CustomersRetrieve
            | Flow::CustomersUpdate
            | Flow::CustomersDelete
            | Flow::CustomersGetMandates
            | Flow::CustomersList => Self::Customers,

            Flow::EphemeralKeyCreate | Flow::EphemeralKeyDelete => Self::Ephemeral,

            Flow::DeepHealthCheck => Self::Health,
            Flow::MandatesRetrieve | Flow::MandatesRevoke | Flow::MandatesList => Self::Mandates,

            Flow::PaymentMethodsCreate
            | Flow::PaymentMethodsList
            | Flow::CustomerPaymentMethodsList
            | Flow::PaymentMethodsRetrieve
            | Flow::PaymentMethodsUpdate
            | Flow::PaymentMethodsDelete
            | Flow::ValidatePaymentMethod => Self::PaymentMethods,

            Flow::PmAuthLinkTokenCreate | Flow::PmAuthExchangeToken => Self::PaymentMethodAuth,

            Flow::PaymentsCreate
            | Flow::PaymentsRetrieve
            | Flow::PaymentsUpdate
            | Flow::PaymentsConfirm
            | Flow::PaymentsCapture
            | Flow::PaymentsCancel
            | Flow::PaymentsApprove
            | Flow::PaymentsReject
            | Flow::PaymentsSessionToken
            | Flow::PaymentsStart
            | Flow::PaymentsList
            | Flow::PaymentsRedirect
            | Flow::PaymentsIncrementalAuthorization => Self::Payments,

            Flow::PayoutsCreate
            | Flow::PayoutsRetrieve
            | Flow::PayoutsUpdate
            | Flow::PayoutsCancel
            | Flow::PayoutsFulfill
            | Flow::PayoutsAccounts => Self::Payouts,

            Flow::RefundsCreate
            | Flow::RefundsRetrieve
            | Flow::RefundsUpdate
            | Flow::RefundsList => Self::Refunds,

            Flow::FrmFulfillment | Flow::IncomingWebhookReceive => Self::Webhooks,

            Flow::ApiKeyCreate
            | Flow::ApiKeyRetrieve
            | Flow::ApiKeyUpdate
            | Flow::ApiKeyRevoke
            | Flow::ApiKeyList => Self::ApiKeys,

            Flow::DisputesRetrieve
            | Flow::DisputesList
            | Flow::DisputesEvidenceSubmit
            | Flow::AttachDisputeEvidence
            | Flow::RetrieveDisputeEvidence
            | Flow::DeleteDisputeEvidence => Self::Disputes,

            Flow::CardsInfo => Self::CardsInfo,

            Flow::CreateFile | Flow::DeleteFile | Flow::RetrieveFile => Self::Files,

            Flow::CacheInvalidate => Self::Cache,

            Flow::BusinessProfileCreate
            | Flow::BusinessProfileUpdate
            | Flow::BusinessProfileRetrieve
            | Flow::BusinessProfileDelete
            | Flow::BusinessProfileList => Self::Business,

            Flow::PaymentLinkRetrieve
            | Flow::PaymentLinkInitiate
            | Flow::PaymentLinkList
            | Flow::PaymentLinkStatus => Self::PaymentLink,

            Flow::Verification => Self::Verification,

            Flow::RustLockerMigration => Self::RustLockerMigration,
            Flow::GsmRuleCreate
            | Flow::GsmRuleRetrieve
            | Flow::GsmRuleUpdate
            | Flow::GsmRuleDelete => Self::Gsm,

            Flow::UserConnectAccount
            | Flow::UserSignUp
            | Flow::UserSignInWithoutInviteChecks
            | Flow::UserSignIn
            | Flow::Signout
            | Flow::ChangePassword
            | Flow::SetDashboardMetadata
            | Flow::GetMutltipleDashboardMetadata
            | Flow::VerifyPaymentConnector
            | Flow::InternalUserSignup
            | Flow::SwitchMerchant
            | Flow::UserMerchantAccountCreate
            | Flow::GenerateSampleData
            | Flow::DeleteSampleData
            | Flow::UserMerchantAccountList
            | Flow::GetUserDetails
            | Flow::ForgotPassword
            | Flow::ResetPassword
            | Flow::InviteUser
            | Flow::InviteMultipleUser
            | Flow::ReInviteUser
            | Flow::UserSignUpWithMerchantId
            | Flow::VerifyEmailWithoutInviteChecks
            | Flow::VerifyEmail
            | Flow::VerifyEmailRequest
            | Flow::UpdateUserAccountDetails => Self::User,

            Flow::ListRoles
            | Flow::GetRole
            | Flow::GetRoleFromToken
            | Flow::UpdateUserRole
            | Flow::GetAuthorizationInfo
            | Flow::AcceptInvitation
            | Flow::DeleteUserRole => Self::UserRole,

            Flow::GetActionUrl | Flow::SyncOnboardingStatus | Flow::ResetTrackingId => {
                Self::ConnectorOnboarding
            }

            Flow::ReconMerchantUpdate
            | Flow::ReconTokenRequest
            | Flow::ReconServiceRequest
            | Flow::ReconVerifyToken => Self::Recon,
        }
    }
}
