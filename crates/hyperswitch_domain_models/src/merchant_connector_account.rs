#[cfg(feature = "v2")]
use std::collections::HashMap;

use common_utils::{
    crypto::Encryptable,
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    ext_traits::{StringExt, ValueExt},
    id_type, pii, type_name,
    types::keymanager::{Identifier, KeyManagerState, ToEncryptable},
};
use common_enums as enums;
use error_stack::ResultExt;
use hyperswitch_masking::{PeekInterface, Secret};
use rustc_hash::FxHashMap;
use serde_json::Value;

#[cfg(feature = "v2")]
use crate::errors::api_error_response;
use crate::{
    mandates::CommonMandateReference,
    merchant_key_store::MerchantKeyStore,
    router_data,
    type_encryption::{crypto_operation, CryptoOperation},
};

#[cfg(feature = "v1")]
#[derive(Clone, Debug, router_derive::ToEncryption)]
pub struct MerchantConnectorAccount {
    pub merchant_id: id_type::MerchantId,
    pub connector_name: String,
    #[encrypt]
    pub connector_account_details: Encryptable<Secret<Value>>,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,
    pub payment_methods_enabled: Option<Vec<pii::SecretSerdeValue>>,
    pub connector_type: enums::ConnectorType,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub frm_configs: Option<Vec<pii::SecretSerdeValue>>,
    pub connector_label: Option<String>,
    pub business_country: Option<enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub business_sub_label: Option<String>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub connector_webhook_details: Option<pii::SecretSerdeValue>,
    pub profile_id: id_type::ProfileId,
    pub applepay_verified_domains: Option<Vec<String>>,
    pub pm_auth_config: Option<pii::SecretSerdeValue>,
    pub status: enums::ConnectorStatus,
    #[encrypt]
    pub connector_wallets_details: Option<Encryptable<Secret<Value>>>,
    #[encrypt]
    pub additional_merchant_data: Option<Encryptable<Secret<Value>>>,
    pub version: common_enums::ApiVersion,
    pub connector_webhook_registration_details: Option<Value>,
}

#[cfg(feature = "v1")]
impl MerchantConnectorAccount {
    pub fn get_id(&self) -> id_type::MerchantConnectorAccountId {
        self.merchant_connector_id.clone()
    }
    pub fn get_connector_account_details(
        &self,
    ) -> error_stack::Result<router_data::ConnectorAuthType, common_utils::errors::ParsingError>
    {
        self.connector_account_details
            .get_inner()
            .clone()
            .parse_value("ConnectorAuthType")
    }

    pub fn get_connector_wallets_details(&self) -> Option<Secret<Value>> {
        self.connector_wallets_details.as_deref().cloned()
    }

    pub fn get_connector_test_mode(&self) -> Option<bool> {
        self.test_mode
    }

    pub fn get_connector_name_as_string(&self) -> String {
        self.connector_name.clone()
    }

    pub fn get_metadata(&self) -> Option<Secret<Value>> {
        self.metadata.clone()
    }

    pub fn get_ctp_service_provider(
        &self,
    ) -> error_stack::Result<
        Option<common_enums::CtpServiceProvider>,
        common_utils::errors::ParsingError,
    > {
        let provider = self
            .connector_name
            .clone()
            .parse_enum("CtpServiceProvider")
            .attach_printable_lazy(|| {
                format!(
                    "Failed to parse ctp service provider from connector_name: {}",
                    self.connector_name
                )
            })?;

        Ok(Some(provider))
    }

    pub fn should_construct_webhook_setup_capability(&self) -> bool {
        matches!(self.connector_type, enums::ConnectorType::PaymentProcessor)
    }

