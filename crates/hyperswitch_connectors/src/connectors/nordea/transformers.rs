use std::collections::HashMap;

use common_utils::{pii, request::Method, types::StringMajorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankDebitData, PaymentMethodData},
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{Authorize, PreProcessing},
    router_request_types::{PaymentsAuthorizeData, PaymentsPreProcessingData, ResponseId},
    router_response_types::{PaymentsResponseData, RedirectForm},
    types::{
        self, AccessTokenAuthenticationRouterData, PaymentsAuthorizeRouterData,
        PaymentsPreProcessingRouterData, PaymentsSyncRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use rand::distributions::DistString;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    connectors::nordea::{
        requests::{
            AccessScope, AccountNumber, AccountType, CreditorAccount, CreditorBank, DebitorAccount,
            GrantType, NordeaOAuthExchangeRequest, NordeaOAuthRequest,
            NordeaPaymentsConfirmRequest, NordeaPaymentsRequest, NordeaRouterData, PaymentsUrgency,
        },
        responses::{
            NordeaErrorBody, NordeaFailures, NordeaOAuthExchangeResponse, NordeaPaymentStatus,
            NordeaPaymentsConfirmResponse, NordeaPaymentsInitiateResponse,
        },
    },
    types::{
        PaymentsPreprocessingResponseRouterData, PaymentsSyncResponseRouterData, ResponseRouterData,
    },
    utils::{self, get_unimplemented_payment_method_error_message, RouterData as _},
};

type Error = error_stack::Report<errors::ConnectorError>;

impl<T> From<(StringMajorUnit, T)> for NordeaRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

// Auth Struct
#[derive(Debug)]
pub struct NordeaAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
    /// PEM format private key for eIDAS signing
    /// Should be base64 encoded
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NordeaConnectorMetadataObject {
    /// Account number of the beneficiary (merchant)
    pub destination_account_number: Secret<String>,
    /// Account type (example: IBAN, BBAN_SE, BBAN_DK, BBAN_NO, BGNR, PGNR, GIRO_DK, BBAN_OTHER)
    pub account_type: String,
    /// Name of the beneficiary (merchant)
    pub merchant_name: Secret<String>,
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

impl TryFrom<&AccessTokenAuthenticationRouterData> for NordeaOAuthRequest {
    type Error = Error;
    fn try_from(item: &AccessTokenAuthenticationRouterData) -> Result<Self, Self::Error> {
        let country = item.get_billing_country()?;

        // Set refresh_token maximum expiry duration to 180 days (259200 / 60 = 180)
        // Minimum is 1 minute
        let duration = Some(259200);
        let maximum_transaction_history = Some(18);
        let redirect_uri = "https://hyperswitch.io".to_string();
        let scope = [
            AccessScope::AccountsBasic,
            AccessScope::AccountsDetails,
            AccessScope::AccountsBalances,
            AccessScope::AccountsTransactions,
            AccessScope::PaymentsMultiple,
        ]
        .to_vec();
        let state = rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 15);

        Ok(Self {
            country,
            duration,
            maximum_transaction_history,
            redirect_uri,
            scope,
            state: state.into(),
        })
    }
}

