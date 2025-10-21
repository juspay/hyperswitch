pub mod request;
pub mod response;
use api_models::{
    payments::{MandateReferenceId, PaymentIdType},
    webhooks::{IncomingWebhookEvent, RefundIdType},
};
use common_enums::{enums, AttemptStatus, CaptureMethod, CountryAlpha2, CountryAlpha3};
use common_utils::{errors::CustomResult, types::MinorUnit};
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
use hyperswitch_interfaces::{consts, disputes::DisputePayload, errors::ConnectorError};
use masking::{ExposeInterface, Secret};
pub use request::*;
pub use response::*;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    unimplemented_payment_method,
    utils::{
        self, get_unimplemented_payment_method_error_message, AddressDetailsData, CardData,
        RouterData as _,
    },
};

pub struct FinixRouterData<'a, Flow, Req, Res> {
    pub amount: MinorUnit,
    pub router_data: &'a RouterData<Flow, Req, Res>,
    pub merchant_id: Secret<String>,
    pub merchant_identity_id: Secret<String>,
}

impl TryFrom<&Option<common_utils::pii::SecretSerdeValue>> for FinixMeta {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(
        meta_data: &Option<common_utils::pii::SecretSerdeValue>,
    ) -> Result<Self, Self::Error> {
        let metadata = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(ConnectorError::InvalidConnectorConfig { config: "metadata" })?;
        Ok(metadata)
    }
}

