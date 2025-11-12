use std::collections::HashMap;

use api_models::webhooks::IncomingWebhookEvent;
use common_enums::enums;
use common_utils::{ext_traits::ValueExt, types::StringMajorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    address::AddressDetails,
    payment_method_data::PaymentMethodData,
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorAuthType, ConnectorResponseData,
        ErrorResponse, RouterData,
    },
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{MandateReference, PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData,
        SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use masking::{ExposeOptionInterface, Secret};
use serde::Deserialize;

use super::{requests, responses};
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        get_unimplemented_payment_method_error_message, is_manual_capture, AddressDetailsData,
        CardData, PaymentsAuthorizeRequestData, PaymentsSetupMandateRequestData,
        RouterData as OtherRouterData,
    },
};

type Error = error_stack::Report<errors::ConnectorError>;

fn build_payload_cards_request_data(
    payment_method_data: &PaymentMethodData,
    connector_auth_type: &ConnectorAuthType,
    currency: enums::Currency,
    amount: StringMajorUnit,
    billing_address: &AddressDetails,
    capture_method: Option<enums::CaptureMethod>,
    is_mandate: bool,
) -> Result<requests::PayloadCardsRequestData, Error> {
    if let PaymentMethodData::Card(req_card) = payment_method_data {
        let payload_auth = PayloadAuth::try_from((connector_auth_type, currency))?;

        let card = requests::PayloadCard {
            number: req_card.clone().card_number,
            expiry: req_card
                .clone()
                .get_card_expiry_month_year_2_digit_with_delimiter("/".to_owned())?,
            cvc: req_card.card_cvc.clone(),
        };

        let city = billing_address.get_city()?.to_owned();
        let country = billing_address.get_country()?.to_owned();
        let postal_code = billing_address.get_zip()?.to_owned();
        let state_province = billing_address.get_state()?.to_owned();
        let street_address = billing_address.get_line1()?.to_owned();

        let billing_address = requests::BillingAddress {
            city,
            country,
            postal_code,
            state_province,
            street_address,
        };

        // For manual capture, set status to "authorized"
        let status = if is_manual_capture(capture_method) {
            Some(responses::PayloadPaymentStatus::Authorized)
        } else {
            None
        };

        Ok(requests::PayloadCardsRequestData {
            amount,
            card,
            transaction_types: requests::TransactionTypes::Payment,
            payment_method_type: "card".to_string(),
            status,
            billing_address,
            processing_id: payload_auth.processing_account_id,
            keep_active: is_mandate,
        })
    } else {
        Err(
            errors::ConnectorError::NotImplemented(get_unimplemented_payment_method_error_message(
                "Payload",
            ))
            .into(),
        )
    }
}

