#[cfg(feature = "v2")]
use std::collections::HashMap;

#[cfg(feature = "v2")]
use common_utils::transformers::ForeignTryFrom;
use common_utils::{
    crypto::Encryptable,
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    ext_traits::ValueExt,
    id_type, pii, type_name,
    types::keymanager::{Identifier, KeyManagerState, ToEncryptable},
};
#[cfg(feature = "v2")]
use diesel_models::merchant_connector_account::{
    BillingAccountReference as DieselBillingAccountReference,
    MerchantConnectorAccountFeatureMetadata as DieselMerchantConnectorAccountFeatureMetadata,
    RevenueRecoveryMetadata as DieselRevenueRecoveryMetadata,
};
use diesel_models::{enums, merchant_connector_account::MerchantConnectorAccountUpdateInternal};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use rustc_hash::FxHashMap;
use serde_json::Value;

use super::behaviour;
#[cfg(feature = "v2")]
use crate::errors::{self, api_error_response};
use crate::{
    mandates::CommonMandateReference,
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
}

#[cfg(feature = "v2")]
/// Holds the payment methods enabled for a connector along with the connector name
/// This struct is a flattened representation of the payment methods enabled for a connector
#[derive(Debug)]
pub struct PaymentMethodsEnabledForConnector {
    pub payment_methods_enabled: common_types::payment_methods::RequestPaymentMethodTypes,
    pub payment_method: common_enums::PaymentMethod,
    pub connector: common_enums::connector_enums::Connector,
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
                    connector.payment_methods_enabled.unwrap_or_default(),
                    connector.connector_name,
                )
            })
            .flat_map(|(payment_method_enabled, connector_name)| {
                payment_method_enabled
                    .into_iter()
                    .flat_map(move |payment_method| {
                        let request_payment_methods_enabled =
                            payment_method.payment_method_subtypes.unwrap_or_default();
                        let length = request_payment_methods_enabled.len();
                        request_payment_methods_enabled.into_iter().zip(
                            std::iter::repeat((connector_name, payment_method.payment_method_type))
                                .take(length),
                        )
                    })
            })
            .map(
                |(request_payment_methods, (connector_name, payment_method))| {
                    PaymentMethodsEnabledForConnector {
                        payment_methods_enabled: request_payment_methods,
                        connector: connector_name,
                        payment_method,
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

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl behaviour::Conversion for MerchantConnectorAccount {
    type DstType = diesel_models::merchant_connector_account::MerchantConnectorAccount;
    type NewDstType = diesel_models::merchant_connector_account::MerchantConnectorAccountNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(
            diesel_models::merchant_connector_account::MerchantConnectorAccount {
                merchant_id: self.merchant_id,
                connector_name: self.connector_name,
                connector_account_details: self.connector_account_details.into(),
                test_mode: self.test_mode,
                disabled: self.disabled,
                merchant_connector_id: self.merchant_connector_id,
                payment_methods_enabled: self.payment_methods_enabled,
                connector_type: self.connector_type,
                metadata: self.metadata,
                frm_configs: None,
                frm_config: self.frm_configs,
                business_country: self.business_country,
                business_label: self.business_label,
                connector_label: self.connector_label,
                business_sub_label: self.business_sub_label,
                created_at: self.created_at,
                modified_at: self.modified_at,
                connector_webhook_details: self.connector_webhook_details,
                profile_id: Some(self.profile_id),
                applepay_verified_domains: self.applepay_verified_domains,
                pm_auth_config: self.pm_auth_config,
                status: self.status,
                connector_wallets_details: self.connector_wallets_details.map(Encryption::from),
                additional_merchant_data: self.additional_merchant_data.map(|data| data.into()),
                version: self.version,
            },
        )
    }

    async fn convert_back(
        state: &KeyManagerState,
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError> {
        let identifier = Identifier::Merchant(other.merchant_id.clone());
        let decrypted_data = crypto_operation(
            state,
            type_name!(Self::DstType),
            CryptoOperation::BatchDecrypt(EncryptedMerchantConnectorAccount::to_encryptable(
                EncryptedMerchantConnectorAccount {
                    connector_account_details: other.connector_account_details,
                    additional_merchant_data: other.additional_merchant_data,
                    connector_wallets_details: other.connector_wallets_details,
                },
            )),
            identifier.clone(),
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting connector account details".to_string(),
        })?;

        let decrypted_data = EncryptedMerchantConnectorAccount::from_encryptable(decrypted_data)
            .change_context(ValidationError::InvalidValue {
                message: "Failed while decrypting connector account details".to_string(),
            })?;

        Ok(Self {
            merchant_id: other.merchant_id,
            connector_name: other.connector_name,
            connector_account_details: decrypted_data.connector_account_details,
            test_mode: other.test_mode,
            disabled: other.disabled,
            merchant_connector_id: other.merchant_connector_id,
            payment_methods_enabled: other.payment_methods_enabled,
            connector_type: other.connector_type,
            metadata: other.metadata,

            frm_configs: other.frm_config,
            business_country: other.business_country,
            business_label: other.business_label,
            connector_label: other.connector_label,
            business_sub_label: other.business_sub_label,
            created_at: other.created_at,
            modified_at: other.modified_at,
            connector_webhook_details: other.connector_webhook_details,
            profile_id: other
                .profile_id
                .ok_or(ValidationError::MissingRequiredField {
                    field_name: "profile_id".to_string(),
                })?,
            applepay_verified_domains: other.applepay_verified_domains,
            pm_auth_config: other.pm_auth_config,
            status: other.status,
            connector_wallets_details: decrypted_data.connector_wallets_details,
            additional_merchant_data: decrypted_data.additional_merchant_data,
            version: other.version,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
        Ok(Self::NewDstType {
            merchant_id: Some(self.merchant_id),
            connector_name: Some(self.connector_name),
            connector_account_details: Some(self.connector_account_details.into()),
            test_mode: self.test_mode,
            disabled: self.disabled,
            merchant_connector_id: self.merchant_connector_id,
            payment_methods_enabled: self.payment_methods_enabled,
            connector_type: Some(self.connector_type),
            metadata: self.metadata,
            frm_configs: None,
            frm_config: self.frm_configs,
            business_country: self.business_country,
            business_label: self.business_label,
            connector_label: self.connector_label,
            business_sub_label: self.business_sub_label,
            created_at: now,
            modified_at: now,
            connector_webhook_details: self.connector_webhook_details,
            profile_id: Some(self.profile_id),
            applepay_verified_domains: self.applepay_verified_domains,
            pm_auth_config: self.pm_auth_config,
            status: self.status,
            connector_wallets_details: self.connector_wallets_details.map(Encryption::from),
            additional_merchant_data: self.additional_merchant_data.map(|data| data.into()),
            version: self.version,
        })
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl behaviour::Conversion for MerchantConnectorAccount {
    type DstType = diesel_models::merchant_connector_account::MerchantConnectorAccount;
    type NewDstType = diesel_models::merchant_connector_account::MerchantConnectorAccountNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(
            diesel_models::merchant_connector_account::MerchantConnectorAccount {
                id: self.id,
                merchant_id: self.merchant_id,
                connector_name: self.connector_name,
                connector_account_details: self.connector_account_details.into(),
                disabled: self.disabled,
                payment_methods_enabled: self.payment_methods_enabled,
                connector_type: self.connector_type,
                metadata: self.metadata,
                frm_config: self.frm_configs,
                connector_label: self.connector_label,
                created_at: self.created_at,
                modified_at: self.modified_at,
                connector_webhook_details: self.connector_webhook_details,
                profile_id: self.profile_id,
                applepay_verified_domains: self.applepay_verified_domains,
                pm_auth_config: self.pm_auth_config,
                status: self.status,
                connector_wallets_details: self.connector_wallets_details.map(Encryption::from),
                additional_merchant_data: self.additional_merchant_data.map(|data| data.into()),
                version: self.version,
                feature_metadata: self.feature_metadata.map(From::from),
            },
        )
    }

    async fn convert_back(
        state: &KeyManagerState,
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError> {
        let identifier = Identifier::Merchant(other.merchant_id.clone());

        let decrypted_data = crypto_operation(
            state,
            type_name!(Self::DstType),
            CryptoOperation::BatchDecrypt(EncryptedMerchantConnectorAccount::to_encryptable(
                EncryptedMerchantConnectorAccount {
                    connector_account_details: other.connector_account_details,
                    additional_merchant_data: other.additional_merchant_data,
                    connector_wallets_details: other.connector_wallets_details,
                },
            )),
            identifier.clone(),
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting connector account details".to_string(),
        })?;

        let decrypted_data = EncryptedMerchantConnectorAccount::from_encryptable(decrypted_data)
            .change_context(ValidationError::InvalidValue {
                message: "Failed while decrypting connector account details".to_string(),
            })?;

        Ok(Self {
            id: other.id,
            merchant_id: other.merchant_id,
            connector_name: other.connector_name,
            connector_account_details: decrypted_data.connector_account_details,
            disabled: other.disabled,
            payment_methods_enabled: other.payment_methods_enabled,
            connector_type: other.connector_type,
            metadata: other.metadata,

            frm_configs: other.frm_config,
            connector_label: other.connector_label,
            created_at: other.created_at,
            modified_at: other.modified_at,
            connector_webhook_details: other.connector_webhook_details,
            profile_id: other.profile_id,
            applepay_verified_domains: other.applepay_verified_domains,
            pm_auth_config: other.pm_auth_config,
            status: other.status,
            connector_wallets_details: decrypted_data.connector_wallets_details,
            additional_merchant_data: decrypted_data.additional_merchant_data,
            version: other.version,
            feature_metadata: other.feature_metadata.map(From::from),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
        Ok(Self::NewDstType {
            id: self.id,
            merchant_id: Some(self.merchant_id),
            connector_name: Some(self.connector_name),
            connector_account_details: Some(self.connector_account_details.into()),
            disabled: self.disabled,
            payment_methods_enabled: self.payment_methods_enabled,
            connector_type: Some(self.connector_type),
            metadata: self.metadata,
            frm_config: self.frm_configs,
            connector_label: self.connector_label,
            created_at: now,
            modified_at: now,
            connector_webhook_details: self.connector_webhook_details,
            profile_id: self.profile_id,
            applepay_verified_domains: self.applepay_verified_domains,
            pm_auth_config: self.pm_auth_config,
            status: self.status,
            connector_wallets_details: self.connector_wallets_details.map(Encryption::from),
            additional_merchant_data: self.additional_merchant_data.map(|data| data.into()),
            version: self.version,
            feature_metadata: self.feature_metadata.map(From::from),
        })
    }
}

#[cfg(feature = "v1")]
impl From<MerchantConnectorAccountUpdate> for MerchantConnectorAccountUpdateInternal {
    fn from(merchant_connector_account_update: MerchantConnectorAccountUpdate) -> Self {
        match merchant_connector_account_update {
            MerchantConnectorAccountUpdate::Update {
                connector_type,
                connector_name,
                connector_account_details,
                test_mode,
                disabled,
                merchant_connector_id,
                payment_methods_enabled,
                metadata,
                frm_configs,
                connector_webhook_details,
                applepay_verified_domains,
                pm_auth_config,
                connector_label,
                status,
                connector_wallets_details,
                additional_merchant_data,
            } => Self {
                connector_type,
                connector_name,
                connector_account_details: connector_account_details.map(Encryption::from),
                test_mode,
                disabled,
                merchant_connector_id,
                payment_methods_enabled,
                metadata,
                frm_configs: None,
                frm_config: frm_configs,
                modified_at: Some(date_time::now()),
                connector_webhook_details: *connector_webhook_details,
                applepay_verified_domains,
                pm_auth_config: *pm_auth_config,
                connector_label,
                status,
                connector_wallets_details: connector_wallets_details.map(Encryption::from),
                additional_merchant_data: additional_merchant_data.map(Encryption::from),
            },
            MerchantConnectorAccountUpdate::ConnectorWalletDetailsUpdate {
                connector_wallets_details,
            } => Self {
                connector_wallets_details: Some(Encryption::from(connector_wallets_details)),
                connector_type: None,
                connector_name: None,
                connector_account_details: None,
                connector_label: None,
                test_mode: None,
                disabled: None,
                merchant_connector_id: None,
                payment_methods_enabled: None,
                frm_configs: None,
                metadata: None,
                modified_at: None,
                connector_webhook_details: None,
                frm_config: None,
                applepay_verified_domains: None,
                pm_auth_config: None,
                status: None,
                additional_merchant_data: None,
            },
        }
    }
}

#[cfg(feature = "v2")]
impl From<MerchantConnectorAccountUpdate> for MerchantConnectorAccountUpdateInternal {
    fn from(merchant_connector_account_update: MerchantConnectorAccountUpdate) -> Self {
        match merchant_connector_account_update {
            MerchantConnectorAccountUpdate::Update {
                connector_type,
                connector_account_details,
                disabled,
                payment_methods_enabled,
                metadata,
                frm_configs,
                connector_webhook_details,
                applepay_verified_domains,
                pm_auth_config,
                connector_label,
                status,
                connector_wallets_details,
                additional_merchant_data,
                feature_metadata,
            } => Self {
                connector_type,
                connector_account_details: connector_account_details.map(Encryption::from),
                disabled,
                payment_methods_enabled,
                metadata,
                frm_config: frm_configs,
                modified_at: Some(date_time::now()),
                connector_webhook_details: *connector_webhook_details,
                applepay_verified_domains,
                pm_auth_config: *pm_auth_config,
                connector_label,
                status,
                connector_wallets_details: connector_wallets_details.map(Encryption::from),
                additional_merchant_data: additional_merchant_data.map(Encryption::from),
                feature_metadata: feature_metadata.map(From::from),
            },
            MerchantConnectorAccountUpdate::ConnectorWalletDetailsUpdate {
                connector_wallets_details,
            } => Self {
                connector_wallets_details: Some(Encryption::from(connector_wallets_details)),
                connector_type: None,
                connector_account_details: None,
                connector_label: None,
                disabled: None,
                payment_methods_enabled: None,
                metadata: None,
                modified_at: None,
                connector_webhook_details: None,
                frm_config: None,
                applepay_verified_domains: None,
                pm_auth_config: None,
                status: None,
                additional_merchant_data: None,
                feature_metadata: None,
            },
        }
    }
}

common_utils::create_list_wrapper!(
    MerchantConnectorAccounts,
    MerchantConnectorAccount,
    impl_functions: {
        #[cfg(feature = "v2")]
        pub fn get_connector_and_supporting_payment_method_type_for_session_call(
            &self,
        ) -> Vec<(&MerchantConnectorAccount, common_enums::PaymentMethodType)> {
            // This vector is created to work around lifetimes
            let ref_vector = Vec::default();

            let connector_and_supporting_payment_method_type = self.iter().flat_map(|connector_account| {
                connector_account
                    .payment_methods_enabled.as_ref()
                    .unwrap_or(&Vec::default())
                    .iter()
                    .flat_map(|payment_method_types| payment_method_types.payment_method_subtypes.as_ref().unwrap_or(&ref_vector))
                    .filter(|payment_method_types_enabled| {
                        payment_method_types_enabled.payment_experience == Some(api_models::enums::PaymentExperience::InvokeSdkClient)
                    })
                    .map(|payment_method_types| {
                        (connector_account, payment_method_types.payment_method_subtype)
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
