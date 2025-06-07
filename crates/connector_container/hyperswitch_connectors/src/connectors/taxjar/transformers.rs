use common_enums::enums;
use common_utils::types::{FloatMajorUnit, FloatMajorUnitForConnector};
use error_stack::report;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_request_types::PaymentsTaxCalculationData,
    router_response_types::TaxCalculationResponseData,
    types,
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::ResponseRouterData,
    utils::{self, AddressDetailsData},
};

pub struct TaxjarRouterData<T> {
    pub amount: FloatMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub order_amount: FloatMajorUnit,
    pub shipping: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, FloatMajorUnit, FloatMajorUnit, T)> for TaxjarRouterData<T> {
    fn from(
        (amount, order_amount, shipping, item): (FloatMajorUnit, FloatMajorUnit, FloatMajorUnit, T),
    ) -> Self {
        Self {
            amount,
            order_amount,
            shipping,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct TaxjarPaymentsRequest {
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
        let shipping = &item
            .router_data
            .request
            .shipping_address
            .address
            .clone()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "address",
            })?;

        match request.order_details.clone() {
            Some(order_details) => {
                let line_items: Result<Vec<LineItem>, error_stack::Report<errors::ConnectorError>> =
                    order_details
                        .iter()
                        .map(|line_item| {
                            Ok(LineItem {
                                id: line_item.product_id.clone(),
                                quantity: Some(line_item.quantity),
                                product_tax_code: line_item.product_tax_code.clone(),
                                unit_price: Some(item.order_amount),
                            })
                        })
                        .collect();

                Ok(Self {
                    to_country: shipping.get_country()?.to_owned(),
                    to_zip: shipping.get_zip()?.to_owned(),
                    to_state: shipping.to_state_code()?.to_owned(),
                    to_city: shipping.get_optional_city(),
                    to_street: shipping.get_optional_line1(),
                    amount: item.amount,
                    shipping: item.shipping,
                    line_items: line_items?,
                })
            }
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

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxjarPaymentsResponse {
    tax: Tax,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tax {
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
        let calculated_tax = utils::convert_back_amount_to_minor_units(
            &FloatMajorUnitForConnector,
            amount_to_collect,
            currency,
        )?;

        Ok(Self {
            response: Ok(TaxCalculationResponseData {
                order_tax_amount: calculated_tax,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaxjarErrorResponse {
    pub status: i64,
    pub error: String,
    pub detail: String,
}
