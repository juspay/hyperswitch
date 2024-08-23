use router_env::Flow;

#[derive(Clone, Debug, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum ApiIdentifier {
    Payments,
    Refunds,
    Webhooks,
    Organization,
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
    Role,
    User,
    UserRole,
    ConnectorOnboarding,
    Recon,
    Poll,
    ApplePayCertificatesMigration,
}

impl From<Flow> for ApiIdentifier {
    fn from(flow: Flow) -> Self {
        match flow {
            Flow::MerchantsAccountCreate
            | Flow::MerchantsAccountRetrieve
            | Flow::MerchantsAccountUpdate
            | Flow::MerchantsAccountDelete
            | Flow::MerchantTransferKey
            | Flow::MerchantAccountList => Self::MerchantAccount,

            Flow::OrganizationCreate | Flow::OrganizationRetrieve | Flow::OrganizationUpdate => {
                Self::Organization
            }

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
            Flow::ToggleBlocklistGuard => Self::Blocklist,

            Flow::MerchantConnectorsCreate
            | Flow::MerchantConnectorsRetrieve
            | Flow::MerchantConnectorsUpdate
            | Flow::MerchantConnectorsDelete
            | Flow::MerchantConnectorsList => Self::MerchantConnector,

            Flow::ConfigKeyCreate
            | Flow::ConfigKeyFetch
            | Flow::ConfigKeyUpdate
            | Flow::ConfigKeyDelete
            | Flow::CreateConfigKey => Self::Configs,

            Flow::CustomersCreate
            | Flow::CustomersRetrieve
            | Flow::CustomersUpdate
            | Flow::CustomersDelete
            | Flow::CustomersGetMandates
            | Flow::CustomersList => Self::Customers,

            Flow::EphemeralKeyCreate | Flow::EphemeralKeyDelete => Self::Ephemeral,

            Flow::DeepHealthCheck | Flow::HealthCheck => Self::Health,
            Flow::MandatesRetrieve | Flow::MandatesRevoke | Flow::MandatesList => Self::Mandates,

            Flow::PaymentMethodsCreate
            | Flow::PaymentMethodsMigrate
            | Flow::PaymentMethodsList
            | Flow::CustomerPaymentMethodsList
            | Flow::PaymentMethodsRetrieve
            | Flow::PaymentMethodsUpdate
            | Flow::PaymentMethodsDelete
            | Flow::PaymentMethodCollectLink
            | Flow::ValidatePaymentMethod
            | Flow::ListCountriesCurrencies
            | Flow::DefaultPaymentMethodsSet
            | Flow::PaymentMethodSave => Self::PaymentMethods,

            Flow::PmAuthLinkTokenCreate | Flow::PmAuthExchangeToken => Self::PaymentMethodAuth,

            Flow::PaymentsCreate
            | Flow::PaymentsRetrieve
            | Flow::PaymentsRetrieveForceSync
            | Flow::PaymentsUpdate
            | Flow::PaymentsConfirm
            | Flow::PaymentsCapture
            | Flow::PaymentsCancel
            | Flow::PaymentsApprove
            | Flow::PaymentsReject
            | Flow::PaymentsSessionToken
            | Flow::PaymentsStart
            | Flow::PaymentsList
            | Flow::PaymentsFilters
            | Flow::PaymentsAggregate
            | Flow::PaymentsRedirect
            | Flow::PaymentsIncrementalAuthorization
            | Flow::PaymentsExternalAuthentication
            | Flow::PaymentsAuthorize
            | Flow::GetExtendedCardInfo
            | Flow::PaymentsCompleteAuthorize
            | Flow::PaymentsManualUpdate => Self::Payments,

            Flow::PayoutsCreate
            | Flow::PayoutsRetrieve
            | Flow::PayoutsUpdate
            | Flow::PayoutsCancel
            | Flow::PayoutsFulfill
            | Flow::PayoutsList
            | Flow::PayoutsFilter
            | Flow::PayoutsAccounts
            | Flow::PayoutsConfirm
            | Flow::PayoutLinkInitiate => Self::Payouts,

            Flow::RefundsCreate
            | Flow::RefundsRetrieve
            | Flow::RefundsRetrieveForceSync
            | Flow::RefundsUpdate
            | Flow::RefundsList
            | Flow::RefundsFilters
            | Flow::RefundsManualUpdate => Self::Refunds,

            Flow::FrmFulfillment
            | Flow::IncomingWebhookReceive
            | Flow::WebhookEventInitialDeliveryAttemptList
            | Flow::WebhookEventDeliveryAttemptList
            | Flow::WebhookEventDeliveryRetry => Self::Webhooks,

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
            | Flow::BusinessProfileList
            | Flow::ToggleExtendedCardInfo
            | Flow::ToggleConnectorAgnosticMit => Self::Business,

            Flow::PaymentLinkRetrieve
            | Flow::PaymentLinkInitiate
            | Flow::PaymentSecureLinkInitiate
            | Flow::PaymentLinkList
            | Flow::PaymentLinkStatus => Self::PaymentLink,

            Flow::Verification => Self::Verification,

            Flow::RustLockerMigration => Self::RustLockerMigration,
            Flow::GsmRuleCreate
            | Flow::GsmRuleRetrieve
            | Flow::GsmRuleUpdate
            | Flow::GsmRuleDelete => Self::Gsm,

            Flow::ApplePayCertificatesMigration => Self::ApplePayCertificatesMigration,

            Flow::UserConnectAccount
            | Flow::UserSignUp
            | Flow::UserSignIn
            | Flow::Signout
            | Flow::ChangePassword
            | Flow::SetDashboardMetadata
            | Flow::GetMultipleDashboardMetadata
            | Flow::VerifyPaymentConnector
            | Flow::InternalUserSignup
            | Flow::SwitchMerchant
            | Flow::UserMerchantAccountCreate
            | Flow::GenerateSampleData
            | Flow::DeleteSampleData
            | Flow::UserMerchantAccountList
            | Flow::GetUserDetails
            | Flow::GetUserRoleDetails
            | Flow::ListUsersForMerchantAccount
            | Flow::ForgotPassword
            | Flow::ResetPassword
            | Flow::RotatePassword
            | Flow::InviteMultipleUser
            | Flow::ReInviteUser
            | Flow::UserSignUpWithMerchantId
            | Flow::VerifyEmail
            | Flow::AcceptInviteFromEmail
            | Flow::VerifyEmailRequest
            | Flow::UpdateUserAccountDetails
            | Flow::TotpBegin
            | Flow::TotpReset
            | Flow::TotpVerify
            | Flow::TotpUpdate
            | Flow::RecoveryCodeVerify
            | Flow::RecoveryCodesGenerate
            | Flow::TerminateTwoFactorAuth
            | Flow::TwoFactorAuthStatus
            | Flow::CreateUserAuthenticationMethod
            | Flow::UpdateUserAuthenticationMethod
            | Flow::ListUserAuthenticationMethods
            | Flow::UserTransferKey
            | Flow::GetSsoAuthUrl
            | Flow::SignInWithSso
            | Flow::ListOrgForUser
            | Flow::ListMerchantsForUserInOrg
            | Flow::ListProfileForUserInOrgAndMerchant
            | Flow::AuthSelect => Self::User,

            Flow::ListRoles
            | Flow::GetRole
            | Flow::GetRoleFromToken
            | Flow::UpdateUserRole
            | Flow::GetAuthorizationInfo
            | Flow::GetRolesInfo
            | Flow::AcceptInvitation
            | Flow::MerchantSelect
            | Flow::DeleteUserRole
            | Flow::TransferOrgOwnership
            | Flow::CreateRole
            | Flow::UpdateRole
            | Flow::UserFromEmail => Self::UserRole,

            Flow::GetActionUrl | Flow::SyncOnboardingStatus | Flow::ResetTrackingId => {
                Self::ConnectorOnboarding
            }

            Flow::ReconMerchantUpdate
            | Flow::ReconTokenRequest
            | Flow::ReconServiceRequest
            | Flow::ReconVerifyToken => Self::Recon,

            Flow::RetrievePollStatus => Self::Poll,
        }
    }
}
