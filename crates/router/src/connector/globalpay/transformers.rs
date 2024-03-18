use common_utils::crypto::{self, GenerateDigest};
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use rand::distributions::DistString;
use serde::{Deserialize, Serialize};
use url::Url;

use super::{
    requests::{
        self, ApmProvider, GlobalpayPaymentsRequest, GlobalpayRefreshTokenRequest, Initiator,
        PaymentMethodData, Sequence, StoredCredential,
    },
    response::{GlobalpayPaymentStatus, GlobalpayPaymentsResponse, GlobalpayRefreshTokenResponse},
};
use crate::{
    connector::utils::{self, CardData, PaymentsAuthorizeRequestData, RouterData, WalletData},
    consts,
    core::errors,
    services::{self, RedirectForm},
    types::{self, api, domain, storage::enums, ErrorResponse},
};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalPayMeta {
    account_name: Secret<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for GlobalpayPaymentsRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let metadata: GlobalPayMeta =
            utils::to_connector_meta_from_secret(item.connector_meta_data.clone())?;
        let account_name = metadata.account_name;
        let (initiator, stored_credential, brand_reference) = get_mandate_details(item)?;
        let payment_method_data = get_payment_method_data(item, brand_reference)?;
        Ok(Self {
            account_name,
            amount: Some(item.request.amount.to_string()),
            currency: item.request.currency.to_string(),
            reference: item.connector_request_reference_id.to_string(),
            country: item.get_billing_country()?,
            capture_mode: Some(requests::CaptureMode::from(item.request.capture_method)),
            payment_method: requests::PaymentMethod {
                payment_method_data,
                authentication: None,
                encryption: None,
                entry_mode: Default::default(),
                fingerprint_mode: None,
                first_name: None,
                id: None,
                last_name: None,
                name: None,
                narrative: None,
                storage_mode: None,
            },
            notifications: Some(requests::Notifications {
                return_url: get_return_url(item),
                challenge_return_url: None,
                decoupled_challenge_return_url: None,
                status_url: item.request.webhook_url.clone(),
                three_ds_method_return_url: None,
            }),
            authorization_mode: None,
            cashback_amount: None,
            channel: Default::default(),
            convenience_amount: None,
            currency_conversion: None,
            description: None,
            device: None,
            gratuity_amount: None,
            initiator,
            ip_address: None,
            language: None,
            lodging: None,
            order: None,
            payer_reference: None,
            site_reference: None,
            stored_credential,
            surcharge_amount: None,
            total_capture_count: None,
            globalpay_payments_request_type: None,
            user_reference: None,
        })
    }
}

impl TryFrom<&types::PaymentsCaptureRouterData> for requests::GlobalpayCaptureRequest {
    type Error = Error;
    fn try_from(value: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: Some(value.request.amount_to_capture.to_string()),
            capture_sequence: value.request.multiple_capture_data.clone().map(|mcd| {
                if mcd.capture_sequence == 1 {
                    Sequence::First
                } else {
                    Sequence::Subsequent
                }
            }),
            reference: value
                .request
                .multiple_capture_data
                .as_ref()
                .map(|mcd| mcd.capture_reference.clone()),
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for requests::GlobalpayCancelRequest {
    type Error = Error;
    fn try_from(value: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: value.request.amount.map(|amount| amount.to_string()),
        })
    }
}

pub struct GlobalpayAuthType {
    pub app_id: Secret<String>,
    pub key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for GlobalpayAuthType {
    type Error = Error;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                app_id: key1.to_owned(),
                key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl TryFrom<GlobalpayRefreshTokenResponse> for types::AccessToken {
    type Error = error_stack::Report<errors::ParsingError>;

    fn try_from(item: GlobalpayRefreshTokenResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            token: item.token,
            expires: item.seconds_to_expire,
        })
    }
}

