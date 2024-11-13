use async_trait::async_trait;
use common_utils::{
    crypto::{self, Encryptable},
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    id_type, pii, type_name,
    types::keymanager::{Identifier, KeyManagerState, ToEncryptable},
};
use diesel_models::{address::AddressUpdateInternal, enums};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret, SwitchStrategy};
use rustc_hash::FxHashMap;
use time::{OffsetDateTime, PrimitiveDateTime};

use super::{behaviour, types};

#[derive(Clone, Debug, serde::Serialize, router_derive::ToEncryption)]
pub struct Address {
    pub address_id: String,
    pub city: Option<String>,
    pub country: Option<enums::CountryAlpha2>,
    #[encrypt]
    pub line1: Option<Encryptable<Secret<String>>>,
    #[encrypt]
    pub line2: Option<Encryptable<Secret<String>>>,
    #[encrypt]
    pub line3: Option<Encryptable<Secret<String>>>,
    #[encrypt]
    pub state: Option<Encryptable<Secret<String>>>,
    #[encrypt]
    pub zip: Option<Encryptable<Secret<String>>>,
    #[encrypt]
    pub first_name: Option<Encryptable<Secret<String>>>,
    #[encrypt]
    pub last_name: Option<Encryptable<Secret<String>>>,
    #[encrypt]
    pub phone_number: Option<Encryptable<Secret<String>>>,
    pub country_code: Option<String>,
    #[serde(skip_serializing)]
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(skip_serializing)]
    #[serde(with = "custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub merchant_id: id_type::MerchantId,
    pub updated_by: String,
    #[encrypt]
    pub email: Option<Encryptable<Secret<String, pii::EmailStrategy>>>,
}

/// Based on the flow, appropriate address has to be used
/// In case of Payments, The `PaymentAddress`[PaymentAddress] has to be used
/// which contains only the `Address`[Address] object and `payment_id` and optional `customer_id`
#[derive(Debug, Clone)]
pub struct PaymentAddress {
    pub address: Address,
    pub payment_id: id_type::PaymentId,
    // This is present in `PaymentAddress` because even `payouts` uses `PaymentAddress`
    pub customer_id: Option<id_type::CustomerId>,
}

#[derive(Debug, Clone)]
pub struct CustomerAddress {
    pub address: Address,
    pub customer_id: id_type::CustomerId,
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
            ..converted_address
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError> {
        let customer_id =
            other
                .customer_id
                .clone()
                .ok_or(ValidationError::MissingRequiredField {
                    field_name: "customer_id".to_string(),
                })?;

        let address = Address::convert_back(state, other, key, key_manager_identifier).await?;

        Ok(Self {
            address,
            customer_id,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let address_new = Address::construct_new(self.address).await?;

        Ok(Self::NewDstType {
            customer_id: Some(self.customer_id),
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
            ..converted_address
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
        key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError> {
        let payment_id = other
            .payment_id
            .clone()
            .ok_or(ValidationError::MissingRequiredField {
                field_name: "payment_id".to_string(),
            })?;

        let customer_id = other.customer_id.clone();

        let address = Address::convert_back(state, other, key, key_manager_identifier).await?;

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
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        other: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError> {
        let identifier = Identifier::Merchant(other.merchant_id.clone());
        let decrypted: FxHashMap<String, Encryptable<Secret<String>>> = types::crypto_operation(
            state,
            type_name!(Self::DstType),
            types::CryptoOperation::BatchDecrypt(EncryptedAddress::to_encryptable(
                EncryptedAddress {
                    line1: other.line1,
                    line2: other.line2,
                    line3: other.line3,
                    state: other.state,
                    zip: other.zip,
                    first_name: other.first_name,
                    last_name: other.last_name,
                    phone_number: other.phone_number,
                    email: other.email,
                },
            )),
            identifier.clone(),
            key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(ValidationError::InvalidValue {
            message: "Failed while decrypting".to_string(),
        })?;
        let encryptable_address = EncryptedAddress::from_encryptable(decrypted).change_context(
            ValidationError::InvalidValue {
                message: "Failed while decrypting".to_string(),
            },
        )?;
        Ok(Self {
            address_id: other.address_id,
            city: other.city,
            country: other.country,
            line1: encryptable_address.line1,
            line2: encryptable_address.line2,
            line3: encryptable_address.line3,
            state: encryptable_address.state,
            zip: encryptable_address.zip,
            first_name: encryptable_address.first_name,
            last_name: encryptable_address.last_name,
            phone_number: encryptable_address.phone_number,
            country_code: other.country_code,
            created_at: other.created_at,
            modified_at: other.modified_at,
            updated_by: other.updated_by,
            merchant_id: other.merchant_id,
            email: encryptable_address.email.map(|email| {
                let encryptable: Encryptable<Secret<String, pii::EmailStrategy>> = Encryptable::new(
                    email.clone().into_inner().switch_strategy(),
                    email.into_encrypted(),
                );
                encryptable
            }),
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
