use std::{convert::From, default::Default};

use api_models::payment_methods as api_types;
use common_utils::{
    crypto::Encryptable,
    date_time,
    pii::{self, Email},
};
use serde::{Deserialize, Serialize};

use crate::{logger, types::api};

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateCustomerRequest {
    pub email: Option<Email>,
    pub invoice_prefix: Option<String>,
    pub name: Option<masking::Secret<String>>,
    pub phone: Option<masking::Secret<String>>,
    pub address: Option<masking::Secret<serde_json::Value>>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub description: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomerUpdateRequest {
    pub description: Option<String>,
    pub email: Option<Email>,
    pub phone: Option<masking::Secret<String, masking::WithType>>,
    pub name: Option<masking::Secret<String>>,
    pub address: Option<masking::Secret<serde_json::Value>>,
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Default, Serialize, PartialEq, Eq)]
pub struct CreateCustomerResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub description: Option<String>,
    pub email: Option<Email>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub name: Option<masking::Secret<String>>,
    pub phone: Option<masking::Secret<String, masking::WithType>>,
}

pub type CustomerRetrieveResponse = CreateCustomerResponse;
pub type CustomerUpdateResponse = CreateCustomerResponse;

#[derive(Default, Serialize, PartialEq, Eq)]
pub struct CustomerDeleteResponse {
    pub id: String,
    pub deleted: bool,
}

impl From<CreateCustomerRequest> for api::CustomerRequest {
    fn from(req: CreateCustomerRequest) -> Self {
        Self {
            customer_id: api_models::customers::generate_customer_id(),
            name: req.name,
            phone: req.phone,
            email: req.email,
            description: req.description,
            metadata: req.metadata,
            address: req.address,
            ..Default::default()
        }
    }
}

impl From<CustomerUpdateRequest> for api::CustomerRequest {
    fn from(req: CustomerUpdateRequest) -> Self {
        Self {
            name: req.name,
            phone: req.phone,
            email: req.email,
            description: req.description,
            metadata: req.metadata,
            ..Default::default()
        }
    }
}

impl From<api::CustomerResponse> for CreateCustomerResponse {
    fn from(cust: api::CustomerResponse) -> Self {
        let cust = cust.into_inner();
        Self {
            id: cust.customer_id,
            object: "customer".to_owned(),
            created: u64::try_from(cust.created_at.assume_utc().unix_timestamp()).unwrap_or_else(
                |error| {
                    logger::error!(
                        %error,
                        "incorrect value for `customer.created_at` provided {}", cust.created_at
                    );
                    // Current timestamp converted to Unix timestamp should have a positive value
                    // for many years to come
                    u64::try_from(date_time::now().assume_utc().unix_timestamp())
                        .unwrap_or_default()
                },
            ),
            description: cust.description,
            email: cust.email.map(|inner| inner.into()),
            metadata: cust.metadata,
            name: cust.name.map(Encryptable::into_inner),
            phone: cust.phone.map(Encryptable::into_inner),
        }
    }
}

impl From<api::CustomerDeleteResponse> for CustomerDeleteResponse {
    fn from(cust: api::CustomerDeleteResponse) -> Self {
        Self {
            id: cust.customer_id,
            deleted: cust.customer_deleted,
        }
    }
}

#[derive(Default, Serialize, PartialEq, Eq)]
pub struct CustomerPaymentMethodListResponse {
    pub object: &'static str,
    pub data: Vec<PaymentMethodData>,
}

#[derive(Default, Serialize, PartialEq, Eq)]
pub struct PaymentMethodData {
    pub id: String,
    pub object: &'static str,
    pub card: Option<CardDetails>,
    pub created: Option<time::PrimitiveDateTime>,
}

#[derive(Default, Serialize, PartialEq, Eq)]
pub struct CardDetails {
    pub country: Option<String>,
    pub last4: Option<String>,
    pub exp_month: Option<masking::Secret<String>>,
    pub exp_year: Option<masking::Secret<String>>,
    pub fingerprint: Option<masking::Secret<String>>,
}

impl From<api::CustomerPaymentMethodsListResponse> for CustomerPaymentMethodListResponse {
    fn from(item: api::CustomerPaymentMethodsListResponse) -> Self {
        let customer_payment_methods = item.customer_payment_methods;
        let data = customer_payment_methods
            .into_iter()
            .map(From::from)
            .collect();
        Self {
            object: "list",
            data,
        }
    }
}

impl From<api_types::CustomerPaymentMethod> for PaymentMethodData {
    fn from(item: api_types::CustomerPaymentMethod) -> Self {
        Self {
            id: item.payment_token,
            object: "payment_method",
            card: item.card.map(From::from),
            created: item.created,
        }
    }
}

impl From<api_types::CardDetailFromLocker> for CardDetails {
    fn from(item: api_types::CardDetailFromLocker) -> Self {
        Self {
            country: item.issuer_country,
            last4: item.last4_digits,
            exp_month: item.expiry_month,
            exp_year: item.expiry_year,
            fingerprint: item.card_fingerprint,
        }
    }
}
