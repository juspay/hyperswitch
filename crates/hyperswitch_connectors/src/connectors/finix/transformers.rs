pub mod request;
pub mod response;
use api_models::payments::MandateReferenceId;
use base64::Engine;
use common_enums::{enums, AttemptStatus, CaptureMethod, CountryAlpha2, CountryAlpha3};
use common_utils::types::MinorUnit;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, ErrorResponse, PaymentMethodToken, RouterData},
    router_flow_types::{
        self as flows,
        refunds::{Execute, RSync},
        Authorize, Capture,
    },
    router_request_types::{
        ConnectorCustomerData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCaptureData, RefundsData, ResponseId, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorCustomerResponseData, MandateReference, PaymentsResponseData, RefundsResponseData,
    },
    types::RefundsRouterData,
};
use hyperswitch_interfaces::{consts, errors::ConnectorError};
use masking::{ExposeInterface, Secret};
pub use request::*;
pub use response::*;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    unimplemented_payment_method,
    utils::{
        get_unimplemented_payment_method_error_message, AddressDetailsData, CardData,
        RouterData as _,
    },
};

pub struct FinixRouterData<'a, Flow, Req, Res> {
    pub amount: MinorUnit,
    pub router_data: &'a RouterData<Flow, Req, Res>,
    pub merchant_id: Secret<String>,
    pub merchant_identity_id: Secret<String>,
}

impl<'a, Flow, Req, Res> TryFrom<(MinorUnit, &'a RouterData<Flow, Req, Res>)>
    for FinixRouterData<'a, Flow, Req, Res>
{
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(value: (MinorUnit, &'a RouterData<Flow, Req, Res>)) -> Result<Self, Self::Error> {
        let (amount, router_data) = value;
        let auth = FinixAuthType::try_from(&router_data.connector_auth_type)?;

        Ok(Self {
            amount,
            router_data,
            merchant_id: auth.merchant_id,
            merchant_identity_id: auth.merchant_identity_id,
        })
    }
}

impl
    TryFrom<
        &FinixRouterData<
            '_,
            flows::CreateConnectorCustomer,
            ConnectorCustomerData,
            PaymentsResponseData,
        >,
    > for FinixCreateIdentityRequest
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &FinixRouterData<
            '_,
            flows::CreateConnectorCustomer,
            ConnectorCustomerData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let customer_data: &ConnectorCustomerData = &item.router_data.request;
        let personal_address = item.router_data.get_optional_billing().and_then(|address| {
            let billing = address.address.as_ref();
            billing.map(|billing_address| FinixAddress {
                line1: billing_address.get_optional_line1(),
                line2: billing_address.get_optional_line2(),
                city: billing_address.get_optional_city(),
                region: billing_address.get_optional_state(),
                postal_code: billing_address.get_optional_zip(),
                country: billing_address
                    .get_optional_country()
                    .map(CountryAlpha2::from_alpha2_to_alpha3),
            })
        });
        let entity = FinixIdentityEntity {
            phone: customer_data.phone.clone(),
            first_name: item.router_data.get_optional_billing_first_name(),
            last_name: item.router_data.get_optional_billing_last_name(),
            email: item.router_data.get_optional_billing_email(),
            personal_address,
        };

        Ok(Self {
            entity,
            tags: None,
            identity_type: FinixIdentityType::PERSONAL,
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, FinixIdentityResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FinixIdentityResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                ConnectorCustomerResponseData::new_with_customer_id(item.response.id),
            )),
            ..item.data
        })
    }
}

