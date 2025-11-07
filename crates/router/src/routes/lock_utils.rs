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
    Profile,
    Verification,
    ApiKeys,
    PaymentLink,
    Routing,
    Subscription,
    Blocklist,
    Forex,
    RustLockerMigration,
    Gsm,
    Role,
    User,
    UserRole,
    ConnectorOnboarding,
    Recon,
    AiWorkflow,
    Poll,
    ApplePayCertificatesMigration,
    Relay,
    Documentation,
    CardNetworkTokenization,
    Hypersense,
    PaymentMethodSession,
    ProcessTracker,
    Authentication,
    Proxy,
    ProfileAcquirer,
    ThreeDsDecisionRule,
    GenericTokenization,
    RecoveryRecovery,
}

impl From<Flow> for ApiIdentifier {
    fn from(flow: Flow) -> Self {
        match flow {
            Flow::MerchantsAccountCreate
            | Flow::MerchantsAccountRetrieve
            | Flow::MerchantsAccountUpdate
            | Flow::MerchantsAccountDelete
            | Flow::MerchantTransferKey
            | Flow::MerchantAccountList
            | Flow::EnablePlatformAccount => Self::MerchantAccount,
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
            | Flow::ToggleDynamicRouting
            | Flow::CreateDynamicRoutingConfig
            | Flow::UpdateDynamicRoutingConfigs
            | Flow::DecisionManagerUpsertConfig
            | Flow::RoutingEvaluateRule
            | Flow::DecisionEngineRuleMigration
            | Flow::VolumeSplitOnRoutingType
            | Flow::DecisionEngineDecideGatewayCall
            | Flow::DecisionEngineGatewayFeedbackCall => Self::Routing,
            Flow::CreateSubscription
            | Flow::ConfirmSubscription
            | Flow::CreateAndConfirmSubscription
            | Flow::GetSubscription
            | Flow::UpdateSubscription
            | Flow::GetSubscriptionEstimate
            | Flow::GetPlansForSubscription
            | Flow::PauseSubscription
            | Flow::ResumeSubscription
            | Flow::CancelSubscription => Self::Subscription,
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
            | Flow::CustomersList
            | Flow::CustomersListWithConstraints => Self::Customers,
            Flow::EphemeralKeyCreate | Flow::EphemeralKeyDelete => Self::Ephemeral,
            Flow::DeepHealthCheck | Flow::HealthCheck => Self::Health,
            Flow::MandatesRetrieve | Flow::MandatesRevoke | Flow::MandatesList => Self::Mandates,
            Flow::PaymentMethodsCreate
            | Flow::PaymentMethodsMigrate
            | Flow::PaymentMethodsBatchUpdate
            | Flow::PaymentMethodsList
            | Flow::CustomerPaymentMethodsList
            | Flow::GetPaymentMethodTokenData
            | Flow::PaymentMethodsRetrieve
            | Flow::PaymentMethodsUpdate
            | Flow::PaymentMethodsDelete
            | Flow::NetworkTokenStatusCheck
            | Flow::PaymentMethodCollectLink
            | Flow::ValidatePaymentMethod
            | Flow::ListCountriesCurrencies
            | Flow::DefaultPaymentMethodsSet
            | Flow::PaymentMethodSave
            | Flow::TotalPaymentMethodCount => Self::PaymentMethods,
            Flow::PmAuthLinkTokenCreate | Flow::PmAuthExchangeToken => Self::PaymentMethodAuth,
            Flow::PaymentsCreate
            | Flow::PaymentsRetrieve
            | Flow::PaymentsRetrieveForceSync
            | Flow::PaymentsUpdate
            | Flow::PaymentsConfirm
            | Flow::PaymentsCapture
            | Flow::PaymentsCancel
            | Flow::PaymentsCancelPostCapture
            | Flow::PaymentsApprove
            | Flow::PaymentsReject
            | Flow::PaymentsSessionToken
            | Flow::PaymentsStart
            | Flow::PaymentsList
            | Flow::PaymentsFilters
            | Flow::PaymentsAggregate
            | Flow::PaymentsRedirect
            | Flow::PaymentsIncrementalAuthorization
            | Flow::PaymentsExtendAuthorization
            | Flow::PaymentsExternalAuthentication
            | Flow::PaymentsAuthorize
            | Flow::GetExtendedCardInfo
            | Flow::PaymentsCompleteAuthorize
            | Flow::PaymentsManualUpdate
            | Flow::SessionUpdateTaxCalculation
            | Flow::PaymentsConfirmIntent
            | Flow::PaymentsCreateIntent
            | Flow::PaymentsGetIntent
            | Flow::PaymentMethodBalanceCheck
            | Flow::ApplyPaymentMethodData
            | Flow::PaymentsPostSessionTokens
            | Flow::PaymentsUpdateMetadata
            | Flow::PaymentsUpdateIntent
            | Flow::PaymentsCreateAndConfirmIntent
            | Flow::PaymentStartRedirection
            | Flow::ProxyConfirmIntent
            | Flow::PaymentsRetrieveUsingMerchantReferenceId
            | Flow::PaymentAttemptsList
            | Flow::RecoveryPaymentsCreate
            | Flow::PaymentsSubmitEligibility => Self::Payments,
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
            | Flow::RefundsAggregate
            | Flow::RefundsManualUpdate => Self::Refunds,
            Flow::Relay | Flow::RelayRetrieve => Self::Relay,
            Flow::FrmFulfillment
            | Flow::IncomingWebhookReceive
            | Flow::IncomingRelayWebhookReceive
            | Flow::WebhookEventInitialDeliveryAttemptList
            | Flow::WebhookEventDeliveryAttemptList
            | Flow::WebhookEventDeliveryRetry
            | Flow::RecoveryIncomingWebhookReceive
            | Flow::IncomingNetworkTokenWebhookReceive => Self::Webhooks,
            Flow::ApiKeyCreate
            | Flow::ApiKeyRetrieve
            | Flow::ApiKeyUpdate
            | Flow::ApiKeyRevoke
            | Flow::ApiKeyList => Self::ApiKeys,
            Flow::DisputesRetrieve
            | Flow::DisputesList
            | Flow::DisputesFilters
            | Flow::DisputesEvidenceSubmit
            | Flow::AttachDisputeEvidence
            | Flow::RetrieveDisputeEvidence
            | Flow::DisputesAggregate
            | Flow::DeleteDisputeEvidence => Self::Disputes,
            Flow::CardsInfo
            | Flow::CardsInfoCreate
            | Flow::CardsInfoUpdate
            | Flow::CardsInfoMigrate => Self::CardsInfo,
            Flow::CreateFile | Flow::DeleteFile | Flow::RetrieveFile => Self::Files,
            Flow::CacheInvalidate => Self::Cache,
            Flow::ProfileCreate
            | Flow::ProfileUpdate
            | Flow::ProfileRetrieve
            | Flow::ProfileDelete
            | Flow::ProfileList
            | Flow::ToggleExtendedCardInfo
            | Flow::ToggleConnectorAgnosticMit => Self::Profile,
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
            | Flow::TenantUserCreate
            | Flow::SwitchOrg
            | Flow::SwitchMerchantV2
            | Flow::SwitchProfile
            | Flow::CreatePlatformAccount
            | Flow::UserOrgMerchantCreate
            | Flow::UserMerchantAccountCreate
            | Flow::GenerateSampleData
            | Flow::DeleteSampleData
            | Flow::GetUserDetails
            | Flow::GetUserRoleDetails
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
            | Flow::ListInvitationsForUser
            | Flow::AuthSelect
            | Flow::GetThemeUsingLineage
            | Flow::GetThemeUsingThemeId
            | Flow::UploadFileToThemeStorage
            | Flow::CreateTheme
            | Flow::UpdateTheme
            | Flow::DeleteTheme
            | Flow::CreateUserTheme
            | Flow::UpdateUserTheme
            | Flow::DeleteUserTheme
            | Flow::GetUserThemeUsingThemeId
            | Flow::UploadFileToUserThemeStorage
            | Flow::GetUserThemeUsingLineage
            | Flow::ListAllThemesInLineage
            | Flow::CloneConnector => Self::User,

            Flow::GetDataFromHyperswitchAiFlow | Flow::ListAllChatInteractions => Self::AiWorkflow,

            Flow::ListRolesV2
            | Flow::ListInvitableRolesAtEntityLevel
            | Flow::ListUpdatableRolesAtEntityLevel
            | Flow::GetRole
            | Flow::GetRoleV2
            | Flow::GetRoleFromToken
            | Flow::GetRoleFromTokenV2
            | Flow::GetParentGroupsInfoForRoleFromToken
            | Flow::UpdateUserRole
            | Flow::GetAuthorizationInfo
            | Flow::GetRolesInfo
            | Flow::GetParentGroupInfo
            | Flow::AcceptInvitationsV2
            | Flow::AcceptInvitationsPreAuth
            | Flow::DeleteUserRole
            | Flow::CreateRole
            | Flow::CreateRoleV2
            | Flow::UpdateRole
            | Flow::UserFromEmail
            | Flow::ListUsersInLineage => Self::UserRole,
            Flow::GetActionUrl | Flow::SyncOnboardingStatus | Flow::ResetTrackingId => {
                Self::ConnectorOnboarding
            }
            Flow::ReconMerchantUpdate
            | Flow::ReconTokenRequest
            | Flow::ReconServiceRequest
            | Flow::ReconVerifyToken => Self::Recon,
            Flow::RetrievePollStatus => Self::Poll,
            Flow::FeatureMatrix => Self::Documentation,
            Flow::TokenizeCard
            | Flow::TokenizeCardUsingPaymentMethodId
            | Flow::TokenizeCardBatch => Self::CardNetworkTokenization,
            Flow::HypersenseTokenRequest
            | Flow::HypersenseVerifyToken
            | Flow::HypersenseSignoutToken => Self::Hypersense,
            Flow::PaymentMethodSessionCreate
            | Flow::PaymentMethodSessionRetrieve
            | Flow::PaymentMethodSessionConfirm
            | Flow::PaymentMethodSessionUpdateSavedPaymentMethod
            | Flow::PaymentMethodSessionDeleteSavedPaymentMethod
            | Flow::PaymentMethodSessionUpdate => Self::PaymentMethodSession,
            Flow::RevenueRecoveryRetrieve | Flow::RevenueRecoveryResume => Self::ProcessTracker,
            Flow::AuthenticationCreate
            | Flow::AuthenticationEligibility
            | Flow::AuthenticationSync
            | Flow::AuthenticationSyncPostUpdate
            | Flow::AuthenticationAuthenticate
            | Flow::AuthenticationSessionToken
            | Flow::AuthenticationEligibilityCheck
            | Flow::AuthenticationRetrieveEligibilityCheck => Self::Authentication,
            Flow::Proxy => Self::Proxy,
            Flow::ProfileAcquirerCreate | Flow::ProfileAcquirerUpdate => Self::ProfileAcquirer,
            Flow::ThreeDsDecisionRuleExecute => Self::ThreeDsDecisionRule,
            Flow::TokenizationCreate | Flow::TokenizationRetrieve | Flow::TokenizationDelete => {
                Self::GenericTokenization
            }

            Flow::RecoveryDataBackfill | Flow::RevenueRecoveryRedis => Self::RecoveryRecovery,
        }
    }
}
