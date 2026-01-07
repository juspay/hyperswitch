pub mod request;
pub mod response;
use api_models::payments::{MandateIds, MandateReferenceId};
use base64::Engine;
use common_enums::{enums, Currency, PaymentChannel};
use common_utils::{
    consts::BASE64_ENGINE, errors::CustomResult, ext_traits::OptionExt, pii::SecretSerdeValue,
    types::MinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    address::Address,
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::*,
    router_request_types::*,
    router_response_types::*,
    types::*,
};
use hyperswitch_interfaces::{api::*, errors::ConnectorError};
use masking::{ExposeInterface as _, PeekInterface, Secret};
pub use request::*;
pub use response::*;
use serde::{Deserialize, Serialize};

use crate::{
    types::PaymentsResponseRouterData,
    utils::{
        get_unimplemented_payment_method_error_message, to_connector_meta_from_secret, ApplePay,
        PaymentsAuthorizeRequestData as _, RouterData as _,
    },
};
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorldpaymodularConnectorMetadataObject {
    pub merchant_name: Option<Secret<String>>,
}
impl TryFrom<Option<&SecretSerdeValue>> for WorldpaymodularConnectorMetadataObject {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(meta_data: Option<&SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = to_connector_meta_from_secret::<Self>(meta_data.cloned())
            .change_context(ConnectorError::InvalidConnectorConfig { config: "metadata" })?;
        Ok(metadata)
    }
}

#[derive(Debug, Serialize)]
pub struct WorldpaymodularRouterData<T> {
    amount: MinorUnit,
    router_data: T,
}
impl<T> TryFrom<(&CurrencyUnit, Currency, MinorUnit, T)> for WorldpaymodularRouterData<T> {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, minor_amount, item): (&CurrencyUnit, Currency, MinorUnit, T),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: minor_amount,
            router_data: item,
        })
    }
}
fn fetch_payment_instrument(
    payment_method: PaymentMethodData,
    billing_address: Option<&Address>,
    connector_mandate_id: Option<MandateIds>,
) -> CustomResult<PaymentInstrument, ConnectorError> {
    let billing_address =
        if let Some(address) = billing_address.and_then(|addr| addr.address.clone()) {
            Some(BillingAddress {
                address1: address.line1,
                address2: address.line2,
                address3: address.line3,
                city: address.city,
                state: address.state,
                postal_code: address
                    .zip
                    .get_required_value("zip")
                    .change_context(ConnectorError::MissingRequiredField { field_name: "zip" })?,
                country_code: address
                    .country
                    .get_required_value("country_code")
                    .change_context(ConnectorError::MissingRequiredField {
                        field_name: "country_code",
                    })?,
            })
        } else {
            None
        };
    match payment_method {
        PaymentMethodData::MandatePayment => {
            let mandate_id = connector_mandate_id
                .as_ref()
                .and_then(|mandate_ids| {
                    mandate_ids
                        .mandate_reference_id
                        .as_ref()
                        .and_then(|mandate_ref_id| match mandate_ref_id {
                            MandateReferenceId::ConnectorMandateId(id) => {
                                id.get_connector_mandate_id()
                            }
                            _ => None,
                        })
                })
                .ok_or(ConnectorError::MissingConnectorMandateID)?;
            Ok(PaymentInstrument::CardToken(CardToken {
                payment_type: PaymentType::CardToken,
                href: mandate_id.into(),
            }))
        }
        PaymentMethodData::Wallet(WalletData::GooglePay(data)) => {
            Ok(PaymentInstrument::Googlepay(WalletPayment {
                payment_type: PaymentType::Googlepay,
                wallet_token: data
                    .tokenization_data
                    .get_encrypted_google_pay_token()
                    .change_context(ConnectorError::MissingRequiredField {
                        field_name: "gpay wallet_token",
                    })?
                    .into(),
                billing_address,
            }))
        }
        PaymentMethodData::Wallet(WalletData::ApplePay(data)) => {
            Ok(PaymentInstrument::Applepay(WalletPayment {
                payment_type: PaymentType::Applepay,
                wallet_token: data.get_applepay_decoded_payment_data()?,
                billing_address,
            }))
        }
        _ => Err(
            ConnectorError::NotImplemented(get_unimplemented_payment_method_error_message(
                "worldpaymodular",
            ))
            .into(),
        ),
    }
}

