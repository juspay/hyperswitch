use common_utils::pii;
use masking::Secret;
use time::PrimitiveDateTime;

use crate::errors::{CustomResult, ValidationError};

#[derive(Clone, Debug)]
pub struct Customer {
    pub id: Option<i32>,
    pub customer_id: String,
    pub merchant_id: String,
    pub name: Option<String>,
    pub email: Option<Secret<String, pii::Email>>,
    pub phone: Option<Secret<String>>,
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
            name: self.name,
            email: self.email,
            phone: self.phone,
            phone_country_code: self.phone_country_code,
            description: self.description,
            created_at: self.created_at,
            metadata: self.metadata,
        })
    }

    async fn convert_back(item: Self::DstType) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        Ok(Self {
            id: Some(item.id),
            customer_id: item.customer_id,
            merchant_id: item.merchant_id,
            name: item.name,
            email: item.email,
            phone: item.phone,
            phone_country_code: item.phone_country_code,
            description: item.description,
            created_at: item.created_at,
            metadata: item.metadata,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(storage_models::customers::CustomerNew {
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
            name: self.name,
            email: self.email,
            phone: self.phone,
            description: self.description,
            phone_country_code: self.phone_country_code,
            metadata: self.metadata,
        })
    }
}
