use api_models::customers::CustomerRequestWithEncryption;
use common_utils::{
    crypto, date_time,
    encryption::Encryption,
    id_type, pii,
    types::keymanager::{Identifier, KeyManagerState, ToEncryptable},
};
use diesel_models::customers::CustomerUpdateInternal;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

use super::types;
use crate::errors::{CustomResult, ValidationError};

#[derive(Clone, Debug)]
pub struct Customer {
    pub id: Option<i32>,
    pub customer_id: id_type::CustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub name: crypto::OptionalEncryptableName,
    pub email: crypto::OptionalEncryptableEmail,
    pub phone: crypto::OptionalEncryptablePhone,
    pub phone_country_code: Option<String>,
    pub description: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: PrimitiveDateTime,
    pub connector_customer: Option<serde_json::Value>,
    pub address_id: Option<String>,
    pub default_payment_method_id: Option<String>,
    pub updated_by: Option<String>,
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for Customer {
    type DstType = diesel_models::customers::Customer;
    type NewDstType = diesel_models::customers::CustomerNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::customers::Customer {
            id: self.id.ok_or(ValidationError::MissingRequiredField {
                field_name: "id".to_string(),
            })?,
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            name: self.name.map(|value| value.into()),
            email: self.email.map(|value| value.into()),
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
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_store_ref_id: String,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let decrypted = types::batch_decrypt(
            state,
            CustomerRequestWithEncryption::to_encryptable(CustomerRequestWithEncryption {
                name: item.name.clone(),
                phone: item.phone.clone(),
                email: item.email.clone(),
            }),
            Identifier::Merchant(item.merchant_id.clone()),
            key.peek(),
        )
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting customer data".to_string(),
        })?;
        let encryptable_customer = CustomerRequestWithEncryption::from_encryptable(decrypted)
            .change_context(ValidationError::InvalidValue {
                message: "Failed while decrypting customer data".to_string(),
            })?;

        Ok(Self {
            id: Some(item.id),
            customer_id: item.customer_id,
            merchant_id: item.merchant_id,
            name: encryptable_customer.name,
            email: encryptable_customer.email,
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
        })
    }
}

#[derive(Clone, Debug)]
pub enum CustomerUpdate {
    Update {
        name: crypto::OptionalEncryptableName,
        email: crypto::OptionalEncryptableEmail,
        phone: Box<crypto::OptionalEncryptablePhone>,
        description: Option<String>,
        phone_country_code: Option<String>,
        metadata: Option<pii::SecretSerdeValue>,
        connector_customer: Option<serde_json::Value>,
        address_id: Option<String>,
    },
    ConnectorCustomer {
        connector_customer: Option<serde_json::Value>,
    },
    UpdateDefaultPaymentMethod {
        default_payment_method_id: Option<Option<String>>,
    },
}

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
                connector_customer,
                modified_at: Some(date_time::now()),
                address_id,
                ..Default::default()
            },
            CustomerUpdate::ConnectorCustomer { connector_customer } => Self {
                connector_customer,
                modified_at: Some(date_time::now()),
                ..Default::default()
            },
            CustomerUpdate::UpdateDefaultPaymentMethod {
                default_payment_method_id,
            } => Self {
                default_payment_method_id,
                modified_at: Some(date_time::now()),
                ..Default::default()
            },
        }
    }
}
