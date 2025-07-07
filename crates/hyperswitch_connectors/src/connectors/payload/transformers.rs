use common_enums::enums;
use common_utils::types::StringMajorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;

use super::{requests, responses};
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{is_manual_capture, CardData, RouterData as OtherRouterData},
};

//TODO: Fill the struct with respective fields
pub struct PayloadRouterData<T> {
    pub amount: StringMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for PayloadRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

impl TryFrom<&PayloadRouterData<&PaymentsAuthorizeRouterData>>
    for requests::PayloadPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayloadRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "Cards 3DS".to_string(),
                connector: "Payload",
            })?
        }

        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = requests::PayloadCard {
                    number: req_card.clone().card_number,
                    expiry: req_card
                        .clone()
                        .get_card_expiry_month_year_2_digit_with_delimiter("/".to_owned())?,
                    cvc: req_card.card_cvc,
                };
                let address = item.router_data.get_billing_address()?;
                let billing_address = requests::BillingAddress {
                    city: address.city.clone().unwrap_or_default(),
                    country: address.country.unwrap_or_default(),
                    postal_code: address.zip.clone().unwrap_or_default(),
                    state_province: address.state.clone().unwrap_or_default(),
                    street_address: address.line1.clone().unwrap_or_default(),
                };

                // For manual capture, set status to "authorized"
                let status = if is_manual_capture(item.router_data.request.capture_method) {
                    Some(responses::PayloadPaymentStatus::Authorized)
                } else {
                    None
                };

                Ok(Self::PayloadCardsRequest(
                    requests::PayloadCardsRequestData {
                        amount: item.amount.clone(),
                        card,
                        transaction_types: requests::TransactionTypes::Payment,
                        payment_method_type: "card".to_string(),
                        status,
                        billing_address,
                    },
                ))
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Auth Struct
pub struct PayloadAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PayloadAuthType {
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

impl From<responses::PayloadPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: responses::PayloadPaymentStatus) -> Self {
        match item {
            responses::PayloadPaymentStatus::Authorized => Self::Authorized,
            responses::PayloadPaymentStatus::Processed => Self::Charged,
            responses::PayloadPaymentStatus::Processing => Self::Pending,
            responses::PayloadPaymentStatus::Rejected
            | responses::PayloadPaymentStatus::Declined => Self::Failure,
            responses::PayloadPaymentStatus::Voided => Self::Voided,
        }
    }
}

impl<F, T>
    TryFrom<ResponseRouterData<F, responses::PayloadPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, responses::PayloadPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.clone() {
            responses::PayloadPaymentsResponse::PayloadCardsResponse(response) => {
                let payment_status = response.status;
                let transaction_id = response.transaction_id;

                Ok(Self {
                    status: common_enums::AttemptStatus::from(payment_status),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(transaction_id),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: response.ref_number,
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl<T> TryFrom<&PayloadRouterData<T>> for requests::PayloadCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &PayloadRouterData<T>) -> Result<Self, Self::Error> {
        Ok(Self {
            status: responses::PayloadPaymentStatus::Voided,
        })
    }
}

impl TryFrom<&PayloadRouterData<&PaymentsCaptureRouterData>> for requests::PayloadCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: &PayloadRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: responses::PayloadPaymentStatus::Processed,
        })
    }
}

impl<F> TryFrom<&PayloadRouterData<&RefundsRouterData<F>>> for requests::PayloadRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PayloadRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let connector_transaction_id = item.router_data.request.connector_transaction_id.clone();

        Ok(Self {
            transaction_type: requests::TransactionTypes::Refund,
            amount: item.amount.to_owned(),
            ledger_assoc_transaction_id: connector_transaction_id,
        })
    }
}

impl From<responses::RefundStatus> for enums::RefundStatus {
    fn from(item: responses::RefundStatus) -> Self {
        match item {
            responses::RefundStatus::Processed => Self::Success,
            responses::RefundStatus::Processing => Self::Pending,
            responses::RefundStatus::Declined | responses::RefundStatus::Rejected => Self::Failure,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, responses::PayloadRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, responses::PayloadRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, responses::PayloadRefundResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, responses::PayloadRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}
