use common_utils::{
    crypto::Encryptable,
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    id_type, pii, type_name,
    types::keymanager::{Identifier, KeyManagerState, ToEncryptable},
};
use diesel_models::{enums, merchant_connector_account::MerchantConnectorAccountUpdateInternal};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use rustc_hash::FxHashMap;

use super::behaviour;
#[cfg(feature = "v2")]
use crate::router_data;
use crate::type_encryption::{crypto_operation, CryptoOperation};

#[cfg(feature = "v1")]
#[derive(Clone, Debug)]
pub struct MerchantConnectorAccount {
    pub merchant_id: id_type::MerchantId,
    pub connector_name: String,
    pub connector_account_details: Encryptable<pii::SecretSerdeValue>,
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
    pub connector_wallets_details: Option<Encryptable<pii::SecretSerdeValue>>,
    pub additional_merchant_data: Option<Encryptable<pii::SecretSerdeValue>>,
    pub version: common_enums::ApiVersion,
}

#[cfg(feature = "v1")]
impl MerchantConnectorAccount {
    pub fn get_id(&self) -> id_type::MerchantConnectorAccountId {
        self.merchant_connector_id.clone()
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub struct MerchantConnectorAccount {
    pub id: id_type::MerchantConnectorAccountId,
    pub merchant_id: id_type::MerchantId,
    pub connector_name: String,
    pub connector_account_details: Encryptable<pii::SecretSerdeValue>,
    pub disabled: Option<bool>,
    pub payment_methods_enabled: Option<Vec<pii::SecretSerdeValue>>,
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
    pub connector_wallets_details: Option<Encryptable<pii::SecretSerdeValue>>,
    pub additional_merchant_data: Option<Encryptable<pii::SecretSerdeValue>>,
    pub version: common_enums::ApiVersion,
}

#[cfg(feature = "v2")]
impl MerchantConnectorAccount {
    pub fn get_id(&self) -> id_type::MerchantConnectorAccountId {
        self.id.clone()
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled.unwrap_or(false)
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
        payment_methods_enabled: Option<Vec<pii::SecretSerdeValue>>,
        metadata: Option<pii::SecretSerdeValue>,
        frm_configs: Option<Vec<pii::SecretSerdeValue>>,
        connector_webhook_details: Option<pii::SecretSerdeValue>,
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
            CryptoOperation::BatchDecrypt(EncryptedMca::to_encryptable(EncryptedMca {
                connector_account_details: other.connector_account_details,
                additional_merchant_data: other.additional_merchant_data,
                connector_wallets_details: other.connector_wallets_details,
            })),
            identifier.clone(),
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting connector account details".to_string(),
        })?;

        let decrypted_data = EncryptedMca::from_encryptable(decrypted_data).change_context(
            ValidationError::InvalidValue {
                message: "Failed while decrypting connector account details".to_string(),
            },
        )?;

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
            CryptoOperation::BatchDecrypt(EncryptedMca::to_encryptable(EncryptedMca {
                connector_account_details: other.connector_account_details,
                additional_merchant_data: other.additional_merchant_data,
                connector_wallets_details: other.connector_wallets_details,
            })),
            identifier.clone(),
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting connector account details".to_string(),
        })?;

        let decrypted_data = EncryptedMca::from_encryptable(decrypted_data).change_context(
            ValidationError::InvalidValue {
                message: "Failed while decrypting connector account details".to_string(),
            },
        )?;

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
            } => Self {
                connector_type,
                connector_account_details: connector_account_details.map(Encryption::from),
                disabled,
                payment_methods_enabled,
                metadata,
                frm_config: frm_configs,
                modified_at: Some(date_time::now()),
                connector_webhook_details,
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
            },
        }
    }
}

pub struct McaFromRequestfromUpdate {
    pub connector_account_details: Option<pii::SecretSerdeValue>,
    pub connector_wallets_details: Option<pii::SecretSerdeValue>,
    pub additional_merchant_data: Option<pii::SecretSerdeValue>,
}
pub struct McaFromRequest {
    pub connector_account_details: pii::SecretSerdeValue,
    pub connector_wallets_details: Option<pii::SecretSerdeValue>,
    pub additional_merchant_data: Option<pii::SecretSerdeValue>,
}

