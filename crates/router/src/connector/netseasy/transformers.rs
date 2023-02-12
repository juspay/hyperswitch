use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};
use storage_models::enums::AttemptStatus::AuthenticationPending;
use url::Url;

use crate::{
    core::errors,
    services,
    types::{self, api, storage::enums},
};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetseasyPaymentsRequest {
    pub order: NetseasyOrder,
    pub checkout: NetseasyCheckout,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetseasyOrder {
    pub items: Vec<ItemList>,
    pub amount: i64,
    pub currency: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemList {
    pub reference: String,
    pub name: String,
    pub quantity: u16,
    pub unit: String,
    pub unit_price: i64,
    pub tax_rate: i64,
    pub tax_amount: i64,
    pub gross_total_amount: i64,
    pub net_total_amount: i64,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetseasyCheckout {
    pub url: String,
    pub terms_url: String,
    pub integration_type: String,
    pub return_url: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NetseasyPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let order_details = get_order_details(item);
        let check_details = get_checkout_details(item);
        Ok(Self {
            order: order_details,
            checkout: check_details,
        })
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct NetseasyAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for NetseasyAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_string(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NetseasyPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<NetseasyPaymentStatus> for enums::AttemptStatus {
    fn from(item: NetseasyPaymentStatus) -> Self {
        match item {
            NetseasyPaymentStatus::Succeeded => Self::Charged,
            NetseasyPaymentStatus::Failed => Self::Failure,
            NetseasyPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetseasyPaymentsResponse {
    payment_id: String,
    hosted_payment_page_url: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, NetseasyPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            NetseasyPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let redirection_d = {
            let url = Url::parse(&item.response.hosted_payment_page_url)
                .into_report()
                .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
            let mut base_url = url.clone();
            base_url.set_query(None);
            Some(services::RedirectForm {
                url: base_url.to_string(),
                method: services::Method::Get,
                form_fields: std::collections::HashMap::from_iter(
                    url.query_pairs()
                        .map(|(k, v)| (k.to_string(), v.to_string())),
                ),
            })
        };
        Ok(Self {
            status: AuthenticationPending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.payment_id.to_owned(),
                ),
                redirect: true,
                redirection_data: redirection_d,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}
//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct NetseasyRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for NetseasyRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        todo!()
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
pub struct RefundResponse {}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct NetseasyErrorResponse {}

fn get_order_details(item: &types::PaymentsAuthorizeRouterData) -> NetseasyOrder {
    let amount = item.request.amount;
    let currency = item.request.currency.to_string().clone();
    let order_details = match item.request.order_details.clone() {
        Some(x) => Ok(x),
        None => Err(errors::ConnectorError::MissingRequiredField {
            field_name: ("Order details"),
        }),
    }
    .unwrap();
    let product_name = order_details.product_name.clone();
    let items = vec![ItemList {
        reference: "something".to_string(), //not present in current request format
        name: product_name,
        quantity: order_details.quantity,
        unit: "kg".to_string(), //not present in current request format
        unit_price: item.request.amount,
        tax_rate: 0,
        tax_amount: 0,
        gross_total_amount: item.request.amount,
        net_total_amount: item.request.amount,
    }];
    return NetseasyOrder {
        amount,
        currency,
        items,
    };
}

fn get_checkout_details(_item: &types::PaymentsAuthorizeRouterData) -> NetseasyCheckout {
    let url = "".to_string(); //not present in current request format
    let terms_url = "https://google.com".to_string(); //not present in current request format
    let integration_type = "HostedPaymentPage".to_string(); //not present in current request format
    let return_url = "https://google.com".to_string(); //not present in current request format
    return NetseasyCheckout {
        url,
        terms_url,
        integration_type,
        return_url,
    };
}
