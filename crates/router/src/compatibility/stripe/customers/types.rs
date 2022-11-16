use std::{convert::From, default::Default};

use masking::{Secret, WithType};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    pii::Email,
    types::{api::customers, storage},
};

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CustomerAddress {
    pub(crate) city: Option<String>,
    pub(crate) country: Option<String>,
    pub(crate) line1: Option<String>,
    pub(crate) line2: Option<String>,
    pub(crate) postal_code: Option<String>,
    pub(crate) state: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CreateCustomerRequest {
    pub(crate) email: Option<Secret<String, Email>>,
    pub(crate) invoice_prefix: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) phone: Option<Secret<String, WithType>>,
    pub(crate) address: Option<CustomerAddress>,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CustomerUpdateRequest {
    pub(crate) metadata: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) email: Option<Secret<String, Email>>,
    pub(crate) phone: Option<Secret<String, WithType>>,
    pub(crate) name: Option<String>,
    pub(crate) address: Option<CustomerAddress>,
}

#[derive(Default, Serialize, PartialEq, Eq)]
pub(crate) struct CreateCustomerResponse {
    id: String,
    object: String,
    address: Option<Secret<serde_json::Value>>,
    created: u64,
    description: Option<String>,
    email: Option<Secret<String, Email>>,
    metadata: Option<serde_json::Value>,
    name: Option<String>,
    phone: Option<Secret<String, WithType>>,
}

pub(crate) type CustomerRetrieveResponse = CreateCustomerResponse;
pub(crate) type CustomerUpdateResponse = CreateCustomerResponse;

#[derive(Default, Serialize, PartialEq, Eq)]
pub(crate) struct CustomerDeleteResponse {
    pub(crate) id: String,
    pub(crate) deleted: bool,
}

impl From<CreateCustomerRequest> for customers::CreateCustomerRequest {
    fn from(req: CreateCustomerRequest) -> Self {
        Self {
            customer_id: storage::generate_customer_id(),
            name: req.name,
            phone: req.phone,
            email: req.email,
            description: req.invoice_prefix,
            address: req.address.map(|addr| {
                Secret::new(json!({
                    "city": addr.city,
                    "country": addr.country,
                    "line1": addr.line1,
                    "line2": addr.line2,
                    "postal_code": addr.postal_code,
                    "state": addr.state
                }))
            }),
            ..Default::default()
        }
    }
}

impl From<CustomerUpdateRequest> for customers::CustomerUpdateRequest {
    fn from(req: CustomerUpdateRequest) -> Self {
        Self {
            name: req.name,
            phone: req.phone,
            email: req.email,
            description: req.description,
            address: req.address.map(|addr| {
                Secret::new(json!({
                    "city": addr.city,
                    "country": addr.country,
                    "line1": addr.line1,
                    "line2": addr.line2,
                    "postal_code": addr.postal_code,
                    "state": addr.state
                }))
            }),

            metadata: req
                .metadata
                .map(|v| serde_json::from_str(&v).ok())
                .unwrap_or(None),
            ..Default::default()
        }
    }
}

impl From<customers::CustomerResponse> for CreateCustomerResponse {
    fn from(cust: customers::CustomerResponse) -> Self {
        Self {
            id: cust.customer_id,
            object: "customer".to_owned(),
            address: cust.address,
            created: cust.created_at.assume_utc().unix_timestamp() as u64,
            description: cust.description,
            email: cust.email,
            metadata: cust.metadata,
            name: cust.name,
            phone: cust.phone,
        }
    }
}

impl From<customers::CustomerDeleteResponse> for CustomerDeleteResponse {
    fn from(cust: customers::CustomerDeleteResponse) -> Self {
        Self {
            id: cust.customer_id,
            deleted: cust.deleted,
        }
    }
}
