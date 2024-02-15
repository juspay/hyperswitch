use async_trait::async_trait;
use common_utils::{
    crypto, date_time,
    errors::{CustomResult, ValidationError},
};
use diesel_models::{address::AddressUpdateInternal, encryption::Encryption, enums};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use time::{OffsetDateTime, PrimitiveDateTime};

use super::{
    behaviour,
    types::{self, AsyncLift},
};

#[derive(Clone, Debug, serde::Serialize)]
pub struct Address {
    #[serde(skip_serializing)]
    pub id: Option<i32>,
    pub address_id: String,
    pub city: Option<String>,
    pub country: Option<enums::CountryAlpha2>,
    pub line1: crypto::OptionalEncryptableSecretString,
    pub line2: crypto::OptionalEncryptableSecretString,
    pub line3: crypto::OptionalEncryptableSecretString,
    pub state: crypto::OptionalEncryptableSecretString,
    pub zip: crypto::OptionalEncryptableSecretString,
    pub first_name: crypto::OptionalEncryptableSecretString,
    pub last_name: crypto::OptionalEncryptableSecretString,
    pub phone_number: crypto::OptionalEncryptableSecretString,
    pub country_code: Option<String>,
    #[serde(skip_serializing)]
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(skip_serializing)]
    #[serde(with = "custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub customer_id: Option<String>,
    pub merchant_id: String,
    pub payment_id: Option<String>,
    pub updated_by: String,
}

#[async_trait]
impl behaviour::Conversion for Address {
    type DstType = diesel_models::address::Address;
    type NewDstType = diesel_models::address::AddressNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::address::Address {
            id: self.id,
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
            payment_id: self.payment_id,
            updated_by: self.updated_by,
        })
    }

    async fn convert_back(
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Self, ValidationError> {
        async {
            let inner_decrypt = |inner| types::decrypt(inner, key.peek());
            Ok(Self {
                id: other.id,
                address_id: other.address_id,
                city: other.city,
                country: other.country,
                line1: other.line1.async_lift(inner_decrypt).await?,
                line2: other.line2.async_lift(inner_decrypt).await?,
                line3: other.line3.async_lift(inner_decrypt).await?,
                state: other.state.async_lift(inner_decrypt).await?,
                zip: other.zip.async_lift(inner_decrypt).await?,
                first_name: other.first_name.async_lift(inner_decrypt).await?,
                last_name: other.last_name.async_lift(inner_decrypt).await?,
                phone_number: other.phone_number.async_lift(inner_decrypt).await?,
                country_code: other.country_code,
                created_at: other.created_at,
                modified_at: other.modified_at,
                customer_id: other.customer_id,
                merchant_id: other.merchant_id,
                payment_id: other.payment_id,
                updated_by: other.updated_by,
            })
        }
        .await
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting".to_string(),
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let now = date_time::now();
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
            payment_id: self.payment_id,
            created_at: now,
            modified_at: now,
            updated_by: self.updated_by,
        })
    }
}

#[derive(Debug, Clone)]
pub enum AddressUpdate {
    Update {
        city: Option<String>,
        country: Option<enums::CountryAlpha2>,
        line1: crypto::OptionalEncryptableSecretString,
        line2: crypto::OptionalEncryptableSecretString,
        line3: crypto::OptionalEncryptableSecretString,
        state: crypto::OptionalEncryptableSecretString,
        zip: crypto::OptionalEncryptableSecretString,
        first_name: crypto::OptionalEncryptableSecretString,
        last_name: crypto::OptionalEncryptableSecretString,
        phone_number: crypto::OptionalEncryptableSecretString,
        country_code: Option<String>,
        updated_by: String,
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
                updated_by,
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
                updated_by,
            },
        }
    }
}
