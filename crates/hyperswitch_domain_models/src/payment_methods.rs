#[cfg(feature = "v2")]
use std::collections::HashMap;

#[cfg(feature = "v2")]
use api_models::payment_methods::PaymentMethodsData;
use api_models::{customers, payment_methods, payments};
// specific imports because of using the macro
use common_enums::enums::MerchantStorageScheme;
#[cfg(feature = "v1")]
use common_utils::crypto::OptionalEncryptableValue;
#[cfg(feature = "v2")]
use common_utils::{crypto::Encryptable, encryption::Encryption, types::keymanager::ToEncryptable};
use common_utils::{
    errors::{CustomResult, ParsingError, ValidationError},
    id_type, pii, type_name,
    types::{keymanager, CreatedBy},
};
pub use diesel_models::{enums as storage_enums, PaymentMethodUpdate};
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use masking::ExposeInterface;
use masking::{PeekInterface, Secret};
#[cfg(feature = "v1")]
use router_env::logger;
#[cfg(feature = "v2")]
use rustc_hash::FxHashMap;
#[cfg(feature = "v2")]
use serde_json::Value;
use time::PrimitiveDateTime;

#[cfg(feature = "v2")]
use crate::address::Address;
#[cfg(feature = "v1")]
use crate::type_encryption::AsyncLift;
use crate::{
    mandates::{self, CommonMandateReference},
    merchant_key_store::MerchantKeyStore,
    payment_method_data as domain_payment_method_data,
    transformers::ForeignTryFrom,
    type_encryption::{crypto_operation, CryptoOperation},
};

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct VaultId(String);

impl VaultId {
    pub fn get_string_repr(&self) -> &String {
        &self.0
    }

    pub fn generate(id: String) -> Self {
        Self(id)
    }
}
#[cfg(feature = "v1")]
#[derive(Clone, Debug)]
pub struct PaymentMethod {
    pub customer_id: id_type::CustomerId,
    pub merchant_id: id_type::MerchantId,
    pub payment_method_id: String,
    pub accepted_currency: Option<Vec<storage_enums::Currency>>,
    pub scheme: Option<String>,
    pub token: Option<String>,
    pub cardholder_name: Option<Secret<String>>,
    pub issuer_name: Option<String>,
    pub issuer_country: Option<String>,
    pub payer_country: Option<Vec<String>>,
    pub is_stored: Option<bool>,
    pub swift_code: Option<String>,
    pub direct_debit_token: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub payment_method: Option<storage_enums::PaymentMethod>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<storage_enums::PaymentMethodIssuerCode>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub payment_method_data: OptionalEncryptableValue,
    pub locker_id: Option<String>,
    pub last_used_at: PrimitiveDateTime,
    pub connector_mandate_details: Option<serde_json::Value>,
    pub customer_acceptance: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::PaymentMethodStatus,
    pub network_transaction_id: Option<String>,
    pub client_secret: Option<String>,
    pub payment_method_billing_address: OptionalEncryptableValue,
    pub updated_by: Option<String>,
    pub version: common_enums::ApiVersion,
    pub network_token_requestor_reference_id: Option<String>,
    pub network_token_locker_id: Option<String>,
    pub network_token_payment_method_data: OptionalEncryptableValue,
    pub vault_source_details: PaymentMethodVaultSourceDetails,
    pub created_by: Option<CreatedBy>,
    pub last_modified_by: Option<CreatedBy>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, router_derive::ToEncryption)]
pub struct PaymentMethod {
    /// The identifier for the payment method. Using this recurring payments can be made
    pub id: id_type::GlobalPaymentMethodId,

    /// The customer id against which the payment method is saved
    pub customer_id: id_type::GlobalCustomerId,

    /// The merchant id against which the payment method is saved
    pub merchant_id: id_type::MerchantId,
    /// The merchant connector account id of the external vault where the payment method is saved
    pub external_vault_source: Option<id_type::MerchantConnectorAccountId>,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub payment_method_type: Option<storage_enums::PaymentMethod>,
    pub payment_method_subtype: Option<storage_enums::PaymentMethodType>,
    #[encrypt(ty = Value)]
    pub payment_method_data: Option<Encryptable<PaymentMethodsData>>,
    pub locker_id: Option<VaultId>,
    pub last_used_at: PrimitiveDateTime,
    pub connector_mandate_details: Option<CommonMandateReference>,
    pub customer_acceptance: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::PaymentMethodStatus,
    pub network_transaction_id: Option<String>,
    pub client_secret: Option<String>,
    #[encrypt(ty = Value)]
    pub payment_method_billing_address: Option<Encryptable<Address>>,
    pub updated_by: Option<String>,
    pub locker_fingerprint_id: Option<String>,
    pub version: common_enums::ApiVersion,
    pub network_token_requestor_reference_id: Option<String>,
    pub network_token_locker_id: Option<String>,
    #[encrypt(ty = Value)]
    pub network_token_payment_method_data:
        Option<Encryptable<domain_payment_method_data::PaymentMethodsData>>,
    #[encrypt(ty = Value)]
    pub external_vault_token_data:
        Option<Encryptable<api_models::payment_methods::ExternalVaultTokenData>>,
    pub vault_type: Option<storage_enums::VaultType>,
    pub created_by: Option<CreatedBy>,
    pub last_modified_by: Option<CreatedBy>,
}

impl PaymentMethod {
    #[cfg(feature = "v1")]
    pub fn get_id(&self) -> &String {
        &self.payment_method_id
    }

