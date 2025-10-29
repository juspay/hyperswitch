use common_utils::{
    crypto::{self, GenerateDigest},
    errors::ParsingError,
    pii,
    request::Method,
    types::{AmountConvertor, MinorUnit, StringMinorUnit, StringMinorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, PaymentMethodToken, RouterData},
    router_flow_types::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{
        MandateReference, PaymentsResponseData, RedirectForm, RefundsResponseData,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefreshTokenRouterData, RefundExecuteRouterData, RefundsRouterData,
        TokenizationRouterData,
    },
};
use hyperswitch_interfaces::{
    consts::{self, NO_ERROR_MESSAGE},
    errors,
};
use masking::{ExposeInterface, PeekInterface, Secret};
use rand::distributions::DistString;
use serde::{Deserialize, Serialize};
use url::Url;

use super::{
    requests::{
        self, ApmProvider, GlobalPayRouterData, GlobalpayCancelRouterData,
        GlobalpayPaymentsRequest, GlobalpayRefreshTokenRequest, Initiator, PaymentMethodData,
        Sequence, StoredCredential,
    },
    response::{GlobalpayPaymentStatus, GlobalpayPaymentsResponse, GlobalpayRefreshTokenResponse},
};
use crate::{
    connectors::globalpay::{
        requests::CommonPaymentMethodData, response::GlobalpayPaymentMethodsResponse,
    },
    types::{PaymentsSyncResponseRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, construct_captures_response_hashmap, CardData, ForeignTryFrom,
        MultipleCaptureSyncResponse, PaymentMethodTokenizationRequestData,
        PaymentsAuthorizeRequestData, RouterData as _, WalletData,
    },
};

impl<T> From<(StringMinorUnit, T)> for GlobalPayRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

impl<T> From<(Option<StringMinorUnit>, T)> for GlobalpayCancelRouterData<T> {
    fn from((amount, item): (Option<StringMinorUnit>, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalPayMeta {
    account_name: Secret<String>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for GlobalPayMeta {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

impl TryFrom<&GlobalPayRouterData<&PaymentsAuthorizeRouterData>> for GlobalpayPaymentsRequest {
    type Error = Error;

    fn try_from(
        item: &GlobalPayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            return Err(errors::ConnectorError::NotSupported {
                message: "3DS flow".to_string(),
                connector: "Globalpay",
            }
            .into());
        }

        let metadata = GlobalPayMeta::try_from(&item.router_data.connector_meta_data)?;
        let account_name = metadata.account_name;

        let (initiator, stored_credential, connector_mandate_id) =
            if item.router_data.request.is_mandate_payment() {
                let connector_mandate_id =
                    item.router_data
                        .request
                        .mandate_id
                        .as_ref()
                        .and_then(|mandate_ids| match &mandate_ids.mandate_reference_id {
                            Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                                connector_mandate_ids,
                            )) => connector_mandate_ids.get_connector_mandate_id(),
                            _ => None,
                        });

                let initiator = Some(match item.router_data.request.off_session {
                    Some(true) => Initiator::Merchant,
                    _ => Initiator::Payer,
                });

                let stored_credential = Some(StoredCredential {
                    model: Some(if connector_mandate_id.is_some() {
                        requests::Model::Recurring
                    } else {
                        requests::Model::Unscheduled
                    }),
                    sequence: Some(if connector_mandate_id.is_some() {
                        Sequence::Subsequent
                    } else {
                        Sequence::First
                    }),
                });

                (initiator, stored_credential, connector_mandate_id)
            } else {
                (None, None, None)
            };

        let payment_method = match &item.router_data.request.payment_method_data {
            payment_method_data::PaymentMethodData::Card(ccard) => {
                requests::GlobalPayPaymentMethodData::Common(CommonPaymentMethodData {
                    payment_method_data: PaymentMethodData::Card(requests::Card {
                        number: ccard.card_number.clone(),
                        expiry_month: ccard.card_exp_month.clone(),
                        expiry_year: ccard.get_card_expiry_year_2_digit()?,
                        cvv: ccard.card_cvc.clone(),
                    }),
                    entry_mode: Default::default(),
                })
            }

            payment_method_data::PaymentMethodData::Wallet(wallet_data) => {
                requests::GlobalPayPaymentMethodData::Common(CommonPaymentMethodData {
                    payment_method_data: get_wallet_data(wallet_data)?,
                    entry_mode: Default::default(),
                })
            }

            payment_method_data::PaymentMethodData::BankRedirect(bank_redirect) => {
                requests::GlobalPayPaymentMethodData::Common(CommonPaymentMethodData {
                    payment_method_data: PaymentMethodData::try_from(bank_redirect)?,
                    entry_mode: Default::default(),
                })
            }

            payment_method_data::PaymentMethodData::MandatePayment => {
                requests::GlobalPayPaymentMethodData::Mandate(requests::MandatePaymentMethodData {
                    entry_mode: Default::default(),
                    id: connector_mandate_id,
                })
            }

            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment methods".to_string(),
            ))?,
        };

        Ok(Self {
            account_name,
            amount: Some(item.amount.to_owned()),
            currency: item.router_data.request.currency.to_string(),
            reference: item.router_data.connector_request_reference_id.to_string(),
            country: item.router_data.get_billing_country()?,
            capture_mode: Some(requests::CaptureMode::from(
                item.router_data.request.capture_method,
            )),
            payment_method,
            notifications: Some(requests::Notifications {
                return_url: get_return_url(item.router_data),
                status_url: item.router_data.request.webhook_url.clone(),
                cancel_url: get_return_url(item.router_data),
            }),
            stored_credential,
            channel: Default::default(),
            initiator,
        })
    }
}

