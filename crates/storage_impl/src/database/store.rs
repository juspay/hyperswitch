use async_bb8_diesel::{AsyncConnection, ConnectionError};
use bb8::CustomizeConnection;
use common_utils::{
    types::{keymanager, TenantConfig},
    DbConnectionParams,
};
use diesel::PgConnection;
use error_stack::ResultExt;

use crate::{
    config::Database,
    errors::{StorageError, StorageResult},
};

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;
pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

/// Indicates whether a read operation should use the master database
/// or can tolerate eventual consistency from a read replica.
///
/// - `MasterDB` (default): Always reads from the master database.
///   Use this for OLTP operations that require up-to-date data, such as
///   payment confirm, capture, or any read-after-write scenario.
/// - `ReplicaDB`: Reads from the replica pool when available.
///   Use this for OLAP operations like listing, filtering, or analytics
///   where slight staleness is acceptable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadPreference {
    #[default]
    MasterDB,
    ReplicaDB,
}

impl ReadPreference {
    /// Creates a ReadPreference from a Flow.
    /// OLAP flows (list, filter) use ReplicaDB.
    /// All other flows default to MasterDB.
    pub fn from_flow(flow: &router_env::logger::types::Flow) -> Self {
        use router_env::logger::types::Flow;

        match flow {
            Flow::PaymentsList
            | Flow::PaymentsFilters
            | Flow::PaymentsAggregate
            | Flow::PaymentAttemptsList
            | Flow::RefundsList
            | Flow::RefundsFilters
            | Flow::RefundsAggregate
            | Flow::DisputesList
            | Flow::DisputesFilters
            | Flow::CustomersList
            | Flow::CustomersListWithConstraints
            | Flow::MandatesList
            | Flow::PaymentMethodsList
            | Flow::CustomerPaymentMethodsList
            | Flow::MerchantAccountList
            | Flow::MerchantConnectorsList
            | Flow::ApiKeyList
            | Flow::ListBlocklist => ReadPreference::ReplicaDB,

            Flow::HealthCheck
            | Flow::DeepHealthCheck
            | Flow::OidcDiscovery
            | Flow::OidcJwks
            | Flow::OidcAuthorize
            | Flow::OidcToken
            | Flow::OrganizationCreate
            | Flow::OrganizationRetrieve
            | Flow::OrganizationUpdate
            | Flow::MerchantsAccountCreate
            | Flow::MerchantsAccountRetrieve
            | Flow::MerchantsAccountUpdate
            | Flow::MerchantsAccountDelete
            | Flow::MerchantConnectorsCreate
            | Flow::MerchantConnectorsRetrieve
            | Flow::MerchantConnectorsUpdate
            | Flow::MerchantConnectorsDelete
            | Flow::MerchantTransferKey
            | Flow::MerchantConnectorWebhookRegister
            | Flow::MerchantConnectorWebhookList
            | Flow::ConfigKeyCreate
            | Flow::ConfigKeyFetch
            | Flow::EnablePlatformAccount
            | Flow::ConfigKeyUpdate
            | Flow::ConfigKeyDelete
            | Flow::CustomersCreate
            | Flow::CustomersRetrieve
            | Flow::CustomersUpdate
            | Flow::CustomersDelete
            | Flow::CustomersGetMandates
            | Flow::EphemeralKeyCreate
            | Flow::EphemeralKeyDelete
            | Flow::MandatesRetrieve
            | Flow::MandatesRevoke
            | Flow::PaymentMethodsCreate
            | Flow::PaymentMethodsMigrate
            | Flow::PaymentMethodsBatchUpdate
            | Flow::PaymentMethodsBatchRetrieve
            | Flow::PaymentMethodSave
            | Flow::PaymentMethodGetTokenDetails
            | Flow::GetPaymentMethodTokenData
            | Flow::ListCountriesCurrencies
            | Flow::PaymentMethodCollectLink
            | Flow::PaymentMethodsRetrieve
            | Flow::PaymentMethodsRetrieveOlap
            | Flow::PaymentMethodsUpdate
            | Flow::PaymentMethodsDelete
            | Flow::NetworkTokenStatusCheck
            | Flow::NetworkTokenEligibilityCheck
            | Flow::DefaultPaymentMethodsSet
            | Flow::PaymentsCreate
            | Flow::PaymentsRetrieve
            | Flow::PaymentsRetrieveForceSync
            | Flow::PaymentsRetrieveUsingMerchantReferenceId
            | Flow::PaymentsUpdate
            | Flow::PaymentsConfirm
            | Flow::PaymentsCapture
            | Flow::PaymentsCancel
            | Flow::PaymentsCancelPostCapture
            | Flow::PaymentsApprove
            | Flow::PaymentsReject
            | Flow::PaymentsSessionToken
            | Flow::PaymentsStart
            | Flow::PaymentsCreateIntent
            | Flow::PaymentsGetIntent
            | Flow::PaymentsUpdateIntent
            | Flow::PaymentsConfirmIntent
            | Flow::PaymentsCreateAndConfirmIntent
            | Flow::PayoutsCreate
            | Flow::PayoutsRetrieve
            | Flow::PayoutsUpdate
            | Flow::PayoutsConfirm
            | Flow::PayoutsCancel
            | Flow::PayoutsFulfill
            | Flow::PayoutsList
            | Flow::PayoutsFilter
            | Flow::PayoutsAccounts
            | Flow::PayoutLinkInitiate
            | Flow::PaymentsRedirect
            | Flow::PaymentsCompleteAuthorize
            | Flow::RefundsCreate
            | Flow::RefundsRetrieve
            | Flow::RefundsRetrieveForceSync
            | Flow::RefundsUpdate
            | Flow::RetrieveForexFlow
            | Flow::RoutingCreateConfig
            | Flow::RoutingLinkConfig
            | Flow::RoutingUnlinkConfig
            | Flow::RoutingRetrieveConfig
            | Flow::RoutingRetrieveActiveConfig
            | Flow::RoutingRetrieveDefaultConfig
            | Flow::RoutingRetrieveDictionary
            | Flow::DecisionEngineRuleMigration
            | Flow::RoutingUpdateConfig
            | Flow::RoutingUpdateDefaultConfig
            | Flow::RoutingDeleteConfig
            | Flow::CreateSubscription
            | Flow::GetSubscriptionItemsForSubscription
            | Flow::ConfirmSubscription
            | Flow::CreateAndConfirmSubscription
            | Flow::GetSubscription
            | Flow::UpdateSubscription
            | Flow::GetSubscriptionEstimate
            | Flow::PauseSubscription
            | Flow::ResumeSubscription
            | Flow::CancelSubscription
            | Flow::CreateDynamicRoutingConfig
            | Flow::ToggleDynamicRouting
            | Flow::UpdateDynamicRoutingConfigs
            | Flow::AddCardIssuer
            | Flow::UpdateCardIssuer
            | Flow::DeleteCardIssuer
            | Flow::ListCardIssuers
            | Flow::AddToBlocklist
            | Flow::DeleteFromBlocklist
            | Flow::ToggleBlocklistGuard
            | Flow::IncomingWebhookReceive
            | Flow::RecoveryIncomingWebhookReceive
            | Flow::ValidatePaymentMethod
            | Flow::ApiKeyCreate
            | Flow::ApiKeyRetrieve
            | Flow::ApiKeyUpdate
            | Flow::ApiKeyRevoke
            | Flow::DisputesRetrieve
            | Flow::CardsInfo
            | Flow::CreateFile
            | Flow::DeleteFile
            | Flow::RetrieveFile
            | Flow::DisputesEvidenceSubmit
            | Flow::CreateConfigKey
            | Flow::AttachDisputeEvidence
            | Flow::DeleteDisputeEvidence
            | Flow::DisputesAggregate
            | Flow::RetrieveDisputeEvidence
            | Flow::CacheInvalidate
            | Flow::PaymentLinkRetrieve
            | Flow::PaymentLinkInitiate
            | Flow::PaymentSecureLinkInitiate
            | Flow::PaymentLinkList
            | Flow::PaymentLinkStatus
            | Flow::ProfileCreate
            | Flow::ProfileUpdate
            | Flow::ProfileRetrieve
            | Flow::ProfileDelete
            | Flow::ProfileList
            | Flow::Verification
            | Flow::RustLockerMigration
            | Flow::GsmRuleCreate
            | Flow::GsmRuleRetrieve
            | Flow::GsmRuleUpdate
            | Flow::ApplePayCertificatesMigration
            | Flow::GsmRuleDelete
            | Flow::GetDataFromHyperswitchAiFlow
            | Flow::ListAllChatInteractions
            | Flow::UserSignUp
            | Flow::UserSignUpWithMerchantId
            | Flow::ConvertOrganizationToPlatform
            | Flow::UserSignIn
            | Flow::UserTransferKey
            | Flow::UserConnectAccount
            | Flow::DecisionManagerUpsertConfig
            | Flow::DecisionManagerDeleteConfig
            | Flow::DecisionManagerRetrieveConfig
            | Flow::FrmFulfillment
            | Flow::FeatureMatrix
            | Flow::ChangePassword
            | Flow::Signout
            | Flow::SetDashboardMetadata
            | Flow::GetMultipleDashboardMetadata
            | Flow::VerifyPaymentConnector
            | Flow::InternalUserSignup
            | Flow::TenantUserCreate
            | Flow::SwitchOrg
            | Flow::SwitchMerchantV2
            | Flow::SwitchProfile
            | Flow::GetAuthorizationInfo
            | Flow::GetRolesInfo
            | Flow::GetParentGroupInfo
            | Flow::ListRolesV2
            | Flow::ListInvitableRolesAtEntityLevel
            | Flow::ListUpdatableRolesAtEntityLevel
            | Flow::GetRole
            | Flow::GetRoleV2
            | Flow::GetRoleFromToken
            | Flow::GetRoleFromTokenV2
            | Flow::GetParentGroupsInfoForRoleFromToken
            | Flow::UpdateUserRole
            | Flow::UserMerchantAccountCreate
            | Flow::CreatePlatformAccount
            | Flow::UserOrgMerchantCreate
            | Flow::GenerateSampleData
            | Flow::DeleteSampleData
            | Flow::GetUserDetails
            | Flow::GetUserRoleDetails
            | Flow::PmAuthLinkTokenCreate
            | Flow::PmAuthExchangeToken
            | Flow::ForgotPassword
            | Flow::ResetPassword
            | Flow::RotatePassword
            | Flow::InviteMultipleUser
            | Flow::ReInviteUser
            | Flow::AcceptInviteFromEmail
            | Flow::DeleteUserRole
            | Flow::PaymentsIncrementalAuthorization
            | Flow::PaymentsExtendAuthorization
            | Flow::GetActionUrl
            | Flow::SyncOnboardingStatus
            | Flow::ResetTrackingId
            | Flow::VerifyEmail
            | Flow::VerifyEmailRequest
            | Flow::UpdateUserAccountDetails
            | Flow::AcceptInvitationsV2
            | Flow::AcceptInvitationsPreAuth
            | Flow::PaymentsExternalAuthentication
            | Flow::PaymentsAuthorize
            | Flow::CreateRole
            | Flow::CreateRoleV2
            | Flow::UpdateRole
            | Flow::UserFromEmail
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
            | Flow::GetSsoAuthUrl
            | Flow::SignInWithSso
            | Flow::AuthSelect
            | Flow::ListOrgForUser
            | Flow::ListMerchantsForUserInOrg
            | Flow::ListProfileForUserInOrgAndMerchant
            | Flow::ListUsersInLineage
            | Flow::ListInvitationsForUser
            | Flow::GetThemeUsingLineage
            | Flow::GetThemeUsingThemeId
            | Flow::UploadFileToThemeStorage
            | Flow::CreateTheme
            | Flow::UpdateTheme
            | Flow::DeleteTheme
            | Flow::CreateUserTheme
            | Flow::UpdateUserTheme
            | Flow::DeleteUserTheme
            | Flow::UploadFileToUserThemeStorage
            | Flow::GetUserThemeUsingThemeId
            | Flow::ListAllThemesInLineage
            | Flow::GetUserThemeUsingLineage
            | Flow::GetUserThemeConfigVersion
            | Flow::WebhookEventInitialDeliveryAttemptList
            | Flow::WebhookEventDeliveryAttemptList
            | Flow::WebhookEventDeliveryRetry
            | Flow::RetrievePollStatus
            | Flow::ToggleExtendedCardInfo
            | Flow::ToggleConnectorAgnosticMit
            | Flow::GetExtendedCardInfo
            | Flow::RefundsManualUpdate
            | Flow::PaymentsManualUpdate
            | Flow::PayoutsManualUpdate
            | Flow::SessionUpdateTaxCalculation
            | Flow::ProxyConfirmIntent
            | Flow::PaymentsPostSessionTokens
            | Flow::PaymentsUpdateMetadata
            | Flow::PaymentStartRedirection
            | Flow::VolumeSplitOnRoutingType
            | Flow::RoutingEvaluateRule
            | Flow::Relay
            | Flow::RelayRetrieve
            | Flow::TokenizeCard
            | Flow::TokenizeCardUsingPaymentMethodId
            | Flow::TokenizeCardBatch
            | Flow::IncomingRelayWebhookReceive
            | Flow::HypersenseTokenRequest
            | Flow::HypersenseVerifyToken
            | Flow::HypersenseSignoutToken
            | Flow::PaymentMethodSessionCreate
            | Flow::PaymentMethodSessionRetrieve
            | Flow::PaymentMethodSessionUpdate
            | Flow::PaymentMethodSessionUpdateSavedPaymentMethod
            | Flow::PaymentMethodSessionDeleteSavedPaymentMethod
            | Flow::PaymentMethodSessionConfirm
            | Flow::CardsInfoCreate
            | Flow::CardsInfoUpdate
            | Flow::CardsInfoMigrate
            | Flow::TotalPaymentMethodCount
            | Flow::RevenueRecoveryRetrieve
            | Flow::RevenueRecoveryResume
            | Flow::TokenizationCreate
            | Flow::TokenizationRetrieve
            | Flow::CloneConnector
            | Flow::AuthenticationCreate
            | Flow::AuthenticationEligibility
            | Flow::AuthenticationSync
            | Flow::AuthenticationSyncPostUpdate
            | Flow::AuthenticationAuthenticate
            | Flow::AuthenticationSessionToken
            | Flow::AuthenticationEligibilityCheck
            | Flow::AuthenticationRetrieveEligibilityCheck
            | Flow::Proxy
            | Flow::ProfileAcquirerCreate
            | Flow::ProfileAcquirerUpdate
            | Flow::ThreeDsDecisionRuleExecute
            | Flow::IncomingNetworkTokenWebhookReceive
            | Flow::DecisionEngineDecideGatewayCall
            | Flow::DecisionEngineGatewayFeedbackCall
            | Flow::RecoveryPaymentsCreate
            | Flow::TokenizationDelete
            | Flow::RecoveryDataBackfill
            | Flow::RevenueRecoveryRedis
            | Flow::PaymentMethodBalanceCheck
            | Flow::PaymentsSubmitEligibility
            | Flow::ApplyPaymentMethodData
            | Flow::PayoutsAggregate
            | Flow::GetEmbeddedToken
            | Flow::EmbeddedTokenInfo
            | Flow::GetSuperpositionSdkConfig
            | Flow::GetUserDetailsInternal
            | Flow::ListUsersInternal
            | Flow::ListMembersForEntity
            | Flow::AuthorizeUserToken => ReadPreference::MasterDB,
        }
    }
}

