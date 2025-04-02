use common_enums::enums;
use common_utils::{errors::ParsingError, types::StringMinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, PaymentMethodData},
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types,
};
use hyperswitch_interfaces::errors;
use masking::Secret;

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

use super::{
    requests::{
        FacilitapayAuthRequest, FacilitapayCredentials, FacilitapayPaymentsRequest,
        FacilitapayRefundRequest, FacilitapayRouterData, FacilitapayTransactionRequest,
        PixTransactionRequest,
    },
    responses::{
        FacilitapayAuthResponse, FacilitapayPaymentStatus, FacilitapayPaymentsResponse,
        FacilitapayRefundResponse, RefundStatus,
    },
};

type Error = error_stack::Report<errors::ConnectorError>;

impl<T> From<(StringMinorUnit, T)> for FacilitapayRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

impl TryFrom<&FacilitapayRouterData<&types::PaymentsAuthorizeRouterData>>
    for FacilitapayPaymentsRequest
{
    type Error = Error;
    fn try_from(
        item: &FacilitapayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankTransfer(bank_transfer_data) => match *bank_transfer_data {
                BankTransferData::Pix {
                    from_bank_account_id,
                    to_bank_account_id,
                    ..
                } => {
                    let transaction_data =
                        FacilitapayTransactionRequest::Pix(PixTransactionRequest {
                            subject_id: item.router_data.customer_id.clone().ok_or(
                                errors::ConnectorError::MissingRequiredField {
                                    field_name: "customer id",
                                },
                            )?,
                            from_bank_account_id: from_bank_account_id.clone().ok_or(
                                errors::ConnectorError::MissingRequiredField {
                                    field_name: "source bank account id",
                                },
                            )?,

                            to_bank_account_id: to_bank_account_id.clone().ok_or(
                                errors::ConnectorError::MissingRequiredField {
                                    field_name: "destination bank account id",
                                },
                            )?,
                            currency: item.router_data.request.currency.to_string(),
                            exchange_currency: item.router_data.request.currency.to_string(),
                            value: item.amount.clone(),
                            use_dynamic_pix: true,
                            dynamic_pix_expires_at: None,
                        });

                    Ok(Self {
                        transaction: transaction_data,
                    })
                }
                BankTransferData::AchBankTransfer {}
                | BankTransferData::SepaBankTransfer {}
                | BankTransferData::BacsBankTransfer {}
                | BankTransferData::MultibancoBankTransfer {}
                | BankTransferData::PermataBankTransfer {}
                | BankTransferData::BcaBankTransfer {}
                | BankTransferData::BniVaBankTransfer {}
                | BankTransferData::BriVaBankTransfer {}
                | BankTransferData::CimbVaBankTransfer {}
                | BankTransferData::DanamonVaBankTransfer {}
                | BankTransferData::MandiriVaBankTransfer {}
                | BankTransferData::Pse {}
                | BankTransferData::InstantBankTransfer {}
                | BankTransferData::LocalBankTransfer { .. } => {
                    Err(errors::ConnectorError::NotImplemented(
                        "Selected payment method through Facilitapay".to_string(),
                    )
                    .into())
                }
            },
            PaymentMethodData::Card(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    "Selected payment method through Facilitapay".to_string(),
                )
                .into())
            }
        }
    }
}

// Helper to build the request from Hyperswitch Auth Type
impl TryFrom<&FacilitapayAuthType> for FacilitapayAuthRequest {
    type Error = Error;
    fn try_from(auth: &FacilitapayAuthType) -> Result<Self, Self::Error> {
        Ok(Self {
            user: FacilitapayCredentials {
                username: auth.username.clone(),
                password: auth.password.clone(),
            },
        })
    }
}

// Auth Struct
#[derive(Debug, Clone)]
pub struct FacilitapayAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for FacilitapayAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                username: key1.to_owned(),
                password: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl TryFrom<FacilitapayAuthResponse> for AccessToken {
    type Error = error_stack::Report<ParsingError>;

    fn try_from(item: FacilitapayAuthResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            token: item.jwt,
            expires: 86400, // Facilitapay docs say 24 hours validity. 24 * 60 * 60 = 86400 seconds.
        })
    }
}

impl From<FacilitapayPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: FacilitapayPaymentStatus) -> Self {
        match item {
            FacilitapayPaymentStatus::Pending | FacilitapayPaymentStatus::Unknown => Self::Pending,
            FacilitapayPaymentStatus::Identified | FacilitapayPaymentStatus::Exchanged => {
                Self::Authorized
            }
            FacilitapayPaymentStatus::Wired => Self::Charged,
            FacilitapayPaymentStatus::Canceled => Self::Voided,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, FacilitapayAuthResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, FacilitapayAuthResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.jwt,
                expires: 86400, // Facilitapay docs say 24 hours validity. 24 * 60 * 60 = 86400 seconds.
            }),
            ..item.data
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, FacilitapayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, FacilitapayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let connector_metadata = item
            .response
            .data
            .dynamic_pix_code
            .map(|code| serde_json::json!({ "dynamic_pix_code": code }));

        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.data.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.data.id),
                // Redirection data might be needed if PIX code needs user action
                redirection_data: Box::new(None), // Adjust if needed
                mandate_reference: Box::new(None), // Add logic if mandates are supported
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: None, // Use item.response.data.id or another relevant field if available
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F> TryFrom<&FacilitapayRouterData<&types::RefundsRouterData<F>>> for FacilitapayRefundRequest {
    type Error = Error;
    fn try_from(
        item: &FacilitapayRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.clone(),
        })
    }
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing | RefundStatus::Unknown => Self::Pending,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, FacilitapayRefundResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<Execute, FacilitapayRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.data.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.data.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, FacilitapayRefundResponse>>
    for types::RefundsRouterData<RSync>
{
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<RSync, FacilitapayRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.data.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.data.status),
            }),
            ..item.data
        })
    }
}