impl TryFrom<&FinixRouterData<'_, Authorize, PaymentsAuthorizeData, PaymentsResponseData>>
    for FinixPaymentsRequest
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &FinixRouterData<'_, Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        if matches!(
            item.router_data.request.payment_method_data,
            PaymentMethodData::Card(_)
        ) && matches!(
            item.router_data.auth_type,
            enums::AuthenticationType::ThreeDs
        ) {
            return Err(ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("finix"),
            )
            .into());
        }

        let source =
            match item.router_data.request.payment_method_data.clone() {
                PaymentMethodData::Card(_)
                | PaymentMethodData::Wallet(WalletData::GooglePay(_))
                | PaymentMethodData::Wallet(WalletData::ApplePay(_)) => {
                    let source = item.router_data.get_payment_method_token()?;
                    match source {
                        PaymentMethodToken::Token(token) => token,
                        PaymentMethodToken::ApplePayDecrypt(_) => Err(
                            unimplemented_payment_method!("Apple Pay", "Simplified", "finix"),
                        )?,
                        PaymentMethodToken::PazeDecrypt(_) => {
                            Err(unimplemented_payment_method!("Paze", "finix"))?
                        }
                        PaymentMethodToken::GooglePayDecrypt(_) => {
                            Err(unimplemented_payment_method!("Google Pay", "finix"))?
                        }
                    }
                }
                PaymentMethodData::MandatePayment => Secret::new(
                    item.router_data
                        .request
                        .mandate_id
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
                        .ok_or(ConnectorError::MissingConnectorMandateID)?,
                ),
                _ => Err(ConnectorError::NotImplemented(
                    "Payment method not supported".to_string(),
                ))?,
            };

        Ok(Self {
            amount: item.amount,
            currency: item.router_data.request.currency,
            source,
            merchant: item.merchant_id.clone(),
            idempotency_id: Some(item.router_data.connector_request_reference_id.clone()),
            tags: None,
            three_d_secure: None,
        })
    }
}

impl TryFrom<&FinixRouterData<'_, Capture, PaymentsCaptureData, PaymentsResponseData>>
    for FinixCaptureRequest
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &FinixRouterData<'_, Capture, PaymentsCaptureData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            capture_amount: item.router_data.request.minor_amount_to_capture,
        })
    }
}

fn get_token_request(
    payment_method_data: PaymentMethodData,
    merchant_identity_id: Secret<String>,
    identity: String,
    customer_name: Option<Secret<String>>,
) -> Result<FinixCreatePaymentInstrumentRequest, error_stack::Report<ConnectorError>> {
    match &payment_method_data {
        PaymentMethodData::Card(card_data) => {
            Ok(FinixCreatePaymentInstrumentRequest {
                instrument_type: FinixPaymentInstrumentType::PaymentCard,
                name: card_data.card_holder_name.clone(),
                number: Some(Secret::new(card_data.card_number.clone().get_card_no())),
                security_code: Some(card_data.card_cvc.clone()),
                expiration_month: Some(card_data.get_expiry_month_as_i8()?),
                expiration_year: Some(card_data.get_expiry_year_as_4_digit_i32()?),
                identity: identity.clone(), // This would come from a previously created identity
                tags: None,
                address: None,
                card_brand: None, // Finix determines this from the card number
                card_type: None,  // Finix determines this from the card number
                additional_data: None,
                merchant_identity: None,
                third_party_token: None,
            })
        }
        PaymentMethodData::Wallet(wallet) => match wallet {
            WalletData::GooglePay(google_pay_wallet_data) => {
                let third_party_token = google_pay_wallet_data
                    .tokenization_data
                    .get_encrypted_google_pay_token()
                    .change_context(ConnectorError::MissingRequiredField {
                        field_name: "google_pay_token",
                    })?;
                Ok(FinixCreatePaymentInstrumentRequest {
                    instrument_type: FinixPaymentInstrumentType::GOOGLEPAY,
                    name: customer_name.clone(),
                    identity: identity.clone(),
                    number: None,
                    security_code: None,
                    expiration_month: None,
                    expiration_year: None,
                    tags: None,
                    address: None,
                    card_brand: None,
                    card_type: None,
                    additional_data: None,
                    merchant_identity: Some(merchant_identity_id.clone()),
                    third_party_token: Some(Secret::new(third_party_token)),
                })
            }
            WalletData::ApplePay(apple_pay_wallet_data) => {
                let applepay_encrypt_data = apple_pay_wallet_data
                    .payment_data
                    .get_encrypted_apple_pay_payment_data_mandatory()
                    .change_context(ConnectorError::MissingRequiredField {
                        field_name: "Apple pay encrypted data",
                    })?;

                let decoded_data = base64::prelude::BASE64_STANDARD
                    .decode(applepay_encrypt_data)
                    .change_context(ConnectorError::InvalidDataFormat {
                        field_name: "apple_pay_encrypted_data",
                    })?;

                let apple_pay_token: FinixApplePayEncryptedData = serde_json::from_slice(
                    &decoded_data,
                )
                .change_context(ConnectorError::InvalidDataFormat {
                    field_name: "apple_pay_token_json",
                })?;

                let finix_token = FinixApplePayPaymentToken {
                    token: FinixApplePayToken {
                        payment_data: FinixApplePayEncryptedData {
                            data: apple_pay_token.data.clone(),
                            signature: apple_pay_token.signature.clone(),
                            header: FinixApplePayHeader {
                                public_key_hash: apple_pay_token.header.public_key_hash.clone(),
                                ephemeral_public_key: apple_pay_token
                                    .header
                                    .ephemeral_public_key
                                    .clone(),
                                transaction_id: apple_pay_token.header.transaction_id.clone(),
                            },
                            version: apple_pay_token.version.clone(),
                        },
                        payment_method: FinixApplePayPaymentMethod {
                            display_name: Secret::new(
                                apple_pay_wallet_data.payment_method.display_name.clone(),
                            ),
                            network: Secret::new(
                                apple_pay_wallet_data.payment_method.network.clone(),
                            ),
                            method_type: Secret::new(
                                apple_pay_wallet_data.payment_method.pm_type.clone(),
                            ),
                        },
                        transaction_identifier: apple_pay_wallet_data
                            .transaction_identifier
                            .clone(),
                    },
                };

                let third_party_token = serde_json::to_string(&finix_token).change_context(
                    ConnectorError::InvalidDataFormat {
                        field_name: "apple pay token",
                    },
                )?;

                Ok(FinixCreatePaymentInstrumentRequest {
                    instrument_type: FinixPaymentInstrumentType::ApplePay,
                    name: customer_name.clone(),
                    number: None,
                    security_code: None,
                    expiration_month: None,
                    expiration_year: None,
                    identity: identity.clone(),
                    tags: None,
                    address: None,
                    card_brand: None,
                    card_type: None,
                    additional_data: None,
                    merchant_identity: Some(merchant_identity_id.clone()),
                    third_party_token: Some(Secret::new(third_party_token)),
                })
            }
            _ => Err(ConnectorError::NotImplemented(
                "Payment method not supported for tokenization".to_string(),
            )
            .into()),
        },
        _ => Err(ConnectorError::NotImplemented(
            "Payment method not supported for tokenization".to_string(),
        )
        .into()),
    }
}
impl
    TryFrom<
        &FinixRouterData<
            '_,
            flows::PaymentMethodToken,
            PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    > for FinixCreatePaymentInstrumentRequest
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &FinixRouterData<
            '_,
            flows::PaymentMethodToken,
            PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let tokenization_data: &PaymentMethodTokenizationData = &item.router_data.request;
        get_token_request(
            tokenization_data.payment_method_data.clone(),
            item.merchant_identity_id.clone(),
            item.router_data.get_connector_customer_id()?,
            item.router_data.get_optional_billing_full_name(),
        )
    }
}

