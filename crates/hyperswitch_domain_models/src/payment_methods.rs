#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use common_utils::crypto::Encryptable;
use common_utils::{
    crypto::OptionalEncryptableValue,
    errors::{CustomResult, ValidationError},
    pii, type_name,
    types::keymanager,
};
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use crate::type_encryption::OptionalEncryptableJsonType;
use crate::type_encryption::{crypto_operation, AsyncLift, CryptoOperation};

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct VaultId(String);

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl VaultId {
    pub fn get_string_repr(&self) -> &String {
        &self.0
    }

    pub fn generate(id: String) -> Self {
        Self(id)
    }
}
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Clone, Debug)]
pub struct PaymentMethod {
    pub customer_id: common_utils::id_type::CustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
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
    pub transaction_flow: Option<storage_enums::PaymentDirection>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Clone, Debug)]
pub struct PaymentMethod {
    pub customer_id: common_utils::id_type::GlobalCustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub payment_method_type: Option<storage_enums::PaymentMethod>,
    pub payment_method_subtype: Option<storage_enums::PaymentMethodType>,
    pub payment_method_data:
        OptionalEncryptableJsonType<api_models::payment_methods::PaymentMethodsData>,
    pub locker_id: Option<VaultId>,
    pub last_used_at: PrimitiveDateTime,
    pub connector_mandate_details: Option<diesel_models::PaymentsMandateReference>,
    pub customer_acceptance: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::PaymentMethodStatus,
    pub network_transaction_id: Option<String>,
    pub client_secret: Option<String>,
    pub payment_method_billing_address: OptionalEncryptableValue,
    pub updated_by: Option<String>,
    pub locker_fingerprint_id: Option<String>,
    pub id: common_utils::id_type::GlobalPaymentMethodId,
    pub version: common_enums::ApiVersion,
    pub network_token_requestor_reference_id: Option<String>,
    pub network_token_locker_id: Option<String>,
    pub network_token_payment_method_data: OptionalEncryptableValue,
    pub transaction_flow: Option<storage_enums::PaymentDirection>,
}

impl PaymentMethod {
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    pub fn get_id(&self) -> &String {
        &self.payment_method_id
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    pub fn get_id(&self) -> &common_utils::id_type::GlobalPaymentMethodId {
        &self.id
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    pub fn get_payment_method_type(&self) -> Option<storage_enums::PaymentMethod> {
        self.payment_method
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    pub fn get_payment_method_type(&self) -> Option<storage_enums::PaymentMethod> {
        self.payment_method_type
    }

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    pub fn get_payment_method_subtype(&self) -> Option<storage_enums::PaymentMethodType> {
        self.payment_method_type
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    pub fn get_payment_method_subtype(&self) -> Option<storage_enums::PaymentMethodType> {
        self.payment_method_subtype
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[async_trait::async_trait]
impl super::behaviour::Conversion for PaymentMethod {
    type DstType = diesel_models::payment_method::PaymentMethod;
    type NewDstType = diesel_models::payment_method::PaymentMethodNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
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
            transaction_flow: self.transaction_flow,
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
        async {
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
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
                payment_method_data: item
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
                    .await?,
                locker_id: item.locker_id,
                last_used_at: item.last_used_at,
                connector_mandate_details: item.connector_mandate_details,
                customer_acceptance: item.customer_acceptance,
                status: item.status,
                network_transaction_id: item.network_transaction_id,
                client_secret: item.client_secret,
                payment_method_billing_address: item
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
                    .await?,
                updated_by: item.updated_by,
                version: item.version,
                network_token_requestor_reference_id: item.network_token_requestor_reference_id,
                network_token_locker_id: item.network_token_locker_id,
                network_token_payment_method_data: item
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
                    .await?,
                transaction_flow: item.transaction_flow,
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
            transaction_flow: self.transaction_flow,
        })
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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
            connector_mandate_details: self.connector_mandate_details,
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
            transaction_flow: self.transaction_flow,
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
        async {
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                customer_id: item.customer_id,
                merchant_id: item.merchant_id,
                id: item.id,
                created_at: item.created_at,
                last_modified: item.last_modified,
                payment_method_type: item.payment_method_type_v2,
                payment_method_subtype: item.payment_method_subtype,
                payment_method_data: item
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
                    .await?,
                locker_id: item.locker_id.map(VaultId::generate),
                last_used_at: item.last_used_at,
                connector_mandate_details: item.connector_mandate_details,
                customer_acceptance: item.customer_acceptance,
                status: item.status,
                network_transaction_id: item.network_transaction_id,
                client_secret: item.client_secret,
                payment_method_billing_address: item
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
                    .await?,
                updated_by: item.updated_by,
                locker_fingerprint_id: item.locker_fingerprint_id,
                version: item.version,
                network_token_requestor_reference_id: item.network_token_requestor_reference_id,
                network_token_locker_id: item.network_token_locker_id,
                network_token_payment_method_data: item
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
                    .await?,
                transaction_flow: item.transaction_flow,
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
            connector_mandate_details: self.connector_mandate_details,
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
            transaction_flow: self.transaction_flow,
        })
    }
}
