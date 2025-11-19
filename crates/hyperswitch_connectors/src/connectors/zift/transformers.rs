use api_models::payments::AdditionalPaymentData;
use common_enums::{enums, CountryAlpha2};
use common_utils::{pii, types::StringMinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{refunds::Execute, PSync},
    router_request_types::{PaymentsAuthorizeData, PaymentsSyncData, ResponseId},
    router_response_types::{MandateReference, PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{
        PaymentsCancelResponseRouterData, PaymentsCaptureResponseRouterData,
        RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{AdditionalCardInfo, CardData, PaymentsAuthorizeRequestData, RouterData as _},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZiftAuthType {
    user_name: Secret<String>,
    password: Secret<String>,
    account_id: Secret<String>,
}
impl TryFrom<&ConnectorAuthType> for ZiftAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                user_name: api_key.to_owned(),
                password: api_secret.to_owned(),
                account_id: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

pub struct ZiftRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for ZiftRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RequestType {
    Sale,
    #[serde(rename = "sale-auth")]
    Auth,
    Capture,
    Refund,
    Find,
    Void,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentRequestType {
    Sale,
    #[serde(rename = "sale-auth")]
    Auth,
    Capture,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum AccountType {
    #[serde(rename = "R")]
    PaymentCard,
    #[serde(rename = "S")]
    Savings,
    #[serde(rename = "C")]
    Checking,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionIndustryType {
    #[serde(rename = "DM")]
    CardNotPresent,
    #[serde(rename = "RE")]
    CardPresent,
    #[serde(rename = "RS")]
    Restaurant,
    #[serde(rename = "LD")]
    Lodging,
    #[serde(rename = "PT")]
    Petroleum,
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum HolderType {
    #[serde(rename = "P")]
    Personal,
    #[serde(rename = "O")]
    Organizational,
}
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ZiftPaymentsRequest {
    Card(ZiftCardPaymentRequest),
    Mandate(ZiftMandatePaymentRequest),
    ExternalThreeDs(ZiftExternalThreeDsPaymentRequest),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZiftCardPaymentRequest {
    request_type: RequestType,
    #[serde(flatten)]
    auth: ZiftAuthType,
    account_type: AccountType,
    account_number: cards::CardNumber,
    account_accessory: Secret<String>,
    csc: Secret<String>,
    transaction_industry_type: TransactionIndustryType,
    holder_name: Secret<String>,
    holder_type: HolderType,
    amount: StringMinorUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    street: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zip_code: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    country_code: Option<CountryAlpha2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<pii::Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phone: Option<Secret<String>>,
}
// Mandate payment (MIT - Merchant Initiated)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZiftMandatePaymentRequest {
    request_type: RequestType,
    #[serde(flatten)]
    auth: ZiftAuthType,
    account_type: AccountType,
    token: Secret<String>,
    account_accessory: Secret<String>,
    // NO csc for MIT payments
    transaction_industry_type: TransactionIndustryType,
    holder_name: Secret<String>,
    holder_type: HolderType,
    amount: StringMinorUnit,
    transaction_mode_type: TransactionModeType,

    // Required for MIT
    transaction_category_type: TransactionCategoryType,
    sequence_number: i32,
    // Billing address
    #[serde(skip_serializing_if = "Option::is_none")]
    street: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zip_code: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    country_code: Option<CountryAlpha2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<pii::Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phone: Option<Secret<String>>,
}

// External 3DS payment request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZiftExternalThreeDsPaymentRequest {
    request_type: RequestType,
    #[serde(flatten)]
    auth: ZiftAuthType,
    account_type: AccountType,
    account_number: cards::CardNumber,
    account_accessory: Secret<String>,
    transaction_industry_type: TransactionIndustryType,
    holder_name: Secret<String>,
    holder_type: HolderType,
    amount: StringMinorUnit,
    // 3DS authentication fields
    authentication_status: AuthenticationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    authentication_code: Option<Secret<String>>,
    authentication_verification_value: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authentication_version: Option<Secret<String>>,
    // Billing address
    #[serde(skip_serializing_if = "Option::is_none")]
    street: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zip_code: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    country_code: Option<CountryAlpha2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<pii::Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phone: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
pub enum AuthenticationStatus {
    #[serde(rename = "Y")]
    Success,
    #[serde(rename = "A")]
    Attempted,
    #[serde(rename = "U")]
    Unavailable,
}

impl TryFrom<&hyperswitch_domain_models::router_request_types::AuthenticationData>
    for AuthenticationStatus
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        auth_data: &hyperswitch_domain_models::router_request_types::AuthenticationData,
    ) -> Result<Self, Self::Error> {
        // Map authentication status based on trans_status field
        let authentication_status = match auth_data.transaction_status {
            Some(common_enums::TransactionStatus::Success) => Self::Success,
            Some(common_enums::TransactionStatus::NotVerified) => Self::Attempted,
            Some(common_enums::TransactionStatus::VerificationNotPerformed)
            | Some(common_enums::TransactionStatus::Rejected)
            | Some(common_enums::TransactionStatus::InformationOnly)
            | Some(common_enums::TransactionStatus::Failure)
            | Some(common_enums::TransactionStatus::ChallengeRequired)
            | Some(common_enums::TransactionStatus::ChallengeRequiredDecoupledAuthentication)
            | None => Self::Unavailable,
        };
        Ok(authentication_status)
    }
}

#[derive(Debug, Serialize)]
pub enum TransactionModeType {
    #[serde(rename = "P")]
    CardPresent,
    #[serde(rename = "N")]
    CardNotPresent,
}
#[derive(Debug, Serialize)]
pub enum TransactionCategoryType {
    #[serde(rename = "R")]
    Recurring,
    #[serde(rename = "I")]
    Installment,
    #[serde(rename = "B")]
    BillPayment,
}

impl TryFrom<&ZiftRouterData<&PaymentsAuthorizeRouterData>> for ZiftPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ZiftRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        let auth = ZiftAuthType::try_from(&item.router_data.connector_auth_type)?;
        let request_type = if item.router_data.request.is_auto_capture()? {
            RequestType::Sale
        } else {
            RequestType::Auth
        };
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(card) => {
                // Check if this is an external 3DS payment - both is_three_ds() and authentication_data must be present
                if item.router_data.is_three_ds()
                    && item.router_data.request.authentication_data.is_some()
                {
                    // Handle external 3DS authentication
                    let auth_data = item
                        .router_data
                        .request
                        .authentication_data
                        .as_ref()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "authentication_data",
                        })?;

                    let authentication_status = AuthenticationStatus::try_from(auth_data)?;

                    let external_3ds_request = ZiftExternalThreeDsPaymentRequest {
                        request_type,
                        auth,
                        account_number: card.card_number.clone(),
                        account_accessory: card.get_expiry_date_as_mmyy()?,
                        transaction_industry_type: TransactionIndustryType::CardNotPresent,
                        holder_name: item.router_data.get_billing_full_name()?,
                        amount: item.amount.to_owned(),
                        account_type: AccountType::PaymentCard,
                        holder_type: HolderType::Personal,
                        authentication_status,
                        authentication_code: auth_data.ds_trans_id.clone().map(Secret::new),
                        authentication_verification_value: auth_data.cavv.clone(),
                        authentication_version: auth_data
                            .message_version
                            .as_ref()
                            .map(|v| Secret::new(v.to_string())),
                        street: item.router_data.get_optional_billing_line1(),
                        city: item.router_data.get_optional_billing_city(),
                        state: item.router_data.get_optional_billing_state(),
                        zip_code: item.router_data.get_optional_billing_zip(),
                        country_code: item.router_data.get_optional_billing_country(),
                        email: item.router_data.get_optional_billing_email(),
                        phone: item.router_data.get_optional_billing_phone_number(),
                    };
                    Ok(Self::ExternalThreeDs(external_3ds_request))
                } else {
                    let card_request = ZiftCardPaymentRequest {
                        request_type,
                        auth,
                        account_number: card.card_number.clone(),
                        account_accessory: card.get_expiry_date_as_mmyy()?,
                        transaction_industry_type: TransactionIndustryType::CardPresent,
                        holder_name: item.router_data.get_billing_full_name()?,
                        amount: item.amount.to_owned(),
                        account_type: AccountType::PaymentCard,
                        holder_type: HolderType::Personal,
                        csc: card.card_cvc,
                        street: item.router_data.get_optional_billing_line1(),
                        city: item.router_data.get_optional_billing_city(),
                        state: item.router_data.get_optional_billing_state(),
                        zip_code: item.router_data.get_optional_billing_zip(),
                        country_code: item.router_data.get_optional_billing_country(),
                        email: item.router_data.get_optional_billing_email(),
                        phone: item.router_data.get_optional_billing_phone_number(),
                    };
                    Ok(Self::Card(card_request))
                }
            }
            PaymentMethodData::MandatePayment => {
                let additional_card_details = match item
                    .router_data
                    .request
                    .additional_payment_method_data
                    .clone()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "additional_payment_method_data",
                    })? {
                    AdditionalPaymentData::Card(card) => *card,
                    _ => Err(errors::ConnectorError::NotSupported {
                        message: "Payment Method Not Supported".to_string(),
                        connector: "DataTrans",
                    })?,
                };
                let mandate_request = ZiftMandatePaymentRequest {
                    request_type,
                    auth,
                    account_type: AccountType::PaymentCard,
                    token: Secret::new(item.router_data.request.connector_mandate_id().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "connector_mandate_id",
                        },
                    )?),
                    account_accessory: additional_card_details.get_expiry_date_as_mmyy()?,
                    transaction_industry_type: TransactionIndustryType::CardNotPresent,
                    holder_name: additional_card_details.get_card_holder_name()?,
                    holder_type: HolderType::Personal,
                    amount: item.amount.to_owned(),
                    transaction_mode_type: TransactionModeType::CardNotPresent,
                    transaction_category_type: TransactionCategoryType::Recurring,
                    sequence_number: 2, // Its required for MIT
                    street: item.router_data.get_optional_billing_line1(),
                    city: item.router_data.get_optional_billing_city(),
                    state: item.router_data.get_optional_billing_state(),
                    zip_code: item.router_data.get_optional_billing_zip(),
                    country_code: item.router_data.get_optional_billing_country(),
                    email: item.router_data.get_optional_billing_email(),
                    phone: item.router_data.get_optional_billing_phone_number(),
                };
                Ok(Self::Mandate(mandate_request))
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