    pub fn get_connector_webhook_registration_details(&self) -> Option<Value> {
        self.connector_webhook_registration_details.clone()
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub enum MerchantConnectorAccountTypeDetails {
    MerchantConnectorAccount(Box<MerchantConnectorAccount>),
    MerchantConnectorDetails(common_types::domain::MerchantConnectorAuthDetails),
}

#[cfg(feature = "v2")]
impl MerchantConnectorAccountTypeDetails {
    pub fn get_connector_account_details(
        &self,
    ) -> error_stack::Result<router_data::ConnectorAuthType, common_utils::errors::ParsingError>
    {
        match self {
            Self::MerchantConnectorAccount(merchant_connector_account) => {
                merchant_connector_account
                    .connector_account_details
                    .peek()
                    .clone()
                    .parse_value("ConnectorAuthType")
            }
            Self::MerchantConnectorDetails(merchant_connector_details) => {
                merchant_connector_details
                    .merchant_connector_creds
                    .peek()
                    .clone()
                    .parse_value("ConnectorAuthType")
            }
        }
    }

    pub fn is_disabled(&self) -> bool {
        match self {
            Self::MerchantConnectorAccount(merchant_connector_account) => {
                merchant_connector_account.disabled.unwrap_or(false)
            }
            Self::MerchantConnectorDetails(_) => false,
        }
    }

    pub fn get_metadata(&self) -> Option<Secret<Value>> {
        match self {
            Self::MerchantConnectorAccount(merchant_connector_account) => {
                merchant_connector_account.metadata.to_owned()
            }
            Self::MerchantConnectorDetails(_) => None,
        }
    }

    pub fn get_id(&self) -> Option<id_type::MerchantConnectorAccountId> {
        match self {
            Self::MerchantConnectorAccount(merchant_connector_account) => {
                Some(merchant_connector_account.id.clone())
            }
            Self::MerchantConnectorDetails(_) => None,
        }
    }

    pub fn get_mca_id(&self) -> Option<id_type::MerchantConnectorAccountId> {
        match self {
            Self::MerchantConnectorAccount(merchant_connector_account) => {
                Some(merchant_connector_account.get_id())
            }
            Self::MerchantConnectorDetails(_) => None,
        }
    }

    pub fn get_connector_name(&self) -> common_enums::connector_enums::Connector {
        match self {
            Self::MerchantConnectorAccount(merchant_connector_account) => {
                merchant_connector_account.connector_name
            }
            Self::MerchantConnectorDetails(merchant_connector_details) => {
                merchant_connector_details.connector_name
            }
        }
    }

    pub fn get_connector_name_as_string(&self) -> String {
        match self {
            Self::MerchantConnectorAccount(merchant_connector_account) => {
                merchant_connector_account.connector_name.to_string()
            }
            Self::MerchantConnectorDetails(merchant_connector_details) => {
                merchant_connector_details.connector_name.to_string()
            }
        }
    }

    pub fn get_inner_db_merchant_connector_account(&self) -> Option<&MerchantConnectorAccount> {
        match self {
            Self::MerchantConnectorAccount(merchant_connector_account) => {
                Some(merchant_connector_account)
            }
            Self::MerchantConnectorDetails(_) => None,
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, router_derive::ToEncryption)]
pub struct MerchantConnectorAccount {
    pub id: id_type::MerchantConnectorAccountId,
    pub merchant_id: id_type::MerchantId,
    pub connector_name: common_enums::connector_enums::Connector,
    #[encrypt]
    pub connector_account_details: Encryptable<Secret<Value>>,
    pub disabled: Option<bool>,
    pub payment_methods_enabled: Option<Vec<common_types::payment_methods::PaymentMethodsEnabled>>,
    pub connector_type: enums::ConnectorType,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub frm_configs: Option<Vec<pii::SecretSerdeValue>>,
    pub connector_label: Option<String>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub connector_webhook_details: Option<pii::SecretSerdeValue>,
    pub profile_id: id_type::ProfileId,
    pub applepay_verified_domains: Option<Vec<String>>,
    pub pm_auth_config: Option<pii::SecretSerdeValue>,
    pub status: enums::ConnectorStatus,
    #[encrypt]
    pub connector_wallets_details: Option<Encryptable<Secret<Value>>>,
    #[encrypt]
    pub additional_merchant_data: Option<Encryptable<Secret<Value>>>,
    pub version: common_enums::ApiVersion,
    pub feature_metadata: Option<MerchantConnectorAccountFeatureMetadata>,
}

#[cfg(feature = "v2")]
impl MerchantConnectorAccount {
    pub fn get_retry_threshold(&self) -> Option<u16> {
        self.feature_metadata
            .as_ref()
            .and_then(|metadata| metadata.revenue_recovery.as_ref())
            .map(|recovery| recovery.billing_connector_retry_threshold)
    }

    pub fn get_id(&self) -> id_type::MerchantConnectorAccountId {
        self.id.clone()
    }

    pub fn get_metadata(&self) -> Option<pii::SecretSerdeValue> {
        self.metadata.clone()
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled.unwrap_or(false)
    }

    pub fn get_connector_account_details(
        &self,
    ) -> error_stack::Result<router_data::ConnectorAuthType, common_utils::errors::ParsingError>
    {
        use common_utils::ext_traits::ValueExt;

        self.connector_account_details
            .get_inner()
            .clone()
            .parse_value("ConnectorAuthType")
    }

    pub fn get_connector_wallets_details(&self) -> Option<Secret<Value>> {
        self.connector_wallets_details.as_deref().cloned()
    }

    pub fn get_connector_test_mode(&self) -> Option<bool> {
        todo!()
    }

    pub fn get_connector_name_as_string(&self) -> String {
        self.connector_name.clone().to_string()
    }

    #[cfg(feature = "v2")]
    pub fn get_connector_name(&self) -> common_enums::connector_enums::Connector {
        self.connector_name
    }

    pub fn get_payment_merchant_connector_account_id_using_account_reference_id(
        &self,
        account_reference_id: String,
    ) -> Option<id_type::MerchantConnectorAccountId> {
        self.feature_metadata.as_ref().and_then(|metadata| {
            metadata.revenue_recovery.as_ref().and_then(|recovery| {
                recovery
                    .mca_reference
                    .billing_to_recovery
                    .get(&account_reference_id)
                    .cloned()
            })
        })
    }
    pub fn get_account_reference_id_using_payment_merchant_connector_account_id(
        &self,
        payment_merchant_connector_account_id: id_type::MerchantConnectorAccountId,
    ) -> Option<String> {
        self.feature_metadata.as_ref().and_then(|metadata| {
            metadata.revenue_recovery.as_ref().and_then(|recovery| {
                recovery
                    .mca_reference
                    .recovery_to_billing
                    .get(&payment_merchant_connector_account_id)
                    .cloned()
            })
        })
    }
}

#[cfg(feature = "v2")]
/// Holds the payment methods enabled for a connector along with the connector name
/// This struct is a flattened representation of the payment methods enabled for a connector
#[derive(Debug)]
pub struct PaymentMethodsEnabledForConnector {
    pub payment_methods_enabled: common_types::payment_methods::RequestPaymentMethodTypes,
    pub payment_method: common_enums::PaymentMethod,
    pub connector: common_enums::connector_enums::Connector,
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone)]
pub struct MerchantConnectorAccountFeatureMetadata {
    pub revenue_recovery: Option<RevenueRecoveryMetadata>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone)]
pub struct RevenueRecoveryMetadata {
    pub max_retry_count: u16,
    pub billing_connector_retry_threshold: u16,
    pub mca_reference: AccountReferenceMap,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExternalVaultConnectorMetadata {
    pub proxy_url: common_utils::types::Url,
    pub certificate: Secret<String>,
}
#[cfg(feature = "v2")]
#[derive(Debug, Clone)]
pub struct AccountReferenceMap {
    pub recovery_to_billing: HashMap<id_type::MerchantConnectorAccountId, String>,
    pub billing_to_recovery: HashMap<String, id_type::MerchantConnectorAccountId>,
}

#[cfg(feature = "v2")]
impl AccountReferenceMap {
    pub fn new(
        hash_map: HashMap<id_type::MerchantConnectorAccountId, String>,
    ) -> Result<Self, api_error_response::ApiErrorResponse> {
        Self::validate(&hash_map)?;

        let recovery_to_billing = hash_map.clone();
        let mut billing_to_recovery = HashMap::new();

        for (key, value) in &hash_map {
            billing_to_recovery.insert(value.clone(), key.clone());
        }

        Ok(Self {
            recovery_to_billing,
            billing_to_recovery,
        })
    }

