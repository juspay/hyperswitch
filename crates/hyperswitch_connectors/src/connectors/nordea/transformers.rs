use std::collections::HashMap;

use common_enums::enums;
use common_utils::{pii, request::Method, types::StringMajorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankDebitData, PaymentMethodData},
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsPreProcessingRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Deserializer, Serialize};

use super::{
    requests::{
        AccountNumber, AccountType, CreditorAccount, CreditorAccountReference, CreditorBank,
        DebitorAccount, NordeaOAuthTokenExchangeRequest, NordeaPaymentsConfirmRequest,
        NordeaPaymentsRequest, NordeaRefundRequest, NordeaRouterData, PaymentsUrgency,
    },
    responses::{
        NordeaErrorBody, NordeaFailures, NordeaOAuthTokenExchangeResponse, NordeaPaymentStatus,
        NordeaPaymentsResponse, NordeaRefundResponse, NordeaRefundStatus,
    },
};
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, get_unimplemented_payment_method_error_message, RouterData as _},
};

type Error = error_stack::Report<errors::ConnectorError>;

impl<T> From<(StringMajorUnit, T)> for NordeaRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct NordeaAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
    /// PEM format private key for eIDAS signing
    pub(super) eidas_private_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for NordeaAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                client_id: key1.to_owned(),
                client_secret: api_key.to_owned(),
                eidas_private_key: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// impl TryFrom<&types::RefreshTokenRouterData> for NordeaOAuthTokenExchangeRequest {
//     type Error = Error;
//     fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
//         let auth_type = NordeaAuthType::try_from(&item.connector_auth_type)?;
//         // TODO: Update Along side Access Token flow
//         Ok(Self {
//             grant_type: "".to_string(),
//             code: None,
//             redirect_uri: None,
//             refresh_token: None,
//         })
//     }
// }

// impl<F, T> TryFrom<ResponseRouterData<F, NordeaOAuthTokenExchangeResponse, T, AccessToken>>
//     for RouterData<F, T, AccessToken>
// {
//     type Error = Error;
//     fn try_from(
//         item: ResponseRouterData<F, NordeaOAuthTokenExchangeResponse, T, AccessToken>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             response: Ok(AccessToken {
//                 token: item.response.access_token,
//                 expires: item.response.expires_in,
//             }),
//             ..item.data
//         })
//     }
// }

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NordeaConnectorMetadataObject {
    #[serde(rename = "value")]
    pub creditor_account_value: Secret<String>,
    #[serde(rename = "_type")]
    pub creditor_account_type: String,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for NordeaConnectorMetadataObject {
    type Error = Error;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "merchant_connector_account.metadata",
            })?;
        Ok(metadata)
    }
}

impl TryFrom<&str> for AccountType {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_uppercase().as_str() {
            "IBAN" => Ok(Self::Iban),
            "BBAN_SE" => Ok(Self::BbanSe),
            "BBAN_DK" => Ok(Self::BbanDk),
            "BBAN_NO" => Ok(Self::BbanNo),
            "BGNR" => Ok(Self::Bgnr),
            "PGNR" => Ok(Self::Pgnr),
            "GIRO_DK" => Ok(Self::GiroDk),
            "BBAN_OTHER" => Ok(Self::BbanOther),
            _ => Err(errors::ConnectorError::InvalidConnectorConfig {
                config: "account_type",
            }
            .into()),
        }
    }
}

impl<'de> Deserialize<'de> for PaymentsUrgency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?.to_lowercase();
        match s.as_str() {
            "standard" => Ok(PaymentsUrgency::Standard),
            "express" => Ok(PaymentsUrgency::Express),
            "sameday" => Ok(PaymentsUrgency::Sameday),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["standard", "express", "sameday"],
            )),
        }
    }
}

fn get_creditor_account_from_metadata(
    router_data: &PaymentsPreProcessingRouterData,
) -> Result<CreditorAccount, Error> {
    let metadata: NordeaConnectorMetadataObject =
        utils::to_connector_meta_from_secret(router_data.connector_meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "merchant_connector_account.metadata",
            })?;
    let creditor_account = CreditorAccount {
        account: AccountNumber {
            account_type: AccountType::try_from(metadata.creditor_account_type.as_str())
                .unwrap_or(AccountType::Iban),
            currency: router_data.request.currency,
            value: metadata.creditor_account_value,
        },
        country: router_data.get_optional_billing_country(),
        // Merchant is the beneficiary in this case
        name: None,
        message: None,
        bank: CreditorBank {
            address: None,
            bank_code: None,
            bank_name: None,
            business_identifier_code: None,
            country: router_data.get_billing_country()?,
        },
        creditor_address: None,
        // Reference is optional field in the examples given in the doc.
        // It is considered as a required field in the api contract
        reference: CreditorAccountReference {
            creditor_reference_type: "RF".to_string(), // Assuming RF for SEPA payments
            value: None,
        },
    };
    Ok(creditor_account)
}