// Implement response handling for tokenization
impl<F, T> TryFrom<ResponseRouterData<F, FinixInstrumentResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FinixInstrumentResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.id,
            }),
            ..item.data
        })
    }
}

pub(crate) fn get_setup_mandate_router_data<Request>(
    item: ResponseRouterData<
        flows::SetupMandate,
        FinixInstrumentResponse,
        Request,
        PaymentsResponseData,
    >,
) -> Result<
    RouterData<flows::SetupMandate, Request, PaymentsResponseData>,
    error_stack::Report<ConnectorError>,
> {
    Ok(RouterData {
        status: AttemptStatus::Charged,
        response: Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(Some(MandateReference {
                connector_mandate_id: Some(item.response.id),
                payment_method_id: None,
                mandate_metadata: None,
                connector_mandate_request_reference_id: None,
            })),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        }),
        ..item.data
    })
}

//setup mandate

impl
    TryFrom<
        &FinixRouterData<'_, flows::SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
    > for FinixCreatePaymentInstrumentRequest
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &FinixRouterData<
            '_,
            flows::SetupMandate,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let tokenization_data = &item.router_data.request;

        get_token_request(
            tokenization_data.payment_method_data.clone(),
            item.merchant_identity_id.clone(),
            item.router_data.get_connector_customer_id()?,
            item.router_data.get_optional_billing_full_name(),
        )
    }
}

// Auth Struct

