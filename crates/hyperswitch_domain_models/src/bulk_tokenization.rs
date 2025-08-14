use api_models::{payment_methods as payment_methods_api, payments as payments_api};
use cards::CardNumber;
use common_enums as enums;
use common_utils::{
    errors,
    ext_traits::OptionExt,
    id_type, pii,
    transformers::{ForeignFrom, ForeignTryFrom},
};
use error_stack::report;

use crate::{
    address::{Address, AddressDetails, PhoneDetails},
    router_request_types::CustomerDetails,
};

#[derive(Debug)]
pub struct CardNetworkTokenizeRequest {
    pub data: TokenizeDataRequest,
    pub customer: CustomerDetails,
    pub billing: Option<Address>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub payment_method_issuer: Option<String>,
}

#[derive(Debug)]
pub enum TokenizeDataRequest {
    Card(TokenizeCardRequest),
    ExistingPaymentMethod(TokenizePaymentMethodRequest),
}

#[derive(Clone, Debug)]
pub struct TokenizeCardRequest {
    pub raw_card_number: CardNumber,
    pub card_expiry_month: masking::Secret<String>,
    pub card_expiry_year: masking::Secret<String>,
    pub card_cvc: Option<masking::Secret<String>>,
    pub card_holder_name: Option<masking::Secret<String>>,
    pub nick_name: Option<masking::Secret<String>>,
    pub card_issuing_country: Option<String>,
    pub card_network: Option<enums::CardNetwork>,
    pub card_issuer: Option<String>,
    pub card_type: Option<payment_methods_api::CardType>,
}

#[derive(Clone, Debug)]
pub struct TokenizePaymentMethodRequest {
    pub payment_method_id: String,
    pub card_cvc: Option<masking::Secret<String>>,
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
pub struct CardNetworkTokenizeRecord {
    // Card details
    pub raw_card_number: Option<CardNumber>,
    pub card_expiry_month: Option<masking::Secret<String>>,
    pub card_expiry_year: Option<masking::Secret<String>>,
    pub card_cvc: Option<masking::Secret<String>>,
    pub card_holder_name: Option<masking::Secret<String>>,
    pub nick_name: Option<masking::Secret<String>>,
    pub card_issuing_country: Option<String>,
    pub card_network: Option<enums::CardNetwork>,
    pub card_issuer: Option<String>,
    pub card_type: Option<payment_methods_api::CardType>,

    // Payment method details
    pub payment_method_id: Option<String>,
    pub payment_method_type: Option<payment_methods_api::CardType>,
    pub payment_method_issuer: Option<String>,

    // Customer details
    pub customer_id: id_type::CustomerId,
    #[serde(rename = "name")]
    pub customer_name: Option<masking::Secret<String>>,
    #[serde(rename = "email")]
    pub customer_email: Option<pii::Email>,
    #[serde(rename = "phone")]
    pub customer_phone: Option<masking::Secret<String>>,
    #[serde(rename = "phone_country_code")]
    pub customer_phone_country_code: Option<String>,
    #[serde(rename = "tax_registration_id")]
    pub customer_tax_registration_id: Option<masking::Secret<String>>,
    // Billing details
    pub billing_address_city: Option<String>,
    pub billing_address_country: Option<enums::CountryAlpha2>,
    pub billing_address_line1: Option<masking::Secret<String>>,
    pub billing_address_line2: Option<masking::Secret<String>>,
    pub billing_address_line3: Option<masking::Secret<String>>,
    pub billing_address_zip: Option<masking::Secret<String>>,
    pub billing_address_state: Option<masking::Secret<String>>,
    pub billing_address_first_name: Option<masking::Secret<String>>,
    pub billing_address_last_name: Option<masking::Secret<String>>,
    pub billing_phone_number: Option<masking::Secret<String>>,
    pub billing_phone_country_code: Option<String>,
    pub billing_email: Option<pii::Email>,