impl<'a, Flow, Req, Res> TryFrom<(MinorUnit, &'a RouterData<Flow, Req, Res>)>
    for FinixRouterData<'a, Flow, Req, Res>
{
    type Error = error_stack::Report<ConnectorError>;

    fn try_from(value: (MinorUnit, &'a RouterData<Flow, Req, Res>)) -> Result<Self, Self::Error> {
        let (amount, router_data) = value;
        let auth = FinixAuthType::try_from(&router_data.connector_auth_type)?;
        let connector_meta = FinixMeta::try_from(&router_data.connector_meta_data)?;

        Ok(Self {
            amount,
            router_data,
            merchant_id: auth.merchant_id,
            merchant_identity_id: connector_meta.merchant_id,
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
            item.router_data.auth_type,
            enums::AuthenticationType::ThreeDs
        ) {
            return Err(ConnectorError::NotImplemented(
                get_unimplemented_payment_method_error_message("finix"),
            )
            .into());
        }
        let source = match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(_) | PaymentMethodData::Wallet(WalletData::GooglePay(_)) => {
                let source = item.router_data.get_payment_method_token()?;
                match source {
                    PaymentMethodToken::Token(token) => token,
                    PaymentMethodToken::ApplePayDecrypt(_) => Err(unimplemented_payment_method!(
                        "Apple Pay",
                        "Simplified",
                        "Stax"
                    ))?,
                    PaymentMethodToken::PazeDecrypt(_) => {
                        Err(unimplemented_payment_method!("Paze", "Stax"))?
                    }
                    PaymentMethodToken::GooglePayDecrypt(_) => {
                        Err(unimplemented_payment_method!("Google Pay", "Stax"))?
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
        let tokenization_data = &item.router_data.request;

        match &tokenization_data.payment_method_data {
            PaymentMethodData::Card(card_data) => {
                Ok(Self {
                    instrument_type: FinixPaymentInstrumentType::PaymentCard,
                    name: card_data.card_holder_name.clone(),
                    number: Some(Secret::new(card_data.card_number.clone().get_card_no())),
                    security_code: Some(card_data.card_cvc.clone()),
                    expiration_month: Some(card_data.get_expiry_month_as_i8()?),
                    expiration_year: Some(card_data.get_expiry_year_as_4_digit_i32()?),
                    identity: item.router_data.get_connector_customer_id()?, // This would come from a previously created identity
                    tags: None,
                    address: None,
                    card_brand: None, // Finix determines this from the card number
                    card_type: None,  // Finix determines this from the card number
                    additional_data: None,
                    merchant_identity: None,
                    third_party_token: None,
                })
            }
            PaymentMethodData::Wallet(WalletData::GooglePay(google_pay_wallet_data)) => {
                let third_party_token = google_pay_wallet_data
                    .tokenization_data
                    .get_encrypted_google_pay_token()
                    .change_context(ConnectorError::MissingRequiredField {
                        field_name: "google_pay_token",
                    })?;
                Ok(Self {
                    instrument_type: FinixPaymentInstrumentType::GOOGLEPAY,
                    name: item.router_data.get_optional_billing_full_name(),
                    identity: item.router_data.get_connector_customer_id()?,
                    number: None,
                    security_code: None,
                    expiration_month: None,
                    expiration_year: None,
                    tags: None,
                    address: None,
                    card_brand: None,
                    card_type: None,
                    additional_data: None,
                    merchant_identity: Some(item.merchant_identity_id.clone()),
                    third_party_token: Some(Secret::new(third_party_token)),
                })
            }
            _ => Err(ConnectorError::NotImplemented(
                "Payment method not supported for tokenization".to_string(),
            )
            .into()),
        }
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
            status: AttemptStatus::Pending,
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

        match &tokenization_data.payment_method_data {
            PaymentMethodData::Card(card_data) => {
                Ok(Self {
                    instrument_type: FinixPaymentInstrumentType::PaymentCard,
                    name: card_data.card_holder_name.clone(),
                    number: Some(Secret::new(card_data.card_number.clone().get_card_no())),
                    security_code: Some(card_data.card_cvc.clone()),
                    expiration_month: Some(card_data.get_expiry_month_as_i8()?),
                    expiration_year: Some(card_data.get_expiry_year_as_4_digit_i32()?),
                    identity: item.router_data.get_connector_customer_id()?, // This would come from a previously created identity
                    tags: None,
                    address: None,
                    card_brand: None, // Finix determines this from the card number
                    card_type: None,  // Finix determines this from the card number
                    additional_data: None,
                    merchant_identity: None,
                    third_party_token: None,
                })
            }
            PaymentMethodData::Wallet(WalletData::GooglePay(google_pay_wallet_data)) => {
                let third_party_token = google_pay_wallet_data
                    .tokenization_data
                    .get_encrypted_google_pay_token()
                    .change_context(ConnectorError::MissingRequiredField {
                        field_name: "google_pay_token",
                    })?;
                Ok(Self {
                    instrument_type: FinixPaymentInstrumentType::GOOGLEPAY,
                    name: item.router_data.get_optional_billing_full_name(),
                    identity: item.router_data.get_connector_customer_id()?,
                    number: None,
                    security_code: None,
                    expiration_month: None,
                    expiration_year: None,
                    tags: None,
                    address: None,
                    card_brand: None,
                    card_type: None,
                    additional_data: None,
                    merchant_identity: Some(item.merchant_identity_id.clone()),
                    third_party_token: Some(Secret::new(third_party_token)),
                })
            }
            _ => Err(ConnectorError::NotImplemented(
                "Payment method not supported for tokenization".to_string(),
            )
            .into()),
        }
    }
}

// Auth Struct

impl TryFrom<&ConnectorAuthType> for FinixAuthType {
    type Error = error_stack::Report<ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                finix_user_name: api_key.clone(),
                finix_password: api_secret.clone(),
                merchant_id: key1.clone(),
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

impl FinixWebhookBody {
    pub fn get_webhook_object_reference_id(
        &self,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, ConnectorError> {
        match &self.webhook_embedded {
            FinixEmbedded::Authorizations { authorizations } => {
                let authorization = authorizations
                    .first()
                    .ok_or(ConnectorError::WebhookBodyDecodingFailed)?;
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    PaymentIdType::ConnectorTransactionId(authorization.id.to_string()),
                ))
            }
            FinixEmbedded::Transfers { transfers } => {
                let transfer = transfers
                    .first()
                    .ok_or(ConnectorError::WebhookBodyDecodingFailed)?;
                match transfer.payment_type {
                    Some(FinixPaymentType::REVERSAL) => {
                        Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                            RefundIdType::ConnectorRefundId(transfer.id.to_string()),
                        ))
                    }
                    // finix platform fee ignored
                    Some(FinixPaymentType::FEE) => {
                        Err(ConnectorError::WebhookEventTypeNotFound.into())
                    }
                    _ => Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                        PaymentIdType::ConnectorTransactionId(transfer.id.to_string()),
                    )),
                }
            }

            FinixEmbedded::Disputes { disputes } => {
                let dispute = disputes
                    .first()
                    .ok_or(ConnectorError::WebhookBodyDecodingFailed)?;
                Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                    PaymentIdType::ConnectorTransactionId(dispute.transfer.to_string()),
                ))
            }
            FinixEmbedded::Unsupported(_) => Err(ConnectorError::WebhooksNotImplemented.into()),
        }
    }
    pub fn get_webhook_event_type(&self) -> CustomResult<IncomingWebhookEvent, ConnectorError> {
        match &self.webhook_embedded {
            FinixEmbedded::Authorizations { authorizations } => {
                let authorizations = authorizations
                    .first()
                    .ok_or(ConnectorError::WebhookBodyDecodingFailed)?;
                if authorizations.is_void == Some(true) {
                    match authorizations.state {
                        FinixState::FAILED | FinixState::CANCELED | FinixState::UNKNOWN => {
                            Ok(IncomingWebhookEvent::PaymentIntentCancelFailure)
                        }
                        FinixState::PENDING => Ok(IncomingWebhookEvent::PaymentIntentCancelled),
                        FinixState::SUCCEEDED => Ok(IncomingWebhookEvent::PaymentIntentCancelled),
                    }
                } else {
                    match authorizations.state {
                        FinixState::PENDING => {
                            Ok(IncomingWebhookEvent::PaymentIntentAuthorizationSuccess)
                        }
                        FinixState::SUCCEEDED => Ok(IncomingWebhookEvent::PaymentIntentProcessing),
                        FinixState::FAILED | FinixState::CANCELED | FinixState::UNKNOWN => {
                            Ok(IncomingWebhookEvent::PaymentIntentAuthorizationFailure)
                        }
                    }
                }
            }
            FinixEmbedded::Transfers { transfers } => {
                let transfers = transfers
                    .first()
                    .ok_or(ConnectorError::WebhookBodyDecodingFailed)?;

                if transfers.payment_type == Some(FinixPaymentType::REVERSAL) {
                    match transfers.state {
                        FinixState::SUCCEEDED => Ok(IncomingWebhookEvent::RefundSuccess),
                        FinixState::PENDING
                        | FinixState::FAILED
                        | FinixState::CANCELED
                        | FinixState::UNKNOWN => Ok(IncomingWebhookEvent::RefundFailure),
                    }
                } else {
                    match transfers.state {
                        FinixState::PENDING => Ok(IncomingWebhookEvent::PaymentIntentProcessing),
                        FinixState::SUCCEEDED => Ok(IncomingWebhookEvent::PaymentIntentSuccess),
                        FinixState::FAILED | FinixState::CANCELED | FinixState::UNKNOWN => {
                            Ok(IncomingWebhookEvent::PaymentIntentFailure)
                        }
                    }
                }
            }
            FinixEmbedded::Disputes { disputes } => {
                let dispute = disputes
                    .first()
                    .ok_or(ConnectorError::WebhookBodyDecodingFailed)?;

                match dispute.state {
                    FinixDisputeState::PENDING => Ok(IncomingWebhookEvent::DisputeOpened),
                    FinixDisputeState::INQUIRY => Ok(IncomingWebhookEvent::DisputeChallenged),
                    FinixDisputeState::LOST => Ok(IncomingWebhookEvent::DisputeLost),
                    FinixDisputeState::WON => Ok(IncomingWebhookEvent::DisputeWon),
                }
            }
            FinixEmbedded::Unsupported(secret) => {
                Err(ConnectorError::WebhooksNotImplemented.into())
            }
        }
    }

    pub fn get_dispute_details(&self) -> CustomResult<DisputePayload, ConnectorError> {
        match &self.webhook_embedded {
            FinixEmbedded::Disputes { disputes } => {
                let dispute = disputes
                    .first()
                    .ok_or(ConnectorError::WebhookBodyDecodingFailed)?;

                // Ok(DisputePayload {
                //     amount: dispute.amount.clone(),
                //     currency: (),
                //     dispute_stage: (),
                //     connector_status: (),
                //     connector_dispute_id: (),
                //     connector_reason: (),
                //     connector_reason_code: (),
                //     challenge_required_by: (),
                //     created_at: (),
                //     updated_at: (),
                // })
                todo!()
            }
            FinixEmbedded::Authorizations { authorizations: _ }
            | FinixEmbedded::Transfers { transfers: _ }
            | FinixEmbedded::Unsupported(_) => {
                Err(ConnectorError::ResponseDeserializationFailed)
                    .attach_printable("Expected Dispute webhooks,but found other webhooks")?
            }
        }
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