pub struct DecryptedMca {
    pub connector_account_details: Encryptable<pii::SecretSerdeValue>,
    pub connector_wallets_details: Option<Encryptable<pii::SecretSerdeValue>>,
    pub additional_merchant_data: Option<Encryptable<pii::SecretSerdeValue>>,
}

pub struct EncryptedMca {
    pub connector_account_details: Encryption,
    pub connector_wallets_details: Option<Encryption>,
    pub additional_merchant_data: Option<Encryption>,
}

pub struct DecryptedUpdateMca {
    pub connector_account_details: Option<Encryptable<pii::SecretSerdeValue>>,
    pub connector_wallets_details: Option<Encryptable<pii::SecretSerdeValue>>,
    pub additional_merchant_data: Option<Encryptable<pii::SecretSerdeValue>>,
}

impl ToEncryptable<DecryptedMca, Secret<serde_json::Value>, Encryption> for EncryptedMca {
    fn from_encryptable(
        mut hashmap: FxHashMap<String, Encryptable<Secret<serde_json::Value>>>,
    ) -> CustomResult<DecryptedMca, common_utils::errors::ParsingError> {
        Ok(DecryptedMca {
            connector_account_details: hashmap.remove("connector_account_details").ok_or(
                error_stack::report!(common_utils::errors::ParsingError::EncodeError(
                    "Unable to convert from HashMap to DecryptedMca",
                )),
            )?,
            connector_wallets_details: hashmap.remove("connector_wallets_details"),
            additional_merchant_data: hashmap.remove("additional_merchant_data"),
        })
    }

    fn to_encryptable(self) -> FxHashMap<String, Encryption> {
        let mut map = FxHashMap::with_capacity_and_hasher(3, Default::default());

        map.insert(
            "connector_account_details".to_string(),
            self.connector_account_details,
        );
        self.connector_wallets_details
            .map(|s| map.insert("connector_wallets_details".to_string(), s));
        self.additional_merchant_data
            .map(|s| map.insert("additional_merchant_data".to_string(), s));
        map
    }
}

impl ToEncryptable<DecryptedUpdateMca, Secret<serde_json::Value>, Secret<serde_json::Value>>
    for McaFromRequestfromUpdate
{
    fn from_encryptable(
        mut hashmap: FxHashMap<String, Encryptable<Secret<serde_json::Value>>>,
    ) -> CustomResult<DecryptedUpdateMca, common_utils::errors::ParsingError> {
        Ok(DecryptedUpdateMca {
            connector_account_details: hashmap.remove("connector_account_details"),
            connector_wallets_details: hashmap.remove("connector_wallets_details"),
            additional_merchant_data: hashmap.remove("additional_merchant_data"),
        })
    }

    fn to_encryptable(self) -> FxHashMap<String, Secret<serde_json::Value>> {
        let mut map = FxHashMap::with_capacity_and_hasher(3, Default::default());

        self.connector_account_details
            .map(|cad| map.insert("connector_account_details".to_string(), cad));

        self.connector_wallets_details
            .map(|s| map.insert("connector_wallets_details".to_string(), s));
        self.additional_merchant_data
            .map(|s| map.insert("additional_merchant_data".to_string(), s));
        map
    }
}

impl ToEncryptable<DecryptedMca, Secret<serde_json::Value>, Secret<serde_json::Value>>
    for McaFromRequest
{
    fn from_encryptable(
        mut hashmap: FxHashMap<String, Encryptable<Secret<serde_json::Value>>>,
    ) -> CustomResult<DecryptedMca, common_utils::errors::ParsingError> {
        Ok(DecryptedMca {
            connector_account_details: hashmap.remove("connector_account_details").ok_or(
                error_stack::report!(common_utils::errors::ParsingError::EncodeError(
                    "Unable to convert from HashMap to DecryptedMca",
                )),
            )?,
            connector_wallets_details: hashmap.remove("connector_wallets_details"),
            additional_merchant_data: hashmap.remove("additional_merchant_data"),
        })
    }

    fn to_encryptable(self) -> FxHashMap<String, Secret<serde_json::Value>> {
        let mut map = FxHashMap::with_capacity_and_hasher(3, Default::default());

        map.insert(
            "connector_account_details".to_string(),
            self.connector_account_details,
        );
        self.connector_wallets_details
            .map(|s| map.insert("connector_wallets_details".to_string(), s));
        self.additional_merchant_data
            .map(|s| map.insert("additional_merchant_data".to_string(), s));
        map
    }
}
