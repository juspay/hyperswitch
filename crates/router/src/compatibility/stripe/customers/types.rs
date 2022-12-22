use std::{convert::From, default::Default};

use common_utils::date_time;
use masking;
use serde::{Deserialize, Serialize};

use crate::{logger, pii, types::api};

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomerAddress {
    pub city: Option<pii::Secret<String>>,
    pub country: Option<pii::Secret<String>>,
    pub line1: Option<pii::Secret<String>>,
    pub line2: Option<pii::Secret<String>>,
    pub postal_code: Option<pii::Secret<String>>,
    pub state: Option<pii::Secret<String>>,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateCustomerRequest {
    pub email: Option<masking::Secret<String, pii::Email>>,
    pub invoice_prefix: Option<String>,
    pub name: Option<String>,
    pub phone: Option<masking::Secret<String>>,
    pub address: Option<CustomerAddress>,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomerUpdateRequest {
    pub metadata: Option<String>,
    pub description: Option<String>,
    pub email: Option<masking::Secret<String, pii::Email>>,
    pub phone: Option<masking::Secret<String, masking::WithType>>,
    pub name: Option<String>,
    pub address: Option<CustomerAddress>,
}

#[derive(Default, Serialize, PartialEq, Eq)]
pub struct CreateCustomerResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub description: Option<String>,
    pub email: Option<masking::Secret<String, pii::Email>>,
    pub metadata: Option<serde_json::Value>,
    pub name: Option<String>,
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
            description: req.invoice_prefix,
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
            metadata: req
                .metadata
                .map(|v| serde_json::from_str(&v).ok())
                .unwrap_or(None),
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
            email: cust.email,
            metadata: cust.metadata,
            name: cust.name,
            phone: cust.phone,
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
