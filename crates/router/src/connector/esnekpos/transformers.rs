use common_utils::{
    pii::{self, Email},
    types::StringMajorUnit,
};
use hyperswitch_connectors::utils::PhoneDetailsData;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    types::{self, domain, storage::enums},
};

pub struct EsnekposRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for EsnekposRouterData<T> {
    fn from((amount, router_data): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct EsnekposPaymentRequestConfig {
    merchant: String,
    merchant_key: String,
    back_url: String,
    prices_currency: String,
    order_ref_number: String,
    order_amount: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct EsnekposPaymentRequestCustomer {
    mail: Email,
    phone: String,
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    city: Option<String>,
    state: Option<Secret<String>>,
    address: Option<Secret<String>>,
    client_ip: Option<Secret<String, pii::IpAddress>>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct EsnekposPaymentRequestProduct {
    product_id: Option<String>,
    product_name: Option<String>,
    product_category: Option<String>,
    product_description: Option<String>,
    product_amount: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct EsnekposPaymentsRequest {
    config: EsnekposPaymentRequestConfig,
    credit_card: EsnekposCard,
    customer: EsnekposPaymentRequestCustomer,
    product: Vec<EsnekposPaymentRequestProduct>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct EsnekposCard {
    cc_number: cards::CardNumber,
    exp_month: Secret<String>,
    exp_year: Secret<String>,
    cc_cvc: Secret<String>,
    cc_owner: Secret<String>,
    installment_number: Secret<i32>,
    complete: bool,
}

impl TryFrom<&EsnekposRouterData<&types::PaymentsAuthorizeRouterData>> for EsnekposPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &EsnekposRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            domain::PaymentMethodData::Card(req_card) => {
                let card_holder_name: Secret<String> = item.router_data.get_billing_full_name()?;

                let credit_card = EsnekposCard {
                    cc_number: req_card.card_number,
                    exp_month: req_card.card_exp_month,
                    exp_year: req_card.card_exp_year,
                    cc_cvc: req_card.card_cvc,
                    cc_owner: card_holder_name,
                    // TODO(adnanjpg): get the installment number from the request
                    installment_number: Secret::new(0),
                    complete: item.router_data.request.is_auto_capture()?,
                };

                let req = item.router_data.request.clone();

                let mail = req.get_email()?;

                let phone_details = item.router_data.get_billing_phone()?;
                let phone = phone_details
                    .get_number_with_nullable_country_code()?
                    .expose();

                let customer = EsnekposPaymentRequestCustomer {
                    // pub struct Email(Secret<String, EmailStrategy>);
                    mail,
                    phone,
                    first_name: item.router_data.get_optional_billing_first_name(),
                    last_name: item.router_data.get_optional_billing_last_name(),
                    city: item.router_data.get_optional_billing_city(),
                    state: item.router_data.get_optional_billing_state(),
                    address: item.router_data.get_optional_line1_and_line2(),
                    client_ip: req.get_ip_address_as_optional().clone(),
                };

                let auth = EsnekposAuthType::try_from(&item.router_data.connector_auth_type)?;

                Ok(Self {
                    config: EsnekposPaymentRequestConfig {
                        merchant: auth.merchant.expose(),
                        merchant_key: auth.merchant_key.expose(),
                        back_url: item.router_data.request.get_return_url()?,
                        order_ref_number: item.router_data.payment_id.clone(),
                        prices_currency: item.router_data.request.currency.to_string(),
                        order_amount: item.amount.get_amount_as_string(),
                    },
                    credit_card,
                    customer,
                    product: vec![],
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

pub struct EsnekposAuthType {
    pub(super) merchant: Secret<String>,
    pub(super) merchant_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for EsnekposAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                merchant: key1.to_owned(),
                merchant_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EsnekposPaymentStatus {
    Succeeded,
    Error,
    #[default]
    FieldError,
    AuthenticationError,
    LimitError,
    CommissionError,
    InsertError,
    BrandError,
    PaymentError,
    Not3DAuthentication,
    BlockedError,
    OrderCancel,
    ProcessQuery,
    DirectPayment,
}

impl From<EsnekposPaymentStatus> for enums::AttemptStatus {
    fn from(item: EsnekposPaymentStatus) -> Self {
        match item {
            EsnekposPaymentStatus::Succeeded => Self::Charged,
            EsnekposPaymentStatus::Error => Self::Failure,
            EsnekposPaymentStatus::FieldError => Self::Failure,
            EsnekposPaymentStatus::AuthenticationError => Self::AuthenticationFailed,
            EsnekposPaymentStatus::LimitError => Self::Failure,
            EsnekposPaymentStatus::CommissionError => Self::Failure,
            EsnekposPaymentStatus::InsertError => Self::Failure,
            EsnekposPaymentStatus::BrandError => Self::Failure,
            EsnekposPaymentStatus::PaymentError => Self::Failure,
            EsnekposPaymentStatus::Not3DAuthentication => Self::AuthenticationFailed,
            EsnekposPaymentStatus::BlockedError => Self::Failure,
            EsnekposPaymentStatus::OrderCancel => Self::Failure,
            EsnekposPaymentStatus::ProcessQuery => Self::Failure,
            EsnekposPaymentStatus::DirectPayment => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct EsnekposPaymentResponse {
    pub order_ref_number: String,
    pub status: EsnekposPaymentStatus,
    pub return_code: String,
    pub return_message: String,
    pub return_message_tr: Option<String>,
    pub error_code: Option<String>,
    pub auth_hash: Option<String>,
    pub bank_auth_code: Option<String>,
    pub date: String,
    pub url_3ds: Option<String>,
    pub refno: String,
    pub hash: String,
    pub commission_rate: String,
    pub customer_name: String,
    pub customer_mail: String,
    pub customer_phone: String,
    pub customer_address: Option<String>,
    pub customer_cc_number: String,
    pub customer_cc_name: Option<String>,
    pub is_not_3d_payment: bool,
    pub virtual_pos_values: Option<String>,
    pub return_message_3d: Option<String>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, EsnekposPaymentResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, EsnekposPaymentResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.refno),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct EsnekposErrorResponse {
    pub return_code: String,
    pub return_message: String,
    pub status: Option<String>,
}