    #[cfg(feature = "v1")]
    pub fn get_payment_methods_data(
        &self,
    ) -> Option<domain_payment_method_data::PaymentMethodsData> {
        self.payment_method_data
            .clone()
            .map(|value| value.into_inner().expose())
            .and_then(|value| {
                serde_json::from_value::<domain_payment_method_data::PaymentMethodsData>(value)
                    .map_err(|error| {
                        logger::warn!(
                            ?error,
                            "Failed to parse payment method data in payment method info"
                        );
                    })
                    .ok()
            })
    }

    #[cfg(feature = "v2")]
    pub fn get_id(&self) -> &id_type::GlobalPaymentMethodId {
        &self.id
    }

    #[cfg(feature = "v1")]
    pub fn get_payment_method_type(&self) -> Option<storage_enums::PaymentMethod> {
        self.payment_method
    }

    #[cfg(feature = "v2")]
    pub fn get_payment_method_type(&self) -> Option<storage_enums::PaymentMethod> {
        self.payment_method_type
    }

    #[cfg(feature = "v1")]
    pub fn get_payment_method_subtype(&self) -> Option<storage_enums::PaymentMethodType> {
        self.payment_method_type
    }

    #[cfg(feature = "v2")]
    pub fn get_payment_method_subtype(&self) -> Option<storage_enums::PaymentMethodType> {
        self.payment_method_subtype
    }

    #[cfg(feature = "v1")]
    pub fn get_common_mandate_reference(&self) -> Result<CommonMandateReference, ParsingError> {
        let payments_data = self
            .connector_mandate_details
            .clone()
            .map(|mut mandate_details| {
                mandate_details
                    .as_object_mut()
                    .map(|obj| obj.remove("payouts"));

                serde_json::from_value::<mandates::PaymentsMandateReference>(mandate_details)
                    .inspect_err(|err| {
                        router_env::logger::error!("Failed to parse payments data: {:?}", err);
                    })
            })
            .transpose()
            .map_err(|err| {
                router_env::logger::error!("Failed to parse payments data: {:?}", err);
                ParsingError::StructParseFailure("Failed to parse payments data")
            })?;

        let payouts_data = self
            .connector_mandate_details
            .clone()
            .map(|mandate_details| {
                serde_json::from_value::<Option<CommonMandateReference>>(mandate_details)
                    .inspect_err(|err| {
                        router_env::logger::error!("Failed to parse payouts data: {:?}", err);
                    })
                    .map(|optional_common_mandate_details| {
                        optional_common_mandate_details
                            .and_then(|common_mandate_details| common_mandate_details.payouts)
                    })
            })
            .transpose()
            .map_err(|err| {
                router_env::logger::error!("Failed to parse payouts data: {:?}", err);
                ParsingError::StructParseFailure("Failed to parse payouts data")
            })?
            .flatten();

        Ok(CommonMandateReference {
            payments: payments_data,
            payouts: payouts_data,
        })
    }

    #[cfg(feature = "v2")]
    pub fn get_common_mandate_reference(&self) -> Result<CommonMandateReference, ParsingError> {
        if let Some(value) = &self.connector_mandate_details {
            Ok(value.clone())
        } else {
            Ok(CommonMandateReference {
                payments: None,
                payouts: None,
            })
        }
    }

    #[cfg(feature = "v1")]
    pub fn get_payment_connector_customer_id(
        &self,
        merchant_connector_account_id: id_type::MerchantConnectorAccountId,
    ) -> Result<Option<String>, ParsingError> {
        let common_mandate_reference = self.get_common_mandate_reference()?;
        Ok(common_mandate_reference
            .payments
            .as_ref()
            .and_then(|payments| payments.get(&merchant_connector_account_id))
            .and_then(|record| record.connector_customer_id.clone()))
    }

    #[cfg(feature = "v2")]
    pub fn get_payment_connector_customer_id(
        &self,
        merchant_connector_account_id: id_type::MerchantConnectorAccountId,
    ) -> Result<Option<String>, ParsingError> {
        todo!()
    }

    #[cfg(feature = "v1")]
    pub fn get_payout_connector_customer_id(
        &self,
        merchant_connector_account_id: id_type::MerchantConnectorAccountId,
    ) -> Result<Option<String>, ParsingError> {
        let common_mandate_reference = self.get_common_mandate_reference()?;
        Ok(common_mandate_reference
            .payouts
            .as_ref()
            .and_then(|payouts| payouts.get(&merchant_connector_account_id))
            .and_then(|record| record.connector_customer_id.clone()))
    }

    #[cfg(feature = "v2")]
    pub fn get_payout_connector_customer_id(
        &self,
        merchant_connector_account_id: id_type::MerchantConnectorAccountId,
    ) -> Result<Option<String>, ParsingError> {
        todo!()
    }

    #[cfg(feature = "v2")]
    pub fn set_payment_method_type(&mut self, payment_method_type: common_enums::PaymentMethod) {
        self.payment_method_type = Some(payment_method_type);
    }