impl TryFrom<&NordeaRouterData<&PaymentsPreProcessingRouterData>> for NordeaPaymentsRequest {
    type Error = Error;
    fn try_from(
        item: &NordeaRouterData<&PaymentsPreProcessingRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            Some(PaymentMethodData::BankDebit(bank_debit_data)) => match bank_debit_data {
                BankDebitData::SepaBankDebit { iban, .. } => {
                    let creditor_account = get_creditor_account_from_metadata(item.router_data)?;
                    let debitor_account = DebitorAccount {
                        account: AccountNumber {
                            account_type: AccountType::Iban,
                            currency: item.router_data.request.currency,
                            value: iban,
                        },
                        message: item.router_data.description.clone(),
                    };

                    let instructed_amount = super::requests::InstructedAmount {
                        amount: item.amount.clone(),
                        currency: item.router_data.request.currency.ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "amount",
                            },
                        )?,
                    };

                    Ok(Self {
                        creditor_account,
                        debitor_account,
                        end_to_end_identification: None,
                        external_id: Some(item.router_data.connector_request_reference_id.clone()),
                        instructed_amount,
                        recurring: None,
                        request_availability_of_funds: None,
                        requested_execution_date: None,
                        tpp_messages: None,
                        urgency: None,
                    })
                }
                BankDebitData::AchBankDebit { .. }
                | BankDebitData::BacsBankDebit { .. }
                | BankDebitData::BecsBankDebit { .. } => {
                    Err(errors::ConnectorError::NotImplemented(
                        get_unimplemented_payment_method_error_message("Nordea"),
                    )
                    .into())
                }
            },
            Some(PaymentMethodData::CardRedirect(_))
            | Some(PaymentMethodData::CardDetailsForNetworkTransactionId(_))
            | Some(PaymentMethodData::Wallet(_))
            | Some(PaymentMethodData::PayLater(_))
            | Some(PaymentMethodData::BankRedirect(_))
            | Some(PaymentMethodData::BankTransfer(_))
            | Some(PaymentMethodData::Crypto(_))
            | Some(PaymentMethodData::MandatePayment)
            | Some(PaymentMethodData::Reward)
            | Some(PaymentMethodData::RealTimePayment(_))
            | Some(PaymentMethodData::MobilePayment(_))
            | Some(PaymentMethodData::Upi(_))
            | Some(PaymentMethodData::Voucher(_))
            | Some(PaymentMethodData::GiftCard(_))
            | Some(PaymentMethodData::OpenBanking(_))
            | Some(PaymentMethodData::CardToken(_))
            | Some(PaymentMethodData::NetworkToken(_))
            | Some(PaymentMethodData::Card(_))
            | None => {
                Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into())
            }
        }
    }
}

impl TryFrom<&NordeaRouterData<&PaymentsAuthorizeRouterData>> for NordeaPaymentsConfirmRequest {
    type Error = Error;
    fn try_from(
        item: &NordeaRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_ids = match &item.router_data.response {
            Ok(response_data) => response_data
                .get_connector_transaction_id()
                .map_err(|_| errors::ConnectorError::MissingConnectorTransactionID)?,
            Err(_) => return Err(errors::ConnectorError::ResponseDeserializationFailed.into()),
        };

        Ok(Self {
            authentication_method: None,
            language: None,
            payments_ids: vec![payment_ids],
            redirect_url: None,
            state: None,
        })
    }
}

impl From<NordeaPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: NordeaPaymentStatus) -> Self {
        match item {
            NordeaPaymentStatus::Confirmed | NordeaPaymentStatus::Paid => Self::Charged,

            NordeaPaymentStatus::PendingConfirmation
            | NordeaPaymentStatus::PendingSecondConfirmation
            | NordeaPaymentStatus::PendingUserApproval => Self::AuthenticationPending,

            NordeaPaymentStatus::OnHold | NordeaPaymentStatus::Unknown => Self::Pending,

            NordeaPaymentStatus::Rejected
            | NordeaPaymentStatus::InsufficientFunds
            | NordeaPaymentStatus::LimitExceeded
            | NordeaPaymentStatus::UserApprovalFailed
            | NordeaPaymentStatus::UserApprovalTimeout
            | NordeaPaymentStatus::UserApprovalCancelled => Self::Failure,
        }
    }
}

pub fn get_error_data(error_response: Option<&NordeaErrorBody>) -> Option<&NordeaFailures> {
    error_response
        .and_then(|error| error.nordea_failures.as_ref())
        .and_then(|failures| failures.first())
}

impl<F, T> TryFrom<ResponseRouterData<F, NordeaPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, NordeaPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = match &item.response.payments_response {
            Some(payment_response) => {
                let resource_id =
                    ResponseId::ConnectorTransactionId(payment_response.payment_id.clone());

                let redirection_data = payment_response
                    .links
                    .as_ref()
                    .and_then(|links| {
                        links.iter().find(|link| {
                            link.rel
                                .as_ref()
                                .map(|rel| rel == "signing")
                                .unwrap_or(false)
                        })
                    })
                    .and_then(|link| link.href.clone())
                    .map(|redirect_url| RedirectForm::Form {
                        endpoint: redirect_url,
                        method: Method::Get,
                        form_fields: HashMap::new(),
                    });

                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data: Box::new(redirection_data),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(payment_response.payment_id.clone()),
                    incremental_authorization_allowed: None,
                    charges: None,
                })
            }
            None => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        };

        let status = item
            .response
            .payments_response
            .as_ref()
            .map(|r| match r.payment_status {
                NordeaPaymentStatus::PendingConfirmation
                | NordeaPaymentStatus::PendingUserApproval => {
                    common_enums::AttemptStatus::AuthenticationPending
                }
                _ => common_enums::AttemptStatus::from(r.payment_status.clone()),
            })
            .unwrap_or(common_enums::AttemptStatus::Failure);

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<F> TryFrom<&NordeaRouterData<&RefundsRouterData<F>>> for NordeaRefundRequest {
    type Error = Error;
    fn try_from(item: &NordeaRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, NordeaRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<Execute, NordeaRefundResponse>,
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

impl TryFrom<RefundsResponseRouterData<RSync, NordeaRefundResponse>> for RefundsRouterData<RSync> {
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<RSync, NordeaRefundResponse>,
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

impl From<NordeaRefundStatus> for enums::RefundStatus {
    fn from(item: NordeaRefundStatus) -> Self {
        match item {
            NordeaRefundStatus::Succeeded => Self::Success,
            NordeaRefundStatus::Failed => Self::Failure,
            NordeaRefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}
