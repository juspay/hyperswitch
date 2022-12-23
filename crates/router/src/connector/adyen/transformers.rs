use std::{collections::HashMap, str::FromStr};

use error_stack::{IntoReport, ResultExt};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    consts,
    core::errors,
    pii, services,
    types::{
        self,
        api::{self, enums as api_enums},
        storage::enums as storage_enums,
    },
    utils::OptionExt,
};

// Adyen Types Definition
// Payments Request and Response Types
#[derive(Default, Debug, Serialize, Deserialize)]
pub enum AdyenShopperInteraction {
    #[default]
    Ecommerce,
    #[serde(rename = "ContAuth")]
    ContinuedAuthentication,
    Moto,
    #[serde(rename = "POS")]
    Pos,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdyenRecurringModel {
    UnscheduledCardOnFile,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPaymentRequest {
    amount: Amount,
    merchant_account: String,
    payment_method: AdyenPaymentMethod,
    reference: String,
    return_url: String,
    browser_info: Option<AdyenBrowserInfo>,
    shopper_interaction: AdyenShopperInteraction,
    #[serde(skip_serializing_if = "Option::is_none")]
    recurring_processing_model: Option<AdyenRecurringModel>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdyenBrowserInfo {
    user_agent: String,
    accept_header: String,
    language: String,
    color_depth: u8,
    screen_height: u32,
    screen_width: u32,
    time_zone_offset: i32,
    java_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AdyenRedirectRequest {
    pub details: AdyenRedirectRequestTypes,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum AdyenRedirectRequestTypes {
    AdyenRedirection(AdyenRedirection),
    AdyenThreeDS(AdyenThreeDS),
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRedirection {
    #[serde(rename = "redirectResult")]
    pub redirect_result: String,
    #[serde(rename = "type")]
    pub type_of_redirection_result: Option<String>,
    pub result_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdyenThreeDS {
    #[serde(rename = "threeDSResult")]
    pub three_ds_result: String,
    #[serde(rename = "type")]
    pub type_of_redirection_result: Option<String>,
    pub result_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AdyenPaymentResponse {
    AdyenResponse(AdyenResponse),
    AdyenRedirectResponse(AdyenRedirectionResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenResponse {
    psp_reference: String,
    result_code: String,
    amount: Option<Amount>,
    merchant_reference: String,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRedirectionResponse {
    result_code: String,
    action: AdyenRedirectionAction,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRedirectionAction {
    payment_method_type: String,
    url: String,
    method: String,
    #[serde(rename = "type")]
    type_of_response: String,
    data: Option<HashMap<String, String>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Amount {
    currency: String,
    value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AdyenPaymentMethod {
    AdyenCard(AdyenCard),
    AdyenPaypal(AdyenPaypal),
    Gpay(AdyenGPay),
    ApplePay(AdyenApplePay),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCard {
    #[serde(rename = "type")]
    payment_type: String,
    number: Option<pii::Secret<String, pii::CardNumber>>,
    expiry_month: Option<pii::Secret<String>>,
    expiry_year: Option<pii::Secret<String>>,
    cvc: Option<pii::Secret<String>>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCancelRequest {
    merchant_account: String,
    original_reference: String,
    reference: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCancelResponse {
    merchant_account: String,
    psp_reference: String,
    response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPaypal {
    #[serde(rename = "type")]
    payment_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdyenGPay {
    #[serde(rename = "type")]
    payment_type: String,
    #[serde(rename = "googlePayToken")]
    google_pay_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdyenApplePay {
    #[serde(rename = "type")]
    payment_type: String,
    #[serde(rename = "applePayToken")]
    apple_pay_token: String,
}

// Refunds Request and Response
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRefundRequest {
    merchant_account: String,
    reference: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRefundResponse {
    merchant_account: String,
    psp_reference: String,
    payment_psp_reference: String,
    reference: String,
    status: String,
}

pub struct AdyenAuthType {
    pub(super) api_key: String,
    pub(super) merchant_account: String,
}

impl TryFrom<&types::ConnectorAuthType> for AdyenAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
                merchant_account: key1.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

impl TryFrom<&types::BrowserInformation> for AdyenBrowserInfo {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::BrowserInformation) -> Result<Self, Self::Error> {
        Ok(Self {
            accept_header: item.accept_header.clone(),
            language: item.language.clone(),
            screen_height: item.screen_height,
            screen_width: item.screen_width,
            color_depth: item.color_depth,
            user_agent: item.user_agent.clone(),
            time_zone_offset: item.time_zone,
            java_enabled: item.java_enabled,
        })
    }
}

// Payment Request Transform
impl TryFrom<&types::PaymentsAuthorizeRouterData> for AdyenPaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        let reference = item.payment_id.to_string();
        let amount = Amount {
            currency: item.request.currency.to_string(),
            value: item.request.amount,
        };
        let ccard = match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => Some(ccard),
            api::PaymentMethod::BankTransfer
            | api::PaymentMethod::Wallet(_)
            | api::PaymentMethod::PayLater(_)
            | api::PaymentMethod::Paypal => None,
        };

        let wallet_data = match item.request.payment_method_data {
            api::PaymentMethod::Wallet(ref wallet_data) => Some(wallet_data),
            _ => None,
        };

        let shopper_interaction = match item.request.off_session {
            Some(true) => AdyenShopperInteraction::ContinuedAuthentication,
            _ => AdyenShopperInteraction::Ecommerce,
        };

        let recurring_processing_model = match item.request.setup_future_usage {
            Some(storage_enums::FutureUsage::OffSession) => {
                Some(AdyenRecurringModel::UnscheduledCardOnFile)
            }
            _ => None,
        };

        let payment_type = match item.payment_method {
            storage_enums::PaymentMethodType::Card => "scheme".to_string(),
            storage_enums::PaymentMethodType::Paypal => "paypal".to_string(),
            storage_enums::PaymentMethodType::Wallet => wallet_data
                .get_required_value("issuer_name")
                .change_context(errors::ConnectorError::RequestEncodingFailed)?
                .issuer_name
                .to_string(),
            _ => "None".to_string(),
        };

        let payment_method = match item.payment_method {
            storage_enums::PaymentMethodType::Card => {
                let card = AdyenCard {
                    payment_type,
                    number: ccard.map(|x| x.card_number.clone()),
                    expiry_month: ccard.map(|x| x.card_exp_month.clone()),
                    expiry_year: ccard.map(|x| x.card_exp_year.clone()),
                    cvc: ccard.map(|x| x.card_cvc.clone()),
                };

                Ok(AdyenPaymentMethod::AdyenCard(card))
            }

            storage_enums::PaymentMethodType::Wallet => match wallet_data
                .get_required_value("issuer_name")
                .change_context(errors::ConnectorError::RequestEncodingFailed)?
                .issuer_name
            {
                api_enums::WalletIssuer::GooglePay => {
                    let gpay_data = AdyenGPay {
                        payment_type,
                        google_pay_token: wallet_data
                            .get_required_value("token")
                            .change_context(errors::ConnectorError::RequestEncodingFailed)?
                            .token
                            .to_string(),
                    };
                    Ok(AdyenPaymentMethod::Gpay(gpay_data))
                }

                api_enums::WalletIssuer::ApplePay => {
                    let apple_pay_data = AdyenApplePay {
                        payment_type,
                        apple_pay_token: wallet_data
                            .get_required_value("token")
                            .change_context(errors::ConnectorError::RequestEncodingFailed)?
                            .token
                            .to_string(),
                    };
                    Ok(AdyenPaymentMethod::ApplePay(apple_pay_data))
                }

                api_enums::WalletIssuer::Paypal => Err(errors::ConnectorError::NotImplemented(
                    "Adyen - Paypal".to_string(),
                )),
            },

            storage_enums::PaymentMethodType::Paypal => {
                let wallet = AdyenPaypal { payment_type };
                Ok(AdyenPaymentMethod::AdyenPaypal(wallet))
            }

            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method".to_string(),
            }),
        }?;

        let browser_info = if matches!(item.auth_type, storage_enums::AuthenticationType::ThreeDs) {
            item.request
                .browser_info
                .clone()
                .map(|d| AdyenBrowserInfo::try_from(&d))
                .transpose()?
        } else {
            None
        };

        Ok(Self {
            amount,
            merchant_account: auth_type.merchant_account,
            payment_method,
            reference,
            return_url: item.orca_return_url.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "orca_return_url".into(),
                },
            )?,
            shopper_interaction,
            recurring_processing_model,
            browser_info,
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for AdyenCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            merchant_account: auth_type.merchant_account,
            original_reference: item.request.connector_transaction_id.to_string(),
            reference: item.payment_id.to_string(),
        })
    }
}

impl TryFrom<types::PaymentsCancelResponseRouterData<AdyenCancelResponse>>
    for types::PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::PaymentsCancelResponseRouterData<AdyenCancelResponse>,
    ) -> Result<Self, Self::Error> {
        let status = match item.response.response.as_str() {
            "received" => storage_enums::AttemptStatus::Voided,
            "processing" => storage_enums::AttemptStatus::Pending,
            _ => storage_enums::AttemptStatus::VoidFailed,
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.psp_reference),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
            }),
            ..item.data
        })
    }
}

pub fn get_adyen_response(
    response: AdyenResponse,
) -> errors::CustomResult<
    (
        storage_enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ParsingError,
> {
    let result = response.result_code;
    let status = match result.as_str() {
        "Authorised" => storage_enums::AttemptStatus::Charged,
        "Refused" => storage_enums::AttemptStatus::Failure,
        _ => storage_enums::AttemptStatus::Pending,
    };
    let error = if response.refusal_reason.is_some() || response.refusal_reason_code.is_some() {
        Some(types::ErrorResponse {
            code: response
                .refusal_reason_code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .refusal_reason
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
        })
    } else {
        None
    };

    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::ConnectorTransactionId(response.psp_reference),
        redirection_data: None,
        redirect: false,
        mandate_reference: None,
    };
    Ok((status, error, payments_response_data))
}

pub fn get_redirection_response(
    response: AdyenRedirectionResponse,
) -> errors::CustomResult<
    (
        storage_enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ParsingError,
> {
    let result = response.result_code;
    let status = match result.as_str() {
        "Authorised" => storage_enums::AttemptStatus::Charged,
        "Refused" => storage_enums::AttemptStatus::Failure,
        "Cancelled" => storage_enums::AttemptStatus::Failure,
        "RedirectShopper" => storage_enums::AttemptStatus::AuthenticationPending,
        _ => storage_enums::AttemptStatus::Pending,
    };

    let error = if response.refusal_reason.is_some() || response.refusal_reason_code.is_some() {
        Some(types::ErrorResponse {
            code: response
                .refusal_reason_code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .refusal_reason
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
        })
    } else {
        None
    };

    let redirection_url_response = Url::parse(&response.action.url)
        .into_report()
        .change_context(errors::ParsingError)
        .attach_printable("Failed to parse redirection url")?;

    let form_field_for_redirection = match response.action.data {
        Some(data) => data,
        None => std::collections::HashMap::from_iter(
            redirection_url_response
                .query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        ),
    };

    let redirection_data = services::RedirectForm {
        url: redirection_url_response.to_string(),
        method: services::Method::from_str(&response.action.method)
            .into_report()
            .change_context(errors::ParsingError)?,
        form_fields: form_field_for_redirection,
    };

    // We don't get connector transaction id for redirections in Adyen.
    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::NoResponseId,
        redirection_data: Some(redirection_data),
        redirect: true,
        mandate_reference: None,
    };
    Ok((status, error, payments_response_data))
}

impl<F, Req>
    TryFrom<types::ResponseRouterData<F, AdyenPaymentResponse, Req, types::PaymentsResponseData>>
    for types::RouterData<F, Req, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::ResponseRouterData<F, AdyenPaymentResponse, Req, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (status, error, payment_response_data) = match item.response {
            AdyenPaymentResponse::AdyenResponse(response) => get_adyen_response(response)?,
            AdyenPaymentResponse::AdyenRedirectResponse(response) => {
                get_redirection_response(response)?
            }
        };

        Ok(Self {
            status,
            response: error.map_or_else(|| Ok(payment_response_data), Err),

            ..item.data
        })
    }
}

/*
// This is a repeated code block from Stripe inegration. Can we avoid the repetition in every integration
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdyenPaymentStatus {
    Succeeded,
    Failed,
    Processing,
    RequiresCustomerAction,
    RequiresPaymentMethod,
    RequiresConfirmation,
}

// Default always be Processing
impl Default for AdyenPaymentStatus {
    fn default() -> Self {
        AdyenPaymentStatus::Processing
    }
}

impl From<AdyenPaymentStatus> for enums::Status {
    fn from(item: AdyenPaymentStatus) -> Self {
        match item {
            AdyenPaymentStatus::Succeeded => enums::Status::Charged,
            AdyenPaymentStatus::Failed => enums::Status::Failure,
            AdyenPaymentStatus::Processing
            | AdyenPaymentStatus::RequiresCustomerAction
            | AdyenPaymentStatus::RequiresPaymentMethod
            | AdyenPaymentStatus::RequiresConfirmation => enums::Status::Pending,
        }
    }
}
*/
// Refund Request Transform
impl<F> TryFrom<&types::RefundsRouterData<F>> for AdyenRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            merchant_account: auth_type.merchant_account,
            reference: item.request.refund_id.clone(),
        })
    }
}

// Refund Response Transform
impl<F> TryFrom<types::RefundsResponseRouterData<F, AdyenRefundResponse>>
    for types::RefundsRouterData<F>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<F, AdyenRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.status.as_str() {
            // From the docs, the only value returned is "received", outcome of refund is available
            // through refund notification webhook
            "received" => storage_enums::RefundStatus::Success,
            _ => storage_enums::RefundStatus::Pending,
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.reference,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub status: i32,
    pub error_code: String,
    pub message: String,
    pub error_type: String,
    pub psp_reference: Option<String>,
}

// #[cfg(test)]
// mod test_adyen_transformers {
//     use super::*;

//     #[test]
//     fn verify_tranform_from_router_to_adyen_req() {
//         let router_req = PaymentsRequest {
//             amount: 0.0,
//             currency: "None".to_string(),
//             ..Default::default()
//         };
//         println!("{:#?}", &router_req);
//         let adyen_req = AdyenPaymentRequest::from(router_req);
//         println!("{:#?}", &adyen_req);
//         let adyen_req_json: String = serde_json::to_string(&adyen_req).unwrap();
//         println!("{}", adyen_req_json);
//         assert_eq!(true, true)
//     }
// }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenAdditionalDataWH {
    pub hmac_signature: String,
}

#[derive(Debug, Deserialize)]
pub struct AdyenAmountWH {
    pub value: i64,
    pub currency: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenNotificationRequestItemWH {
    pub additional_data: AdyenAdditionalDataWH,
    pub amount: AdyenAmountWH,
    pub original_reference: Option<String>,
    pub psp_reference: String,
    pub event_code: String,
    pub merchant_account_code: String,
    pub merchant_reference: String,
    pub success: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdyenItemObjectWH {
    pub notification_request_item: AdyenNotificationRequestItemWH,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenIncomingWebhook {
    pub notification_items: Vec<AdyenItemObjectWH>,
}

impl From<AdyenNotificationRequestItemWH> for AdyenResponse {
    fn from(notif: AdyenNotificationRequestItemWH) -> Self {
        Self {
            psp_reference: notif.psp_reference,
            merchant_reference: notif.merchant_reference,
            result_code: String::from(match notif.success.as_str() {
                "true" => "Authorised",
                _ => "Refused",
            }),
            amount: Some(Amount {
                value: notif.amount.value,
                currency: notif.amount.currency,
            }),
            refusal_reason: None,
            refusal_reason_code: None,
        }
    }
}