impl TryFrom<&types::RefreshTokenRouterData> for NordeaOAuthExchangeRequest {
    type Error = Error;
    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let code = item
            .request
            .authentication_token
            .as_ref()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "authorization_code",
            })?
            .code
            .clone();
        let grant_type = GrantType::AuthorizationCode;
        let redirect_uri = Some("https://hyperswitch.io".to_string());

        Ok(Self {
            code: Some(code),
            grant_type,
            redirect_uri,
            refresh_token: None, // We're not using refresh_token to generate new access_token
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, NordeaOAuthExchangeResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, NordeaOAuthExchangeResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        let access_token =
            item.response
                .access_token
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "access_token",
                })?;

        let expires_in = item.response.expires_in.unwrap_or(3600); // Default to 1 hour if not provided

        Ok(Self {
            status: common_enums::AttemptStatus::AuthenticationSuccessful,
            response: Ok(AccessToken {
                token: access_token.clone(),
                expires: expires_in,
            }),
            ..item.data
        })
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
            "standard" => Ok(Self::Standard),
            "express" => Ok(Self::Express),
            "sameday" => Ok(Self::Sameday),
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
            account_type: AccountType::try_from(metadata.account_type.as_str())
                .unwrap_or(AccountType::Iban),
            currency: router_data.request.currency,
            value: metadata.destination_account_number,
        },
        country: router_data.get_optional_billing_country(),
        // Merchant is the beneficiary in this case
        name: Some(metadata.merchant_name),
        message: router_data
            .description
            .as_ref()
            .map(|desc| desc.chars().take(20).collect::<String>()),
        bank: Some(CreditorBank {
            address: None,
            bank_code: None,
            bank_name: None,
            business_identifier_code: None,
            country: router_data.get_billing_country()?,
        }),
        creditor_address: None,
        // Either Reference or Message must be supplied in the request
        reference: None,
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
                        message: item
                            .router_data
                            .description
                            .as_ref()
                            .map(|desc| desc.chars().take(20).collect::<String>()),
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
                | BankDebitData::BecsBankDebit { .. }
                | BankDebitData::SepaGuarenteedBankDebit { .. } => {
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
            | NordeaPaymentStatus::PendingSecondConfirmation => Self::ConfirmationAwaited,
            NordeaPaymentStatus::PendingUserApproval => Self::AuthenticationPending,

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

// Helper function to convert NordeaPaymentsInitiateResponse to common response data
fn convert_nordea_payment_response(
    response: &NordeaPaymentsInitiateResponse,
) -> Result<(PaymentsResponseData, common_enums::AttemptStatus), Error> {
    let payment_response = response
        .payments_response
        .as_ref()
        .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

    let resource_id = ResponseId::ConnectorTransactionId(payment_response.payment_id.clone());

    let response_data = PaymentsResponseData::TransactionResponse {
        resource_id,
        redirection_data: Box::new(None),
        mandate_reference: Box::new(None),
        connector_metadata: None,
        network_txn_id: None,
        connector_response_reference_id: payment_response.external_id.clone(),
        incremental_authorization_allowed: None,
        charges: None,
    };

    let status = common_enums::AttemptStatus::from(payment_response.payment_status.clone());

    Ok((response_data, status))
}

impl TryFrom<PaymentsPreprocessingResponseRouterData<NordeaPaymentsInitiateResponse>>
    for RouterData<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: PaymentsPreprocessingResponseRouterData<NordeaPaymentsInitiateResponse>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = convert_nordea_payment_response(&item.response)?;
        Ok(Self {
            status,
            response: Ok(response),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            Authorize,
            NordeaPaymentsConfirmResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<
            Authorize,
            NordeaPaymentsConfirmResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        // First check if there are any errors in the response
        if let Some(errors) = &item.response.errors {
            if !errors.is_empty() {
                // Get the first error for the error response
                let first_error = errors
                    .first()
                    .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

                return Ok(Self {
                    status: common_enums::AttemptStatus::Failure,
                    response: Err(ErrorResponse {
                        code: first_error
                            .error
                            .clone()
                            .unwrap_or_else(|| "UNKNOWN_ERROR".to_string()),
                        message: first_error
                            .error_description
                            .clone()
                            .unwrap_or_else(|| "Payment confirmation failed".to_string()),
                        reason: first_error.error_description.clone(),
                        status_code: item.http_code,
                        attempt_status: Some(common_enums::AttemptStatus::Failure),
                        connector_transaction_id: first_error.payment_id.clone(),
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                });
            }
        }

        // If no errors, proceed with normal response handling
        // Check if there's a redirect link at the top level only
        let redirection_data = item
            .response
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

        let (response, status) = match &item.response.nordea_payments_response {
            Some(payment_response_wrapper) => {
                // Get the first payment from the payments array
                let payment = payment_response_wrapper
                    .payments
                    .first()
                    .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

                let resource_id = ResponseId::ConnectorTransactionId(payment.payment_id.clone());

                let response = Ok(PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data: Box::new(redirection_data),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: payment.external_id.clone(),
                    incremental_authorization_allowed: None,
                    charges: None,
                });

                let status = common_enums::AttemptStatus::from(payment.payment_status.clone());

                (response, status)
            }
            None => {
                // No payment response, but we might still have a redirect link
                if let Some(redirect) = redirection_data {
                    let response = Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::NoResponseId,
                        redirection_data: Box::new(Some(redirect)),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    });
                    (response, common_enums::AttemptStatus::AuthenticationPending)
                } else {
                    return Err(errors::ConnectorError::ResponseHandlingFailed.into());
                }
            }
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl TryFrom<PaymentsSyncResponseRouterData<NordeaPaymentsInitiateResponse>>
    for PaymentsSyncRouterData
{
    type Error = Error;
    fn try_from(
        item: PaymentsSyncResponseRouterData<NordeaPaymentsInitiateResponse>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = convert_nordea_payment_response(&item.response)?;
        Ok(Self {
            status,
            response: Ok(response),
            ..item.data
        })
    }
}
