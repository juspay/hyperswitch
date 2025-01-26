#[cfg(feature = "v2")]
use api_models::payment_methods::PaymentMethodsData;
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use common_utils::{crypto::Encryptable, encryption::Encryption, types::keymanager::ToEncryptable};
use common_utils::{
    crypto::OptionalEncryptableValue,
    errors::{CustomResult, ValidationError},
    pii, type_name,
    types::keymanager,
};
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
// specific imports because of using the macro
#[cfg(feature = "v2")]
use rustc_hash::FxHashMap;
#[cfg(feature = "v2")]
use serde_json::Value;
use time::PrimitiveDateTime;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use crate::address::Address;
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
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Clone, Debug, router_derive::ToEncryption)]
pub struct PaymentMethod {
    /// The identifier for the payment method. Using this recurring payments can be made
    pub id: common_utils::id_type::GlobalPaymentMethodId,

    /// The customer id against which the payment method is saved
    pub customer_id: common_utils::id_type::GlobalCustomerId,

    /// The merchant id against which the payment method is saved
    pub merchant_id: common_utils::id_type::MerchantId,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub payment_method_type: Option<storage_enums::PaymentMethod>,
    pub payment_method_subtype: Option<storage_enums::PaymentMethodType>,
    #[encrypt(ty = Value)]
    pub payment_method_data: Option<Encryptable<PaymentMethodsData>>,
    pub locker_id: Option<VaultId>,
    pub last_used_at: PrimitiveDateTime,
    pub connector_mandate_details: Option<diesel_models::PaymentsMandateReference>,
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
    pub network_token_payment_method_data: Option<Encryptable<Value>>,
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
        use masking::ExposeInterface;

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

            let network_token_payment_method_data =
                data.network_token_payment_method_data
                    .map(|network_token_payment_method_data| {
                        network_token_payment_method_data.map(|value| value.expose())
                    });

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
                connector_mandate_details: storage_model.connector_mandate_details,
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
        })
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, router_derive::ToEncryption)]
pub struct PaymentMethodsSession {
    pub id: common_utils::id_type::GlobalPaymentMethodSessionId,
    pub customer_id: common_utils::id_type::GlobalCustomerId,
    #[encrypt(ty = Value)]
    pub billing: Option<Encryptable<Address>>,
    pub psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
    pub network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for PaymentMethodsSession {
    type DstType = diesel_models::payment_methods_session::PaymentMethodsSession;
    type NewDstType = diesel_models::payment_methods_session::PaymentMethodsSession;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(Self::DstType {
            id: self.id,
            customer_id: self.customer_id,
            billing: self.billing.map(|val| val.into()),
            psp_tokenization: self.psp_tokenization,
            network_tokeinzation: self.network_tokenization,
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
                CryptoOperation::BatchDecrypt(EncryptedPaymentMethodsSession::to_encryptable(
                    EncryptedPaymentMethodsSession {
                        billing: storage_model.billing,
                    },
                )),
                key_manager_identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_batchoperation())?;

            let data = EncryptedPaymentMethodsSession::from_encryptable(decrypted_data)
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
                network_tokenization: storage_model.network_tokeinzation,
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
            network_tokeinzation: self.network_tokenization,
        })
    }
}
