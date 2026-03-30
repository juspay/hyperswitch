use common_utils::{
    crypto::Encryptable,
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    pii,
    types::keymanager::{self, KeyManagerState, ToEncryptable},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    customer::{Customer, EncryptedCustomer},
    type_encryption::{self as types, AsyncLift},
};
use hyperswitch_masking::{PeekInterface, Secret, SwitchStrategy};

use crate::behaviour::Conversion;

#[async_trait::async_trait]
#[cfg(feature = "v1")]
impl Conversion for Customer {
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
            tax_registration_id: self.tax_registration_id.map(Encryption::from),
            document_details: self.document_details.map(Encryption::from),
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            last_modified_by: self
                .last_modified_by
                .map(|last_modified_by| last_modified_by.to_string()),
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
                    tax_registration_id: item.tax_registration_id.clone(),
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

        let document_details = item
            .document_details
            .async_lift(|inner| async {
                types::crypto_operation(
                    state,
                    common_utils::type_name!(Self),
                    types::CryptoOperation::DecryptOptional(inner),
                    keymanager::Identifier::Merchant(item.merchant_id.clone()),
                    key.peek(),
                )
                .await
                .and_then(|val| val.try_into_optionaloperation())
            })
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed to decrypt document details".to_string(),
            })?;

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
            tax_registration_id: encryptable_customer.tax_registration_id,
            document_details,
            created_by: item
                .created_by
                .and_then(|created_by| created_by.parse::<common_utils::types::CreatedBy>().ok()),
            last_modified_by: item
                .last_modified_by
                .and_then(|last_modified_by| last_modified_by.parse::<common_utils::types::CreatedBy>().ok()),
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
            tax_registration_id: self.tax_registration_id.map(Encryption::from),
            document_details: self.document_details.map(Encryption::from),
            created_by: self
                .created_by
                .as_ref()
                .map(|created_by| created_by.to_string()),
            last_modified_by: self.created_by.map(|created_by| created_by.to_string()),
        })
    }
}

#[async_trait::async_trait]
#[cfg(feature = "v2")]
impl Conversion for Customer {
    type DstType = diesel_models::customers::Customer;
    type NewDstType = diesel_models::customers::CustomerNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::customers::Customer {
            id: self.id.clone(),
            customer_id: Some(self.id),
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
            tax_registration_id: self.tax_registration_id.map(Encryption::from),
            document_details: self.document_details.map(Encryption::from),
            created_by: self.created_by.map(|created_by| created_by.to_string()),
            last_modified_by: self
                .last_modified_by
                .map(|last_modified_by| last_modified_by.to_string()),
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
                    tax_registration_id: item.tax_registration_id.clone(),
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

        let default_billing_address = item
            .default_billing_address
            .async_lift(|inner| async {
                types::crypto_operation(
                    state,
                    common_utils::type_name!(Self),
                    types::CryptoOperation::DecryptOptional(inner),
                    keymanager::Identifier::Merchant(item.merchant_id.clone()),
                    key.peek(),
                )
                .await
                .and_then(|val| val.try_into_optionaloperation())
            })
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed to decrypt default billing address".to_string(),
            })?;

        let default_shipping_address = item
            .default_shipping_address
            .async_lift(|inner| async {
                types::crypto_operation(
                    state,
                    common_utils::type_name!(Self),
                    types::CryptoOperation::DecryptOptional(inner),
                    keymanager::Identifier::Merchant(item.merchant_id.clone()),
                    key.peek(),
                )
                .await
                .and_then(|val| val.try_into_optionaloperation())
            })
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed to decrypt default shipping address".to_string(),
            })?;

        let document_details = item
            .document_details
            .async_lift(|inner| async {
                types::crypto_operation(
                    state,
                    common_utils::type_name!(Self),
                    types::CryptoOperation::DecryptOptional(inner),
                    keymanager::Identifier::Merchant(item.merchant_id.clone()),
                    key.peek(),
                )
                .await
                .and_then(|val| val.try_into_optionaloperation())
            })
            .await
            .change_context(ValidationError::InvalidValue {
                message: "Failed to decrypt document details".to_string(),
            })?;

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
            default_billing_address,
            default_shipping_address,
            version: item.version,
            status: item.status,
            tax_registration_id: encryptable_customer.tax_registration_id,
            document_details,
            created_by: item
                .created_by
                .and_then(|created_by| created_by.parse::<common_utils::types::CreatedBy>().ok()),
            last_modified_by: item
                .last_modified_by
                .and_then(|last_modified_by| last_modified_by.parse::<common_utils::types::CreatedBy>().ok()),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
        Ok(diesel_models::customers::CustomerNew {
            id: self.id.clone(),
            customer_reference_id: self.merchant_reference_id,
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
            default_billing_address: self.default_billing_address.map(Encryption::from),
            default_shipping_address: self.default_shipping_address.map(Encryption::from),
            version: common_types::consts::API_VERSION,
            status: self.status,
            tax_registration_id: self.tax_registration_id.map(Encryption::from),
            document_details: self.document_details.map(Encryption::from),
            created_by: self
                .created_by
                .as_ref()
                .map(|created_by| created_by.to_string()),
            last_modified_by: self.created_by.map(|created_by| created_by.to_string()),
            customer_id: Some(self.id),
        })
    }
}