impl TryFrom<&GlobalPayRouterData<&PaymentsCaptureRouterData>>
    for requests::GlobalpayCaptureRequest
{
    type Error = Error;
    fn try_from(
        value: &GlobalPayRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: Some(value.amount.to_owned()),
            capture_sequence: value
                .router_data
                .request
                .multiple_capture_data
                .clone()
                .map(|mcd| {
                    if mcd.capture_sequence == 1 {
                        Sequence::First
                    } else {
                        Sequence::Subsequent
                    }
                }),
            reference: value
                .router_data
                .request
                .multiple_capture_data
                .as_ref()
                .map(|mcd| mcd.capture_reference.clone()),
        })
    }
}

impl TryFrom<&TokenizationRouterData> for requests::GlobalPayPaymentMethodsRequest {
    type Error = Error;
    fn try_from(item: &TokenizationRouterData) -> Result<Self, Self::Error> {
        if !item.request.is_mandate_payment() {
            return Err(errors::ConnectorError::FlowNotSupported {
                flow: "Tokenization apart from Mandates".to_string(),
                connector: "Globalpay".to_string(),
            }
            .into());
        }
        Ok(Self {
            reference: item.connector_request_reference_id.clone(),
            usage_mode: Some(requests::UsageMode::Multiple),
            card: match &item.request.payment_method_data {
                payment_method_data::PaymentMethodData::Card(card_data) => Some(requests::Card {
                    number: card_data.card_number.clone(),
                    expiry_month: card_data.card_exp_month.clone(),
                    expiry_year: card_data.get_card_expiry_year_2_digit()?,
                    cvv: card_data.card_cvc.clone(),
                }),
                _ => None,
            },
        })
    }
}

impl TryFrom<&GlobalpayCancelRouterData<&PaymentsCancelRouterData>>
    for requests::GlobalpayCancelRequest
{
    type Error = Error;
    fn try_from(
        value: &GlobalpayCancelRouterData<&PaymentsCancelRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: value.amount.clone(),
        })
    }
}

pub struct GlobalpayAuthType {
    pub app_id: Secret<String>,
    pub key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for GlobalpayAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                app_id: key1.to_owned(),
                key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl TryFrom<GlobalpayRefreshTokenResponse> for AccessToken {
    type Error = error_stack::Report<ParsingError>;

    fn try_from(item: GlobalpayRefreshTokenResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            token: item.token,
            expires: item.seconds_to_expire,
        })
    }
}

impl TryFrom<&RefreshTokenRouterData> for GlobalpayRefreshTokenRequest {
    type Error = Error;

    fn try_from(item: &RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let globalpay_auth = GlobalpayAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)
            .attach_printable("Could not convert connector_auth to globalpay_auth")?;

        let nonce = rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 12);
        let nonce_with_api_key = format!("{}{}", nonce, globalpay_auth.key.peek());
        let secret_vec = crypto::Sha512
            .generate_digest(nonce_with_api_key.as_bytes())
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("error creating request nonce")?;

        let secret = hex::encode(secret_vec);

        Ok(Self {
            app_id: globalpay_auth.app_id,
            nonce: Secret::new(nonce),
            secret: Secret::new(secret),
            grant_type: "client_credentials".to_string(),
        })
    }
}