    fn validate(
        hash_map: &HashMap<id_type::MerchantConnectorAccountId, String>,
    ) -> Result<(), api_error_response::ApiErrorResponse> {
        let mut seen_values = std::collections::HashSet::new(); // To check uniqueness of values

        for value in hash_map.values() {
            if !seen_values.insert(value.clone()) {
                return Err(api_error_response::ApiErrorResponse::InvalidRequestData {
                    message: "Duplicate account reference IDs found in Recovery feature metadata. Each account reference ID must be unique.".to_string(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(feature = "v2")]
/// Holds the payment methods enabled for a connector
pub struct FlattenedPaymentMethodsEnabled {
    pub payment_methods_enabled: Vec<PaymentMethodsEnabledForConnector>,
}

#[cfg(feature = "v2")]
impl FlattenedPaymentMethodsEnabled {
    /// This functions flattens the payment methods enabled from the connector accounts
    /// Retains the connector name and payment method in every flattened element
    pub fn from_payment_connectors_list(payment_connectors: Vec<MerchantConnectorAccount>) -> Self {
        let payment_methods_enabled_flattened_with_connector = payment_connectors
            .into_iter()
            .map(|connector| {
                (
                    connector
                        .payment_methods_enabled
                        .clone()
                        .unwrap_or_default(),
                    connector.connector_name,
                    connector.get_id(),
                )
            })
            .flat_map(
                |(payment_method_enabled, connector, merchant_connector_id)| {
                    payment_method_enabled
                        .into_iter()
                        .flat_map(move |payment_method| {
                            let request_payment_methods_enabled =
                                payment_method.payment_method_subtypes.unwrap_or_default();
                            let length = request_payment_methods_enabled.len();
                            request_payment_methods_enabled
                                .into_iter()
                                .zip(std::iter::repeat_n(
                                    (
                                        connector,
                                        merchant_connector_id.clone(),
                                        payment_method.payment_method_type,
                                    ),
                                    length,
                                ))
                        })
                },
            )
            .map(
                |(request_payment_methods, (connector, merchant_connector_id, payment_method))| {
                    PaymentMethodsEnabledForConnector {
                        payment_methods_enabled: request_payment_methods,
                        connector,
                        payment_method,
                        merchant_connector_id,
                    }
                },
            )
            .collect();

        Self {
            payment_methods_enabled: payment_methods_enabled_flattened_with_connector,
        }
    }
}

#[cfg(feature = "v1")]
#[derive(Debug)]
pub enum MerchantConnectorAccountUpdate {
    Update {
        connector_type: Option<enums::ConnectorType>,
        connector_name: Option<String>,
        connector_account_details: Box<Option<Encryptable<pii::SecretSerdeValue>>>,
        test_mode: Option<bool>,
        disabled: Option<bool>,
        merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
        payment_methods_enabled: Option<Vec<pii::SecretSerdeValue>>,
        metadata: Option<pii::SecretSerdeValue>,
        frm_configs: Option<Vec<pii::SecretSerdeValue>>,
        connector_webhook_details: Box<Option<pii::SecretSerdeValue>>,
        applepay_verified_domains: Option<Vec<String>>,
        pm_auth_config: Box<Option<pii::SecretSerdeValue>>,
        connector_label: Option<String>,
        status: Option<enums::ConnectorStatus>,
        connector_wallets_details: Box<Option<Encryptable<pii::SecretSerdeValue>>>,
        additional_merchant_data: Box<Option<Encryptable<pii::SecretSerdeValue>>>,
    },
    ConnectorWalletDetailsUpdate {
        connector_wallets_details: Encryptable<pii::SecretSerdeValue>,
    },
    ConnectorWebhookRegisterationUpdate {
        connector_webhook_registration_details: Option<Value>,
    },
}

#[cfg(feature = "v2")]
#[derive(Debug)]
pub enum MerchantConnectorAccountUpdate {
    Update {
        connector_type: Option<enums::ConnectorType>,
        connector_account_details: Box<Option<Encryptable<pii::SecretSerdeValue>>>,
        disabled: Option<bool>,
        payment_methods_enabled: Option<Vec<common_types::payment_methods::PaymentMethodsEnabled>>,
        metadata: Option<pii::SecretSerdeValue>,
        frm_configs: Option<Vec<pii::SecretSerdeValue>>,
        connector_webhook_details: Box<Option<pii::SecretSerdeValue>>,
        applepay_verified_domains: Option<Vec<String>>,
        pm_auth_config: Box<Option<pii::SecretSerdeValue>>,
        connector_label: Option<String>,
        status: Option<enums::ConnectorStatus>,
        connector_wallets_details: Box<Option<Encryptable<pii::SecretSerdeValue>>>,
        additional_merchant_data: Box<Option<Encryptable<pii::SecretSerdeValue>>>,
        feature_metadata: Box<Option<MerchantConnectorAccountFeatureMetadata>>,
    },
    ConnectorWalletDetailsUpdate {
        connector_wallets_details: Encryptable<pii::SecretSerdeValue>,
    },
}

common_utils::create_list_wrapper!(
    MerchantConnectorAccounts,
    MerchantConnectorAccount,
    impl_functions: {
        fn filter_and_map<'a, T>(
            &'a self,
            filter: impl Fn(&'a MerchantConnectorAccount) -> bool,
            func: impl Fn(&'a MerchantConnectorAccount) -> T,
        ) -> rustc_hash::FxHashSet<T>
        where
            T: std::hash::Hash + Eq,
        {
            self.0
                .iter()
                .filter(|mca| filter(mca))
                .map(func)
                .collect::<rustc_hash::FxHashSet<_>>()
        }

        pub fn filter_by_profile<'a, T>(
            &'a self,
            profile_id: &'a id_type::ProfileId,
            func: impl Fn(&'a MerchantConnectorAccount) -> T,
        ) -> rustc_hash::FxHashSet<T>
        where
            T: std::hash::Hash + Eq,
        {
            self.filter_and_map(|mca| mca.profile_id == *profile_id, func)
        }
        #[cfg(feature = "v2")]
        pub fn get_connector_and_supporting_payment_method_type_for_session_call(
            &self,
        ) -> Vec<(&MerchantConnectorAccount, common_enums::PaymentMethodType, common_enums::PaymentMethod)> {
            // This vector is created to work around lifetimes
            let ref_vector = Vec::default();

            let connector_and_supporting_payment_method_type = self.iter().flat_map(|connector_account| {
                connector_account
                    .payment_methods_enabled.as_ref()
                    .unwrap_or(&Vec::default())
                    .iter()
                    .flat_map(|payment_method_types| payment_method_types.payment_method_subtypes.as_ref().unwrap_or(&ref_vector).iter().map(|payment_method_subtype| (payment_method_subtype, payment_method_types.payment_method_type)).collect::<Vec<_>>())
                    .filter(|(payment_method_types_enabled, _)| {
                        payment_method_types_enabled.payment_experience == Some(api_models::enums::PaymentExperience::InvokeSdkClient)
                    })
                    .map(|(payment_method_subtypes, payment_method_type)| {
                        (connector_account, payment_method_subtypes.payment_method_subtype, payment_method_type)
                    })
                    .collect::<Vec<_>>()
            }).collect();
            connector_and_supporting_payment_method_type
        }
        pub fn filter_based_on_profile_and_connector_type(
            self,
            profile_id: &id_type::ProfileId,
            connector_type: common_enums::ConnectorType,
        ) -> Self {
            self.into_iter()
                .filter(|mca| &mca.profile_id == profile_id && mca.connector_type == connector_type)
                .collect()
        }
        pub fn is_merchant_connector_account_id_in_connector_mandate_details(
            &self,
            profile_id: Option<&id_type::ProfileId>,
            connector_mandate_details: &CommonMandateReference,
        ) -> bool {
            let mca_ids = self
                .iter()
                .filter(|mca| {
                    mca.disabled.is_some_and(|disabled| !disabled)
                        && profile_id.is_some_and(|profile_id| *profile_id == mca.profile_id)
                })
                .map(|mca| mca.get_id())
                .collect::<std::collections::HashSet<_>>();

            connector_mandate_details
            .payments
            .as_ref()
            .as_ref().is_some_and(|payments| {
                payments.0.keys().any(|mca_id| mca_ids.contains(mca_id))
            })
        }
    }
);

#[cfg(feature = "v2")]
impl From<MerchantConnectorAccountFeatureMetadata>
    for DieselMerchantConnectorAccountFeatureMetadata
{
    fn from(feature_metadata: MerchantConnectorAccountFeatureMetadata) -> Self {
        let revenue_recovery = feature_metadata.revenue_recovery.map(|recovery_metadata| {
            DieselRevenueRecoveryMetadata {
                max_retry_count: recovery_metadata.max_retry_count,
                billing_connector_retry_threshold: recovery_metadata
                    .billing_connector_retry_threshold,
                billing_account_reference: DieselBillingAccountReference(
                    recovery_metadata.mca_reference.recovery_to_billing,
                ),
            }
        });
        Self { revenue_recovery }
    }
}

#[cfg(feature = "v2")]
impl From<DieselMerchantConnectorAccountFeatureMetadata>
    for MerchantConnectorAccountFeatureMetadata
{
    fn from(feature_metadata: DieselMerchantConnectorAccountFeatureMetadata) -> Self {
        let revenue_recovery = feature_metadata.revenue_recovery.map(|recovery_metadata| {
            let mut billing_to_recovery = HashMap::new();
            for (key, value) in &recovery_metadata.billing_account_reference.0 {
                billing_to_recovery.insert(value.to_string(), key.clone());
            }
            RevenueRecoveryMetadata {
                max_retry_count: recovery_metadata.max_retry_count,
                billing_connector_retry_threshold: recovery_metadata
                    .billing_connector_retry_threshold,
                mca_reference: AccountReferenceMap {
                    recovery_to_billing: recovery_metadata.billing_account_reference.0,
                    billing_to_recovery,
                },
            }
        });
        Self { revenue_recovery }
    }
}

#[async_trait::async_trait]
pub trait MerchantConnectorAccountInterface
{
    type Error;
    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        merchant_id: &id_type::MerchantId,
        connector_label: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<MerchantConnectorAccount, Self::Error>;

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        profile_id: &id_type::ProfileId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<MerchantConnectorAccount, Self::Error>;

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        merchant_id: &id_type::MerchantId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<MerchantConnectorAccount>, Self::Error>;

    async fn insert_merchant_connector_account(
        &self,
        t: MerchantConnectorAccount,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<MerchantConnectorAccount, Self::Error>;

    #[cfg(feature = "v1")]
    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &id_type::MerchantId,
        merchant_connector_id: &id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<MerchantConnectorAccount, Self::Error>;

    #[cfg(feature = "v2")]
    async fn find_merchant_connector_account_by_id(
        &self,
        id: &id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<MerchantConnectorAccount, Self::Error>;

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &id_type::MerchantId,
        get_disabled: bool,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<MerchantConnectorAccounts, Self::Error>;

    #[cfg(all(feature = "olap", feature = "v2"))]
    async fn list_connector_account_by_profile_id(
        &self,
        profile_id: &id_type::ProfileId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<MerchantConnectorAccount>, Self::Error>;

    async fn list_enabled_connector_accounts_by_profile_id(
        &self,
        profile_id: &id_type::ProfileId,
        key_store: &MerchantKeyStore,
        connector_type: common_enums::ConnectorType,
    ) -> CustomResult<Vec<MerchantConnectorAccount>, Self::Error>;

    async fn update_merchant_connector_account(
        &self,
        this: MerchantConnectorAccount,
        merchant_connector_account: MerchantConnectorAccountUpdate,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<MerchantConnectorAccount, Self::Error>;

    async fn update_multiple_merchant_connector_accounts(
        &self,
        this: Vec<(
            MerchantConnectorAccount,
            MerchantConnectorAccountUpdate,
        )>,
    ) -> CustomResult<(), Self::Error>;

    #[cfg(feature = "v1")]
    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &id_type::MerchantId,
        merchant_connector_id: &id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, Self::Error>;

    #[cfg(feature = "v2")]
    async fn delete_merchant_connector_account_by_id(
        &self,
        id: &id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, Self::Error>;
}
