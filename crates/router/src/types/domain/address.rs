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
    pub merchant_id: String,
    pub updated_by: String,
    pub email: crypto::OptionalEncryptableEmail,
}

/// Based on the flow, appropriate address has to be used
/// In case of Payments, The `PaymentAddress`[PaymentAddress] has to be used
/// which contains only the `Address`[Address] object and `payment_id` and optional `customer_id`
#[derive(Debug, Clone)]
pub struct PaymentAddress {
    pub address: Address,
    pub payment_id: String,
    // This is present in `PaymentAddress` because even `payouts` uses `PaymentAddress`
    pub customer_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CustomerAddress {
    pub address: Address,
    pub customer_id: String,
}

#[derive(Debug, Clone)]
pub struct PaymentMethodAddress {
    pub address: Address,
    pub payment_method_id: String,
}

#[async_trait]
impl behaviour::Conversion for CustomerAddress {
    type DstType = diesel_models::address::Address;
    type NewDstType = diesel_models::address::AddressNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let converted_address = Address::convert(self.address).await?;
        Ok(diesel_models::address::Address {
            customer_id: Some(self.customer_id),
            payment_id: None,
            payment_method_id: None,
            ..converted_address
        })
    }

    async fn convert_back(
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Self, ValidationError> {
        let customer_id =
            other
                .customer_id
                .clone()
                .ok_or(ValidationError::MissingRequiredField {
                    field_name: "cutomer_id".to_string(),
                })?;

        let address = Address::convert_back(other, key).await?;

        Ok(Self {
            address,
            customer_id,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let address_new = Address::construct_new(self.address).await?;

        Ok(Self::NewDstType {
            customer_id: Some(self.customer_id),
            payment_method_id: None,
            payment_id: None,
            ..address_new
        })
    }
}

#[async_trait]
impl behaviour::Conversion for PaymentAddress {
    type DstType = diesel_models::address::Address;
    type NewDstType = diesel_models::address::AddressNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let converted_address = Address::convert(self.address).await?;
        Ok(diesel_models::address::Address {
            customer_id: self.customer_id,
            payment_id: Some(self.payment_id),
            payment_method_id: None,
            ..converted_address
        })
    }

    async fn convert_back(
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Self, ValidationError> {
        let payment_id = other
            .payment_id
            .clone()
            .ok_or(ValidationError::MissingRequiredField {
                field_name: "payment_id".to_string(),
            })?;

        let customer_id = other.customer_id.clone();

        let address = Address::convert_back(other, key).await?;

        Ok(Self {
            address,
            payment_id,
            customer_id,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let address_new = Address::construct_new(self.address).await?;

        Ok(Self::NewDstType {
            customer_id: self.customer_id,
            payment_id: Some(self.payment_id),
            payment_method_id: None,
            ..address_new
        })
    }
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
            merchant_id: self.merchant_id,
            updated_by: self.updated_by,
            email: self.email.map(Encryption::from),
            payment_id: None,
            customer_id: None,
            payment_method_id: None,
        })
    }

    async fn convert_back(
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Self, ValidationError> {
        async {
            let inner_decrypt = |inner| types::decrypt(inner, key.peek());
            let inner_decrypt_email = |inner| types::decrypt(inner, key.peek());
            Ok::<Self, error_stack::Report<common_utils::errors::CryptoError>>(Self {
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
                updated_by: other.updated_by,
                merchant_id: other.merchant_id,
                email: other.email.async_lift(inner_decrypt_email).await?,
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
            merchant_id: self.merchant_id,
            created_at: now,
            modified_at: now,
            updated_by: self.updated_by,
            email: self.email.map(Encryption::from),
            customer_id: None,
            payment_id: None,
            payment_method_id: None,
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
        email: crypto::OptionalEncryptableEmail,
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
                email,
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
                email: email.map(Encryption::from),
            },
        }
    }
}