    #[cfg(feature = "v2")]
    pub fn set_payment_method_subtype(
        &mut self,
        payment_method_subtype: common_enums::PaymentMethodType,
    ) {
        self.payment_method_subtype = Some(payment_method_subtype);
    }
}

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl super::behaviour::Conversion for PaymentMethod {
    type DstType = diesel_models::payment_method::PaymentMethod;
    type NewDstType = diesel_models::payment_method::PaymentMethodNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let (vault_type, external_vault_source) = self.vault_source_details.into();
        Ok(Self::DstType {
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            payment_method_id: self.payment_method_id,
            accepted_currency: self.accepted_currency,
            scheme: self.scheme,
            token: self.token,
            cardholder_name: self.cardholder_name,
            issuer_name: self.issuer_name,
            issuer_country: self.issuer_country,
            payer_country: self.payer_country,
            is_stored: self.is_stored,
            swift_code: self.swift_code,
            direct_debit_token: self.direct_debit_token,
            created_at: self.created_at,
            last_modified: self.last_modified,
            payment_method: self.payment_method,
            payment_method_type: self.payment_method_type,
            payment_method_issuer: self.payment_method_issuer,
            payment_method_issuer_code: self.payment_method_issuer_code,
            metadata: self.metadata,
            payment_method_data: self.payment_method_data.map(|val| val.into()),
            locker_id: self.locker_id,
            last_used_at: self.last_used_at,
            connector_mandate_details: self.connector_mandate_details,
            customer_acceptance: self.customer_acceptance,
            status: self.status,
            network_transaction_id: self.network_transaction_id,
            client_secret: self.client_secret,
            payment_method_billing_address: self
                .payment_method_billing_address
                .map(|val| val.into()),
            updated_by: self.updated_by,
            version: self.version,
            network_token_requestor_reference_id: self.network_token_requestor_reference_id,
            network_token_locker_id: self.network_token_locker_id,
            network_token_payment_method_data: self
                .network_token_payment_method_data
                .map(|val| val.into()),
            external_vault_source,
            vault_type,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            last_modified_by: self
                .last_modified_by
                .map(|last_modified_by| last_modified_by.to_string()),
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        // Decrypt encrypted fields first
        let (
            payment_method_data,
            payment_method_billing_address,
            network_token_payment_method_data,
        ) = async {
            let payment_method_data = item
                .payment_method_data
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let payment_method_billing_address = item
                .payment_method_billing_address
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            let network_token_payment_method_data = item
                .network_token_payment_method_data
                .async_lift(|inner| async {
                    crypto_operation(
                        state,
                        type_name!(Self::DstType),
                        CryptoOperation::DecryptOptional(inner),
                        key_manager_identifier.clone(),
                        key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await?;

            Ok::<_, error_stack::Report<common_utils::errors::CryptoError>>((
                payment_method_data,
                payment_method_billing_address,
                network_token_payment_method_data,
            ))
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment method data".to_string(),
        })?;

        let vault_source_details = PaymentMethodVaultSourceDetails::try_from((
            item.vault_type,
            item.external_vault_source,
        ))?;

        // Construct the domain type
        Ok(Self {
            customer_id: item.customer_id,
            merchant_id: item.merchant_id,
            payment_method_id: item.payment_method_id,
            accepted_currency: item.accepted_currency,
            scheme: item.scheme,
            token: item.token,
            cardholder_name: item.cardholder_name,
            issuer_name: item.issuer_name,
            issuer_country: item.issuer_country,
            payer_country: item.payer_country,
            is_stored: item.is_stored,
            swift_code: item.swift_code,
            direct_debit_token: item.direct_debit_token,
            created_at: item.created_at,
            last_modified: item.last_modified,
            payment_method: item.payment_method,
            payment_method_type: item.payment_method_type,
            payment_method_issuer: item.payment_method_issuer,
            payment_method_issuer_code: item.payment_method_issuer_code,
            metadata: item.metadata,
            payment_method_data,
            locker_id: item.locker_id,
            last_used_at: item.last_used_at,
            connector_mandate_details: item.connector_mandate_details,
            customer_acceptance: item.customer_acceptance,
            status: item.status,
            network_transaction_id: item.network_transaction_id,
            client_secret: item.client_secret,
            payment_method_billing_address,
            updated_by: item.updated_by,
            version: item.version,
            network_token_requestor_reference_id: item.network_token_requestor_reference_id,
            network_token_locker_id: item.network_token_locker_id,
            network_token_payment_method_data,
            vault_source_details,
            created_by: item
                .created_by
                .and_then(|created_by| created_by.parse::<CreatedBy>().ok()),
            last_modified_by: item
                .last_modified_by
                .and_then(|last_modified_by| last_modified_by.parse::<CreatedBy>().ok()),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let (vault_type, external_vault_source) = self.vault_source_details.into();
        Ok(Self::NewDstType {
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            payment_method_id: self.payment_method_id,
            accepted_currency: self.accepted_currency,
            scheme: self.scheme,
            token: self.token,
            cardholder_name: self.cardholder_name,
            issuer_name: self.issuer_name,
            issuer_country: self.issuer_country,
            payer_country: self.payer_country,
            is_stored: self.is_stored,
            swift_code: self.swift_code,
            direct_debit_token: self.direct_debit_token,
            created_at: self.created_at,
            last_modified: self.last_modified,
            payment_method: self.payment_method,
            payment_method_type: self.payment_method_type,
            payment_method_issuer: self.payment_method_issuer,
            payment_method_issuer_code: self.payment_method_issuer_code,
            metadata: self.metadata,
            payment_method_data: self.payment_method_data.map(|val| val.into()),
            locker_id: self.locker_id,
            last_used_at: self.last_used_at,
            connector_mandate_details: self.connector_mandate_details,
            customer_acceptance: self.customer_acceptance,
            status: self.status,
            network_transaction_id: self.network_transaction_id,
            client_secret: self.client_secret,
            payment_method_billing_address: self
                .payment_method_billing_address
                .map(|val| val.into()),
            updated_by: self.updated_by,
            version: self.version,
            network_token_requestor_reference_id: self.network_token_requestor_reference_id,
            network_token_locker_id: self.network_token_locker_id,
            network_token_payment_method_data: self
                .network_token_payment_method_data
                .map(|val| val.into()),
            external_vault_source,
            vault_type,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            last_modified_by: self
                .last_modified_by
                .map(|last_modified_by| last_modified_by.to_string()),
        })
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl super::behaviour::Conversion for PaymentMethod {
    type DstType = diesel_models::payment_method::PaymentMethod;
    type NewDstType = diesel_models::payment_method::PaymentMethodNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(Self::DstType {
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            id: self.id,
            created_at: self.created_at,
            last_modified: self.last_modified,
            payment_method_type_v2: self.payment_method_type,
            payment_method_subtype: self.payment_method_subtype,
            payment_method_data: self.payment_method_data.map(|val| val.into()),
            locker_id: self.locker_id.map(|id| id.get_string_repr().clone()),
            last_used_at: self.last_used_at,
            connector_mandate_details: self.connector_mandate_details.map(|cmd| cmd.into()),
            customer_acceptance: self.customer_acceptance,
            status: self.status,
            network_transaction_id: self.network_transaction_id,
            client_secret: self.client_secret,
            payment_method_billing_address: self
                .payment_method_billing_address
                .map(|val| val.into()),
            updated_by: self.updated_by,
            locker_fingerprint_id: self.locker_fingerprint_id,
            version: self.version,
            network_token_requestor_reference_id: self.network_token_requestor_reference_id,
            network_token_locker_id: self.network_token_locker_id,
            network_token_payment_method_data: self
                .network_token_payment_method_data
                .map(|val| val.into()),
            external_vault_source: self.external_vault_source,
            external_vault_token_data: self.external_vault_token_data.map(|val| val.into()),
            vault_type: self.vault_type,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            last_modified_by: self
                .last_modified_by
                .map(|last_modified_by| last_modified_by.to_string()),
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        storage_model: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        use common_utils::ext_traits::ValueExt;

        async {
            let decrypted_data = crypto_operation(
                state,
                type_name!(Self::DstType),
                CryptoOperation::BatchDecrypt(EncryptedPaymentMethod::to_encryptable(
                    EncryptedPaymentMethod {
                        payment_method_data: storage_model.payment_method_data,
                        payment_method_billing_address: storage_model
                            .payment_method_billing_address,
                        network_token_payment_method_data: storage_model
                            .network_token_payment_method_data,
                        external_vault_token_data: storage_model.external_vault_token_data,
                    },
                )),
                key_manager_identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_batchoperation())?;

            let data = EncryptedPaymentMethod::from_encryptable(decrypted_data)
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Invalid batch operation data")?;

            let payment_method_billing_address = data
                .payment_method_billing_address
                .map(|billing| {
                    billing.deserialize_inner_value(|value| value.parse_value("Address"))
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Address")?;

            let payment_method_data = data
                .payment_method_data
                .map(|payment_method_data| {
                    payment_method_data
                        .deserialize_inner_value(|value| value.parse_value("Payment Method Data"))
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Payment Method Data")?;

            let network_token_payment_method_data = data
                .network_token_payment_method_data
                .map(|network_token_payment_method_data| {
                    network_token_payment_method_data.deserialize_inner_value(|value| {
                        value.parse_value("Network token Payment Method Data")
                    })
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Network token Payment Method Data")?;

            let external_vault_token_data = data
                .external_vault_token_data
                .map(|external_vault_token_data| {
                    external_vault_token_data.deserialize_inner_value(|value| {
                        value.parse_value("External Vault Token Data")
                    })
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing External Vault Token Data")?;

            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                customer_id: storage_model.customer_id,
                merchant_id: storage_model.merchant_id,
                id: storage_model.id,
                created_at: storage_model.created_at,
                last_modified: storage_model.last_modified,
                payment_method_type: storage_model.payment_method_type_v2,
                payment_method_subtype: storage_model.payment_method_subtype,
                payment_method_data,
                locker_id: storage_model.locker_id.map(VaultId::generate),
                last_used_at: storage_model.last_used_at,
                connector_mandate_details: storage_model.connector_mandate_details.map(From::from),
                customer_acceptance: storage_model.customer_acceptance,
                status: storage_model.status,
                network_transaction_id: storage_model.network_transaction_id,
                client_secret: storage_model.client_secret,
                payment_method_billing_address,
                updated_by: storage_model.updated_by,
                locker_fingerprint_id: storage_model.locker_fingerprint_id,
                version: storage_model.version,
                network_token_requestor_reference_id: storage_model
                    .network_token_requestor_reference_id,
                network_token_locker_id: storage_model.network_token_locker_id,
                network_token_payment_method_data,
                external_vault_source: storage_model.external_vault_source,
                external_vault_token_data,
                vault_type: storage_model.vault_type,
                created_by: storage_model
                    .created_by
                    .and_then(|created_by| created_by.parse::<CreatedBy>().ok()),
                last_modified_by: storage_model
                    .last_modified_by
                    .and_then(|last_modified_by| last_modified_by.parse::<CreatedBy>().ok()),
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment method data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(Self::NewDstType {
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            id: self.id,
            created_at: self.created_at,
            last_modified: self.last_modified,
            payment_method_type_v2: self.payment_method_type,
            payment_method_subtype: self.payment_method_subtype,
            payment_method_data: self.payment_method_data.map(|val| val.into()),
            locker_id: self.locker_id.map(|id| id.get_string_repr().clone()),
            last_used_at: self.last_used_at,
            connector_mandate_details: self.connector_mandate_details.map(|cmd| cmd.into()),
            customer_acceptance: self.customer_acceptance,
            status: self.status,
            network_transaction_id: self.network_transaction_id,
            client_secret: self.client_secret,
            payment_method_billing_address: self
                .payment_method_billing_address
                .map(|val| val.into()),
            updated_by: self.updated_by,
            locker_fingerprint_id: self.locker_fingerprint_id,
            version: self.version,
            network_token_requestor_reference_id: self.network_token_requestor_reference_id,
            network_token_locker_id: self.network_token_locker_id,
            network_token_payment_method_data: self
                .network_token_payment_method_data
                .map(|val| val.into()),
            external_vault_token_data: self.external_vault_token_data.map(|val| val.into()),
            vault_type: self.vault_type,
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            last_modified_by: self
                .last_modified_by
                .map(|last_modified_by| last_modified_by.to_string()),
        })
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, router_derive::ToEncryption)]
pub struct PaymentMethodSession {
    pub id: id_type::GlobalPaymentMethodSessionId,
    pub customer_id: id_type::GlobalCustomerId,
    #[encrypt(ty = Value)]
    pub billing: Option<Encryptable<Address>>,
    pub return_url: Option<common_utils::types::Url>,
    pub psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
    pub network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
    pub tokenization_data: Option<pii::SecretSerdeValue>,
    pub expires_at: PrimitiveDateTime,
    pub associated_payment_methods: Option<Vec<String>>,
    pub associated_payment: Option<id_type::GlobalPaymentId>,
    pub associated_token_id: Option<id_type::GlobalTokenId>,
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl super::behaviour::Conversion for PaymentMethodSession {
    type DstType = diesel_models::payment_methods_session::PaymentMethodSession;
    type NewDstType = diesel_models::payment_methods_session::PaymentMethodSession;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(Self::DstType {
            id: self.id,
            customer_id: self.customer_id,
            billing: self.billing.map(|val| val.into()),
            psp_tokenization: self.psp_tokenization,
            network_tokenization: self.network_tokenization,
            tokenization_data: self.tokenization_data,
            expires_at: self.expires_at,
            associated_payment_methods: self.associated_payment_methods,
            associated_payment: self.associated_payment,
            return_url: self.return_url,
            associated_token_id: self.associated_token_id,
        })
    }

    async fn convert_back(
        state: &keymanager::KeyManagerState,
        storage_model: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        use common_utils::ext_traits::ValueExt;

        async {
            let decrypted_data = crypto_operation(
                state,
                type_name!(Self::DstType),
                CryptoOperation::BatchDecrypt(EncryptedPaymentMethodSession::to_encryptable(
                    EncryptedPaymentMethodSession {
                        billing: storage_model.billing,
                    },
                )),
                key_manager_identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_batchoperation())?;

            let data = EncryptedPaymentMethodSession::from_encryptable(decrypted_data)
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Invalid batch operation data")?;

            let billing = data
                .billing
                .map(|billing| {
                    billing.deserialize_inner_value(|value| value.parse_value("Address"))
                })
                .transpose()
                .change_context(common_utils::errors::CryptoError::DecodingFailed)
                .attach_printable("Error while deserializing Address")?;

            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                id: storage_model.id,
                customer_id: storage_model.customer_id,
                billing,
                psp_tokenization: storage_model.psp_tokenization,
                network_tokenization: storage_model.network_tokenization,
                tokenization_data: storage_model.tokenization_data,
                expires_at: storage_model.expires_at,
                associated_payment_methods: storage_model.associated_payment_methods,
                associated_payment: storage_model.associated_payment,
                return_url: storage_model.return_url,
                associated_token_id: storage_model.associated_token_id,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting payment method data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(Self::NewDstType {
            id: self.id,
            customer_id: self.customer_id,
            billing: self.billing.map(|val| val.into()),
            psp_tokenization: self.psp_tokenization,
            network_tokenization: self.network_tokenization,
            tokenization_data: self.tokenization_data,
            expires_at: self.expires_at,
            associated_payment_methods: self.associated_payment_methods,
            associated_payment: self.associated_payment,
            return_url: self.return_url,
            associated_token_id: self.associated_token_id,
        })
    }
}

#[async_trait::async_trait]
pub trait PaymentMethodInterface {
    type Error;
    #[cfg(feature = "v1")]
    async fn find_payment_method(
        &self,
        key_store: &MerchantKeyStore,
        payment_method_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentMethod, Self::Error>;

    #[cfg(feature = "v2")]
    async fn find_payment_method(
        &self,
        key_store: &MerchantKeyStore,
        payment_method_id: &id_type::GlobalPaymentMethodId,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentMethod, Self::Error>;

    #[cfg(feature = "v1")]
    async fn find_payment_method_by_locker_id(
        &self,
        key_store: &MerchantKeyStore,
        locker_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentMethod, Self::Error>;

    #[cfg(feature = "v1")]
    async fn find_payment_method_by_customer_id_merchant_id_list(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<PaymentMethod>, Self::Error>;

    // Need to fix this once we start moving to v2 for payment method
    #[cfg(feature = "v2")]
    async fn find_payment_method_list_by_global_customer_id(
        &self,
        key_store: &MerchantKeyStore,
        id: &id_type::GlobalCustomerId,
        limit: Option<i64>,
    ) -> CustomResult<Vec<PaymentMethod>, Self::Error>;

    #[cfg(feature = "v1")]
    #[allow(clippy::too_many_arguments)]
    async fn find_payment_method_by_customer_id_merchant_id_status(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentMethod>, Self::Error>;

    #[cfg(feature = "v2")]
    #[allow(clippy::too_many_arguments)]
    async fn find_payment_method_by_global_customer_id_merchant_id_status(
        &self,
        key_store: &MerchantKeyStore,
        customer_id: &id_type::GlobalCustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
        limit: Option<i64>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentMethod>, Self::Error>;

    #[cfg(feature = "v1")]
    async fn get_payment_method_count_by_customer_id_merchant_id_status(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, Self::Error>;

    async fn get_payment_method_count_by_merchant_id_status(
        &self,
        merchant_id: &id_type::MerchantId,
        status: common_enums::PaymentMethodStatus,
    ) -> CustomResult<i64, Self::Error>;

    async fn insert_payment_method(
        &self,
        key_store: &MerchantKeyStore,
        payment_method: PaymentMethod,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentMethod, Self::Error>;

    async fn update_payment_method(
        &self,
        key_store: &MerchantKeyStore,
        payment_method: PaymentMethod,
        payment_method_update: PaymentMethodUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentMethod, Self::Error>;

    #[cfg(feature = "v2")]
    async fn delete_payment_method(
        &self,
        key_store: &MerchantKeyStore,
        payment_method: PaymentMethod,
    ) -> CustomResult<PaymentMethod, Self::Error>;

    #[cfg(feature = "v2")]
    async fn find_payment_method_by_fingerprint_id(
        &self,
        key_store: &MerchantKeyStore,
        fingerprint_id: &str,
    ) -> CustomResult<PaymentMethod, Self::Error>;

    #[cfg(feature = "v1")]
    async fn delete_payment_method_by_merchant_id_payment_method_id(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method_id: &str,
    ) -> CustomResult<PaymentMethod, Self::Error>;
}

#[cfg(feature = "v2")]
pub enum PaymentMethodsSessionUpdateEnum {
    GeneralUpdate {
        billing: Box<Option<Encryptable<Address>>>,
        psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
        network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
        tokenization_data: Option<pii::SecretSerdeValue>,
    },
    UpdateAssociatedPaymentMethods {
        associated_payment_methods: Option<Vec<String>>,
    },
}

#[cfg(feature = "v2")]
impl From<PaymentMethodsSessionUpdateEnum> for PaymentMethodsSessionUpdateInternal {
    fn from(update: PaymentMethodsSessionUpdateEnum) -> Self {
        match update {
            PaymentMethodsSessionUpdateEnum::GeneralUpdate {
                billing,
                psp_tokenization,
                network_tokenization,
                tokenization_data,
            } => Self {
                billing: *billing,
                psp_tokenization,
                network_tokenization,
                tokenization_data,
                associated_payment_methods: None,
            },
            PaymentMethodsSessionUpdateEnum::UpdateAssociatedPaymentMethods {
                associated_payment_methods,
            } => Self {
                billing: None,
                psp_tokenization: None,
                network_tokenization: None,
                tokenization_data: None,
                associated_payment_methods,
            },
        }
    }
}

#[cfg(feature = "v2")]
impl PaymentMethodSession {
    pub fn apply_changeset(self, update_session: PaymentMethodsSessionUpdateInternal) -> Self {
        let Self {
            id,
            customer_id,
            billing,
            psp_tokenization,
            network_tokenization,
            tokenization_data,
            expires_at,
            return_url,
            associated_payment_methods,
            associated_payment,
            associated_token_id,
        } = self;
        Self {
            id,
            customer_id,
            billing: update_session.billing.or(billing),
            psp_tokenization: update_session.psp_tokenization.or(psp_tokenization),
            network_tokenization: update_session.network_tokenization.or(network_tokenization),
            tokenization_data: update_session.tokenization_data.or(tokenization_data),
            expires_at,
            return_url,
            associated_payment_methods: update_session
                .associated_payment_methods
                .or(associated_payment_methods),
            associated_payment,
            associated_token_id,
        }
    }
}

#[cfg(feature = "v2")]
pub struct PaymentMethodsSessionUpdateInternal {
    pub billing: Option<Encryptable<Address>>,
    pub psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
    pub network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
    pub tokenization_data: Option<pii::SecretSerdeValue>,
    pub associated_payment_methods: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ConnectorCustomerDetails {
    pub connector_customer_id: String,
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PaymentMethodCustomerMigrate {
    pub customer: customers::CustomerRequest,
    pub connector_customer_details: Option<Vec<ConnectorCustomerDetails>>,
}

#[cfg(feature = "v1")]
impl TryFrom<(payment_methods::PaymentMethodRecord, id_type::MerchantId)>
    for PaymentMethodCustomerMigrate
{
    type Error = error_stack::Report<ValidationError>;
    fn try_from(
        value: (payment_methods::PaymentMethodRecord, id_type::MerchantId),
    ) -> Result<Self, Self::Error> {
        let (record, merchant_id) = value;
        let connector_customer_details = record
            .connector_customer_id
            .and_then(|connector_customer_id| {
                // Handle single merchant_connector_id
                record
                    .merchant_connector_id
                    .as_ref()
                    .map(|merchant_connector_id| {
                        Ok(vec![ConnectorCustomerDetails {
                            connector_customer_id: connector_customer_id.clone(),
                            merchant_connector_id: merchant_connector_id.clone(),
                        }])
                    })
                    // Handle comma-separated merchant_connector_ids
                    .or_else(|| {
                        record
                            .merchant_connector_ids
                            .as_ref()
                            .map(|merchant_connector_ids_str| {
                                merchant_connector_ids_str
                                    .split(',')
                                    .map(|id| id.trim())
                                    .filter(|id| !id.is_empty())
                                    .map(|merchant_connector_id| {
                                        id_type::MerchantConnectorAccountId::wrap(
                                            merchant_connector_id.to_string(),
                                        )
                                        .map_err(|_| {
                                            error_stack::report!(ValidationError::InvalidValue {
                                                message: format!(
                                                    "Invalid merchant_connector_account_id: {merchant_connector_id}"
                                                ),
                                            })
                                        })
                                        .map(
                                            |merchant_connector_id| ConnectorCustomerDetails {
                                                connector_customer_id: connector_customer_id
                                                    .clone(),
                                                merchant_connector_id,
                                            },
                                        )
                                    })
                                    .collect::<Result<Vec<_>, _>>()
                            })
                    })
            })
            .transpose()?;

        Ok(Self {
            customer: customers::CustomerRequest {
                customer_id: Some(record.customer_id),
                merchant_id,
                name: record.name,
                email: record.email,
                phone: record.phone,
                description: None,
                phone_country_code: record.phone_country_code,
                address: Some(payments::AddressDetails {
                    city: record.billing_address_city,
                    country: record.billing_address_country,
                    line1: record.billing_address_line1,
                    line2: record.billing_address_line2,
                    state: record.billing_address_state,
                    line3: record.billing_address_line3,
                    zip: record.billing_address_zip,
                    first_name: record.billing_address_first_name,
                    last_name: record.billing_address_last_name,
                    origin_zip: None,
                }),
                metadata: None,
                tax_registration_id: None,
            },
            connector_customer_details,
        })
    }
}

#[cfg(feature = "v1")]
impl ForeignTryFrom<(&[payment_methods::PaymentMethodRecord], id_type::MerchantId)>
    for Vec<PaymentMethodCustomerMigrate>
{
    type Error = error_stack::Report<ValidationError>;

    fn foreign_try_from(
        (records, merchant_id): (&[payment_methods::PaymentMethodRecord], id_type::MerchantId),
    ) -> Result<Self, Self::Error> {
        let (customers_migration, migration_errors): (Self, Vec<_>) = records
            .iter()
            .map(|record| {
                PaymentMethodCustomerMigrate::try_from((record.clone(), merchant_id.clone()))
            })
            .fold((Self::new(), Vec::new()), |mut acc, result| {
                match result {
                    Ok(customer) => acc.0.push(customer),
                    Err(e) => acc.1.push(e.to_string()),
                }
                acc
            });

        migration_errors
            .is_empty()
            .then_some(customers_migration)
            .ok_or_else(|| {
                error_stack::report!(ValidationError::InvalidValue {
                    message: migration_errors.join(", "),
                })
            })
    }
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Default)]
pub enum PaymentMethodVaultSourceDetails {
    ExternalVault {
        external_vault_source: id_type::MerchantConnectorAccountId,
    },
    #[default]
    InternalVault,
}

#[cfg(feature = "v1")]
impl
    TryFrom<(
        Option<storage_enums::VaultType>,
        Option<id_type::MerchantConnectorAccountId>,
    )> for PaymentMethodVaultSourceDetails
{
    type Error = error_stack::Report<ValidationError>;
    fn try_from(
        value: (
            Option<storage_enums::VaultType>,
            Option<id_type::MerchantConnectorAccountId>,
        ),
    ) -> Result<Self, Self::Error> {
        match value {
            (Some(storage_enums::VaultType::External), Some(external_vault_source)) => {
                Ok(Self::ExternalVault {
                    external_vault_source,
                })
            }
            (Some(storage_enums::VaultType::External), None) => {
                Err(ValidationError::MissingRequiredField {
                    field_name: "external vault mca id".to_string(),
                }
                .into())
            }
            (Some(storage_enums::VaultType::Internal), _) | (None, _) => Ok(Self::InternalVault), // defaulting to internal vault if vault type is none
        }
    }
}
#[cfg(feature = "v1")]
impl From<PaymentMethodVaultSourceDetails>
    for (
        Option<storage_enums::VaultType>,
        Option<id_type::MerchantConnectorAccountId>,
    )
{
    fn from(value: PaymentMethodVaultSourceDetails) -> Self {
        match value {
            PaymentMethodVaultSourceDetails::ExternalVault {
                external_vault_source,
            } => (
                Some(storage_enums::VaultType::External),
                Some(external_vault_source),
            ),
            PaymentMethodVaultSourceDetails::InternalVault => {
                (Some(storage_enums::VaultType::Internal), None)
            }
        }
    }
}

/// This struct stores information to generate the key to identify
/// a unique payment method balance entry in the HashMap stored in Redis
#[cfg(feature = "v2")]
#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub struct PaymentMethodBalanceKey {
    pub payment_method_type: common_enums::PaymentMethod,
    pub payment_method_subtype: common_enums::PaymentMethodType,
    pub payment_method_key: String,
}

#[cfg(feature = "v2")]
impl PaymentMethodBalanceKey {
    pub fn get_redis_key(&self) -> String {
        format!(
            "{}_{}_{}",
            self.payment_method_type, self.payment_method_subtype, self.payment_method_key
        )
    }
}

/// This struct stores the balance and currency information for a specific
/// payment method to be stored in the HashMap in Redis
#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodBalance {
    pub balance: common_utils::types::MinorUnit,
    pub currency: common_enums::Currency,
}

#[cfg(feature = "v2")]
pub struct PaymentMethodBalanceData<'a> {
    pub pm_balance_data: HashMap<PaymentMethodBalanceKey, PaymentMethodBalance>,
    pub payment_intent_id: &'a id_type::GlobalPaymentId,
}

#[cfg(feature = "v2")]
impl<'a> PaymentMethodBalanceData<'a> {
    pub fn new(payment_intent_id: &'a id_type::GlobalPaymentId) -> Self {
        Self {
            pm_balance_data: HashMap::new(),
            payment_intent_id,
        }
    }

    pub fn get_pm_balance_redis_key(&self) -> String {
        format!("pm_balance_{}", self.payment_intent_id.get_string_repr())
    }

    pub fn is_empty(&self) -> bool {
        self.pm_balance_data.is_empty()
    }

    pub fn get_individual_pm_balance_key_value_pairs(&self) -> Vec<(String, PaymentMethodBalance)> {
        self.pm_balance_data
            .iter()
            .map(|(pm_balance_key, pm_balance_value)| {
                let key = pm_balance_key.get_redis_key();
                (key, pm_balance_value.to_owned())
            })
            .collect()
    }
}

#[cfg(feature = "v1")]
#[cfg(test)]
mod tests {
    use id_type::MerchantConnectorAccountId;

    use super::*;

    fn get_payment_method_with_mandate_data(
        mandate_data: Option<serde_json::Value>,
    ) -> PaymentMethod {
        let payment_method = PaymentMethod {
            customer_id: id_type::CustomerId::default(),
            merchant_id: id_type::MerchantId::default(),
            payment_method_id: String::from("abc"),
            accepted_currency: None,
            scheme: None,
            token: None,
            cardholder_name: None,
            issuer_name: None,
            issuer_country: None,
            payer_country: None,
            is_stored: None,
            swift_code: None,
            direct_debit_token: None,
            created_at: common_utils::date_time::now(),
            last_modified: common_utils::date_time::now(),
            payment_method: None,
            payment_method_type: None,
            payment_method_issuer: None,
            payment_method_issuer_code: None,
            metadata: None,
            payment_method_data: None,
            locker_id: None,
            last_used_at: common_utils::date_time::now(),
            connector_mandate_details: mandate_data,
            customer_acceptance: None,
            status: storage_enums::PaymentMethodStatus::Active,
            network_transaction_id: None,
            client_secret: None,
            payment_method_billing_address: None,
            updated_by: None,
            version: common_enums::ApiVersion::V1,
            network_token_requestor_reference_id: None,
            network_token_locker_id: None,
            network_token_payment_method_data: None,
            vault_source_details: Default::default(),
            created_by: None,
            last_modified_by: None,
        };
        payment_method.clone()
    }

    #[test]
    fn test_get_common_mandate_reference_payments_only() {
        let connector_mandate_details = serde_json::json!({
            "mca_kGz30G8B95MxRwmeQqy6": {
                "mandate_metadata": null,
                "payment_method_type": null,
                "connector_mandate_id": "RcBww0a02c-R22w22w22wNJV-V14o20u24y18sTB18sB24y06g04eVZ04e20u14o",
                "connector_mandate_status": "active",
                "original_payment_authorized_amount": 51,
                "original_payment_authorized_currency": "USD",
                "connector_mandate_request_reference_id": "RowbU9ULN9H59bMhWk"
            }
        });

        let payment_method = get_payment_method_with_mandate_data(Some(connector_mandate_details));

        let result = payment_method.get_common_mandate_reference();

        assert!(result.is_ok());
        let common_mandate = result.unwrap();

        assert!(common_mandate.payments.is_some());
        assert!(common_mandate.payouts.is_none());

        let payments = common_mandate.payments.unwrap();
        let result_mca = MerchantConnectorAccountId::wrap("mca_kGz30G8B95MxRwmeQqy6".to_string());
        assert!(
            result_mca.is_ok(),
            "Expected Ok, but got Err: {result_mca:?}",
        );
        let mca = result_mca.unwrap();
        assert!(payments.0.contains_key(&mca));
    }

    #[test]
    fn test_get_common_mandate_reference_empty_details() {
        let payment_method = get_payment_method_with_mandate_data(None);
        let result = payment_method.get_common_mandate_reference();

        assert!(result.is_ok());
        let common_mandate = result.unwrap();

        assert!(common_mandate.payments.is_none());
        assert!(common_mandate.payouts.is_none());
    }

    #[test]
    fn test_get_common_mandate_reference_payouts_only() {
        let connector_mandate_details = serde_json::json!({
            "payouts": {
                "mca_DAHVXbXpbYSjnL7fQWEs": {
                    "transfer_method_id": "TRM-678ab3997b16cb7cd"
                }
            }
        });

        let payment_method = get_payment_method_with_mandate_data(Some(connector_mandate_details));

        let result = payment_method.get_common_mandate_reference();

        assert!(result.is_ok());
        let common_mandate = result.unwrap();

        assert!(common_mandate.payments.is_some());
        assert!(common_mandate.payouts.is_some());

        let payouts = common_mandate.payouts.unwrap();
        let result_mca = MerchantConnectorAccountId::wrap("mca_DAHVXbXpbYSjnL7fQWEs".to_string());
        assert!(
            result_mca.is_ok(),
            "Expected Ok, but got Err: {result_mca:?}",
        );
        let mca = result_mca.unwrap();
        assert!(payouts.0.contains_key(&mca));
    }

    #[test]
    fn test_get_common_mandate_reference_invalid_data() {
        let connector_mandate_details = serde_json::json!("invalid");
        let payment_method = get_payment_method_with_mandate_data(Some(connector_mandate_details));
        let result = payment_method.get_common_mandate_reference();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_common_mandate_reference_with_payments_and_payouts_details() {
        let connector_mandate_details = serde_json::json!({
            "mca_kGz30G8B95MxRwmeQqy6": {
                "mandate_metadata": null,
                "payment_method_type": null,
                "connector_mandate_id": "RcBww0a02c-R22w22w22wNJV-V14o20u24y18sTB18sB24y06g04eVZ04e20u14o",
                "connector_mandate_status": "active",
                "original_payment_authorized_amount": 51,
                "original_payment_authorized_currency": "USD",
                "connector_mandate_request_reference_id": "RowbU9ULN9H59bMhWk"
            },
            "payouts": {
                "mca_DAHVXbXpbYSjnL7fQWEs": {
                    "transfer_method_id": "TRM-678ab3997b16cb7cd"
                }
            }
        });

        let payment_method = get_payment_method_with_mandate_data(Some(connector_mandate_details));

        let result = payment_method.get_common_mandate_reference();

        assert!(result.is_ok());
        let common_mandate = result.unwrap();

        assert!(common_mandate.payments.is_some());
        assert!(common_mandate.payouts.is_some());
    }
}