    // Other details
    pub line_number: Option<u64>,
    pub merchant_id: Option<id_type::MerchantId>,
}

impl ForeignFrom<&CardNetworkTokenizeRecord> for payments_api::CustomerDetails {
    fn foreign_from(record: &CardNetworkTokenizeRecord) -> Self {
        Self {
            id: record.customer_id.clone(),
            name: record.customer_name.clone(),
            email: record.customer_email.clone(),
            phone: record.customer_phone.clone(),
            phone_country_code: record.customer_phone_country_code.clone(),
            tax_registration_id: record.customer_tax_registration_id.clone(),
        }
    }
}

impl ForeignFrom<&CardNetworkTokenizeRecord> for payments_api::Address {
    fn foreign_from(record: &CardNetworkTokenizeRecord) -> Self {
        Self {
            address: Some(payments_api::AddressDetails {
                first_name: record.billing_address_first_name.clone(),
                last_name: record.billing_address_last_name.clone(),
                line1: record.billing_address_line1.clone(),
                line2: record.billing_address_line2.clone(),
                line3: record.billing_address_line3.clone(),
                city: record.billing_address_city.clone(),
                zip: record.billing_address_zip.clone(),
                state: record.billing_address_state.clone(),
                country: record.billing_address_country,
                origin_zip: None,
            }),
            phone: Some(payments_api::PhoneDetails {
                number: record.billing_phone_number.clone(),
                country_code: record.billing_phone_country_code.clone(),
            }),
            email: record.billing_email.clone(),
        }
    }
}

impl ForeignTryFrom<CardNetworkTokenizeRecord> for payment_methods_api::CardNetworkTokenizeRequest {
    type Error = error_stack::Report<errors::ValidationError>;
    fn foreign_try_from(record: CardNetworkTokenizeRecord) -> Result<Self, Self::Error> {
        let billing = Some(payments_api::Address::foreign_from(&record));
        let customer = payments_api::CustomerDetails::foreign_from(&record);
        let merchant_id = record.merchant_id.get_required_value("merchant_id")?;

        match (
            record.raw_card_number,
            record.card_expiry_month,
            record.card_expiry_year,
            record.payment_method_id,
        ) {
            (Some(raw_card_number), Some(card_expiry_month), Some(card_expiry_year), None) => {
                Ok(Self {
                    merchant_id,
                    data: payment_methods_api::TokenizeDataRequest::Card(
                        payment_methods_api::TokenizeCardRequest {
                            raw_card_number,
                            card_expiry_month,
                            card_expiry_year,
                            card_cvc: record.card_cvc,
                            card_holder_name: record.card_holder_name,
                            nick_name: record.nick_name,
                            card_issuing_country: record.card_issuing_country,
                            card_network: record.card_network,
                            card_issuer: record.card_issuer,
                            card_type: record.card_type.clone(),
                        },
                    ),
                    billing,
                    customer,
                    metadata: None,
                    payment_method_issuer: record.payment_method_issuer,
                })
            }
            (None, None, None, Some(payment_method_id)) => Ok(Self {
                merchant_id,
                data: payment_methods_api::TokenizeDataRequest::ExistingPaymentMethod(
                    payment_methods_api::TokenizePaymentMethodRequest {
                        payment_method_id,
                        card_cvc: record.card_cvc,
                    },
                ),
                billing,
                customer,
                metadata: None,
                payment_method_issuer: record.payment_method_issuer,
            }),
            _ => Err(report!(errors::ValidationError::InvalidValue {
                message: "Invalid record in bulk tokenization - expected one of card details or payment method details".to_string()
            })),
        }
    }
}

impl ForeignFrom<&TokenizeCardRequest> for payment_methods_api::MigrateCardDetail {
    fn foreign_from(card: &TokenizeCardRequest) -> Self {
        Self {
            card_number: masking::Secret::new(card.raw_card_number.get_card_no()),
            card_exp_month: card.card_expiry_month.clone(),
            card_exp_year: card.card_expiry_year.clone(),
            card_holder_name: card.card_holder_name.clone(),
            nick_name: card.nick_name.clone(),
            card_issuing_country: card.card_issuing_country.clone(),
            card_network: card.card_network.clone(),
            card_issuer: card.card_issuer.clone(),
            card_type: card
                .card_type
                .as_ref()
                .map(|card_type| card_type.to_string()),
        }
    }
}

impl ForeignTryFrom<CustomerDetails> for payments_api::CustomerDetails {
    type Error = error_stack::Report<errors::ValidationError>;
    fn foreign_try_from(customer: CustomerDetails) -> Result<Self, Self::Error> {
        Ok(Self {
            id: customer.customer_id.get_required_value("customer_id")?,
            name: customer.name,
            email: customer.email,
            phone: customer.phone,
            phone_country_code: customer.phone_country_code,
            tax_registration_id: customer.tax_registration_id,
        })
    }
}

impl ForeignFrom<payment_methods_api::CardNetworkTokenizeRequest> for CardNetworkTokenizeRequest {
    fn foreign_from(req: payment_methods_api::CardNetworkTokenizeRequest) -> Self {
        Self {
            data: TokenizeDataRequest::foreign_from(req.data),
            customer: CustomerDetails::foreign_from(req.customer),
            billing: req.billing.map(ForeignFrom::foreign_from),
            metadata: req.metadata,
            payment_method_issuer: req.payment_method_issuer,
        }
    }
}

impl ForeignFrom<payment_methods_api::TokenizeDataRequest> for TokenizeDataRequest {
    fn foreign_from(req: payment_methods_api::TokenizeDataRequest) -> Self {
        match req {
            payment_methods_api::TokenizeDataRequest::Card(card) => {
                Self::Card(TokenizeCardRequest {
                    raw_card_number: card.raw_card_number,
                    card_expiry_month: card.card_expiry_month,
                    card_expiry_year: card.card_expiry_year,
                    card_cvc: card.card_cvc,
                    card_holder_name: card.card_holder_name,
                    nick_name: card.nick_name,
                    card_issuing_country: card.card_issuing_country,
                    card_network: card.card_network,
                    card_issuer: card.card_issuer,
                    card_type: card.card_type,
                })
            }
            payment_methods_api::TokenizeDataRequest::ExistingPaymentMethod(pm) => {
                Self::ExistingPaymentMethod(TokenizePaymentMethodRequest {
                    payment_method_id: pm.payment_method_id,
                    card_cvc: pm.card_cvc,
                })
            }
        }
    }
}

impl ForeignFrom<payments_api::CustomerDetails> for CustomerDetails {
    fn foreign_from(req: payments_api::CustomerDetails) -> Self {
        Self {
            customer_id: Some(req.id),
            name: req.name,
            email: req.email,
            phone: req.phone,
            phone_country_code: req.phone_country_code,
            tax_registration_id: req.tax_registration_id,
        }
    }
}

impl ForeignFrom<payments_api::Address> for Address {
    fn foreign_from(req: payments_api::Address) -> Self {
        Self {
            address: req.address.map(ForeignFrom::foreign_from),
            phone: req.phone.map(ForeignFrom::foreign_from),
            email: req.email,
        }
    }
}

impl ForeignFrom<payments_api::AddressDetails> for AddressDetails {
    fn foreign_from(req: payments_api::AddressDetails) -> Self {
        Self {
            city: req.city,
            country: req.country,
            line1: req.line1,
            line2: req.line2,
            line3: req.line3,
            zip: req.zip,
            state: req.state,
            first_name: req.first_name,
            last_name: req.last_name,
            origin_zip: req.origin_zip,
        }
    }
}

impl ForeignFrom<payments_api::PhoneDetails> for PhoneDetails {
    fn foreign_from(req: payments_api::PhoneDetails) -> Self {
        Self {
            number: req.number,
            country_code: req.country_code,
        }
    }
}
