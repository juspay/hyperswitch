use common_utils::{
    pii::{self},
    types::StringMajorUnit,
};
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
    mail: String,
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
        let card_holder_name: Result<Secret<String>, errors::ConnectorError> =
            match item.router_data.get_optional_billing_full_name() {
                Some(name) => Ok(name),
                None => Err(errors::ConnectorError::MissingRequiredField {
                    field_name: "card_holder_name",
                }),
            };

        match card_holder_name {
            Ok(card_holder_name) => {
                match item.router_data.request.payment_method_data.clone() {
                    domain::PaymentMethodData::Card(req_card) => {
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

                        let billing_details = item.router_data.get_billing();

                        match billing_details.as_ref().map(|bd| *bd) {
                            Err(e) => {
                                router_env::logger::error!("Error: {:?}", e);
                                Err(errors::ConnectorError::MissingRequiredField {
                                    field_name: "billing_details",
                                }
                                .into())
                            }
                            Ok(bill) => {
                                let billing_details = bill.clone();

                                let req = item.router_data.request.clone();

                                let mailobj = req.get_email()?;
                                let mail = mailobj.expose().expose();

                                let phoneobj = billing_details.phone.clone();

                                let phone = match phoneobj {
                                    Some(phone) => {
                                        let number = phone.number;
                                        let country_code_with_ex = phone.country_code;

                                        match (number, country_code_with_ex) {
                                            (Some(number), Some(country_code)) => {
                                                // remove the + sign from the country code
                                                let country_code =
                                                    country_code.trim_start_matches('+');
                                                let number =
                                                    format!("{}{}", country_code, number.expose());

                                                number
                                            }
                                            (Some(number), None) => {
                                                format!("{:?}", number.expose())
                                            }
                                            (None, Some(_country_code)) => {
                                                return Err(
                                                    errors::ConnectorError::MissingRequiredField {
                                                        field_name: "phone",
                                                    }
                                                    .into(),
                                                )
                                            }
                                            (None, None) => {
                                                return Err(
                                                    errors::ConnectorError::MissingRequiredField {
                                                        field_name: "phone",
                                                    }
                                                    .into(),
                                                )
                                            }
                                        }
                                    }
                                    None => {
                                        return Err(errors::ConnectorError::MissingRequiredField {
                                            field_name: "phone",
                                        }
                                        .into())
                                    }
                                };

                                let address = billing_details.address.clone();

                                let customer_res: Result<
                                    EsnekposPaymentRequestCustomer,
                                    errors::ConnectorError,
                                > = Ok(EsnekposPaymentRequestCustomer {
                                    // pub struct Email(Secret<String, EmailStrategy>);
                                    mail,
                                    phone,
                                    first_name: match address.clone() {
                                        Some(address) => address.first_name,
                                        None => None,
                                    },
                                    last_name: match address.clone() {
                                        Some(address) => address.last_name,
                                        None => None,
                                    },
                                    city: match address.clone() {
                                        Some(address) => address.city,
                                        None => None,
                                    },
                                    state: match address.clone() {
                                        Some(address) => address.state,
                                        None => None,
                                    },
                                    address: match address.clone() {
                                        Some(address) => {
                                            let line1 = address.line1.clone();
                                            let line2 = address.line2.clone();

                                            match (line1, line2) {
                                                (Some(line1), Some(line2)) => {
                                                    let line1val = format!("{:?}", line1);
                                                    let line2val = format!("{:?}", line2);

                                                    let addstr =
                                                        format!("{}, {}", line1val, line2val);
                                                    Some(Secret::new(addstr))
                                                }
                                                (Some(line1), None) => Some(line1),
                                                (None, Some(line2)) => Some(line2),
                                                (None, None) => None,
                                            }
                                        }
                                        None => None,
                                    },
                                    client_ip: req.get_ip_address_as_optional().clone(),
                                });
                                let customer = match customer_res {
                                    Ok(customer) => customer,
                                    Err(e) => return Err(e.into()),
                                };

                                let auth = EsnekposAuthType::try_from(
                                    &item.router_data.connector_auth_type,
                                )?;

                                Ok(Self {
                                    config: EsnekposPaymentRequestConfig {
                                        merchant: auth.merchant.expose(),
                                        merchant_key: auth.merchant_key.expose(),
                                        back_url: item.router_data.request.get_return_url()?,
                                        order_ref_number: item.router_data.payment_id.clone(),
                                        prices_currency: item
                                            .router_data
                                            .request
                                            .currency
                                            .to_string(),
                                        order_amount: item.amount.get_amount_as_string(),
                                    },
                                    credit_card,
                                    customer,
                                    product: vec![],
                                })
                            }
                        }
                    }
                    _ => Err(
                        errors::ConnectorError::NotImplemented("Payment methods".to_string())
                            .into(),
                    ),
                }
            }
            Err(e) => Err(e.into()),
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
pub struct EsnekposPaymentsResponse {
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
    TryFrom<types::ResponseRouterData<F, EsnekposPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            EsnekposPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.refno),
                redirection_data: None,
                mandate_reference: None,
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
