use std::io::Cursor;

use common_enums::enums;
use common_utils::types::StringMinorUnit;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{Card, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use quick_xml::{
    events::{BytesDecl, BytesText, Event},
    se::Serializer,
    Writer,
};
use router_env::logger;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self as connector_utils, CardData, PaymentsAuthorizeRequestData},
};

pub struct WorldpayxmlRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for WorldpayxmlRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub mod worldpayxml_constants {
    pub const WORLDPAYXML_VERSION: &str = "1.4";
    pub const XML_VERSION: &str = "1.0";
    pub const XML_ENCODING: &str = "UTF-8";
    pub const WORLDPAYXML_DOC_TYPE: &str =
        r#"paymentService PUBLIC "-//Worldpay//DTD Worldpay PaymentService v1//EN" "http://dtd.worldpay.com/paymentService_v1.dtd""#;
}

#[derive(Serialize)]
#[serde(rename = "paymentService")]
pub struct PaymentService {
    #[serde(rename = "@version")]
    version: String,
    #[serde(rename = "@merchantCode")]
    merchant_code: Secret<String>,
    submit: Submit,
}

#[derive(Serialize)]
struct Submit {
    order: Order,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Order {
    #[serde(rename = "@orderCode")]
    order_code: String,
    #[serde(rename = "@captureDelay")]
    capture_delay: AutoCapture,
    description: String,
    amount: WorldpayXmlAmount,
    payment_details: PaymentDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AutoCapture {
    OFF,
    #[serde(rename = "0")]
    ON,
}

#[derive(Serialize)]
struct WorldpayXmlAmount {
    #[serde(rename = "@currencyCode")]
    currency_code: api_models::enums::Currency,
    #[serde(rename = "@exponent")]
    exponent: String,
    #[serde(rename = "@value")]
    value: StringMinorUnit,
}

#[derive(Serialize)]
struct PaymentDetails {
    #[serde(rename = "CARD-SSL")]
    card_ssl: CardSSL,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CardSSL {
    card_number: cards::CardNumber,
    expiry_date: ExpiryDate,
    card_holder_name: Option<Secret<String>>,
    cvc: Secret<String>,
}

#[derive(Serialize)]
#[serde(rename = "expiryDate")]
struct ExpiryDate {
    date: Date,
}

#[derive(Serialize)]
struct Date {
    #[serde(rename = "@month")]
    month: Secret<String>,
    #[serde(rename = "@year")]
    year: Secret<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct WorldpayxmlCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&Card> for PaymentDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(card_data: &Card) -> Result<Self, Self::Error> {
        Ok(Self {
            card_ssl: CardSSL {
                card_number: card_data.card_number.clone(),
                expiry_date: ExpiryDate {
                    date: Date {
                        month: card_data.get_card_expiry_month_2_digit()?,
                        year: card_data.get_expiry_year_4_digit(),
                    },
                },
                card_holder_name: card_data.card_holder_name.to_owned(),
                cvc: card_data.card_cvc.to_owned(),
            },
        })
    }
}

impl TryFrom<(&WorldpayxmlRouterData<&PaymentsAuthorizeRouterData>, &Card)> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: (&WorldpayxmlRouterData<&PaymentsAuthorizeRouterData>, &Card),
    ) -> Result<Self, Self::Error> {
        let authorize_data = item.0;
        let card_data = item.1;
        let auth = WorldpayxmlAuthType::try_from(&authorize_data.router_data.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let order_code = authorize_data
            .router_data
            .connector_request_reference_id
            .to_owned();
        let capture_delay = if authorize_data.router_data.request.is_auto_capture()? {
            AutoCapture::ON
        } else {
            AutoCapture::OFF
        };
        let description = authorize_data.router_data.description.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "description",
            },
        )?;
        let exponent = authorize_data
            .router_data
            .request
            .currency
            .number_of_digits_after_decimal_point()
            .to_string();
        let amount = WorldpayXmlAmount {
            currency_code: authorize_data.router_data.request.currency.to_owned(),
            exponent,
            value: authorize_data.amount.to_owned(),
        };
        let payment_details = PaymentDetails::try_from(card_data)?;
        let submit = Submit {
            order: Order {
                order_code,
                capture_delay,
                description,
                amount,
                payment_details,
            },
        };

        Ok(Self {
            version: worldpayxml_constants::WORLDPAYXML_VERSION.to_string(),
            merchant_code: auth.merchant_code.clone(),
            submit,
        })
    }
}

impl TryFrom<&WorldpayxmlRouterData<&PaymentsAuthorizeRouterData>> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WorldpayxmlRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                PaymentService::try_from((item, &req_card))
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                connector_utils::get_unimplemented_payment_method_error_message("Worldpayxml"),
            ))?,
        }
            
    }
}

pub fn get_worldpay_xml_auth_request(
    item: &T,
) -> Result<Vec<u8>, error_stack::Report<errors::ConnectorError>> {
    let mut xml_bytes = Vec::new();
    let mut writer = Writer::new(Cursor::new(&mut xml_bytes));
    writer.write_event(Event::Decl(BytesDecl::new(worldpayxml_constants::XML_VERSION, Some(worldpayxml_constants::XML_ENCODING), None)))
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to write XML declaration for Worldpay request")?;
    writer.write_event(Event::DocType(BytesText::from_escaped(
        worldpayxml_constants::WORLDPAYXML_DOC_TYPE,
    ))).unwrap();

            let xml_body = quick_xml::se::to_string(&trial).unwrap();
            writer
                .write_event(Event::Text(BytesText::from_escaped(xml_body)))
                .unwrap();

            let tt = String::from_utf8(xml_bytes.clone())
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;

            logger::debug!("sssss {:?}", tt);
            logger::debug!("sssssssssssss {:?}", xml_bytes);
            Ok(xml_bytes)
        }
        _ => Ok(xml_bytes),
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct WorldpayxmlAuthType {
    pub(super) api_username: Secret<String>,
    pub(super) api_password: Secret<String>,
    pub(super) merchant_code: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for WorldpayxmlAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                api_username: api_key.to_owned(),
                api_password: key1.to_owned(),
                merchant_code: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorldpayxmlPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<WorldpayxmlPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: WorldpayxmlPaymentStatus) -> Self {
        match item {
            WorldpayxmlPaymentStatus::Succeeded => Self::Charged,
            WorldpayxmlPaymentStatus::Failed => Self::Failure,
            WorldpayxmlPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldpayxmlPaymentsResponse {
    status: WorldpayxmlPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, WorldpayxmlPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, WorldpayxmlPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct WorldpayxmlRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&WorldpayxmlRouterData<&RefundsRouterData<F>>> for WorldpayxmlRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &WorldpayxmlRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
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

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct WorldpayxmlErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
