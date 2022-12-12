use std::{convert::From, default::Default};

use masking::{Secret, WithType};
use serde::{Deserialize, Serialize};

use crate::{pii::Email, types::api};

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
            created: cust.created_at.assume_utc().unix_timestamp() as u64,
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
