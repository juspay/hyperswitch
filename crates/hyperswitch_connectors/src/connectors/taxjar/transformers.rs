use common_enums::enums;
use common_utils::types::{MinorUnit, StringMinorUnit};
use error_stack::report;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_response_types::{RefundsResponseData, TaxCalculationResponseData},
    types,
    types::RefundsRouterData,
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{AddressDetailsData, RouterData as _},
};

//TODO: Fill the struct with respective fields
pub struct TaxjarRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for TaxjarRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
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
    amount: MinorUnit,
    shipping_cost: MinorUnit,
    line_items: Vec<LineItem>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct LineItem {
    id: Option<String>,
    quantity: Option<u16>,
    product_tax_code: Option<String>,
    unit_price: Option<i64>,
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
        let shipping = item.router_data.get_shipping_address()?;
        // let shipping = item.router_data.request.shipping
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
                amount: request.amount,
                shipping_cost: request.shipping_cost,
                line_items: order_details
                    .iter()
                    .map(|line_item| LineItem {
                        id: line_item.product_id.clone(),
                        quantity: Some(line_item.quantity),
                        product_tax_code: line_item.product_tax_code.clone(),
                        unit_price: Some(line_item.amount),
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
    order_total_amount: i64,
    amount_to_collect: i64, //calculated_tax_amount
}

impl<F, T> TryFrom<ResponseRouterData<F, TaxjarPaymentsResponse, T, TaxCalculationResponseData>>
    for RouterData<F, T, TaxCalculationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, TaxjarPaymentsResponse, T, TaxCalculationResponseData>,
    ) -> Result<Self, Self::Error> {
        let calculated_tax = item.response.amount_to_collect;
        let order_total_amount = item.response.order_total_amount;

        Ok(Self {
            response: Ok(TaxCalculationResponseData {
                order_tax_amount: calculated_tax,
                net_amount: calculated_tax + order_total_amount,
                shipping_address: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct TaxjarRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&TaxjarRouterData<&RefundsRouterData<F>>> for TaxjarRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &TaxjarRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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
pub struct TaxjarErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