#[async_trait::async_trait]
pub trait DatabaseStore: Clone + Send + Sync {
    type Config: Send;
    async fn new(
        config: Self::Config,
        tenant_config: &dyn TenantConfig,
        test_transaction: bool,
        key_manager_state: Option<keymanager::KeyManagerState>,
    ) -> StorageResult<Self>;
    fn get_master_pool(&self) -> &PgPool;
    fn get_replica_pool(&self) -> &PgPool;
    fn get_accounts_master_pool(&self) -> &PgPool;
    fn get_accounts_replica_pool(&self) -> &PgPool;

    /// Returns the current read preference for this store instance.
    /// The default implementation returns `MasterDB`, which is the
    /// safe default for OLTP operations.
    fn get_read_preference(&self) -> ReadPreference {
        ReadPreference::MasterDB
    }
}

#[derive(Debug, Clone)]
pub struct Store {
    pub master_pool: PgPool,
    pub accounts_pool: PgPool,
}

#[async_trait::async_trait]
impl DatabaseStore for Store {
    type Config = Database;
    async fn new(
        config: Database,
        tenant_config: &dyn TenantConfig,
        test_transaction: bool,
        _key_manager_state: Option<keymanager::KeyManagerState>,
    ) -> StorageResult<Self> {
        Ok(Self {
            master_pool: diesel_make_pg_pool(&config, tenant_config.get_schema(), test_transaction)
                .await?,
            accounts_pool: diesel_make_pg_pool(
                &config,
                tenant_config.get_accounts_schema(),
                test_transaction,
            )
            .await?,
        })
    }

