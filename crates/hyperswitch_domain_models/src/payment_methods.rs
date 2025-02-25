#[cfg(feature = "v2")]
use api_models::payment_methods::PaymentMethodsData;
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use common_utils::{crypto::Encryptable, encryption::Encryption, types::keymanager::ToEncryptable};
use common_utils::{
    crypto::OptionalEncryptableValue,
    errors::{CustomResult, ParsingError, ValidationError},
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
use crate::{address::Address, type_encryption::OptionalEncryptableJsonType};
use crate::{
    mandates::{CommonMandateReference, PaymentsMandateReference},
    type_encryption::{crypto_operation, AsyncLift, CryptoOperation},
};

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

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    pub fn get_common_mandate_reference(&self) -> Result<CommonMandateReference, ParsingError> {
        let payments_data = self
            .connector_mandate_details
            .clone()
            .map(|mut mandate_details| {
                mandate_details
                    .as_object_mut()
                    .map(|obj| obj.remove("payouts"));

                serde_json::from_value::<PaymentsMandateReference>(mandate_details).inspect_err(
                    |err| {
                        router_env::logger::error!("Failed to parse payments data: {:?}", err);
                    },
                )
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

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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
    pub expires_at: PrimitiveDateTime,
}

#[cfg(feature = "v2")]
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
            expires_at: self.expires_at,
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
                expires_at: storage_model.expires_at,
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
            expires_at: self.expires_at,
        })
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use common_utils::id_type::MerchantConnectorAccountId;

    use super::*;

    fn get_payment_method_with_mandate_data(
        mandate_data: Option<serde_json::Value>,
    ) -> PaymentMethod {
        let payment_method = PaymentMethod {
            customer_id: common_utils::id_type::CustomerId::default(),
            merchant_id: common_utils::id_type::MerchantId::default(),
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
            "Expected Ok, but got Err: {:?}",
            result_mca
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
            "Expected Ok, but got Err: {:?}",
            result_mca
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
