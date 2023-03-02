use common_utils::crypto::{self, GenerateDigest};
use error_stack::ResultExt;
use rand::distributions::DistString;
use serde::{Deserialize, Serialize};

use super::{
    requests::{self, GlobalpayPaymentsRequest, GlobalpayRefreshTokenRequest},
    response::{GlobalpayPaymentStatus, GlobalpayPaymentsResponse, GlobalpayRefreshTokenResponse},
};
use crate::{
    connector::utils::{self, CardData, PaymentsRequestData},
    consts,
    core::errors,
    services::{self},
    types::{self, api, storage::enums, ErrorResponse},
};

impl TryFrom<&types::PaymentsAuthorizeRouterData> for GlobalpayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let metadata = item
            .connector_meta_data
            .to_owned()
            .ok_or_else(utils::missing_field_err("connector_meta"))?;
        let account_name = metadata
            .as_object()
            .and_then(|acc_name| acc_name.get("account_name"))
            .map(|acc_name| acc_name.to_string())
            .ok_or_else(utils::missing_field_err("connector_meta.account_name"))?;

        match item.request.payment_method_data.clone() {
            api::PaymentMethod::Card(ccard) => Ok(Self {
                account_name,
                amount: Some(item.request.amount.to_string()),
                currency: item.request.currency.to_string(),
                reference: item.attempt_id.to_string(),
                country: item.get_billing_country()?,
                capture_mode: item
                    .request
                    .capture_method
                    .map(|cap_method| match cap_method {
                        enums::CaptureMethod::Manual => requests::CaptureMode::Later,
                        _ => requests::CaptureMode::Auto,
                    }),
                payment_method: requests::PaymentMethod {
                    card: Some(requests::Card {
                        number: ccard.get_card_number(),
                        expiry_month: ccard.get_card_expiry_month(),
                        expiry_year: ccard.get_card_expiry_year_2_digit(),
                        cvv: ccard.get_card_cvc(),
                        account_type: None,
                        authcode: None,
                        avs_address: None,
                        avs_postal_code: None,
                        brand_reference: None,
                        chip_condition: None,
                        cvv_indicator: Default::default(),
                        funding: None,
                        pin_block: None,
                        tag: None,
                        track: None,
                    }),
                    apm: None,
                    authentication: None,
                    bank_transfer: None,
                    digital_wallet: None,
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
                authorization_mode: None,
                cashback_amount: None,
                channel: Default::default(),
                convenience_amount: None,
                currency_conversion: None,
                description: None,
                device: None,
                gratuity_amount: None,
                initiator: None,
                ip_address: None,
                language: None,
                lodging: None,
                notifications: None,
                order: None,
                payer_reference: None,
                site_reference: None,
                stored_credential: None,
                surcharge_amount: None,
                total_capture_count: None,
                globalpay_payments_request_type: None,
                user_reference: None,
            }),
            api::PaymentMethod::Wallet(wallet_data) => match wallet_data.issuer_name {
                api_models::enums::WalletIssuer::Paypal => Ok(Self {
                    account_name,
                    amount: Some(item.request.amount.to_string()),
                    currency: item.request.currency.to_string(),
                    reference: item.attempt_id.to_string(),
                    country: item.get_billing_country()?,
                    capture_mode: item.request.capture_method.map(|cap_mode| match cap_mode {
                        enums::CaptureMethod::Manual => requests::CaptureMode::Later,
                        _ => requests::CaptureMode::Auto,
                    }),
                    payment_method: requests::PaymentMethod {
                        apm: Some(requests::Apm {
                            provider: Some(requests::ApmProvider::Paypal),
                        }),
                        authentication: None,
                        bank_transfer: None,
                        card: None,
                        digital_wallet: None,
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
                        return_url: item.router_return_url.clone(),
                        challenge_return_url: None,
                        decoupled_challenge_return_url: None,
                        status_url: None,
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
                    initiator: None,
                    ip_address: None,
                    language: None,
                    lodging: None,
                    order: None,
                    payer_reference: None,
                    site_reference: None,
                    stored_credential: None,
                    surcharge_amount: None,
                    total_capture_count: None,
                    globalpay_payments_request_type: None,
                    user_reference: None,
                }),
                api_models::enums::WalletIssuer::GooglePay => {
                    let wallet_data_token =
                        wallet_data
                            .token
                            .ok_or(errors::ConnectorError::MissingRequiredField {
                                field_name: "item.payment_method_data.wallet.token",
                            })?;
                    let token: Result<serde_json::Value, serde_json::Error> =
                        serde_json::from_slice(wallet_data_token.as_bytes());
                    Ok(Self {
                        account_name,
                        amount: Some(item.request.amount.to_string()),
                        currency: item.request.currency.to_string(),
                        reference: item.attempt_id.to_string(),
                        country: item.get_billing_country()?,
                        capture_mode: item.request.capture_method.map(|cap_mode| match cap_mode {
                            enums::CaptureMethod::Manual => requests::CaptureMode::Later,
                            _ => requests::CaptureMode::Auto,
                        }),
                        payment_method: requests::PaymentMethod {
                            digital_wallet: Some(requests::DigitalWallet {
                                provider: Some(requests::DigitalWalletProvider::PayByGoogle),
                                payment_token: Some(token.ok().ok_or(
                                    errors::ConnectorError::MissingRequiredField {
                                        field_name: "item.payment_method_data.wallet.token",
                                    },
                                )?),
                            }),
                            authentication: None,
                            bank_transfer: None,
                            card: None,
                            apm: None,
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
                            return_url: item.router_return_url.clone(),
                            challenge_return_url: None,
                            decoupled_challenge_return_url: None,
                            status_url: None,
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
                        initiator: None,
                        ip_address: None,
                        language: None,
                        lodging: None,
                        order: None,
                        payer_reference: None,
                        site_reference: None,
                        stored_credential: None,
                        surcharge_amount: None,
                        total_capture_count: None,
                        globalpay_payments_request_type: None,
                        user_reference: None,
                    })
                }
                _ => Err(
                    errors::ConnectorError::NotImplemented("Payment methods".to_string()).into(),
                ),
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

impl TryFrom<&types::PaymentsCaptureRouterData> for GlobalpayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: value
                .request
                .amount_to_capture
                .map(|amount| amount.to_string()),
            ..Default::default()
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for GlobalpayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_value: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
}

pub struct GlobalpayAuthType {
    pub app_id: String,
    pub key: String,
}

impl TryFrom<&types::ConnectorAuthType> for GlobalpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                app_id: key1.to_string(),
                key: api_key.to_string(),
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
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let globalpay_auth = GlobalpayAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)
            .attach_printable("Could not convert connector_auth to globalpay_auth")?;

        let nonce = rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 12);
        let nonce_with_api_key = format!("{}{}", nonce, globalpay_auth.key);
        let secret_vec = crypto::Sha512
            .generate_digest(nonce_with_api_key.as_bytes())
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("error creating request nonce")?;

        let secret = hex::encode(secret_vec);

        Ok(Self {
            app_id: globalpay_auth.app_id,
            nonce,
            secret,
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

fn get_payment_response(
    status: enums::AttemptStatus,
    response: GlobalpayPaymentsResponse,
) -> Result<types::PaymentsResponseData, ErrorResponse> {
    let redirection_data = response.payment_method.as_ref().and_then(|payment_method| {
        payment_method.redirect_url.as_ref().map(|redirect_url| {
            services::RedirectForm::from((redirect_url.to_owned(), services::Method::Get))
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
            mandate_reference: None,
            connector_metadata: None,
        }),
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, GlobalpayPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            GlobalpayPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.status);
        Ok(Self {
            status,
            response: get_payment_response(status, item.response),
            ..item.data
        })
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
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.refund_amount.to_string(),
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, GlobalpayPaymentsResponse>>
    for types::RefundExecuteRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
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
    type Error = error_stack::Report<errors::ConnectorError>;
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
