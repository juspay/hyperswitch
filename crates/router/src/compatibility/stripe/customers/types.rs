use std::{convert::From, default::Default};

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use api_models::payment_methods as api_types;
use api_models::payments;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use common_utils::{crypto::Encryptable, date_time};
use common_utils::{
    id_type,
    pii::{self, Email},
    types::Description,
};
use serde::{Deserialize, Serialize};

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use crate::logger;
use crate::types::{api, api::enums as api_enums};

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub struct Shipping {
    pub address: StripeAddressDetails,
    pub name: Option<masking::Secret<String>>,
    pub carrier: Option<String>,
    pub phone: Option<masking::Secret<String>>,
    pub tracking_number: Option<masking::Secret<String>>,
}

#[derive(Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
pub struct StripeAddressDetails {
    pub city: Option<String>,
    pub country: Option<api_enums::CountryAlpha2>,
    pub line1: Option<masking::Secret<String>>,
    pub line2: Option<masking::Secret<String>>,
    pub postal_code: Option<masking::Secret<String>>,
    pub state: Option<masking::Secret<String>>,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateCustomerRequest {
    pub email: Option<Email>,
    pub invoice_prefix: Option<String>,
    pub name: Option<masking::Secret<String>>,
    pub phone: Option<masking::Secret<String>>,
    pub address: Option<StripeAddressDetails>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub description: Option<Description>,
    pub shipping: Option<Shipping>,
    pub payment_method: Option<String>,              // not used
    pub balance: Option<i64>,                        // not used
    pub cash_balance: Option<pii::SecretSerdeValue>, // not used
    pub coupon: Option<String>,                      // not used
    pub invoice_settings: Option<pii::SecretSerdeValue>, // not used
    pub next_invoice_sequence: Option<String>,       // not used
    pub preferred_locales: Option<String>,           // not used
    pub promotion_code: Option<String>,              // not used
    pub source: Option<String>,                      // not used
    pub tax: Option<pii::SecretSerdeValue>,          // not used
    pub tax_exempt: Option<String>,                  // not used
    pub tax_id_data: Option<String>,                 // not used
    pub test_clock: Option<String>,                  // not used
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomerUpdateRequest {
    pub description: Option<Description>,
    pub email: Option<Email>,
    pub phone: Option<masking::Secret<String, masking::WithType>>,
    pub name: Option<masking::Secret<String>>,
    pub address: Option<StripeAddressDetails>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub shipping: Option<Shipping>,
    pub payment_method: Option<String>,              // not used
    pub balance: Option<i64>,                        // not used
    pub cash_balance: Option<pii::SecretSerdeValue>, // not used
    pub coupon: Option<String>,                      // not used
    pub default_source: Option<String>,              // not used
    pub invoice_settings: Option<pii::SecretSerdeValue>, // not used
    pub next_invoice_sequence: Option<String>,       // not used
    pub preferred_locales: Option<String>,           // not used
    pub promotion_code: Option<String>,              // not used
    pub source: Option<String>,                      // not used
    pub tax: Option<pii::SecretSerdeValue>,          // not used
    pub tax_exempt: Option<String>,                  // not used
}

#[derive(Serialize, PartialEq, Eq)]
pub struct CreateCustomerResponse {
    pub id: id_type::CustomerId,
    pub object: String,
    pub created: u64,
    pub description: Option<Description>,
    pub email: Option<Email>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub name: Option<masking::Secret<String>>,
    pub phone: Option<masking::Secret<String, masking::WithType>>,
}

pub type CustomerRetrieveResponse = CreateCustomerResponse;
pub type CustomerUpdateResponse = CreateCustomerResponse;

#[derive(Serialize, PartialEq, Eq)]
pub struct CustomerDeleteResponse {
    pub id: id_type::CustomerId,
    pub deleted: bool,
}

impl From<StripeAddressDetails> for payments::AddressDetails {
    fn from(address: StripeAddressDetails) -> Self {
        Self {
            city: address.city,
            country: address.country,
            line1: address.line1,
            line2: address.line2,
            zip: address.postal_code,
            state: address.state,
            first_name: None,
            line3: None,
            last_name: None,
        }
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl From<CreateCustomerRequest> for api::CustomerRequest {
    fn from(req: CreateCustomerRequest) -> Self {
        Self {
            customer_id: Some(common_utils::generate_customer_id_of_default_length()),
            name: req.name,
            phone: req.phone,
            email: req.email,
            description: req.description,
            metadata: req.metadata,
            address: req.address.map(|s| s.into()),
            ..Default::default()
        }
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl From<CustomerUpdateRequest> for api::CustomerUpdateRequest {
    fn from(req: CustomerUpdateRequest) -> Self {
        Self {
            name: req.name,
            phone: req.phone,
            email: req.email,
            description: req.description,
            metadata: req.metadata,
            address: req.address.map(|s| s.into()),
            ..Default::default()
        }
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
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

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
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
    pub id: Option<String>,
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

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
impl From<api_types::CustomerPaymentMethod> for PaymentMethodData {
    fn from(item: api_types::CustomerPaymentMethod) -> Self {
        let card = item.card.map(From::from);
        Self {
            id: Some(item.payment_token),
            object: "payment_method",
            card,
            created: item.created,
        }
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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