impl From<GlobalpayPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: GlobalpayPaymentStatus) -> Self {
        match item {
            GlobalpayPaymentStatus::Captured | GlobalpayPaymentStatus::Funded => Self::Charged,
            GlobalpayPaymentStatus::Declined | GlobalpayPaymentStatus::Rejected => Self::Failure,
            GlobalpayPaymentStatus::Preauthorized => Self::Authorized,
            GlobalpayPaymentStatus::Reversed => Self::Voided,
            GlobalpayPaymentStatus::Initiated => Self::AuthenticationPending,
            GlobalpayPaymentStatus::Pending => Self::Pending,
        }
    }
}

impl From<GlobalpayPaymentStatus> for common_enums::RefundStatus {
    fn from(item: GlobalpayPaymentStatus) -> Self {
        match item {
            GlobalpayPaymentStatus::Captured | GlobalpayPaymentStatus::Funded => Self::Success,
            GlobalpayPaymentStatus::Declined | GlobalpayPaymentStatus::Rejected => Self::Failure,
            GlobalpayPaymentStatus::Initiated | GlobalpayPaymentStatus::Pending => Self::Pending,
            _ => Self::Pending,
        }
    }
}

impl From<Option<common_enums::CaptureMethod>> for requests::CaptureMode {
    fn from(capture_method: Option<common_enums::CaptureMethod>) -> Self {
        match capture_method {
            Some(common_enums::CaptureMethod::Manual) => Self::Later,
            Some(common_enums::CaptureMethod::ManualMultiple) => Self::Multiple,
            _ => Self::Auto,
        }
    }
}

// }

impl<F, T> TryFrom<ResponseRouterData<F, GlobalpayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = Error;

    fn try_from(
        item: ResponseRouterData<F, GlobalpayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = common_enums::AttemptStatus::from(item.response.status);

        let redirect_url = item
            .response
            .payment_method
            .as_ref()
            .and_then(|payment_method| {
                payment_method
                    .apm
                    .as_ref()
                    .and_then(|apm| apm.redirect_url.as_ref())
            })
            .filter(|redirect_str| !redirect_str.is_empty())
            .map(|url| {
                Url::parse(url).change_context(errors::ConnectorError::FailedToObtainIntegrationUrl)
            })
            .transpose()?;

        let redirection_data = redirect_url.map(|url| RedirectForm::from((url, Method::Get)));
        let payment_method_token = item.data.get_payment_method_token();
        let status_code = item.http_code;

        let mandate_reference = payment_method_token.ok().and_then(|token| match token {
            PaymentMethodToken::Token(token_string) => Some(MandateReference {
                connector_mandate_id: Some(token_string.clone().expose()),
                payment_method_id: None,
                mandate_metadata: None,
                connector_mandate_request_reference_id: None,
            }),
            _ => None,
        });

        let response = match status {
            common_enums::AttemptStatus::Failure => Err(Box::new(ErrorResponse {
                message: item
                    .response
                    .payment_method
                    .as_ref()
                    .and_then(|payment_method| payment_method.message.clone())
                    .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                code: item
                    .response
                    .payment_method
                    .as_ref()
                    .and_then(|payment_method| payment_method.result.clone())
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                reason: item
                    .response
                    .payment_method
                    .as_ref()
                    .and_then(|payment_method| payment_method.message.clone()),
                status_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.id.clone()),
                network_decline_code: item
                    .response
                    .payment_method
                    .as_ref()
                    .and_then(|payment_method| payment_method.result.clone()),
                network_advice_code: None,
                network_error_message: item
                    .response
                    .payment_method
                    .as_ref()
                    .and_then(|payment_method| payment_method.message.clone()),
                connector_metadata: None,
            })),
            _ => Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(mandate_reference),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.reference.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }),
        };

        Ok(Self {
            status,
            response: response.map_err(|err| *err),
            ..item.data
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, GlobalpayPaymentMethodsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, GlobalpayPaymentMethodsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let token = item
            .response
            .payment_method_token_id
            .clone()
            .unwrap_or_default();

        Ok(Self {
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: token.expose(),
            }),
            ..item.data
        })
    }
}