pub trait ResponseCodeExt {
    fn is_pending(&self) -> bool;
    fn is_approved(&self) -> bool;
    fn is_failed(&self) -> bool;
}

impl ResponseCodeExt for String {
    fn is_pending(&self) -> bool {
        self == "X02"
    }
    fn is_approved(&self) -> bool {
        self.starts_with('A')
    }
    fn is_failed(&self) -> bool {
        !(self.is_approved() || self.is_pending())
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZiftErrorResponse {
    pub response_code: String,
    pub response_message: String,
    pub failure_code: String,
    pub failure_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZiftAuthPaymentsResponse {
    pub response_code: String,
    pub response_message: String,
    pub transaction_id: Option<i64>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZiftCaptureResponse {
    pub response_code: String,
    pub response_message: String,
}

impl TryFrom<PaymentsCaptureResponseRouterData<ZiftCaptureResponse>> for PaymentsCaptureRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: PaymentsCaptureResponseRouterData<ZiftCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let capture_response = &item.response;

        match capture_response.response_code.is_approved() {
            true => Ok(Self {
                status: common_enums::AttemptStatus::Charged,
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
                ..item.data
            }),

            false => Ok(Self {
                status: common_enums::AttemptStatus::CaptureFailed,
                response: Err(ErrorResponse {
                    code: capture_response.response_code.clone(),
                    message: capture_response.response_message.clone(),
                    reason: Some(capture_response.response_message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            }),
        }
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            ZiftAuthPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            ZiftAuthPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = if item.response.response_code.is_approved() {
            if item.data.request.is_auto_capture()? {
                common_enums::AttemptStatus::Charged
            } else {
                common_enums::AttemptStatus::Authorized
            }
        } else if item.response.response_code.is_pending() {
            common_enums::AttemptStatus::Pending
        } else {
            common_enums::AttemptStatus::Failure
        };
        if status != common_enums::AttemptStatus::Failure {
            let mandate_reference: Box<Option<MandateReference>> =
                if item.data.request.is_customer_initiated_mandate_payment() {
                    Box::new(item.response.token.clone().map(|token| MandateReference {
                        connector_mandate_id: Some(token),
                        payment_method_id: None,
                        mandate_metadata: None,
                        connector_mandate_request_reference_id: None,
                    }))
                } else {
                    Box::new(None)
                };

            let transaction_id = item.response.transaction_id.ok_or_else(|| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "transaction_id",
                }
            })?;

            Ok(Self {
                status,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(transaction_id.to_string()),
                    redirection_data: Box::new(None),
                    mandate_reference,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: item.response.response_code.clone(),
                    message: item.response.response_message.clone(),
                    reason: Some(item.response.response_message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: item.response.transaction_id.map(|id| id.to_string()),
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            })
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZiftRefundRequest {
    request_type: RequestType,
    #[serde(flatten)]
    auth: ZiftAuthType,
    transaction_id: String,
    amount: StringMinorUnit,
    transaction_code: String,
}

impl<F> TryFrom<&ZiftRouterData<&RefundsRouterData<F>>> for ZiftRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ZiftRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth = ZiftAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            request_type: RequestType::Refund,
            auth,
            transaction_id: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount.to_owned(),
            transaction_code: item.router_data.request.refund_id.clone(),
        })
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    transaction_id: Option<String>,
    response_code: String,
    response_message: Option<String>,
    transaction_code: Option<String>,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_response = &item.response;

        let refund_status = if refund_response.response_code.is_approved() {
            enums::RefundStatus::Success
        } else if refund_response.response_code.is_pending() {
            enums::RefundStatus::Pending
        } else {
            enums::RefundStatus::Failure
        };
        let response = if refund_response.response_code.is_approved() {
            Ok(RefundsResponseData {
                connector_refund_id: item
                    .response
                    .transaction_id
                    .clone()
                    .or(item.response.transaction_code.clone())
                    .ok_or(errors::ConnectorError::MissingConnectorRefundID)?,
                refund_status,
            })
        } else {
            Err(ErrorResponse {
                code: refund_response.response_code.clone(),
                message: refund_response
                    .response_message
                    .clone()
                    .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                reason: refund_response.response_message.clone(),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

// impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(
//         item: RefundsResponseRouterData<RSync, RefundResponse>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             response: Ok(RefundsResponseData {
//                 connector_refund_id: item.response.transaction_id.to_string(),
//                 refund_status: enums::RefundStatus::from(item.response.response_code),
//             }),
//             ..item.data
//         })
//     }
// }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    #[serde(rename = "N")]
    Pending,
    #[serde(rename = "P")]
    Processed,
    #[serde(rename = "C")]
    Cancelled,
    #[serde(rename = "R")]
    InRebill,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZiftSyncRequest {
    request_type: RequestType,
    #[serde(flatten)]
    auth: ZiftAuthType,
    transaction_id: i64,
}
impl TryFrom<&PaymentsSyncRouterData> for ZiftSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = ZiftAuthType::try_from(&item.connector_auth_type)?;
        let transaction_id = item
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;

        Ok(Self {
            request_type: RequestType::Find,
            auth,
            transaction_id: transaction_id
                .parse::<i64>()
                .map_err(|_| errors::ConnectorError::ResponseDeserializationFailed)?,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZiftSyncResponse {
    pub transaction_status: TransactionStatus,
    pub transaction_type: PaymentRequestType,
    pub response_message: Option<String>,
    pub response_code: Option<String>,
}

impl TryFrom<ResponseRouterData<PSync, ZiftSyncResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<PSync, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<PSync, ZiftSyncResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let attempt_status = match item.response.transaction_type {
            // Sale transactions
            PaymentRequestType::Sale => match item.response.transaction_status {
                TransactionStatus::Processed => common_enums::AttemptStatus::Charged,
                TransactionStatus::Pending | TransactionStatus::InRebill => {
                    common_enums::AttemptStatus::Pending
                }
                TransactionStatus::Cancelled => common_enums::AttemptStatus::Failure,
            },

            // Auth transactions (sale-auth)
            PaymentRequestType::Auth => match item.response.transaction_status {
                TransactionStatus::Processed => common_enums::AttemptStatus::Authorized,
                TransactionStatus::Pending | TransactionStatus::InRebill => {
                    common_enums::AttemptStatus::Pending
                }
                TransactionStatus::Cancelled => common_enums::AttemptStatus::Failure,
            },

            // Capture transactions
            PaymentRequestType::Capture => match item.response.transaction_status {
                TransactionStatus::Processed => common_enums::AttemptStatus::Charged,
                TransactionStatus::Pending | TransactionStatus::InRebill => {
                    common_enums::AttemptStatus::CaptureInitiated
                }
                TransactionStatus::Cancelled => common_enums::AttemptStatus::CaptureFailed,
            },
        };
        let response = if attempt_status == common_enums::AttemptStatus::Failure {
            Err(ErrorResponse {
                code: item
                    .response
                    .response_code
                    .clone()
                    .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .response_message
                    .clone()
                    .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                reason: item.response.response_message,
                status_code: item.http_code,
                attempt_status: Some(attempt_status),
                connector_transaction_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            })
        };

        Ok(Self {
            status: attempt_status,
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZiftCaptureRequest {
    request_type: RequestType,
    #[serde(flatten)]
    auth: ZiftAuthType,
    transaction_id: i64,
    amount: StringMinorUnit,
}

impl TryFrom<&ZiftRouterData<&PaymentsCaptureRouterData>> for ZiftCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ZiftRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let auth = ZiftAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            request_type: RequestType::Capture,
            auth,
            transaction_id: item
                .router_data
                .request
                .connector_transaction_id
                .parse::<i64>()
                .map_err(|_| errors::ConnectorError::ResponseDeserializationFailed)?,
            amount: item.amount.to_owned(),
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZiftCancelRequest {
    request_type: RequestType,
    #[serde(flatten)]
    auth: ZiftAuthType,
    transaction_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZiftVoidResponse {
    pub response_code: String,
    pub response_message: String,
}

impl TryFrom<&PaymentsCancelRouterData> for ZiftCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = ZiftAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            request_type: RequestType::Void,
            auth,
            transaction_id: item
                .request
                .connector_transaction_id
                .parse::<i64>()
                .map_err(|_| errors::ConnectorError::ResponseDeserializationFailed)?,
        })
    }
}

impl TryFrom<PaymentsCancelResponseRouterData<ZiftVoidResponse>> for PaymentsCancelRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: PaymentsCancelResponseRouterData<ZiftVoidResponse>,
    ) -> Result<Self, Self::Error> {
        let void_response = &item.response;

        let response = if void_response.response_code.is_approved() {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            })
        } else {
            Err(ErrorResponse {
                code: void_response.response_code.to_string(),
                message: void_response.response_message.clone(),
                reason: Some(void_response.response_message.clone()),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        };

        Ok(Self {
            status: if void_response.response_code.is_approved() {
                common_enums::AttemptStatus::Voided
            } else {
                common_enums::AttemptStatus::Failure
            },
            response,
            ..item.data
        })
    }
}
