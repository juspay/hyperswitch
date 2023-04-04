use async_trait::async_trait;
use common_utils::errors::{CustomResult, ValidationError};
use masking::Secret;
use storage_models::enums;
use time::PrimitiveDateTime;

use super::behaviour;

#[derive(Clone, Debug)]
pub struct Address {
    pub id: Option<i32>,
    pub address_id: String,
    pub city: Option<String>,
    pub country: Option<enums::CountryCode>,
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub line3: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub phone_number: Option<Secret<String>>,
    pub country_code: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub customer_id: String,
    pub merchant_id: String,
}

#[async_trait]
impl behaviour::Conversion for Address {
    type DstType = storage_models::address::Address;
    type NewDstType = storage_models::address::AddressNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(storage_models::address::Address {
            id: self.id.ok_or(ValidationError::MissingRequiredField {
                field_name: "id".to_string(),
            })?,
            address_id: self.address_id,
            city: self.city,
            country: self.country,
            line1: self.line1,
            line2: self.line2,
            line3: self.line3,
            state: self.state,
            zip: self.zip,
            first_name: self.first_name,
            last_name: self.last_name,
            phone_number: self.phone_number,
            country_code: self.country_code,
            created_at: self.created_at,
            modified_at: self.modified_at,
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
        })
    }

    async fn convert_back(other: Self::DstType) -> CustomResult<Self, ValidationError> {
        Ok(Self {
            id: Some(other.id),
            address_id: other.address_id,
            city: other.city,
            country: other.country,
            line1: other.line1,
            line2: other.line2,
            line3: other.line3,
            state: other.state,
            zip: other.zip,
            first_name: other.first_name,
            last_name: other.last_name,
            phone_number: other.phone_number,
            country_code: other.country_code,
            created_at: other.created_at,
            modified_at: other.modified_at,
            customer_id: other.customer_id,
            merchant_id: other.merchant_id,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        common_utils::fp_utils::when(self.id.is_some(), || {
            Err(ValidationError::InvalidValue {
                message: "id present while creating a new database entry".to_string(),
            })
        })?;
        Ok(Self::NewDstType {
            address_id: self.address_id,
            city: self.city,
            country: self.country,
            line1: self.line1,
            line2: self.line2,
            line3: self.line3,
            state: self.state,
            zip: self.zip,
            first_name: self.first_name,
            last_name: self.last_name,
            phone_number: self.phone_number,
            country_code: self.country_code,
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
        })
    }
}