    fn get_master_pool(&self) -> &PgPool {
        &self.master_pool
    }

    fn get_replica_pool(&self) -> &PgPool {
        &self.master_pool
    }

    fn get_accounts_master_pool(&self) -> &PgPool {
        &self.accounts_pool
    }

    fn get_accounts_replica_pool(&self) -> &PgPool {
        &self.accounts_pool
    }
}

#[derive(Debug, Clone)]
pub struct ReplicaStore {
    pub master_pool: PgPool,
    pub replica_pool: PgPool,
    pub accounts_master_pool: PgPool,
    pub accounts_replica_pool: PgPool,
}

#[async_trait::async_trait]
impl DatabaseStore for ReplicaStore {
    type Config = (Database, Database);
    async fn new(
        config: (Database, Database),
        tenant_config: &dyn TenantConfig,
        test_transaction: bool,
        _key_manager_state: Option<keymanager::KeyManagerState>,
    ) -> StorageResult<Self> {
        let (master_config, replica_config) = config;
        let master_pool =
            diesel_make_pg_pool(&master_config, tenant_config.get_schema(), test_transaction)
                .await
                .attach_printable("failed to create master pool")?;
        let accounts_master_pool = diesel_make_pg_pool(
            &master_config,
            tenant_config.get_accounts_schema(),
            test_transaction,
        )
        .await
        .attach_printable("failed to create accounts master pool")?;
        let replica_pool = diesel_make_pg_pool(
            &replica_config,
            tenant_config.get_schema(),
            test_transaction,
        )
        .await
        .attach_printable("failed to create replica pool")?;

        let accounts_replica_pool = diesel_make_pg_pool(
            &replica_config,
            tenant_config.get_accounts_schema(),
            test_transaction,
        )
        .await
        .attach_printable("failed to create accounts pool")?;
        Ok(Self {
            master_pool,
            replica_pool,
            accounts_master_pool,
            accounts_replica_pool,
        })
    }