impl TryFrom<&ConnectorAuthType> for FinixAuthType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Ok(Self {
                finix_user_name: api_key.clone(),
                finix_password: api_secret.clone(),
                merchant_id: key1.clone(),
                merchant_identity_id: key2.clone(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

fn get_attempt_status(state: FinixState, flow: FinixFlow, is_void: Option<bool>) -> AttemptStatus {
    if is_void == Some(true) {
        return match state {
            FinixState::FAILED | FinixState::CANCELED | FinixState::UNKNOWN => {
                AttemptStatus::VoidFailed
            }
            FinixState::PENDING => AttemptStatus::Voided,
            FinixState::SUCCEEDED => AttemptStatus::Voided,
        };
    }
    match (flow, state) {
        (FinixFlow::Auth, FinixState::PENDING) => AttemptStatus::AuthenticationPending,
        (FinixFlow::Auth, FinixState::SUCCEEDED) => AttemptStatus::Authorized,
        (FinixFlow::Auth, FinixState::FAILED) => AttemptStatus::AuthorizationFailed,
        (FinixFlow::Auth, FinixState::CANCELED) | (FinixFlow::Auth, FinixState::UNKNOWN) => {
            AttemptStatus::AuthorizationFailed
        }
        (FinixFlow::Transfer, FinixState::PENDING) => AttemptStatus::Pending,
        (FinixFlow::Transfer, FinixState::SUCCEEDED) => AttemptStatus::Charged,
        (FinixFlow::Transfer, FinixState::FAILED)
        | (FinixFlow::Transfer, FinixState::CANCELED)
        | (FinixFlow::Transfer, FinixState::UNKNOWN) => AttemptStatus::Failure,
        (FinixFlow::Capture, FinixState::PENDING) => AttemptStatus::Pending,
        (FinixFlow::Capture, FinixState::SUCCEEDED) => AttemptStatus::Pending, // Psync with Transfer id can determine actuall success
        (FinixFlow::Capture, FinixState::FAILED)
        | (FinixFlow::Capture, FinixState::CANCELED)
        | (FinixFlow::Capture, FinixState::UNKNOWN) => AttemptStatus::Failure,
    }
}

pub(crate) fn get_finix_response<F, T>(
    router_data: ResponseRouterData<F, FinixPaymentsResponse, T, PaymentsResponseData>,
    finix_flow: FinixFlow,
) -> Result<RouterData<F, T, PaymentsResponseData>, error_stack::Report<ConnectorError>> {
    let status = get_attempt_status(
        router_data.response.state.clone(),
        finix_flow,
        router_data.response.is_void,
    );
    Ok(RouterData {
        status,
        response: if router_data.response.state.is_failure() {
            Err(ErrorResponse {
                code: router_data
                    .response
                    .failure_code
                    .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                message: router_data
                    .response
                    .messages
                    .map_or(consts::NO_ERROR_MESSAGE.to_string(), |msg| msg.join(",")),
                reason: router_data.response.failure_message,
                status_code: router_data.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(router_data.response.id.clone()),
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    router_data
                        .response
                        .transfer
                        .unwrap_or(router_data.response.id),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(Some(MandateReference {
                    connector_mandate_id: router_data.response.source.map(|id| id.expose()),
                    payment_method_id: None,
                    mandate_metadata: None,
                    connector_mandate_request_reference_id: None,
                })),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            })
        },
        ..router_data.data
    })
}

impl<F> TryFrom<&FinixRouterData<'_, F, RefundsData, RefundsResponseData>>
    for FinixCreateRefundRequest
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: &FinixRouterData<'_, F, RefundsData, RefundsResponseData>,
    ) -> Result<Self, Self::Error> {
        let refund_amount = item.router_data.request.minor_refund_amount;
        Ok(Self::new(refund_amount))
    }
}

impl From<FinixState> for enums::RefundStatus {
    fn from(item: FinixState) -> Self {
        match item {
            FinixState::PENDING => Self::Pending,
            FinixState::SUCCEEDED => Self::Success,
            FinixState::FAILED | FinixState::CANCELED | FinixState::UNKNOWN => Self::Failure,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, FinixPaymentsResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, FinixPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.state),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, FinixPaymentsResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, FinixPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.state),
            }),
            ..item.data
        })
    }
}

impl FinixErrorResponse {
    pub fn get_message(&self) -> String {
        self.embedded
            .as_ref()
            .and_then(|embedded| embedded.errors.as_ref())
            .and_then(|errors| errors.first())
            .and_then(|error| error.message.clone())
            .unwrap_or(consts::NO_ERROR_MESSAGE.to_string())
    }

    pub fn get_code(&self) -> String {
        self.embedded
            .as_ref()
            .and_then(|embedded| embedded.errors.as_ref())
            .and_then(|errors| errors.first())
            .and_then(|error| error.code.clone())
            .unwrap_or(consts::NO_ERROR_MESSAGE.to_string())
    }
}