impl
    TryFrom<(
        &WorldpaymodularRouterData<
            &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        >,
        &Secret<String>,
    )> for WorldpaymodularPaymentsRequest
{
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(
        req: (
            &WorldpaymodularRouterData<
                &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
            >,
            &Secret<String>,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, entity_id) = req;
        let worldpay_connector_metadata_object: WorldpaymodularConnectorMetadataObject =
            WorldpaymodularConnectorMetadataObject::try_from(
                item.router_data.connector_meta_data.as_ref(),
            )?;

        let merchant_name = worldpay_connector_metadata_object.merchant_name.ok_or(
            ConnectorError::InvalidConnectorConfig {
                config: "metadata.merchant_name",
            },
        )?;

        let customer_agreement = if item.router_data.request.is_cit_mandate_payment() {
            Some(WMCustomerAcceptance::CardOnFile(WMCustomerAgreement {
                stored_card_usage: WMStoredCardUsage::First,
            }))
        } else if item.router_data.request.is_mit_payment() {
            Some(WMCustomerAcceptance::Subscription)
        } else {
            None
        };
        let channel = if item.router_data.request.is_mit_payment() {
            None
        } else {
            match item.router_data.request.payment_channel {
                Some(PaymentChannel::MailOrder) => Some(Channel::Moto),
                _ => Some(Channel::Ecom),
            }
        };
        Ok(Self {
            instruction: Instruction {
                request_auto_settlement: RequestAutoSettlement {
                    enabled: item.router_data.request.capture_method
                        == Some(enums::CaptureMethod::Automatic),
                },
                value: PaymentValue {
                    amount: item.amount,
                    currency: item.router_data.request.currency,
                },
                narrative: InstructionNarrative {
                    line1: merchant_name.expose(),
                    ..Default::default()
                },
                payment_instrument: fetch_payment_instrument(
                    item.router_data.request.payment_method_data.clone(),
                    item.router_data.get_optional_billing(),
                    item.router_data.request.mandate_id.clone(),
                )?,
                debt_repayment: None,
                customer_agreement,
            },
            merchant: Merchant {
                entity: entity_id.clone(),
                ..Default::default()
            },
            transaction_reference: item.router_data.connector_request_reference_id.clone(),
            channel,
            customer: None,
        })
    }
}

