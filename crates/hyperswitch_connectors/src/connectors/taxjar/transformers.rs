use common_enums::enums;
use common_utils::types::{FloatMajorUnit, FloatMajorUnitForConnector, MinorUnit, StringMinorUnit};
use error_stack::report;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_response_types::{RefundsResponseData, TaxCalculationResponseData},
    router_request_types::PaymentsTaxCalculationData,
    types,
    types::RefundsRouterData,
};
use hyperswitch_interfaces::{api, errors};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, AddressDetailsData, RouterData as _},
};

//TODO: Fill the struct with respective fields
pub struct TaxjarRouterData<T> {
    pub amount: FloatMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub shipping: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, FloatMajorUnit, T)> for TaxjarRouterData<T> {
    fn from((amount, shipping, item): (FloatMajorUnit, FloatMajorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            shipping,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct TaxjarPaymentsRequest {
    from_country: enums::CountryAlpha2,
    from_zip: Secret<String>,
    from_state: Secret<String>,
    from_city: Option<String>,
    from_street: Option<Secret<String>>,
    to_country: enums::CountryAlpha2,
    to_zip: Secret<String>,
    to_state: Secret<String>,
    to_city: Option<String>,
    to_street: Option<Secret<String>>,
    amount: FloatMajorUnit,
    shipping: FloatMajorUnit,
    line_items: Vec<LineItem>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct LineItem {
    id: Option<String>,
    quantity: Option<u16>,
    product_tax_code: Option<String>,
    unit_price: Option<FloatMajorUnit>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct TaxjarCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&TaxjarRouterData<&types::PaymentsTaxCalculationRouterData>>
    for TaxjarPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &TaxjarRouterData<&types::PaymentsTaxCalculationRouterData>,
    ) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        // let shipping = item.router_data.get_shipping_address()?;
        // println!("$$swangi{:?}", shipping);
        let currency = item.router_data.request.currency;
        let currency_unit = &api::CurrencyUnit::Base;
        let shipping = &item.router_data.request.shipping_address.address.clone().ok_or(errors::ConnectorError::MissingRequiredField { field_name: "address" })?;
        

        println!("$$shipping123{:?}", shipping);


        match request.order_details.clone() {
            Some(order_details) => Ok(Self {
                from_country: item.router_data.get_billing_country()?,
                from_zip: item.router_data.get_billing_zip()?,
                from_state: item.router_data.get_billing_state_code()?,
                from_city: item.router_data.get_optional_billing_city(),
                from_street: item.router_data.get_optional_billing_line1(),
                to_country: shipping.get_country()?.to_owned(),
                to_zip: shipping.get_zip()?.to_owned(),
                to_state: shipping.to_state_code()?.to_owned(),
                to_city: shipping.get_optional_city(),
                to_street: shipping.get_optional_line1(),
                amount: item.amount,
                shipping: item.shipping,
                line_items: order_details
                    .iter()
                    .map(|line_item| LineItem {
                        id: line_item.product_id.clone(),
                        quantity: Some(line_item.quantity),
                        product_tax_code: line_item.product_tax_code.clone(),
                        unit_price: Some(FloatMajorUnit::new(utils::get_amount_as_f64(currency_unit, line_item.amount, currency).unwrap())),
                    })
                    .collect(),
            }),
            None => Err(report!(errors::ConnectorError::MissingRequiredField {
                field_name: "order_details"
            })),
        }
    }
}

pub struct TaxjarAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for TaxjarAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxjarPaymentsResponse {
    tax: Tax,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tax {
    order_total_amount: FloatMajorUnit,
    amount_to_collect: FloatMajorUnit, //calculated_tax_amount
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            TaxjarPaymentsResponse,
            PaymentsTaxCalculationData,
            TaxCalculationResponseData,
        >,
    > for RouterData<F, PaymentsTaxCalculationData, TaxCalculationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            TaxjarPaymentsResponse,
            PaymentsTaxCalculationData,
            TaxCalculationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let currency = item.data.request.currency;
        let amount_to_collect = item.response.tax.amount_to_collect;
        let order_total_amount = item.response.tax.order_total_amount;
        let calculated_tax = utils::convert_back_amount_to_minor_units(
            &FloatMajorUnitForConnector,
            amount_to_collect,
            currency,
        )?;
        let total_amount = utils::convert_back_amount_to_minor_units(
            &FloatMajorUnitForConnector,
            order_total_amount,
            currency,
        )?;

        Ok(Self {
            response: Ok(TaxCalculationResponseData {
                order_tax_amount: calculated_tax,
                net_amount: (total_amount + calculated_tax), 
                shipping_address: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
// #[derive(Default, Debug, Serialize)]
// pub struct TaxjarRefundRequest {
//     pub amount: StringMinorUnit,
// }

// impl<F> TryFrom<&TaxjarRouterData<&RefundsRouterData<F>>> for TaxjarRefundRequest {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(item: &TaxjarRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
//         Ok(Self {
//             amount: item.amount.to_owned(),
//         })
//     }
// }

// Type definition for Refund Response

// #[allow(dead_code)]
// #[derive(Debug, Serialize, Default, Deserialize, Clone)]
// pub enum RefundStatus {
//     Succeeded,
//     Failed,
//     #[default]
//     Processing,
// }

// impl From<RefundStatus> for enums::RefundStatus {
//     fn from(item: RefundStatus) -> Self {
//         match item {
//             RefundStatus::Succeeded => Self::Success,
//             RefundStatus::Failed => Self::Failure,
//             RefundStatus::Processing => Self::Pending,
//             //TODO: Review mapping
//         }
//     }
// }

//TODO: Fill the struct with respective fields
// #[derive(Default, Debug, Clone, Serialize, Deserialize)]
// pub struct RefundResponse {
//     id: String,
//     status: RefundStatus,
// }

// impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: RefundsResponseRouterData<Execute, RefundResponse>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             response: Ok(RefundsResponseData {
//                 connector_refund_id: item.response.id.to_string(),
//                 refund_status: enums::RefundStatus::from(item.response.status),
//             }),
//             ..item.data
//         })
//     }
// }

// impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: RefundsResponseRouterData<RSync, RefundResponse>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             response: Ok(RefundsResponseData {
//                 connector_refund_id: item.response.id.to_string(),
//                 refund_status: enums::RefundStatus::from(item.response.status),
//             }),
//             ..item.data
//         })
//     }
// }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaxjarErrorResponse {
    pub status: String,
    pub error: String,
    pub detail: String,
}