impl TryFrom<&types::RefreshTokenRouterData> for GlobalpayRefreshTokenRequest {
    type Error = Error;

    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
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

impl From<GlobalpayPaymentStatus> for enums::AttemptStatus {
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

impl From<GlobalpayPaymentStatus> for enums::RefundStatus {
    fn from(item: GlobalpayPaymentStatus) -> Self {
        match item {
            GlobalpayPaymentStatus::Captured | GlobalpayPaymentStatus::Funded => Self::Success,
            GlobalpayPaymentStatus::Declined | GlobalpayPaymentStatus::Rejected => Self::Failure,
            GlobalpayPaymentStatus::Initiated | GlobalpayPaymentStatus::Pending => Self::Pending,
            _ => Self::Pending,
        }
    }
}

impl From<Option<enums::CaptureMethod>> for requests::CaptureMode {
    fn from(capture_method: Option<enums::CaptureMethod>) -> Self {
        match capture_method {
            Some(enums::CaptureMethod::Manual) => Self::Later,
            Some(enums::CaptureMethod::ManualMultiple) => Self::Multiple,
            _ => Self::Auto,
        }
    }
}

fn get_payment_response(
    status: enums::AttemptStatus,
    response: GlobalpayPaymentsResponse,
    redirection_data: Option<RedirectForm>,
) -> Result<types::PaymentsResponseData, ErrorResponse> {
    let mandate_reference = response.payment_method.as_ref().and_then(|pm| {
        pm.card
            .as_ref()
            .and_then(|card| card.brand_reference.to_owned())
            .map(|id| types::MandateReference {
                connector_mandate_id: Some(id.expose()),
                payment_method_id: None,
            })
    });
    match status {
        enums::AttemptStatus::Failure => Err(ErrorResponse {
            message: response
                .payment_method
                .and_then(|pm| pm.message)
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            ..Default::default()
        }),
        _ => Ok(types::PaymentsResponseData::TransactionResponse {
            resource_id: types::ResponseId::ConnectorTransactionId(response.id),
            redirection_data,
            mandate_reference,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: response.reference,
            incremental_authorization_allowed: None,
        }),
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, GlobalpayPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            GlobalpayPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.status);
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
                Url::parse(url)
                    .into_report()
                    .change_context(errors::ConnectorError::FailedToObtainIntegrationUrl)
            })
            .transpose()?;
        let redirection_data =
            redirect_url.map(|url| services::RedirectForm::from((url, services::Method::Get)));
        Ok(Self {
            status,
            response: get_payment_response(status, item.response, redirection_data),
            ..item.data
        })
    }
}

impl
    TryFrom<(
        types::PaymentsSyncResponseRouterData<GlobalpayPaymentsResponse>,
        bool,
    )> for types::PaymentsSyncRouterData
{
    type Error = Error;

    fn try_from(
        (value, is_multiple_capture_sync): (
            types::PaymentsSyncResponseRouterData<GlobalpayPaymentsResponse>,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        if is_multiple_capture_sync {
            let capture_sync_response_list =
                utils::construct_captures_response_hashmap(vec![value.response]);
            Ok(Self {
                response: Ok(types::PaymentsResponseData::MultipleCaptureResponse {
                    capture_sync_response_list,
                }),
                ..value.data
            })
        } else {
            Self::try_from(value)
        }
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, GlobalpayRefreshTokenResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::ResponseRouterData<F, GlobalpayRefreshTokenResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.token,
                expires: item.response.seconds_to_expire,
            }),
            ..item.data
        })
    }
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for requests::GlobalpayRefundRequest {
    type Error = Error;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.refund_amount.to_string(),
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, GlobalpayPaymentsResponse>>
    for types::RefundExecuteRouterData
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, GlobalpayPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, GlobalpayPaymentsResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, GlobalpayPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
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

fn get_payment_method_data(
    item: &types::PaymentsAuthorizeRouterData,
    brand_reference: Option<String>,
) -> Result<PaymentMethodData, Error> {
    match &item.request.payment_method_data {
        domain::PaymentMethodData::Card(ccard) => Ok(PaymentMethodData::Card(requests::Card {
            number: ccard.card_number.clone(),
            expiry_month: ccard.card_exp_month.clone(),
            expiry_year: ccard.get_card_expiry_year_2_digit()?,
            cvv: ccard.card_cvc.clone(),
            account_type: None,
            authcode: None,
            avs_address: None,
            avs_postal_code: None,
            brand_reference,
            chip_condition: None,
            funding: None,
            pin_block: None,
            tag: None,
            track: None,
        })),
        domain::PaymentMethodData::Wallet(wallet_data) => get_wallet_data(wallet_data),
        domain::PaymentMethodData::BankRedirect(bank_redirect) => {
            PaymentMethodData::try_from(bank_redirect)
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            "Payment methods".to_string(),
        ))?,
    }
}