    fn get_master_pool(&self) -> &PgPool {
        &self.master_pool
    }

    fn get_replica_pool(&self) -> &PgPool {
        &self.replica_pool
    }

    fn get_accounts_master_pool(&self) -> &PgPool {
        &self.accounts_master_pool
    }

    fn get_accounts_replica_pool(&self) -> &PgPool {
        &self.accounts_replica_pool
    }
}

pub async fn diesel_make_pg_pool(
    database: &Database,
    schema: &str,
    test_transaction: bool,
) -> StorageResult<PgPool> {
    let database_url = database.get_database_url(schema);
    let manager = async_bb8_diesel::ConnectionManager::<PgConnection>::new(database_url);
    let mut pool = bb8::Pool::builder()
        .max_size(database.pool_size)
        .min_idle(database.min_idle)
        .queue_strategy(database.queue_strategy.into())
        .connection_timeout(std::time::Duration::from_secs(database.connection_timeout))
        .max_lifetime(database.max_lifetime.map(std::time::Duration::from_secs));

    if test_transaction {
        pool = pool.connection_customizer(Box::new(TestTransaction));
    }

    pool.build(manager)
        .await
        .change_context(StorageError::InitializationError)
        .attach_printable("Failed to create PostgreSQL connection pool")
}

#[derive(Debug)]
struct TestTransaction;

#[async_trait::async_trait]
impl CustomizeConnection<PgPooledConn, ConnectionError> for TestTransaction {
    #[allow(clippy::unwrap_used)]
    async fn on_acquire(&self, conn: &mut PgPooledConn) -> Result<(), ConnectionError> {
        use diesel::Connection;

        conn.run(|conn| {
            conn.begin_test_transaction().unwrap();
            Ok(())
        })
        .await
    }
}