pub struct WorldpaymodularAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) entity_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for WorldpaymodularAuthType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => {
                let auth_key = format!("{}:{}", key1.peek(), api_key.peek());
                let auth_header = format!("Basic {}", BASE64_ENGINE.encode(auth_key));
                Ok(Self {
                    api_key: Secret::new(auth_header),
                    entity_id: api_secret.clone(),
                })
            }
            _ => Err(ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

pub fn get_worldpay_combined_psync_response(
    response: WorldpayModularPsyncObjResponse,
    data: &PaymentsSyncRouterData,
) -> CustomResult<PaymentsSyncRouterData, ConnectorError> {
    let attempt_status = match response {
        WorldpayModularPsyncObjResponse::PsyncResponse(payment_outcome) => {
            enums::AttemptStatus::from(payment_outcome.last_event)
        }
        WorldpayModularPsyncObjResponse::Webhook(worldpaymodular_webhook_event_type) => {
            enums::AttemptStatus::from(worldpaymodular_webhook_event_type.event_details.event_type)
        }
    };
    Ok(PaymentsSyncRouterData {
        status: attempt_status,
        response: Ok(PaymentsResponseData::TransactionResponse {
            resource_id: data.request.connector_transaction_id.clone(),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        }),
        ..data.clone()
    })
}

pub fn get_worldpay_combined_capture_response(
    response: WorldpaymodularCaptureResponse,
    data: &PaymentsCaptureRouterData,
) -> CustomResult<PaymentsCaptureRouterData, ConnectorError> {
    let mandate = response.links.get_mandate_id();
    Ok(PaymentsCaptureRouterData {
        status: enums::AttemptStatus::Charged,
        response: Ok(PaymentsResponseData::TransactionResponse {
            resource_id: response.links.get_resource_id()?,
            redirection_data: Box::new(None),
            mandate_reference: Box::new(mandate),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        }),
        ..data.clone()
    })
}
pub fn get_worldpay_void_response(
    _response: WorldpaymodularVoidResponse,
    data: &PaymentsCancelRouterData,
) -> CustomResult<PaymentsCancelRouterData, ConnectorError> {
    Ok(PaymentsCancelRouterData {
        status: enums::AttemptStatus::Voided,
        response: Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::NoResponseId,
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        }),
        ..data.clone()
    })
}
impl From<PaymentOutcome> for enums::AttemptStatus {
    fn from(item: PaymentOutcome) -> Self {
        match item {
            PaymentOutcome::Authorized => Self::Authorized,
            PaymentOutcome::Refused => Self::Failure,
            PaymentOutcome::SentForSettlement => Self::Charged,
            PaymentOutcome::SentForRefund => Self::AutoRefunded,
            PaymentOutcome::SentForCancellation => Self::Voided,
        }
    }
}

impl From<EventType> for enums::AttemptStatus {
    fn from(value: EventType) -> Self {
        match value {
            EventType::SentForAuthorization => Self::Authorizing,
            EventType::SentForSettlement => Self::Charged,
            EventType::Authorized => Self::Authorized,
            EventType::SettlementRejected | EventType::Refused | EventType::SettlementFailed => {
                Self::Failure
            }
            EventType::Cancelled
            | EventType::SentForRefund
            | EventType::RefundFailed
            | EventType::Error
            | EventType::Expired
            | EventType::Unknown => Self::Pending,
        }
    }
}

impl From<EventType> for enums::RefundStatus {
    fn from(value: EventType) -> Self {
        match value {
            EventType::SentForRefund => Self::Success,
            EventType::RefundFailed => Self::Failure,
            EventType::Authorized
            | EventType::Cancelled
            | EventType::Refused
            | EventType::Error
            | EventType::SentForSettlement
            | EventType::SentForAuthorization
            | EventType::SettlementFailed
            | EventType::Expired
            | EventType::SettlementRejected
            | EventType::Unknown => Self::Pending,
        }
    }
}

impl TryFrom<PaymentsResponseRouterData<WorldpaymodularPaymentsResponse>>
    for PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: PaymentsResponseRouterData<WorldpaymodularPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let mandate = item.response.links.get_mandate_id();
        let status = enums::AttemptStatus::from(item.response.outcome);

        Ok(Self {
            status,
            description: item.response.description,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: item.response.links.get_resource_id()?,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(mandate),
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

impl<F> TryFrom<(&RefundsRouterData<F>, MinorUnit)> for WorldpaymodularPartialRefundRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(req: (&RefundsRouterData<F>, MinorUnit)) -> Result<Self, Self::Error> {
        let (item, amount) = req;
        Ok(Self {
            reference: item.request.refund_id.clone().replace("_", "-"),
            value: PaymentValue {
                amount,
                currency: item.request.currency,
            },
        })
    }
}

impl TryFrom<(&PaymentsCaptureRouterData, MinorUnit)> for WorldpaymodularPartialCaptureRequest {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(req: (&PaymentsCaptureRouterData, MinorUnit)) -> Result<Self, Self::Error> {
        let (item, amount) = req;
        Ok(Self {
            reference: item
                .connector_request_reference_id
                .clone()
                .replace("_", "-"),
            value: PaymentValue {
                amount,
                currency: item.request.currency,
            },
        })
    }
}