fn get_return_url(item: &types::PaymentsAuthorizeRouterData) -> Option<String> {
    match item.request.payment_method_data.clone() {
        domain::PaymentMethodData::Wallet(api_models::payments::WalletData::PaypalRedirect(_)) => {
            item.request.complete_authorize_url.clone()
        }
        _ => item.request.router_return_url.clone(),
    }
}

type MandateDetails = (Option<Initiator>, Option<StoredCredential>, Option<String>);
fn get_mandate_details(item: &types::PaymentsAuthorizeRouterData) -> Result<MandateDetails, Error> {
    Ok(if item.request.is_mandate_payment() {
        let connector_mandate_id = item.request.mandate_id.as_ref().and_then(|mandate_ids| {
            match mandate_ids.mandate_reference_id.clone() {
                Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                    connector_mandate_ids,
                )) => connector_mandate_ids.connector_mandate_id,
                _ => None,
            }
        });
        (
            Some(match item.request.off_session {
                Some(true) => Initiator::Merchant,
                _ => Initiator::Payer,
            }),
            Some(StoredCredential {
                model: Some(requests::Model::Recurring),
                sequence: Some(match connector_mandate_id.is_some() {
                    true => requests::Sequence::Subsequent,
                    false => requests::Sequence::First,
                }),
            }),
            connector_mandate_id,
        )
    } else {
        (None, None, None)
    })
}

fn get_wallet_data(
    wallet_data: &api_models::payments::WalletData,
) -> Result<PaymentMethodData, Error> {
    match wallet_data {
        api_models::payments::WalletData::PaypalRedirect(_) => {
            Ok(PaymentMethodData::Apm(requests::Apm {
                provider: Some(ApmProvider::Paypal),
            }))
        }
        api_models::payments::WalletData::GooglePay(_) => {
            Ok(PaymentMethodData::DigitalWallet(requests::DigitalWallet {
                provider: Some(requests::DigitalWalletProvider::PayByGoogle),
                payment_token: wallet_data.get_wallet_token_as_json()?,
            }))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            "Payment method".to_string(),
        ))?,
    }
}

impl TryFrom<&api_models::payments::BankRedirectData> for PaymentMethodData {
    type Error = Error;
    fn try_from(value: &api_models::payments::BankRedirectData) -> Result<Self, Self::Error> {
        match value {
            api_models::payments::BankRedirectData::Eps { .. } => Ok(Self::Apm(requests::Apm {
                provider: Some(ApmProvider::Eps),
            })),
            api_models::payments::BankRedirectData::Giropay { .. } => {
                Ok(Self::Apm(requests::Apm {
                    provider: Some(ApmProvider::Giropay),
                }))
            }
            api_models::payments::BankRedirectData::Ideal { .. } => Ok(Self::Apm(requests::Apm {
                provider: Some(ApmProvider::Ideal),
            })),
            api_models::payments::BankRedirectData::Sofort { .. } => Ok(Self::Apm(requests::Apm {
                provider: Some(ApmProvider::Sofort),
            })),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method".to_string(),
            ))
            .into_report(),
        }
    }
}

impl utils::MultipleCaptureSyncResponse for GlobalpayPaymentsResponse {
    fn get_connector_capture_id(&self) -> String {
        self.id.clone()
    }

    fn get_capture_attempt_status(&self) -> diesel_models::enums::AttemptStatus {
        enums::AttemptStatus::from(self.status)
    }

    fn is_capture_response(&self) -> bool {
        true
    }

    fn get_amount_captured(&self) -> Option<i64> {
        match self.amount.clone() {
            Some(amount) => amount.parse().ok(),
            None => None,
        }
    }
    fn get_connector_reference_id(&self) -> Option<String> {
        self.reference.clone()
    }
}
