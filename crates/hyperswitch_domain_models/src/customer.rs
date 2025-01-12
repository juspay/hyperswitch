#[cfg(all(feature = "v2", feature = "customer_v2"))]
use common_enums::DeleteStatus;
use common_utils::{
    crypto::{self, Encryptable},
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    id_type, pii,
    types::{
        keymanager::{self, KeyManagerState, ToEncryptable},
        Description,
    },
};
use diesel_models::customers::CustomerUpdateInternal;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret, SwitchStrategy};
use rustc_hash::FxHashMap;
use time::PrimitiveDateTime;

use crate::type_encryption as types;

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[derive(Clone, Debug, router_derive::ToEncryption)]
pub struct Customer {
    pub customer_id: id_type::CustomerId,
    pub merchant_id: id_type::MerchantId,
    #[encrypt]
    pub name: Option<Encryptable<Secret<String>>>,
    #[encrypt]
    pub email: Option<Encryptable<Secret<String, pii::EmailStrategy>>>,
    #[encrypt]
    pub phone: Option<Encryptable<Secret<String>>>,
    pub phone_country_code: Option<String>,
    pub description: Option<Description>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: PrimitiveDateTime,
    pub connector_customer: Option<pii::SecretSerdeValue>,
    pub address_id: Option<String>,
    pub default_payment_method_id: Option<String>,
    pub updated_by: Option<String>,
    pub version: common_enums::ApiVersion,
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[derive(Clone, Debug, router_derive::ToEncryption)]
pub struct Customer {
    pub merchant_id: id_type::MerchantId,
    #[encrypt]
    pub name: Option<Encryptable<Secret<String>>>,
    #[encrypt]
    pub email: Option<Encryptable<Secret<String, pii::EmailStrategy>>>,
    #[encrypt]
    pub phone: Option<Encryptable<Secret<String>>>,
    pub phone_country_code: Option<String>,
    pub description: Option<Description>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Option<pii::SecretSerdeValue>,
    pub modified_at: PrimitiveDateTime,
    pub default_payment_method_id: Option<id_type::GlobalPaymentMethodId>,
    pub updated_by: Option<String>,
    pub merchant_reference_id: Option<id_type::CustomerId>,
    pub default_billing_address: Option<Encryption>,
    pub default_shipping_address: Option<Encryption>,
    pub id: id_type::GlobalCustomerId,
    pub version: common_enums::ApiVersion,
    pub status: DeleteStatus,
}

impl Customer {
    /// Get the unique identifier of Customer
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    pub fn get_id(&self) -> &id_type::CustomerId {
        &self.customer_id
    }

