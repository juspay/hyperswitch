use std::{convert::From, default::Default};

use api_models::{payment_methods as api_types, payments};
use common_utils::{
    crypto::Encryptable,
    date_time,
    pii::{self, Email},
};
use serde::{Deserialize, Serialize};

use crate::{
    logger,
    types::{api, api::enums as api_enums},
};

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
    pub description: Option<String>,
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
    pub description: Option<String>,
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

impl From<StripeAddressDetails> for payments::AddressDetails {
        /// Creates a new instance of Self (presumably a StripeAddressDetails) from the given StripeAddressDetails. 
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

impl From<CreateCustomerRequest> for api::CustomerRequest {
        /// Creates a new instance of a struct from the given CreateCustomerRequest. 
    /// It generates a customer ID using the generate_customer_id method from the api_models::customers module,
    /// and initializes the name, phone, email, description, metadata, and address fields with the values 
    /// from the provided request. If the address field in the request is not None, it converts the value 
    /// to the appropriate type and assigns it to the address field of the new instance. 
    /// Finally, it initializes any remaining fields with their default values.
    fn from(req: CreateCustomerRequest) -> Self {
        Self {
            customer_id: api_models::customers::generate_customer_id(),
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

impl From<CustomerUpdateRequest> for api::CustomerRequest {
        /// Converts a CustomerUpdateRequest into a new instance of Self by taking the values of the request and
        /// assigning them to the corresponding fields of the new instance. It also maps the address field of the
        /// request using the provided closure, and initializes any unspecified fields with their default values.
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

impl From<api::CustomerResponse> for CreateCustomerResponse {
        /// Converts a CustomerResponse object into a Customer object, handling any necessary conversions and error logging.
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
        /// Creates a new instance of Self by converting the provided api::CustomerDeleteResponse.
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
        /// Converts an API CustomerPaymentMethodsListResponse into a Self instance.
    /// 
    /// # Arguments
    /// 
    /// * `item` - The CustomerPaymentMethodsListResponse to convert
    /// 
    /// # Returns
    /// 
    /// A new instance of Self with the object set to "list" and the data populated with the converted customer payment methods.
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
        /// Converts a `CustomerPaymentMethod` into a `Self` instance, where `Self` is the type of the implementing struct. It initializes the `id` field with the `payment_token` from the input item, sets the `object` field to "payment_method", converts the `card` field using the `From` trait if it is present, and assigns the `created` field with the value from the input item.
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
        /// Constructs a new instance of the struct by converting a `CardDetailFromLocker` into `Self`.
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
