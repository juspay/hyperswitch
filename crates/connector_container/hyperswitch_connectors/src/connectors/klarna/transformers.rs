use api_models::payments::{KlarnaSessionTokenResponse, SessionToken};
use common_enums::enums;
use common_utils::{pii, types::MinorUnit};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{PayLaterData, PaymentMethodData},
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorAuthType, ConnectorResponseData,
        KlarnaSdkResponse, RouterData,
    },
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsCaptureData, ResponseId},
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types,
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{
        PaymentsResponseRouterData, PaymentsSessionResponseRouterData, RefundsResponseRouterData,
        ResponseRouterData,
    },
    utils::{self, AddressData, AddressDetailsData, PaymentsAuthorizeRequestData, RouterData as _},
};

#[derive(Debug, Serialize)]
pub struct KlarnaRouterData<T> {
    amount: MinorUnit,
    router_data: T,
}

impl<T> From<(MinorUnit, T)> for KlarnaRouterData<T> {
    fn from((amount, router_data): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlarnaConnectorMetadataObject {
    pub klarna_region: Option<KlarnaEndpoint>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KlarnaEndpoint {
    Europe,
    NorthAmerica,
    Oceania,
}

impl From<KlarnaEndpoint> for String {
    fn from(endpoint: KlarnaEndpoint) -> Self {
        Self::from(match endpoint {
            KlarnaEndpoint::Europe => "",
            KlarnaEndpoint::NorthAmerica => "-na",
            KlarnaEndpoint::Oceania => "-oc",
        })
    }
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for KlarnaConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PaymentMethodSpecifics {
    KlarnaCheckout(KlarnaCheckoutRequestData),
    KlarnaSdk,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct MerchantURLs {
    terms: String,
    checkout: String,
    confirmation: String,
    push: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct KlarnaCheckoutRequestData {
    merchant_urls: MerchantURLs,
    options: CheckoutOptions,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct KlarnaPaymentsRequest {
    order_lines: Vec<OrderLines>,
    order_amount: MinorUnit,
    purchase_country: enums::CountryAlpha2,
    purchase_currency: enums::Currency,
    merchant_reference1: Option<String>,
    merchant_reference2: Option<String>,
    shipping_address: Option<KlarnaShippingAddress>,
    auto_capture: Option<bool>,
    order_tax_amount: Option<MinorUnit>,
    #[serde(flatten)]
    payment_method_specifics: Option<PaymentMethodSpecifics>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum KlarnaAuthResponse {
    KlarnaPaymentsAuthResponse(PaymentsResponse),
    KlarnaCheckoutAuthResponse(CheckoutResponse),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaymentsResponse {
    order_id: String,
    fraud_status: KlarnaFraudStatus,
    authorized_payment_method: Option<AuthorizedPaymentMethod>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CheckoutResponse {
    order_id: String,
    status: KlarnaCheckoutStatus,
    html_snippet: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthorizedPaymentMethod {
    #[serde(rename = "type")]
    payment_type: String,
}

impl From<AuthorizedPaymentMethod> for AdditionalPaymentMethodConnectorResponse {
    fn from(item: AuthorizedPaymentMethod) -> Self {
        Self::PayLater {
            klarna_sdk: Some(KlarnaSdkResponse {
                payment_type: Some(item.payment_type),
            }),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct KlarnaSessionRequest {
    intent: KlarnaSessionIntent,
    purchase_country: enums::CountryAlpha2,
    purchase_currency: enums::Currency,
    order_amount: MinorUnit,
    order_lines: Vec<OrderLines>,
    shipping_address: Option<KlarnaShippingAddress>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlarnaShippingAddress {
    city: String,
    country: enums::CountryAlpha2,
    email: pii::Email,
    given_name: Secret<String>,
    family_name: Secret<String>,
    phone: Secret<String>,
    postal_code: Secret<String>,
    region: Secret<String>,
    street_address: Secret<String>,
    street_address2: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CheckoutOptions {
    auto_capture: bool,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct KlarnaSessionResponse {
    pub client_token: Secret<String>,
    pub session_id: String,
}

impl TryFrom<&KlarnaRouterData<&types::PaymentsSessionRouterData>> for KlarnaSessionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &KlarnaRouterData<&types::PaymentsSessionRouterData>,
    ) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        match request.order_details.clone() {
            Some(order_details) => Ok(Self {
                intent: KlarnaSessionIntent::Buy,
                purchase_country: request.country.ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "billing.address.country",
                    },
                )?,
                purchase_currency: request.currency,
                order_amount: item.amount,
                order_lines: order_details
                    .iter()
                    .map(|data| OrderLines {
                        name: data.product_name.clone(),
                        quantity: data.quantity,
                        unit_price: data.amount,
                        total_amount: data.amount * data.quantity,
                        total_tax_amount: None,
                        tax_rate: None,
                    })
                    .collect(),
                shipping_address: get_address_info(item.router_data.get_optional_shipping())
                    .transpose()?,
            }),
            None => Err(report!(errors::ConnectorError::MissingRequiredField {
                field_name: "order_details",
            })),
        }
    }
}

impl TryFrom<PaymentsSessionResponseRouterData<KlarnaSessionResponse>>
    for types::PaymentsSessionRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsSessionResponseRouterData<KlarnaSessionResponse>,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;
        Ok(Self {
            response: Ok(PaymentsResponseData::SessionResponse {
                session_token: SessionToken::Klarna(Box::new(KlarnaSessionTokenResponse {
                    session_token: response.client_token.clone().expose(),
                    session_id: response.session_id.clone(),
                })),
            }),
            ..item.data
        })
    }
}

impl TryFrom<&KlarnaRouterData<&types::PaymentsAuthorizeRouterData>> for KlarnaPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &KlarnaRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        let payment_method_data = request.payment_method_data.clone();
        let return_url = item.router_data.request.get_router_return_url()?;
        let webhook_url = item.router_data.request.get_webhook_url()?;
        match payment_method_data {
            PaymentMethodData::PayLater(PayLaterData::KlarnaSdk { .. }) => {
                match request.order_details.clone() {
                    Some(order_details) => Ok(Self {
                        purchase_country: item.router_data.get_billing_country()?,
                        purchase_currency: request.currency,
                        order_amount: item.amount,
                        order_lines: order_details
                            .iter()
                            .map(|data| OrderLines {
                                name: data.product_name.clone(),
                                quantity: data.quantity,
                                unit_price: data.amount,
                                total_amount: data.amount * data.quantity,
                                total_tax_amount: None,
                                tax_rate: None,
                            })
                            .collect(),
                        merchant_reference1: Some(
                            item.router_data.connector_request_reference_id.clone(),
                        ),
                        merchant_reference2: item
                            .router_data
                            .request
                            .merchant_order_reference_id
                            .clone(),
                        auto_capture: Some(request.is_auto_capture()?),
                        shipping_address: get_address_info(
                            item.router_data.get_optional_shipping(),
                        )
                        .transpose()?,
                        order_tax_amount: None,
                        payment_method_specifics: None,
                    }),
                    None => Err(report!(errors::ConnectorError::MissingRequiredField {
                        field_name: "order_details"
                    })),
                }
            }
            PaymentMethodData::PayLater(PayLaterData::KlarnaRedirect {}) => {
                match request.order_details.clone() {
                    Some(order_details) => Ok(Self {
                        purchase_country: item.router_data.get_billing_country()?,
                        purchase_currency: request.currency,
                        order_amount: item.amount
                            - request.order_tax_amount.unwrap_or(MinorUnit::zero()),
                        order_tax_amount: request.order_tax_amount,
                        order_lines: order_details
                            .iter()
                            .map(|data| OrderLines {
                                name: data.product_name.clone(),
                                quantity: data.quantity,
                                unit_price: data.amount,
                                total_amount: data.amount * data.quantity,
                                total_tax_amount: data.total_tax_amount,
                                tax_rate: data.tax_rate,
                            })
                            .collect(),
                        payment_method_specifics: Some(PaymentMethodSpecifics::KlarnaCheckout(
                            KlarnaCheckoutRequestData {
                                merchant_urls: MerchantURLs {
                                    terms: return_url.clone(),
                                    checkout: return_url.clone(),
                                    confirmation: return_url,
                                    push: webhook_url,
                                },
                                options: CheckoutOptions {
                                    auto_capture: request.is_auto_capture()?,
                                },
                            },
                        )),
                        shipping_address: get_address_info(
                            item.router_data.get_optional_shipping(),
                        )
                        .transpose()?,
                        merchant_reference1: Some(
                            item.router_data.connector_request_reference_id.clone(),
                        ),
                        merchant_reference2: item
                            .router_data
                            .request
                            .merchant_order_reference_id
                            .clone(),
                        auto_capture: None,
                    }),
                    None => Err(report!(errors::ConnectorError::MissingRequiredField {
                        field_name: "order_details"
                    })),
                }
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

fn get_address_info(
    address: Option<&hyperswitch_domain_models::address::Address>,
) -> Option<Result<KlarnaShippingAddress, error_stack::Report<errors::ConnectorError>>> {
    address.and_then(|add| {
        add.address.as_ref().map(
            |address_details| -> Result<KlarnaShippingAddress, error_stack::Report<errors::ConnectorError>> {
                Ok(KlarnaShippingAddress {
                    city: address_details.get_city()?.to_owned(),
                    country: address_details.get_country()?.to_owned(),
                    email: add.get_email()?.to_owned(),
                    postal_code: address_details.get_zip()?.to_owned(),
                    region: address_details.to_state_code()?.to_owned(),
                    street_address: address_details.get_line1()?.to_owned(),
                    street_address2: address_details.get_optional_line2(),
                    given_name: address_details.get_first_name()?.to_owned(),
                    family_name: address_details.get_last_name()?.to_owned(),
                    phone: add.get_phone_with_country_code()?.to_owned(),
                })
            },
        )
    })
}

impl TryFrom<PaymentsResponseRouterData<KlarnaAuthResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: PaymentsResponseRouterData<KlarnaAuthResponse>) -> Result<Self, Self::Error> {
        match item.response {
            KlarnaAuthResponse::KlarnaPaymentsAuthResponse(ref response) => {
                let connector_response =
                    response
                        .authorized_payment_method
                        .as_ref()
                        .map(|authorized_payment_method| {
                            ConnectorResponseData::with_additional_payment_method_data(
                                AdditionalPaymentMethodConnectorResponse::from(
                                    authorized_payment_method.clone(),
                                ),
                            )
                        });

                Ok(Self {
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(response.order_id.clone()),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(response.order_id.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    status: get_fraud_status(
                        response.fraud_status.clone(),
                        item.data.request.is_auto_capture()?,
                    ),
                    connector_response,
                    ..item.data
                })
            }
            KlarnaAuthResponse::KlarnaCheckoutAuthResponse(ref response) => Ok(Self {
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(response.order_id.clone()),
                    redirection_data: Box::new(Some(RedirectForm::Html {
                        html_data: response.html_snippet.clone(),
                    })),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(response.order_id.clone()),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                status: get_checkout_status(
                    response.status.clone(),
                    item.data.request.is_auto_capture()?,
                ),
                connector_response: None,
                ..item.data
            }),
        }
    }
}
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct OrderLines {
    name: String,
    quantity: u16,
    unit_price: MinorUnit,
    total_amount: MinorUnit,
    total_tax_amount: Option<MinorUnit>,
    tax_rate: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum KlarnaSessionIntent {
    Buy,
    Tokenize,
    BuyAndTokenize,
}

pub struct KlarnaAuthType {
    pub username: Secret<String>,
    pub password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for KlarnaAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                username: key1.to_owned(),
                password: api_key.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum KlarnaFraudStatus {
    Accepted,
    Pending,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KlarnaCheckoutStatus {
    CheckoutComplete,
    CheckoutIncomplete,
}

fn get_fraud_status(
    klarna_status: KlarnaFraudStatus,
    is_auto_capture: bool,
) -> common_enums::AttemptStatus {
    match klarna_status {
        KlarnaFraudStatus::Accepted => {
            if is_auto_capture {
                common_enums::AttemptStatus::Charged
            } else {
                common_enums::AttemptStatus::Authorized
            }
        }
        KlarnaFraudStatus::Pending => common_enums::AttemptStatus::Pending,
        KlarnaFraudStatus::Rejected => common_enums::AttemptStatus::Failure,
    }
}

fn get_checkout_status(
    klarna_status: KlarnaCheckoutStatus,
    is_auto_capture: bool,
) -> common_enums::AttemptStatus {
    match klarna_status {
        KlarnaCheckoutStatus::CheckoutIncomplete => {
            if is_auto_capture {
                common_enums::AttemptStatus::AuthenticationPending
            } else {
                common_enums::AttemptStatus::Authorized
            }
        }
        KlarnaCheckoutStatus::CheckoutComplete => common_enums::AttemptStatus::Charged,
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum KlarnaPsyncResponse {
    KlarnaSDKPsyncResponse(KlarnaSDKSyncResponse),
    KlarnaCheckoutPSyncResponse(KlarnaCheckoutSyncResponse),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlarnaSDKSyncResponse {
    pub order_id: String,
    pub status: KlarnaPaymentStatus,
    pub klarna_reference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlarnaCheckoutSyncResponse {
    pub order_id: String,
    pub status: KlarnaCheckoutStatus,
    pub options: CheckoutOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KlarnaPaymentStatus {
    Authorized,
    PartCaptured,
    Captured,
    Cancelled,
    Expired,
    Closed,
}

impl From<KlarnaPaymentStatus> for enums::AttemptStatus {
    fn from(item: KlarnaPaymentStatus) -> Self {
        match item {
            KlarnaPaymentStatus::Authorized => Self::Authorized,
            KlarnaPaymentStatus::PartCaptured => Self::PartialCharged,
            KlarnaPaymentStatus::Captured => Self::Charged,
            KlarnaPaymentStatus::Cancelled => Self::Voided,
            KlarnaPaymentStatus::Expired | KlarnaPaymentStatus::Closed => Self::Failure,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, KlarnaPsyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, KlarnaPsyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            KlarnaPsyncResponse::KlarnaSDKPsyncResponse(response) => Ok(Self {
                status: enums::AttemptStatus::from(response.status),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(response.order_id.clone()),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: response
                        .klarna_reference
                        .or(Some(response.order_id.clone())),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
            KlarnaPsyncResponse::KlarnaCheckoutPSyncResponse(response) => Ok(Self {
                status: get_checkout_status(response.status.clone(), response.options.auto_capture),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(response.order_id.clone()),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(response.order_id.clone()),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct KlarnaCaptureRequest {
    captured_amount: MinorUnit,
    reference: Option<String>,
}

impl TryFrom<&KlarnaRouterData<&types::PaymentsCaptureRouterData>> for KlarnaCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &KlarnaRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let reference = Some(item.router_data.connector_request_reference_id.clone());
        Ok(Self {
            reference,
            captured_amount: item.amount.to_owned(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlarnaMeta {
    capture_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlarnaCaptureResponse {
    pub capture_id: Option<String>,
}

impl<F>
    TryFrom<ResponseRouterData<F, KlarnaCaptureResponse, PaymentsCaptureData, PaymentsResponseData>>
    for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            KlarnaCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let connector_meta = serde_json::json!(KlarnaMeta {
            capture_id: item.response.capture_id,
        });

        // https://docs.klarna.com/api/ordermanagement/#operation/captureOrder
        // If 201 status code, then order is captured, other status codes are handled by the error handler
        let status = if item.http_code == 201 {
            enums::AttemptStatus::Charged
        } else {
            item.data.status
        };
        let resource_id = item.data.request.connector_transaction_id.clone();

        Ok(Self {
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(resource_id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: Some(connector_meta),
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            status,
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct KlarnaRefundRequest {
    refunded_amount: MinorUnit,
    reference: Option<String>,
}

impl<F> TryFrom<&KlarnaRouterData<&types::RefundsRouterData<F>>> for KlarnaRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &KlarnaRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        Ok(Self {
            refunded_amount: item.amount,
            reference: Some(request.refund_id.clone()),
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KlarnaRefundResponse {
    pub refund_id: String,
}

impl TryFrom<RefundsResponseRouterData<Execute, KlarnaRefundResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, KlarnaRefundResponse>,
    ) -> Result<Self, Self::Error> {
        // https://docs.klarna.com/api/ordermanagement/#operation/refundOrder
        // If 201 status code, then Refund is Successful, other status codes are handled by the error handler
        let status = if item.http_code == 201 {
            enums::RefundStatus::Pending
        } else {
            enums::RefundStatus::Failure
        };
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.refund_id,
                refund_status: status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KlarnaRefundSyncResponse {
    pub refund_id: String,
}

impl TryFrom<RefundsResponseRouterData<RSync, KlarnaRefundSyncResponse>>
    for types::RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, KlarnaRefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        // https://docs.klarna.com/api/ordermanagement/#operation/get
        // If 200 status code, then Refund is Successful, other status codes are handled by the error handler
        let status = if item.http_code == 200 {
            enums::RefundStatus::Success
        } else {
            enums::RefundStatus::Failure
        };
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.refund_id,
                refund_status: status,
            }),
            ..item.data
        })
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct KlarnaErrorResponse {
    pub error_code: String,
    pub error_messages: Option<Vec<String>>,
    pub error_message: Option<String>,
}
