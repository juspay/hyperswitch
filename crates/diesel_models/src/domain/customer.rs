use common_utils::{crypto, date_time, pii};
use crate::{customers::CustomerUpdateInternal, encryption::Encryption};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

use super::types::{self, AsyncLift};
use common_utils::errors::{CustomResult, ValidationError};

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
    pub modified_at: PrimitiveDateTime,
    pub connector_customer: Option<serde_json::Value>,
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for Customer {
    type DstType = crate::customers::Customer;
    type NewDstType = crate::customers::CustomerNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(crate::customers::Customer {
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
        })
    }

    async fn convert_back(
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        async {
            let inner_decrypt = |inner| types::decrypt(inner, key.peek());
            let inner_decrypt_email = |inner| types::decrypt(inner, key.peek());
            Ok(Self {
                id: Some(item.id),
                customer_id: item.customer_id,
                merchant_id: item.merchant_id,
                name: item.name.async_lift(inner_decrypt).await?,
                email: item.email.async_lift(inner_decrypt_email).await?,
                phone: item.phone.async_lift(inner_decrypt).await?,
                phone_country_code: item.phone_country_code,
                description: item.description,
                created_at: item.created_at,
                metadata: item.metadata,
                modified_at: item.modified_at,
                connector_customer: item.connector_customer,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting customer data".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
        Ok(crate::customers::CustomerNew {
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
        connector_customer: Option<serde_json::Value>,
    },
    ConnectorCustomer {
        connector_customer: Option<serde_json::Value>,
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
            } => Self {
                name: name.map(Encryption::from),
                email: email.map(Encryption::from),
                phone: phone.map(Encryption::from),
                description,
                phone_country_code,
                metadata,
                connector_customer,
                modified_at: Some(date_time::now()),
            },
            CustomerUpdate::ConnectorCustomer { connector_customer } => Self {
                connector_customer,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
        }
    }
}
