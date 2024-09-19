use common_utils::pii::{self};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    types::{self, api, domain, storage::enums},
};

pub struct EsnekposRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, i64, T)> for EsnekposRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (&api::CurrencyUnit, enums::Currency, i64, T),
    ) -> Result<Self, Self::Error> {
        let amount: String = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
struct EsnekposPaymentRequestConfig {
    #[serde(rename = "MERCHANT")]
    merchant: String,
    #[serde(rename = "MERCHANT_KEY")]
    merchant_key: String,
    #[serde(rename = "BACK_URL")]
    back_url: String,
    #[serde(rename = "PRICES_CURRENCY")]
    prices_currency: String,
    #[serde(rename = "ORDER_REF_NUMBER")]
    order_ref_number: String,
    #[serde(rename = "ORDER_AMOUNT")]
    order_amount: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
struct EsnekposPaymentRequestCustomer {
    #[serde(rename = "MAIL")]
    mail: String,
    #[serde(rename = "PHONE")]
    phone: String,
    #[serde(rename = "FIRST_NAME")]
    first_name: Option<Secret<String>>,
    #[serde(rename = "LAST_NAME")]
    last_name: Option<Secret<String>>,
    #[serde(rename = "CITY")]
    city: Option<String>,
    #[serde(rename = "STATE")]
    state: Option<Secret<String>>,
    #[serde(rename = "ADDRESS")]
    address: Option<Secret<String>>,
    #[serde(rename = "CLIENT_IP")]
    client_ip: Option<Secret<String, pii::IpAddress>>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
struct EsnekposPaymentRequestProduct {
    #[serde(rename = "PRODUCT_ID")]
    product_id: Option<String>,
    #[serde(rename = "PRODUCT_NAME")]
    product_name: Option<String>,
    #[serde(rename = "PRODUCT_CATEGORY")]
    product_category: Option<String>,
    #[serde(rename = "PRODUCT_DESCRIPTION")]
    product_description: Option<String>,
    #[serde(rename = "PRODUCT_AMOUNT")]
    product_amount: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct EsnekposPaymentsRequest {
    #[serde(rename = "Config")]
    config: EsnekposPaymentRequestConfig,
    #[serde(rename = "CreditCard")]
    card: EsnekposCard,
    #[serde(rename = "Customer")]
    customer: EsnekposPaymentRequestCustomer,
    #[serde(rename = "Product")]
    product: Vec<EsnekposPaymentRequestProduct>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct EsnekposCard {
    #[serde(rename = "CC_NUMBER")]
    number: cards::CardNumber,
    #[serde(rename = "EXP_MONTH")]
    expiry_month: Secret<String>,
    #[serde(rename = "EXP_YEAR")]
    expiry_year: Secret<String>,
    #[serde(rename = "CC_CVV")]
    cvc: Secret<String>,
    #[serde(rename = "CC_OWNER")]
    holder_name: Secret<String>,
    #[serde(rename = "INSTALLMENT_NUMBER")]
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
                        let card = EsnekposCard {
                            number: req_card.card_number,
                            expiry_month: req_card.card_exp_month,
                            expiry_year: req_card.card_exp_year,
                            cvc: req_card.card_cvc,
                            holder_name: card_holder_name,
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
                                        order_amount: item.amount.clone(),
                                    },
                                    card,
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
pub enum EsnekposPaymentStatus {
    #[serde(rename = "SUCCESS")]
    Succeeded,
    #[serde(rename = "ERROR")]
    Error,
    #[default]
    #[serde(rename = "FIELD_ERROR")]
    FieldError,
    #[serde(rename = "AUTHENTICATION_ERROR")]
    AuthenticationError,
    #[serde(rename = "LIMIT_ERROR")]
    LimitError,
    #[serde(rename = "COMMISSION_ERROR")]
    CommissionError,
    #[serde(rename = "INSERT_ERROR")]
    InsertError,
    #[serde(rename = "BRAND_ERROR")]
    BrandError,
    #[serde(rename = "PAYMENT_ERROR")]
    PaymentError,
    #[serde(rename = "NOT_3D_AUTHENTICATION")]
    Not3DAuthentication,
    #[serde(rename = "BLOCKED_ERROR")]
    BlockedError,
    #[serde(rename = "ORDER_CANCEL")]
    OrderCancel,
    #[serde(rename = "PROCESS_QUERY")]
    ProcessQuery,
    #[serde(rename = "DIRECT_PAYMENT")]
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
pub struct EsnekposPaymentsResponse {
    #[serde(rename = "ORDER_REF_NUMBER")]
    pub order_ref_number: String,
    #[serde(rename = "STATUS")]
    pub status: EsnekposPaymentStatus,
    #[serde(rename = "RETURN_CODE")]
    pub return_code: String,
    #[serde(rename = "RETURN_MESSAGE")]
    pub return_message: String,
    #[serde(rename = "RETURN_MESSAGE_TR")]
    pub return_message_tr: Option<String>,
    #[serde(rename = "ERROR_CODE")]
    pub error_code: Option<String>,
    #[serde(rename = "AUTH_HASH")]
    pub auth_hash: Option<String>,
    #[serde(rename = "BANK_AUTH_CODE")]
    pub bank_auth_code: Option<String>,
    #[serde(rename = "DATE")]
    pub date: String,
    #[serde(rename = "URL_3DS")]
    pub url_3ds: Option<String>,
    #[serde(rename = "REFNO")]
    pub refno: String,
    #[serde(rename = "HASH")]
    pub hash: String,
    #[serde(rename = "COMMISSION_RATE")]
    pub commission_rate: String,
    #[serde(rename = "CUSTOMER_NAME")]
    pub customer_name: String,
    #[serde(rename = "CUSTOMER_MAIL")]
    pub customer_mail: String,
    #[serde(rename = "CUSTOMER_PHONE")]
    pub customer_phone: String,
    #[serde(rename = "CUSTOMER_ADDRESS")]
    pub customer_address: Option<String>,
    #[serde(rename = "CUSTOMER_CC_NUMBER")]
    pub customer_cc_number: String,
    #[serde(rename = "CUSTOMER_CC_NAME")]
    pub customer_cc_name: Option<String>,
    #[serde(rename = "IS_NOT_3D_PAYMENT")]
    pub is_not_3d_payment: bool,
    #[serde(rename = "VIRTUAL_POS_VALUES")]
    pub virtual_pos_values: Option<String>,
    #[serde(rename = "RETURN_MESSAGE_3D")]
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

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct EsnekposRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&EsnekposRouterData<&types::RefundsRouterData<F>>> for EsnekposRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &EsnekposRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item
                .amount
                .parse::<i64>()
                .map_err(|_| errors::ConnectorError::AmountConversionFailed)?,
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct EsnekposErrorResponse {
    #[serde(rename = "RETURN_CODE")]
    pub code: String,
    #[serde(rename = "RETURN_MESSAGE")]
    pub message: String,
    #[serde(rename = "STATUS")]
    pub status: Option<String>,
}