impl
    ForeignTryFrom<(
        PaymentsSyncResponseRouterData<GlobalpayPaymentsResponse>,
        bool,
    )> for PaymentsSyncRouterData
{
    type Error = Error;

    fn foreign_try_from(
        (value, is_multiple_capture_sync): (
            PaymentsSyncResponseRouterData<GlobalpayPaymentsResponse>,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        if is_multiple_capture_sync {
            let capture_sync_response_list =
                construct_captures_response_hashmap(vec![value.response])?;
            Ok(Self {
                response: Ok(PaymentsResponseData::MultipleCaptureResponse {
                    capture_sync_response_list,
                }),
                ..value.data
            })
        } else {
            Self::try_from(value)
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, GlobalpayRefreshTokenResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<ParsingError>;
    fn try_from(
        item: ResponseRouterData<F, GlobalpayRefreshTokenResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.token,
                expires: item.response.seconds_to_expire,
            }),
            ..item.data
        })
    }
}

impl<F> TryFrom<&GlobalPayRouterData<&RefundsRouterData<F>>> for requests::GlobalpayRefundRequest {
    type Error = Error;
    fn try_from(item: &GlobalPayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, GlobalpayPaymentsResponse>>
    for RefundExecuteRouterData
{
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<Execute, GlobalpayPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: common_enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, GlobalpayPaymentsResponse>>
    for RefundsRouterData<RSync>
{
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<RSync, GlobalpayPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: common_enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct GlobalpayErrorResponse {
    pub error_code: String,
    pub detailed_error_code: String,
    pub detailed_error_description: String,
}

fn get_return_url(item: &PaymentsAuthorizeRouterData) -> Option<String> {
    match item.request.payment_method_data.clone() {
        payment_method_data::PaymentMethodData::Wallet(
            payment_method_data::WalletData::PaypalRedirect(_),
        ) => {
            // Return URL handling for PayPal via Globalpay:
            // - PayPal inconsistency: Return URLs work with HTTP, but cancel URLs require HTTPS
            // - Local development: When testing locally, expose your server via HTTPS and replace
            //   the base URL with an HTTPS URL to ensure proper cancellation flow
            // - Refer to commit 6499d429da87 for more information
            item.request.complete_authorize_url.clone()
        }
        _ => item.request.router_return_url.clone(),
    }
}

fn get_wallet_data(
    wallet_data: &payment_method_data::WalletData,
) -> Result<PaymentMethodData, Error> {
    match wallet_data {
        payment_method_data::WalletData::PaypalRedirect(_) => {
            Ok(PaymentMethodData::Apm(requests::Apm {
                provider: Some(ApmProvider::Paypal),
            }))
        }
        payment_method_data::WalletData::GooglePay(_) => {
            Ok(PaymentMethodData::DigitalWallet(requests::DigitalWallet {
                provider: Some(requests::DigitalWalletProvider::PayByGoogle),
                payment_token: wallet_data.get_wallet_token_as_json("Google Pay".to_string())?,
            }))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            "Payment method".to_string(),
        ))?,
    }
}

impl TryFrom<&payment_method_data::BankRedirectData> for PaymentMethodData {
    type Error = Error;
    fn try_from(value: &payment_method_data::BankRedirectData) -> Result<Self, Self::Error> {
        match value {
            payment_method_data::BankRedirectData::Eps { .. } => Ok(Self::Apm(requests::Apm {
                provider: Some(ApmProvider::Eps),
            })),
            payment_method_data::BankRedirectData::Giropay { .. } => Ok(Self::Apm(requests::Apm {
                provider: Some(ApmProvider::Giropay),
            })),
            payment_method_data::BankRedirectData::Ideal { .. } => Ok(Self::Apm(requests::Apm {
                provider: Some(ApmProvider::Ideal),
            })),
            payment_method_data::BankRedirectData::Sofort { .. } => Ok(Self::Apm(requests::Apm {
                provider: Some(ApmProvider::Sofort),
            })),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl MultipleCaptureSyncResponse for GlobalpayPaymentsResponse {
    fn get_connector_capture_id(&self) -> String {
        self.id.clone()
    }

    fn get_capture_attempt_status(&self) -> common_enums::AttemptStatus {
        common_enums::AttemptStatus::from(self.status)
    }

    fn is_capture_response(&self) -> bool {
        true
    }

    fn get_amount_captured(&self) -> Result<Option<MinorUnit>, error_stack::Report<ParsingError>> {
        match self.amount.clone() {
            Some(amount) => {
                let minor_amount = StringMinorUnitForConnector::convert_back(
                    &StringMinorUnitForConnector,
                    amount,
                    self.currency.unwrap_or_default(), //it is ignored in convert_back function
                )?;
                Ok(Some(minor_amount))
            }
            None => Ok(None),
        }
    }
    fn get_connector_reference_id(&self) -> Option<String> {
        self.reference.clone()
    }
}
