use common_utils::{crypto, date_time, pii};
use diesel_models::{customers::CustomerUpdateInternal, encryption::Encryption};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

use super::types::{self, AsyncLift};
use crate::errors::{CustomResult, ValidationError};

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
    pub address_id: Option<String>,
    pub default_payment_method: Option<String>,
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for Customer {
    type DstType = diesel_models::customers::Customer;
    type NewDstType = diesel_models::customers::CustomerNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::customers::Customer {
            id: __self.id.ok_or(ValidationError::MissingRequiredField {
                field_name: "id".to_string(),
            })?,
            customer_id: __self.customer_id,
            merchant_id: __self.merchant_id,
            name: __self.name.map(|value| value.into()),
            email: __self.email.map(|value| value.into()),
            phone: __self.phone.map(Encryption::from),
            phone_country_code: __self.phone_country_code,
            description: __self.description,
            created_at: __self.created_at,
            metadata: __self.metadata,
            modified_at: __self.modified_at,
            connector_customer: __self.connector_customer,
            address_id: __self.address_id,
            default_payment_method: __self.default_payment_method,
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
                address_id: item.address_id,
                default_payment_method: item.default_payment_method,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting customer data".to_string(),
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
        default_payment_method: Option<String>,
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
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
            CustomerUpdate::UpdateDefaultPaymentMethod {
                default_payment_method,
            } => Self {
                default_payment_method,
                modified_at: Some(common_utils::date_time::now()),
                ..Default::default()
            },
        }
    }
}
