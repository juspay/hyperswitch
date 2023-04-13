use common_utils::{
    crypto::{self, Encryptable, GcmAes256},
    ext_traits::AsyncExt,
    pii,
};
use error_stack::ResultExt;
use storage_models::{customers::CustomerUpdateInternal, encryption::Encryption};
use time::PrimitiveDateTime;

use super::types::{get_key_and_algo, TypeEncryption};
use crate::{
    db::StorageInterface,
    errors::{CustomResult, ValidationError},
};

#[derive(Clone, Debug)]
pub struct Customer {
    pub id: Option<i32>,
    pub customer_id: String,
    pub merchant_id: String,
    pub name: crypto::OptionalEncryptableName,
    pub email: crypto::OptionalEncryptableEmail,
    pub phone: crypto::OptionalEncryptablePhone,
    pub phone_country_code: Option<String>,
    pub description: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for Customer {
    type DstType = storage_models::customers::Customer;
    type NewDstType = storage_models::customers::CustomerNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(storage_models::customers::Customer {
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
        })
    }

    async fn convert_back(
        item: Self::DstType,
        db: &dyn StorageInterface,
        merchant_id: &str,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let key = get_key_and_algo(db, merchant_id).await.change_context(
            ValidationError::InvalidValue {
                message: "Failed while getting key from key store".to_string(),
            },
        )?;
        async {
            Ok(Self {
                id: Some(item.id),
                customer_id: item.customer_id,
                merchant_id: item.merchant_id,
                name: item
                    .name
                    .async_map(|value| Encryptable::decrypt(value, &key, GcmAes256 {}))
                    .await
                    .transpose()?,
                email: item
                    .email
                    .async_map(|value| Encryptable::decrypt(value, &key, GcmAes256 {}))
                    .await
                    .transpose()?,
                phone: item
                    .phone
                    .async_map(|value| Encryptable::decrypt(value, &key, GcmAes256 {}))
                    .await
                    .transpose()?,
                phone_country_code: item.phone_country_code,
                description: item.description,
                created_at: item.created_at,
                metadata: item.metadata,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting customer data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(storage_models::customers::CustomerNew {
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            name: self.name.map(Encryption::from),
            email: self.email.map(Encryption::from),
            phone: self.phone.map(Encryption::from),
            description: self.description,
            phone_country_code: self.phone_country_code,
            metadata: self.metadata,
        })
    }
}

#[derive(Debug)]
pub enum CustomerUpdate {
    Update {
        name: crypto::OptionalEncryptableName,
        email: crypto::OptionalEncryptableEmail,
        phone: crypto::OptionalEncryptablePhone,
        description: Option<String>,
        phone_country_code: Option<String>,
        metadata: Option<pii::SecretSerdeValue>,
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
            } => Self {
                name: name.map(Encryption::from),
                email: email.map(Encryption::from),
                phone: phone.map(Encryption::from),
                description,
                phone_country_code,
                metadata,
            },
        }
    }
}
