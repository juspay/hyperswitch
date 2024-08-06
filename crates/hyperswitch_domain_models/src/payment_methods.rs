use common_utils::{
    crypto::OptionalEncryptableValue,
    // date_time,
    // encryption::Encryption,
    errors::{CustomResult, ValidationError},
    pii,
    types::keymanager,
};
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

use crate::type_encryption::{decrypt_optional, AsyncLift};

// pub struct PaymentMethodReq {
//     pub payment_method_data: Option<Secret<serde_json::Value>>,
//     pub payment_method_billing_address: Option<Secret<serde_json::Value>>,
// }

// pub struct PaymentMethodReqWithEncryption {
//     pub payment_method_data: Option<Encryption>,
//     pub payment_method_billing_address: Option<Encryption>,
// }

// pub struct EncryptablePaymentMethodReq {
//     pub payment_method_data: OptionalEncryptableValue,
//     pub payment_method_billing_address: OptionalEncryptableValue,
// }

// impl keymanager::ToEncryptable<EncryptablePaymentMethodReq, Secret<String>, Encryption>
//     for PaymentMethodReqWithEncryption
// {
//     fn to_encryptable(self) -> FxHashMap<String, Encryption> {
//         let mut map = FxHashMap::with_capacity_and_hasher(3, Default::default());
//         self.payment_method_data
//             .map(|x| map.insert("payment_method_data".to_string(), x));
//         self.payment_method_billing_address
//             .map(|x| map.insert("payment_method_billing_address".to_string(), x));
//         map
//     }

//     fn from_encryptable(
//         mut hashmap: FxHashMap<String, crypto::Encryptable<Secret<String>>>,
//     ) -> common_utils::errors::CustomResult<
//         EncryptablePaymentMethodReq,
//         common_utils::errors::ParsingError,
//     > {
//         Ok(EncryptablePaymentMethodReq {
//             payment_method_data: hashmap.remove("payment_method_data"),
//             payment_method_billing_address: hashmap.remove("payment_method_billing_address"),
//         })
//     }
// }

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[derive(Clone, Debug)]
pub struct PaymentMethod {
    pub customer_id: common_utils::id_type::CustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub payment_method_id: common_utils::id_type::PaymentMethodId,
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
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Clone, Debug)]
pub struct PaymentMethod {
    pub customer_id: common_utils::id_type::CustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub payment_method: Option<storage_enums::PaymentMethod>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
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
    pub id: common_utils::id_type::PaymentMethodId,
}

impl PaymentMethod {
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    pub fn get_id(&self) -> &common_utils::id_type::PaymentMethodId {
        &self.payment_method_id
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    pub fn get_id(&self) -> &common_utils::id_type::PaymentMethodId {
        &self.id
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
            payment_method_id: self.payment_method_id.get_string_repr().to_string(),
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
        // let decrypted = batch_decrypt(
        //     state,
        //     PaymentMethodReqWithEncryption::to_encryptable(PaymentMethodReqWithEncryption {
        //         payment_method_data: item.payment_method_data.clone(),
        //         payment_method_billing_address: item.payment_method_billing_address.clone(),
        //     }),
        //     keymanager::Identifier::Merchant(item.merchant_id.clone()),
        //     key.peek(),
        // )
        // .await
        // .change_context(ValidationError::InvalidValue {
        //     message: "Failed while decrypting customer data".to_string(),
        // })?;

        // let encryptable_pm = PaymentMethodReqWithEncryption::from_encryptable(decrypted)
        //     .change_context(ValidationError::InvalidValue {
        //         message: "Failed while decrypting customer data".to_string(),
        //     })?;

        let payment_method_id =
            common_utils::id_type::PaymentMethodId::from(item.payment_method_id.into())
                .attach_printable("Failed to convert to PaymentMethodId from string")?;

        async {
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                customer_id: item.customer_id,
                merchant_id: item.merchant_id,
                payment_method_id,
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
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, key_manager_identifier.clone(), key.peek())
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
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, key_manager_identifier.clone(), key.peek())
                    })
                    .await?,
                updated_by: item.updated_by,
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
            payment_method_id: self.payment_method_id.get_string_repr().to_string(),
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
            id: self.id.get_string_repr().to_string(),
            created_at: self.created_at,
            last_modified: self.last_modified,
            payment_method: self.payment_method,
            payment_method_type: self.payment_method_type,
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
        // let decrypted = batch_decrypt(
        //     state,
        //     PaymentMethodReqWithEncryption::to_encryptable(PaymentMethodReqWithEncryption {
        //         payment_method_data: item.payment_method_data.clone(),
        //         payment_method_billing_address: item.payment_method_billing_address.clone(),
        //     }),
        //     keymanager::Identifier::Merchant(item.merchant_id.clone()),
        //     key.peek(),
        // )
        // .await
        // .change_context(ValidationError::InvalidValue {
        //     message: "Failed while decrypting customer data".to_string(),
        // })?;

        // let encryptable_pm = PaymentMethodReqWithEncryption::from_encryptable(decrypted)
        //     .change_context(ValidationError::InvalidValue {
        //         message: "Failed while decrypting customer data".to_string(),
        //     })?;

        let id = common_utils::id_type::PaymentMethodId::from(item.id.into())
            .attach_printable("Failed to convert to PaymentMethodId from string")?;

        async {
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
                customer_id: item.customer_id,
                merchant_id: item.merchant_id,
                id,
                created_at: item.created_at,
                last_modified: item.last_modified,
                payment_method: item.payment_method,
                payment_method_type: item.payment_method_type,
                metadata: item.metadata,
                payment_method_data: item
                    .payment_method_data
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, key_manager_identifier.clone(), key.peek())
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
                    .async_lift(|inner| {
                        decrypt_optional(state, inner, key_manager_identifier.clone(), key.peek())
                    })
                    .await?,
                updated_by: item.updated_by,
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
            id: self.id.get_string_repr().to_string(),
            created_at: self.created_at,
            last_modified: self.last_modified,
            payment_method: self.payment_method,
            payment_method_type: self.payment_method_type,
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
        })
    }
}