    /// Get the global identifier of Customer
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    pub fn get_id(&self) -> &id_type::GlobalCustomerId {
        &self.id
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[async_trait::async_trait]
impl super::behaviour::Conversion for Customer {
    type DstType = diesel_models::customers::Customer;
    type NewDstType = diesel_models::customers::CustomerNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::customers::Customer {
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            name: self.name.map(Encryption::from),
            email: self.email.map(Encryption::from),
            phone: self.phone.map(Encryption::from),
            phone_country_code: self.phone_country_code,
            description: self.description,
            created_at: self.created_at,
            metadata: self.metadata,
            modified_at: self.modified_at,
            connector_customer: self.connector_customer,
            address_id: self.address_id,
            default_payment_method_id: self.default_payment_method_id,
            updated_by: self.updated_by,
            version: self.version,
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_store_ref_id: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let decrypted = types::crypto_operation(
            state,
            common_utils::type_name!(Self::DstType),
            types::CryptoOperation::BatchDecrypt(EncryptedCustomer::to_encryptable(
                EncryptedCustomer {
                    name: item.name.clone(),
                    phone: item.phone.clone(),
                    email: item.email.clone(),
                },
            )),
            keymanager::Identifier::Merchant(item.merchant_id.clone()),
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting customer data".to_string(),
        })?;
        let encryptable_customer = EncryptedCustomer::from_encryptable(decrypted).change_context(
            ValidationError::InvalidValue {
                message: "Failed while decrypting customer data".to_string(),
            },
        )?;

        Ok(Self {
            customer_id: item.customer_id,
            merchant_id: item.merchant_id,
            name: encryptable_customer.name,
            email: encryptable_customer.email.map(|email| {
                let encryptable: Encryptable<Secret<String, pii::EmailStrategy>> = Encryptable::new(
                    email.clone().into_inner().switch_strategy(),
                    email.into_encrypted(),
                );
                encryptable
            }),
            phone: encryptable_customer.phone,
            phone_country_code: item.phone_country_code,
            description: item.description,
            created_at: item.created_at,
            metadata: item.metadata,
            modified_at: item.modified_at,
            connector_customer: item.connector_customer,
            address_id: item.address_id,
            default_payment_method_id: item.default_payment_method_id,
            updated_by: item.updated_by,
            version: item.version,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
        Ok(diesel_models::customers::CustomerNew {
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            name: self.name.map(Encryption::from),
            email: self.email.map(Encryption::from),
            phone: self.phone.map(Encryption::from),
            description: self.description,
            phone_country_code: self.phone_country_code,
            metadata: self.metadata,
            created_at: now,
            modified_at: now,
            connector_customer: self.connector_customer,
            address_id: self.address_id,
            updated_by: self.updated_by,
            version: self.version,
        })
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[async_trait::async_trait]
impl super::behaviour::Conversion for Customer {
    type DstType = diesel_models::customers::Customer;
    type NewDstType = diesel_models::customers::CustomerNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::customers::Customer {
            id: self.id,
            merchant_reference_id: self.merchant_reference_id,
            merchant_id: self.merchant_id,
            name: self.name.map(Encryption::from),
            email: self.email.map(Encryption::from),
            phone: self.phone.map(Encryption::from),
            phone_country_code: self.phone_country_code,
            description: self.description,
            created_at: self.created_at,
            metadata: self.metadata,
            modified_at: self.modified_at,
            connector_customer: self.connector_customer,
            default_payment_method_id: self.default_payment_method_id,
            updated_by: self.updated_by,
            default_billing_address: self.default_billing_address.map(Encryption::from),
            default_shipping_address: self.default_shipping_address.map(Encryption::from),
            version: self.version,
            status: self.status,
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_store_ref_id: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let decrypted = types::crypto_operation(
            state,
            common_utils::type_name!(Self::DstType),
            types::CryptoOperation::BatchDecrypt(EncryptedCustomer::to_encryptable(
                EncryptedCustomer {
                    name: item.name.clone(),
                    phone: item.phone.clone(),
                    email: item.email.clone(),
                },
            )),
            keymanager::Identifier::Merchant(item.merchant_id.clone()),
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting customer data".to_string(),
        })?;
        let encryptable_customer = EncryptedCustomer::from_encryptable(decrypted).change_context(
            ValidationError::InvalidValue {
                message: "Failed while decrypting customer data".to_string(),
            },
        )?;

        Ok(Self {
            id: item.id,
            merchant_reference_id: item.merchant_reference_id,
            merchant_id: item.merchant_id,
            name: encryptable_customer.name,
            email: encryptable_customer.email.map(|email| {
                let encryptable: Encryptable<Secret<String, pii::EmailStrategy>> = Encryptable::new(
                    email.clone().into_inner().switch_strategy(),
                    email.into_encrypted(),
                );
                encryptable
            }),
            phone: encryptable_customer.phone,
            phone_country_code: item.phone_country_code,
            description: item.description,
            created_at: item.created_at,
            metadata: item.metadata,
            modified_at: item.modified_at,
            connector_customer: item.connector_customer,
            default_payment_method_id: item.default_payment_method_id,
            updated_by: item.updated_by,
            default_billing_address: item.default_billing_address,
            default_shipping_address: item.default_shipping_address,
            version: item.version,
            status: item.status,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
        Ok(diesel_models::customers::CustomerNew {
            id: self.id,
            merchant_reference_id: self.merchant_reference_id,
            merchant_id: self.merchant_id,
            name: self.name.map(Encryption::from),
            email: self.email.map(Encryption::from),
            phone: self.phone.map(Encryption::from),
            description: self.description,
            phone_country_code: self.phone_country_code,
            metadata: self.metadata,
            default_payment_method_id: None,
            created_at: now,
            modified_at: now,
            connector_customer: self.connector_customer,
            updated_by: self.updated_by,
            default_billing_address: self.default_billing_address,
            default_shipping_address: self.default_shipping_address,
            version: crate::consts::API_VERSION,
            status: self.status,
        })
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[derive(Clone, Debug)]
pub enum CustomerUpdate {
    Update {
        name: crypto::OptionalEncryptableName,
        email: Box<crypto::OptionalEncryptableEmail>,
        phone: Box<crypto::OptionalEncryptablePhone>,
        description: Option<Description>,
        phone_country_code: Option<String>,
        metadata: Option<pii::SecretSerdeValue>,
        connector_customer: Box<Option<pii::SecretSerdeValue>>,
        default_billing_address: Option<Encryption>,
        default_shipping_address: Option<Encryption>,
        default_payment_method_id: Option<Option<id_type::GlobalPaymentMethodId>>,
        status: Option<DeleteStatus>,
    },
    ConnectorCustomer {
        connector_customer: Option<pii::SecretSerdeValue>,
    },
    UpdateDefaultPaymentMethod {
        default_payment_method_id: Option<Option<id_type::GlobalPaymentMethodId>>,
    },
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl From<CustomerUpdate> for CustomerUpdateInternal {
    fn from(customer_update: CustomerUpdate) -> Self {
        match customer_update {
            CustomerUpdate::Update {
                name,
                email,
                phone,
                description,
                phone_country_code,
                metadata,
                connector_customer,
                default_billing_address,
                default_shipping_address,
                default_payment_method_id,
                status,
            } => Self {
                name: name.map(Encryption::from),
                email: email.map(Encryption::from),
                phone: phone.map(Encryption::from),
                description,
                phone_country_code,
                metadata,
                connector_customer: *connector_customer,
                modified_at: date_time::now(),
                default_billing_address,
                default_shipping_address,
                default_payment_method_id,
                updated_by: None,
                status,
            },
            CustomerUpdate::ConnectorCustomer { connector_customer } => Self {
                connector_customer,
                name: None,
                email: None,
                phone: None,
                description: None,
                phone_country_code: None,
                metadata: None,
                modified_at: date_time::now(),
                default_payment_method_id: None,
                updated_by: None,
                default_billing_address: None,
                default_shipping_address: None,
                status: None,
            },
            CustomerUpdate::UpdateDefaultPaymentMethod {
                default_payment_method_id,
            } => Self {
                default_payment_method_id,
                modified_at: date_time::now(),
                name: None,
                email: None,
                phone: None,
                description: None,
                phone_country_code: None,
                metadata: None,
                connector_customer: None,
                updated_by: None,
                default_billing_address: None,
                default_shipping_address: None,
                status: None,
            },
        }
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[derive(Clone, Debug)]
pub enum CustomerUpdate {
    Update {
        name: crypto::OptionalEncryptableName,
        email: crypto::OptionalEncryptableEmail,
        phone: Box<crypto::OptionalEncryptablePhone>,
        description: Option<Description>,
        phone_country_code: Option<String>,
        metadata: Option<pii::SecretSerdeValue>,
        connector_customer: Box<Option<pii::SecretSerdeValue>>,
        address_id: Option<String>,
    },
    ConnectorCustomer {
        connector_customer: Option<pii::SecretSerdeValue>,
    },
    UpdateDefaultPaymentMethod {
        default_payment_method_id: Option<Option<String>>,
    },
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl From<CustomerUpdate> for CustomerUpdateInternal {
    fn from(customer_update: CustomerUpdate) -> Self {
        match customer_update {
            CustomerUpdate::Update {
                name,
                email,
                phone,
                description,
                phone_country_code,
                metadata,
                connector_customer,
                address_id,
            } => Self {
                name: name.map(Encryption::from),
                email: email.map(Encryption::from),
                phone: phone.map(Encryption::from),
                description,
                phone_country_code,
                metadata,
                connector_customer: *connector_customer,
                modified_at: date_time::now(),
                address_id,
                default_payment_method_id: None,
                updated_by: None,
            },
            CustomerUpdate::ConnectorCustomer { connector_customer } => Self {
                connector_customer,
                modified_at: date_time::now(),
                name: None,
                email: None,
                phone: None,
                description: None,
                phone_country_code: None,
                metadata: None,
                default_payment_method_id: None,
                updated_by: None,
                address_id: None,
            },
            CustomerUpdate::UpdateDefaultPaymentMethod {
                default_payment_method_id,
            } => Self {
                default_payment_method_id,
                modified_at: date_time::now(),
                name: None,
                email: None,
                phone: None,
                description: None,
                phone_country_code: None,
                metadata: None,
                connector_customer: None,
                updated_by: None,
                address_id: None,
            },
        }
    }
}