pub struct PayloadRouterData<T> {
    pub amount: StringMajorUnit,
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

// Auth Struct
#[derive(Debug, Clone, Deserialize)]
pub struct PayloadAuth {
    pub api_key: Secret<String>,
    pub processing_account_id: Option<Secret<String>>,
}

#[derive(Debug, Clone)]
pub struct PayloadAuthType {
    pub auths: HashMap<enums::Currency, PayloadAuth>,
}

impl TryFrom<(&ConnectorAuthType, enums::Currency)> for PayloadAuth {
    type Error = Error;
    fn try_from(value: (&ConnectorAuthType, enums::Currency)) -> Result<Self, Self::Error> {
        let (auth_type, currency) = value;
        match auth_type {
            ConnectorAuthType::CurrencyAuthKey { auth_key_map } => {
                let auth_key = auth_key_map.get(&currency).ok_or(
                    errors::ConnectorError::CurrencyNotSupported {
                        message: currency.to_string(),
                        connector: "Payload",
                    },
                )?;

                auth_key
                    .to_owned()
                    .parse_value("PayloadAuth")
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl TryFrom<&ConnectorAuthType> for PayloadAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::CurrencyAuthKey { auth_key_map } => {
                let auths = auth_key_map
                    .iter()
                    .map(|(currency, auth_key)| {
                        let auth: PayloadAuth = auth_key
                            .to_owned()
                            .parse_value("PayloadAuth")
                            .change_context(errors::ConnectorError::InvalidDataFormat {
                                field_name: "auth_key_map",
                            })?;
                        Ok((*currency, auth))
                    })
                    .collect::<Result<_, Self::Error>>()?;
                Ok(Self { auths })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl TryFrom<&SetupMandateRouterData> for requests::PayloadCardsRequestData {
    type Error = Error;
    fn try_from(item: &SetupMandateRouterData) -> Result<Self, Self::Error> {
        match item.request.amount {
            Some(amount) if amount > 0 => Err(errors::ConnectorError::FlowNotSupported {
                flow: "Setup mandate with non zero amount".to_string(),
                connector: "Payload".to_string(),
            }
            .into()),
            _ => {
                let billing_address = item.get_billing_address()?;
                let is_mandate = item.request.is_customer_initiated_mandate_payment();

                build_payload_cards_request_data(
                    &item.request.payment_method_data,
                    &item.connector_auth_type,
                    item.request.currency,
                    StringMajorUnit::zero(),
                    billing_address,
                    item.request.capture_method,
                    is_mandate,
                )
            }
        }
    }
}

impl TryFrom<&PayloadRouterData<&PaymentsAuthorizeRouterData>>
    for requests::PayloadPaymentsRequest
{
    type Error = Error;
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
            PaymentMethodData::Card(_) => {
                let billing_address = item.router_data.get_billing_address()?;
                let is_mandate = item.router_data.request.is_mandate_payment();

                let cards_data = build_payload_cards_request_data(
                    &item.router_data.request.payment_method_data,
                    &item.router_data.connector_auth_type,
                    item.router_data.request.currency,
                    item.amount.clone(),
                    billing_address,
                    item.router_data.request.capture_method,
                    is_mandate,
                )?;

                Ok(Self::PayloadCardsRequest(Box::new(cards_data)))
            }
            PaymentMethodData::MandatePayment => {
                // For manual capture, set status to "authorized"
                let status = if is_manual_capture(item.router_data.request.capture_method) {
                    Some(responses::PayloadPaymentStatus::Authorized)
                } else {
                    None
                };

                Ok(Self::PayloadMandateRequest(Box::new(
                    requests::PayloadMandateRequestData {
                        amount: item.amount.clone(),
                        transaction_types: requests::TransactionTypes::Payment,
                        payment_method_id: Secret::new(
                            item.router_data.request.get_connector_mandate_id()?,
                        ),
                        status,
                    },
                )))
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
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

impl<F: 'static, T>
    TryFrom<ResponseRouterData<F, responses::PayloadPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
where
    T: 'static,
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, responses::PayloadPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.clone() {
            responses::PayloadPaymentsResponse::PayloadCardsResponse(response) => {
                let status = enums::AttemptStatus::from(response.status);

                let router_data: &dyn std::any::Any = &item.data;
                let is_mandate_payment = router_data
                    .downcast_ref::<PaymentsAuthorizeRouterData>()
                    .is_some_and(|router_data| router_data.request.is_mandate_payment())
                    || router_data
                        .downcast_ref::<SetupMandateRouterData>()
                        .is_some();

                let mandate_reference = if is_mandate_payment {
                    let connector_payment_method_id =
                        response.connector_payment_method_id.clone().expose_option();
                    if connector_payment_method_id.is_some() {
                        Some(MandateReference {
                            connector_mandate_id: connector_payment_method_id,
                            payment_method_id: None,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: None,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };

                let connector_response = {
                    response.avs.map(|avs_response| {
                        let payment_checks = serde_json::json!({
                            "avs_result": avs_response
                        });
                        AdditionalPaymentMethodConnectorResponse::Card {
                            authentication_data: None,
                            payment_checks: Some(payment_checks),
                            card_network: None,
                            domestic_network: None,
                        }
                    })
                }
                .map(ConnectorResponseData::with_additional_payment_method_data);

                let response_result = if status == enums::AttemptStatus::Failure {
                    Err(ErrorResponse {
                        attempt_status: None,
                        code: response
                            .status_code
                            .clone()
                            .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
                        message: response
                            .status_message
                            .clone()
                            .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                        reason: response.status_message,
                        status_code: item.http_code,
                        connector_transaction_id: Some(response.transaction_id.clone()),
                        network_decline_code: None,
                        network_advice_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(response.transaction_id),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(mandate_reference),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: response.ref_number,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                };
                Ok(Self {
                    status,
                    response: response_result,
                    connector_response,
                    ..item.data
                })
            }
        }
    }
}

impl<T> TryFrom<&PayloadRouterData<T>> for requests::PayloadCancelRequest {
    type Error = Error;
    fn try_from(_item: &PayloadRouterData<T>) -> Result<Self, Self::Error> {
        Ok(Self {
            status: responses::PayloadPaymentStatus::Voided,
        })
    }
}

impl TryFrom<&PayloadRouterData<&PaymentsCaptureRouterData>> for requests::PayloadCaptureRequest {
    type Error = Error;
    fn try_from(
        _item: &PayloadRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: responses::PayloadPaymentStatus::Processed,
        })
    }
}

impl<F> TryFrom<&PayloadRouterData<&RefundsRouterData<F>>> for requests::PayloadRefundRequest {
    type Error = Error;
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
    type Error = Error;
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
    type Error = Error;
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

// Webhook transformations
impl From<responses::PayloadWebhooksTrigger> for IncomingWebhookEvent {
    fn from(trigger: responses::PayloadWebhooksTrigger) -> Self {
        match trigger {
            // Payment Success Events
            responses::PayloadWebhooksTrigger::Processed => Self::PaymentIntentSuccess,
            responses::PayloadWebhooksTrigger::Authorized => {
                Self::PaymentIntentAuthorizationSuccess
            }
            // Payment Processing Events
            responses::PayloadWebhooksTrigger::Payment
            | responses::PayloadWebhooksTrigger::AutomaticPayment => Self::PaymentIntentProcessing,
            // Payment Failure Events
            responses::PayloadWebhooksTrigger::Decline
            | responses::PayloadWebhooksTrigger::Reject
            | responses::PayloadWebhooksTrigger::BankAccountReject => Self::PaymentIntentFailure,
            responses::PayloadWebhooksTrigger::Void
            | responses::PayloadWebhooksTrigger::Reversal => Self::PaymentIntentCancelled,
            // Dispute Events
            responses::PayloadWebhooksTrigger::Chargeback => Self::DisputeOpened,
            responses::PayloadWebhooksTrigger::ChargebackReversal => Self::DisputeWon,
            // Other payment-related events
            // Events not supported by our standard flows
            responses::PayloadWebhooksTrigger::PaymentActivationStatus
            | responses::PayloadWebhooksTrigger::Refund
            | responses::PayloadWebhooksTrigger::Credit
            | responses::PayloadWebhooksTrigger::Deposit
            | responses::PayloadWebhooksTrigger::PaymentLinkStatus
            | responses::PayloadWebhooksTrigger::ProcessingStatus
            | responses::PayloadWebhooksTrigger::TransactionOperation
            | responses::PayloadWebhooksTrigger::TransactionOperationClear => {
                Self::EventNotSupported
            }
        }
    }
}

impl TryFrom<responses::PayloadWebhooksTrigger> for responses::PayloadPaymentStatus {
    type Error = Error;
    fn try_from(trigger: responses::PayloadWebhooksTrigger) -> Result<Self, Self::Error> {
        match trigger {
            // Payment Success Events
            responses::PayloadWebhooksTrigger::Processed => Ok(Self::Processed),
            responses::PayloadWebhooksTrigger::Authorized => Ok(Self::Authorized),
            // Payment Processing Events
            responses::PayloadWebhooksTrigger::Payment
            | responses::PayloadWebhooksTrigger::AutomaticPayment
            | responses::PayloadWebhooksTrigger::Reversal => Ok(Self::Processing),
            // Payment Failure Events
            responses::PayloadWebhooksTrigger::Decline
            | responses::PayloadWebhooksTrigger::Reject
            | responses::PayloadWebhooksTrigger::BankAccountReject => Ok(Self::Declined),
            responses::PayloadWebhooksTrigger::Void => Ok(Self::Voided),
            responses::PayloadWebhooksTrigger::Refund => {
                Err(errors::ConnectorError::NotSupported {
                    message: "Refund Webhook".to_string(),
                    connector: "Payload",
                }
                .into())
            }
            responses::PayloadWebhooksTrigger::Chargeback
            | responses::PayloadWebhooksTrigger::ChargebackReversal
            | responses::PayloadWebhooksTrigger::PaymentActivationStatus
            | responses::PayloadWebhooksTrigger::Credit
            | responses::PayloadWebhooksTrigger::Deposit
            | responses::PayloadWebhooksTrigger::PaymentLinkStatus
            | responses::PayloadWebhooksTrigger::ProcessingStatus
            | responses::PayloadWebhooksTrigger::TransactionOperation
            | responses::PayloadWebhooksTrigger::TransactionOperationClear => {
                Err(errors::ConnectorError::WebhookEventTypeNotFound.into())
            }
        }
    }
}

impl TryFrom<responses::PayloadWebhookEvent> for responses::PayloadPaymentsResponse {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(webhook_body: responses::PayloadWebhookEvent) -> Result<Self, Self::Error> {
        let status = responses::PayloadPaymentStatus::try_from(webhook_body.trigger.clone())?;
        Ok(Self::PayloadCardsResponse(
            responses::PayloadCardsResponseData {
                amount: None,
                avs: None,
                customer_id: None,
                transaction_id: webhook_body
                    .triggered_on
                    .transaction_id
                    .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                connector_payment_method_id: None,
                processing_id: None,
                processing_method_id: None,
                ref_number: None,
                status,
                status_code: None,
                status_message: None,
                response_type: None,
            },
        ))
    }
}
