use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self,RouterData as _},
};
use common_enums::enums;
use common_utils::types::StringMajorUnit;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, BankDebitData},
    router_data::{ConnectorAuthType, RouterData, AccessToken, PaymentMethodToken},
    router_flow_types::{
        refunds::{Execute, RSync}
    },
    router_request_types::ResponseId,
    router_response_types::{RefundsResponseData, PaymentsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
    types,
};
use hyperswitch_interfaces::errors;
use masking::{Secret, ExposeInterface};
use serde::{Deserialize, Serialize};
use error_stack::ResultExt;

pub struct DwollaAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for DwollaAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey {
                api_key,
                key1,
            } => Ok(Self {
                client_id: api_key.to_owned(),
                client_secret: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct DwollaAccessTokenRequest {
    pub grant_type: String,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct DwollaAccessTokenResponse {
    access_token: Secret<String>,
    expires_in: i64,
    token_type: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, DwollaAccessTokenResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, DwollaAccessTokenResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Debug)]
pub struct DwollaRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> TryFrom<(StringMajorUnit, T)> for DwollaRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, router_data): (StringMajorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data,
        })
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct DwollaCustomerRequest {
    #[serde(rename = "firstName")]
    first_name: Secret<String>,
    #[serde(rename = "lastName")]
    last_name: Secret<String>,
    email: common_utils::pii::Email,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaCustomerResponse {}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaFundingSourceResponse {}

#[derive(Debug, Serialize)]
pub struct DwollaFundingSourceRequest {
    #[serde(rename = "routingNumber")]
    routing_number: Secret<String>,
    #[serde(rename = "accountNumber")]
    account_number: Secret<String>,
    #[serde(rename = "type")]
    account_type: common_enums::BankType,
    name: Secret<String>,
}

#[derive(Default, Debug, Serialize, PartialEq, Deserialize, Clone)]
pub struct DwollaPaymentsRequest {
    #[serde(rename = "_links")]
    links : DwollaPaymentLinks,
    amount : DwollaAmount,
}

#[derive(Default, Debug, Serialize, PartialEq, Deserialize, Clone)]
pub struct DwollaPaymentLinks {
    source: DwollaRequestLink,
    destination: DwollaRequestLink,
}

#[derive(Default, Debug, Serialize, PartialEq, Deserialize, Clone)]
pub struct DwollaRequestLink {
    href : String,
}


#[derive(Default, Debug, Serialize, PartialEq, Deserialize, Clone)]
pub struct DwollaAmount {
    currency : common_enums::Currency,
    value : StringMajorUnit,
}

#[derive(Default, Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct DwollaPSyncResponse {
    id : String,
    status : DwollaPaymentStatus,
    amount : DwollaAmount,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DwollaMetaData {
    pub merchant_funding_source: Secret<String>,
}

impl TryFrom<&types::ConnectorCustomerRouterData> for DwollaCustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &types::ConnectorCustomerRouterData,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            first_name: item.request.name.clone().ok_or_else(|| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "first_name",
                }
            })?,
            last_name: item.request.name.clone().ok_or_else(|| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "last_name",
                }
            })?,
            email: item.request.email.clone().ok_or_else(|| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "email",
                }
            })?,
        })
    }
}

impl TryFrom<&types::TokenizationRouterData> for DwollaFundingSourceRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::BankDebit(bank_details) => match bank_details {
                BankDebitData::AchBankDebit { ref routing_number, ref account_number, ref bank_type, ref bank_account_holder_name, .. } => {
                    let account_type = bank_type.clone().ok_or_else(|| {
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "bank_type",
                        }
                    })?;

                    let name = bank_account_holder_name.clone().ok_or_else(|| {
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "bank_account_holder_name",
                        }
                    })?;

                    let request = Self {
                        routing_number: routing_number.clone(),
                        account_number: account_number.clone(),
                        account_type,
                        name,
                    };
                    Ok(request)
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("dwolla"),))?,
            },
            _ => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("dwolla"),
        ))?,
        }
    }
}

impl TryFrom<&DwollaRouterData<&PaymentsAuthorizeRouterData>> for DwollaPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DwollaRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let source_funding = match item.router_data.get_payment_method_token()? {
            PaymentMethodToken::Token(pm_token) => Ok(pm_token),
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_token",
            }),
        }?;

        let metadata = utils::to_connector_meta_from_secret::<DwollaMetaData>(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "metadata",
                })?;

        let source_url = format!(
            "https://api-sandbox.dwolla.com/funding-sources/{}",
            source_funding.expose()
        );

        let destination_url = format!(
            "https://api-sandbox.dwolla.com/funding-sources/{}",metadata.merchant_funding_source.expose()
        );

        let request = DwollaPaymentsRequest{
            links: DwollaPaymentLinks {
                source: DwollaRequestLink {
                    href: source_url
                },
                destination: DwollaRequestLink {
                    href: destination_url,
                },
            },
            amount: DwollaAmount {
                currency: item.router_data.request.currency,
                value: item.amount.to_owned(),
            },
        };

        Ok(request)
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, DwollaPSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, DwollaPSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let payment_id = item.response.id.clone();
        let status = DwollaPaymentStatus::from(item.response.status);

        Ok(Self {
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(payment_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(payment_id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            status: common_enums::AttemptStatus::from(status),
            ..item.data
        })
    }
}

// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DwollaPaymentStatus {
    Succeeded,
    Failed,
    Pending,
    #[default]
    Processing,
}

impl From<DwollaPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: DwollaPaymentStatus) -> Self {
        match item {
            DwollaPaymentStatus::Succeeded => Self::Charged,
            DwollaPaymentStatus::Failed => Self::Failure,
            DwollaPaymentStatus::Processing => Self::Authorizing,
            DwollaPaymentStatus::Pending => Self::Pending,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DwollaPaymentsResponse {}


//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct DwollaRefundRequest {
    pub amount: StringMajorUnit,
}

impl<F> TryFrom<&DwollaRouterData<&RefundsRouterData<F>>> for DwollaRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &DwollaRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
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
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
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

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaErrorResponse {
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
