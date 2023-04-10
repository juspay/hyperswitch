use async_trait::async_trait;
use common_utils::{
    crypto::{Encryptable, GcmAes256},
    date_time,
    errors::{CustomResult, ValidationError},
    ext_traits::AsyncExt,
};
use error_stack::ResultExt;
use masking::Secret;
use storage_models::{address::AddressUpdateInternal, encryption::Encryption, enums};
use time::{OffsetDateTime, PrimitiveDateTime};

use super::{behaviour, types::TypeEncryption};
use crate::db::StorageInterface;

#[derive(Clone, Debug, serde::Serialize)]
pub struct Address {
    #[serde(skip_serializing)]
    pub id: Option<i32>,
    #[serde(skip_serializing)]
    pub address_id: String,
    pub city: Option<String>,
    pub country: Option<enums::CountryCode>,
    pub line1: Option<Encryptable<Secret<String>>>,
    pub line2: Option<Encryptable<Secret<String>>>,
    pub line3: Option<Encryptable<Secret<String>>>,
    pub state: Option<Encryptable<Secret<String>>>,
    pub zip: Option<Encryptable<Secret<String>>>,
    pub first_name: Option<Encryptable<Secret<String>>>,
    pub last_name: Option<Encryptable<Secret<String>>>,
    pub phone_number: Option<Encryptable<Secret<String>>>,
    pub country_code: Option<String>,
    #[serde(skip_serializing)]
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(skip_serializing)]
    #[serde(with = "custom_serde::iso8601")]
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
            line1: self.line1.map(Encryption::from),
            line2: self.line2.map(Encryption::from),
            line3: self.line3.map(Encryption::from),
            state: self.state.map(Encryption::from),
            zip: self.zip.map(Encryption::from),
            first_name: self.first_name.map(Encryption::from),
            last_name: self.last_name.map(Encryption::from),
            phone_number: self.phone_number.map(Encryption::from),
            country_code: self.country_code,
            created_at: self.created_at,
            modified_at: self.modified_at,
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
        })
    }

    async fn convert_back(
        other: Self::DstType,
        _db: &dyn StorageInterface,
        _merchant_id: &str,
    ) -> CustomResult<Self, ValidationError> {
        let key = &[0];
        Ok(Self {
            id: Some(other.id),
            address_id: other.address_id,
            city: other.city,
            country: other.country,
            line1: other
                .line1
                .async_map(|inner| Encryptable::decrypt(inner, key, GcmAes256 {}))
                .await
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting".to_string(),
                })?,
            line2: other
                .line2
                .async_map(|inner| Encryptable::decrypt(inner, key, GcmAes256 {}))
                .await
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting".to_string(),
                })?,
            line3: other
                .line3
                .async_map(|inner| Encryptable::decrypt(inner, key, GcmAes256 {}))
                .await
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting".to_string(),
                })?,
            state: other
                .state
                .async_map(|inner| Encryptable::decrypt(inner, key, GcmAes256 {}))
                .await
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting".to_string(),
                })?,
            zip: other
                .zip
                .async_map(|inner| Encryptable::decrypt(inner, key, GcmAes256 {}))
                .await
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting".to_string(),
                })?,
            first_name: other
                .first_name
                .async_map(|inner| Encryptable::decrypt(inner, key, GcmAes256 {}))
                .await
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting".to_string(),
                })?,
            last_name: other
                .last_name
                .async_map(|inner| Encryptable::decrypt(inner, key, GcmAes256 {}))
                .await
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting".to_string(),
                })?,
            phone_number: other
                .phone_number
                .async_map(|inner| Encryptable::decrypt(inner, key, GcmAes256 {}))
                .await
                .transpose()
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting".to_string(),
                })?,
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
            line1: self.line1.map(Encryption::from),
            line2: self.line2.map(Encryption::from),
            line3: self.line3.map(Encryption::from),
            state: self.state.map(Encryption::from),
            zip: self.zip.map(Encryption::from),
            first_name: self.first_name.map(Encryption::from),
            last_name: self.last_name.map(Encryption::from),
            phone_number: self.phone_number.map(Encryption::from),
            country_code: self.country_code,
            customer_id: self.customer_id,
            merchant_id: self.merchant_id,
        })
    }
}

#[derive(Debug, frunk::LabelledGeneric)]
pub enum AddressUpdate {
    Update {
        city: Option<String>,
        country: Option<enums::CountryCode>,
        line1: Option<Encryptable<Secret<String>>>,
        line2: Option<Encryptable<Secret<String>>>,
        line3: Option<Encryptable<Secret<String>>>,
        state: Option<Encryptable<Secret<String>>>,
        zip: Option<Encryptable<Secret<String>>>,
        first_name: Option<Encryptable<Secret<String>>>,
        last_name: Option<Encryptable<Secret<String>>>,
        phone_number: Option<Encryptable<Secret<String>>>,
        country_code: Option<String>,
    },
}

impl From<AddressUpdate> for AddressUpdateInternal {
    fn from(address_update: AddressUpdate) -> Self {
        match address_update {
            AddressUpdate::Update {
                city,
                country,
                line1,
                line2,
                line3,
                state,
                zip,
                first_name,
                last_name,
                phone_number,
                country_code,
            } => Self {
                city,
                country,
                line1: line1.map(Encryption::from),
                line2: line2.map(Encryption::from),
                line3: line3.map(Encryption::from),
                state: state.map(Encryption::from),
                zip: zip.map(Encryption::from),
                first_name: first_name.map(Encryption::from),
                last_name: last_name.map(Encryption::from),
                phone_number: phone_number.map(Encryption::from),
                country_code,
                modified_at: date_time::convert_to_pdt(OffsetDateTime::now_utc()),
            },
        }
    }
}
