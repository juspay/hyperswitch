use masking::PeekInterface;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, PaymentsRequestData},
    consts,
    core::errors,
    pii::{self, Email, Secret},
    services,
    types::{
        self,
        api::{self, enums as api_enums},
        storage::enums as storage_enums,
    },
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

#[derive(Default, Debug, Serialize, Deserialize)]
pub enum AuthType {
    #[default]
    PreAuth,
}
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalData {
    authorisation_type: AuthType,
    manual_capture: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopperName {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    city: Option<String>,
    country: Option<String>,
    house_number_or_name: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    state_or_province: Option<Secret<String>>,
    street: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineItem {
    amount_excluding_tax: Option<i64>,
    amount_including_tax: Option<i64>,
    description: Option<String>,
    id: Option<String>,
    tax_amount: Option<i64>,
    quantity: Option<u16>,
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
    additional_data: Option<AdditionalData>,
    shopper_name: Option<ShopperName>,
    shopper_email: Option<Secret<String, Email>>,
    telephone_number: Option<Secret<String>>,
    billing_address: Option<Address>,
    delivery_address: Option<Address>,
    country_code: Option<String>,
    line_items: Option<Vec<LineItem>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdyenStatus {
    Authorised,
    Refused,
    Cancelled,
    RedirectShopper,
}

impl From<AdyenStatus> for storage_enums::AttemptStatus {
    fn from(item: AdyenStatus) -> Self {
        match item {
            AdyenStatus::Authorised => Self::Charged,
            AdyenStatus::Refused => Self::Failure,
            AdyenStatus::Cancelled => Self::Voided,
            AdyenStatus::RedirectShopper => Self::AuthenticationPending,
        }
    }
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

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AdyenPaymentResponse {
    AdyenResponse(AdyenResponse),
    AdyenRedirectResponse(AdyenRedirectionResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenResponse {
    psp_reference: String,
    result_code: AdyenStatus,
    amount: Option<Amount>,
    merchant_reference: String,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRedirectionResponse {
    result_code: AdyenStatus,
    action: AdyenRedirectionAction,
    refusal_reason: Option<String>,
    refusal_reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRedirectionAction {
    payment_method_type: String,
    url: Url,
    method: services::Method,
    #[serde(rename = "type")]
    type_of_response: String,
    data: Option<std::collections::HashMap<String, String>>,
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
    AfterPay(AdyenPayLaterData),
    AdyenKlarna(AdyenPayLaterData),
    AdyenAffirm(AdyenPayLaterData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCard {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    number: Secret<String, pii::CardNumber>,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCancelRequest {
    merchant_account: String,
    reference: String,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCancelResponse {
    psp_reference: String,
    status: CancelStatus,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CancelStatus {
    Received,
    #[default]
    Processing,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPaypal {
    #[serde(rename = "type")]
    payment_type: PaymentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdyenGPay {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    #[serde(rename = "googlePayToken")]
    google_pay_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdyenApplePay {
    #[serde(rename = "type")]
    payment_type: PaymentType,
    #[serde(rename = "applePayToken")]
    apple_pay_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdyenPayLaterData {
    #[serde(rename = "type")]
    payment_type: PaymentType,
}

// Refunds Request and Response
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenRefundRequest {
    merchant_account: String,
    amount: Amount,
    merchant_refund_reason: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentType {
    Scheme,
    Googlepay,
    Applepay,
    Paypal,
    Klarna,
    Affirm,
    Afterpaytouch,
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

// Payment Request Transform
impl TryFrom<&types::PaymentsAuthorizeRouterData> for AdyenPaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.payment_method {
            storage_models::enums::PaymentMethodType::Card => get_card_specific_payment_data(item),
            storage_models::enums::PaymentMethodType::PayLater => {
                get_paylater_specific_payment_data(item)
            }
            storage_models::enums::PaymentMethodType::Wallet => {
                get_wallet_specific_payment_data(item)
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

impl From<&types::PaymentsAuthorizeRouterData> for AdyenShopperInteraction {
    fn from(item: &types::PaymentsAuthorizeRouterData) -> Self {
        match item.request.off_session {
            Some(true) => Self::ContinuedAuthentication,
            _ => Self::Ecommerce,
        }
    }
}

fn get_recurring_processing_model(
    item: &types::PaymentsAuthorizeRouterData,
) -> Option<AdyenRecurringModel> {
    match item.request.setup_future_usage {
        Some(storage_enums::FutureUsage::OffSession) => {
            Some(AdyenRecurringModel::UnscheduledCardOnFile)
        }
        _ => None,
    }
}

fn get_browser_info(item: &types::PaymentsAuthorizeRouterData) -> Option<AdyenBrowserInfo> {
    if matches!(item.auth_type, storage_enums::AuthenticationType::ThreeDs) {
        item.request
            .browser_info
            .as_ref()
            .map(|info| AdyenBrowserInfo {
                accept_header: info.accept_header.clone(),
                language: info.language.clone(),
                screen_height: info.screen_height,
                screen_width: info.screen_width,
                color_depth: info.color_depth,
                user_agent: info.user_agent.clone(),
                time_zone_offset: info.time_zone,
                java_enabled: info.java_enabled,
            })
    } else {
        None
    }
}

fn get_additional_data(item: &types::PaymentsAuthorizeRouterData) -> Option<AdditionalData> {
    match item.request.capture_method {
        Some(storage_models::enums::CaptureMethod::Manual) => Some(AdditionalData {
            authorisation_type: AuthType::PreAuth,
            manual_capture: true,
        }),
        _ => None,
    }
}

fn get_amount_data(item: &types::PaymentsAuthorizeRouterData) -> Amount {
    Amount {
        currency: item.request.currency.to_string(),
        value: item.request.amount,
    }
}

fn get_address_info(address: Option<&api_models::payments::Address>) -> Option<Address> {
    address.and_then(|add| {
        add.address.as_ref().map(|a| Address {
            city: a.city.clone(),
            country: a.country.clone(),
            house_number_or_name: a.line1.clone(),
            postal_code: a.zip.clone(),
            state_or_province: a.state.clone(),
            street: a.line2.clone(),
        })
    })
}

fn get_line_items(item: &types::PaymentsAuthorizeRouterData) -> Vec<LineItem> {
    let order_details = item.request.order_details.as_ref();
    let line_item = LineItem {
        amount_including_tax: Some(item.request.amount),
        amount_excluding_tax: None,
        description: order_details.map(|details| details.product_name.clone()),
        // We support only one product details in payment request as of now, therefore hard coded the id.
        // If we begin to support multiple product details in future then this logic should be made to create ID dynamically
        id: Some(String::from("Items #1")),
        tax_amount: None,
        quantity: order_details.map(|details| details.quantity),
    };
    vec![line_item]
}

fn get_telephone_number(item: &types::PaymentsAuthorizeRouterData) -> Option<Secret<String>> {
    let phone = item
        .address
        .billing
        .as_ref()
        .and_then(|billing| billing.phone.as_ref());
    phone.as_ref().and_then(|phone| {
        phone.number.as_ref().and_then(|number| {
            phone
                .country_code
                .as_ref()
                .map(|cc| Secret::new(format!("{}{}", cc, number.peek())))
        })
    })
}

fn get_shopper_name(item: &types::PaymentsAuthorizeRouterData) -> Option<ShopperName> {
    let address = item
        .address
        .billing
        .as_ref()
        .and_then(|billing| billing.address.as_ref());
    Some(ShopperName {
        first_name: address.and_then(|address| address.first_name.clone()),
        last_name: address.and_then(|address| address.last_name.clone()),
    })
}

fn get_country_code(item: &types::PaymentsAuthorizeRouterData) -> Option<String> {
    let address = item
        .address
        .billing
        .as_ref()
        .and_then(|billing| billing.address.as_ref());
    address.and_then(|address| address.country.clone())
}

fn get_payment_method_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<AdyenPaymentMethod, error_stack::Report<errors::ConnectorError>> {
    match item.request.payment_method_data {
        api::PaymentMethod::Card(ref card) => {
            let adyen_card = AdyenCard {
                payment_type: PaymentType::Scheme,
                number: card.card_number.clone(),
                expiry_month: card.card_exp_month.clone(),
                expiry_year: card.card_exp_year.clone(),
                cvc: Some(card.card_cvc.clone()),
            };
            Ok(AdyenPaymentMethod::AdyenCard(adyen_card))
        }
        api::PaymentMethod::Wallet(ref wallet_data) => match wallet_data.issuer_name {
            api_enums::WalletIssuer::GooglePay => {
                let gpay_data = AdyenGPay {
                    payment_type: PaymentType::Googlepay,
                    google_pay_token: wallet_data
                        .token
                        .clone()
                        .ok_or_else(utils::missing_field_err("token"))?,
                };
                Ok(AdyenPaymentMethod::Gpay(gpay_data))
            }

            api_enums::WalletIssuer::ApplePay => {
                let apple_pay_data = AdyenApplePay {
                    payment_type: PaymentType::Applepay,
                    apple_pay_token: wallet_data
                        .token
                        .clone()
                        .ok_or_else(utils::missing_field_err("token"))?,
                };
                Ok(AdyenPaymentMethod::ApplePay(apple_pay_data))
            }
            api_enums::WalletIssuer::Paypal => {
                let wallet = AdyenPaypal {
                    payment_type: PaymentType::Paypal,
                };
                Ok(AdyenPaymentMethod::AdyenPaypal(wallet))
            }
        },
        api_models::payments::PaymentMethod::PayLater(ref pay_later_data) => match pay_later_data {
            api_models::payments::PayLaterData::KlarnaRedirect { .. } => {
                let klarna = AdyenPayLaterData {
                    payment_type: PaymentType::Klarna,
                };
                Ok(AdyenPaymentMethod::AdyenKlarna(klarna))
            }
            api_models::payments::PayLaterData::AffirmRedirect { .. } => {
                Ok(AdyenPaymentMethod::AdyenAffirm(AdyenPayLaterData {
                    payment_type: PaymentType::Affirm,
                }))
            }
            api_models::payments::PayLaterData::AfterpayClearpayRedirect { .. } => {
                Ok(AdyenPaymentMethod::AfterPay(AdyenPayLaterData {
                    payment_type: PaymentType::Afterpaytouch,
                }))
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        },
        api_models::payments::PaymentMethod::BankTransfer
        | api_models::payments::PaymentMethod::Paypal => {
            Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into())
        }
    }
}

fn get_card_specific_payment_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<AdyenPaymentRequest, error_stack::Report<errors::ConnectorError>> {
    let amount = get_amount_data(item);
    let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
    let shopper_interaction = AdyenShopperInteraction::from(item);
    let recurring_processing_model = get_recurring_processing_model(item);
    let browser_info = get_browser_info(item);
    let additional_data = get_additional_data(item);
    let return_url = item.get_return_url()?;
    let payment_method = get_payment_method_data(item)?;
    Ok(AdyenPaymentRequest {
        amount,
        merchant_account: auth_type.merchant_account,
        payment_method,
        reference: item.payment_id.to_string(),
        return_url,
        shopper_interaction,
        recurring_processing_model,
        browser_info,
        additional_data,
        telephone_number: None,
        shopper_name: None,
        shopper_email: None,
        billing_address: None,
        delivery_address: None,
        country_code: None,
        line_items: None,
    })
}

fn get_wallet_specific_payment_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<AdyenPaymentRequest, error_stack::Report<errors::ConnectorError>> {
    let amount = get_amount_data(item);
    let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
    let browser_info = get_browser_info(item);
    let additional_data = get_additional_data(item);
    let payment_method = get_payment_method_data(item)?;
    let shopper_interaction = AdyenShopperInteraction::from(item);
    let recurring_processing_model = get_recurring_processing_model(item);
    let return_url = item.get_return_url()?;
    Ok(AdyenPaymentRequest {
        amount,
        merchant_account: auth_type.merchant_account,
        payment_method,
        reference: item.payment_id.to_string(),
        return_url,
        shopper_interaction,
        recurring_processing_model,
        browser_info,
        additional_data,
        telephone_number: None,
        shopper_name: None,
        shopper_email: None,
        billing_address: None,
        delivery_address: None,
        country_code: None,
        line_items: None,
    })
}

fn get_paylater_specific_payment_data(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<AdyenPaymentRequest, error_stack::Report<errors::ConnectorError>> {
    let amount = get_amount_data(item);
    let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
    let browser_info = get_browser_info(item);
    let additional_data = get_additional_data(item);
    let payment_method = get_payment_method_data(item)?;
    let shopper_interaction = AdyenShopperInteraction::from(item);
    let recurring_processing_model = get_recurring_processing_model(item);
    let return_url = item.get_return_url()?;
    let shopper_name = get_shopper_name(item);
    let shopper_email = item.request.email.clone();
    let billing_address = get_address_info(item.address.billing.as_ref());
    let delivery_address = get_address_info(item.address.shipping.as_ref());
    let country_code = get_country_code(item);
    let line_items = Some(get_line_items(item));
    let telephone_number = get_telephone_number(item);
    Ok(AdyenPaymentRequest {
        amount,
        merchant_account: auth_type.merchant_account,
        payment_method,
        reference: item.payment_id.to_string(),
        return_url,
        shopper_interaction,
        recurring_processing_model,
        browser_info,
        additional_data,
        telephone_number,
        shopper_name,
        shopper_email,
        billing_address,
        delivery_address,
        country_code,
        line_items,
    })
}

impl TryFrom<&types::PaymentsCancelRouterData> for AdyenCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            merchant_account: auth_type.merchant_account,
            reference: item.payment_id.to_string(),
        })
    }
}

impl From<CancelStatus> for storage_enums::AttemptStatus {
    fn from(status: CancelStatus) -> Self {
        match status {
            CancelStatus::Received => Self::Voided,
            CancelStatus::Processing => Self::Pending,
        }
    }
}

impl TryFrom<types::PaymentsCancelResponseRouterData<AdyenCancelResponse>>
    for types::PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsCancelResponseRouterData<AdyenCancelResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: item.response.status.into(),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.psp_reference),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

pub fn get_adyen_response(
    response: AdyenResponse,
    is_capture_manual: bool,
    status_code: u16,
) -> errors::CustomResult<
    (
        storage_enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status = match response.result_code {
        AdyenStatus::Authorised => {
            if is_capture_manual {
                storage_enums::AttemptStatus::Authorized
            } else {
                storage_enums::AttemptStatus::Charged
            }
        }
        AdyenStatus::Refused | AdyenStatus::Cancelled => storage_enums::AttemptStatus::Failure,
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
            status_code,
        })
    } else {
        None
    };

    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::ConnectorTransactionId(response.psp_reference),
        redirection_data: None,
        mandate_reference: None,
        connector_metadata: None,
    };
    Ok((status, error, payments_response_data))
}

pub fn get_redirection_response(
    response: AdyenRedirectionResponse,
    status_code: u16,
) -> errors::CustomResult<
    (
        storage_enums::AttemptStatus,
        Option<types::ErrorResponse>,
        types::PaymentsResponseData,
    ),
    errors::ConnectorError,
> {
    let status = response.result_code.into();

    let error = if response.refusal_reason.is_some() || response.refusal_reason_code.is_some() {
        Some(types::ErrorResponse {
            code: response
                .refusal_reason_code
                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
            message: response
                .refusal_reason
                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
            status_code,
        })
    } else {
        None
    };

    let form_fields = response.action.data.unwrap_or_else(|| {
        std::collections::HashMap::from_iter(
            response
                .action
                .url
                .query_pairs()
                .map(|(key, value)| (key.to_string(), value.to_string())),
        )
    });

    let redirection_data = services::RedirectForm {
        endpoint: response.action.url.to_string(),
        method: response.action.method,
        form_fields,
    };

    // We don't get connector transaction id for redirections in Adyen.
    let payments_response_data = types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::NoResponseId,
        redirection_data: Some(redirection_data),
        mandate_reference: None,
        connector_metadata: None,
    };
    Ok((status, error, payments_response_data))
}

impl<F, Req>
    TryFrom<(
        types::ResponseRouterData<F, AdyenPaymentResponse, Req, types::PaymentsResponseData>,
        bool,
    )> for types::RouterData<F, Req, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        items: (
            types::ResponseRouterData<F, AdyenPaymentResponse, Req, types::PaymentsResponseData>,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        let item = items.0;
        let is_manual_capture = items.1;
        let (status, error, payment_response_data) = match item.response {
            AdyenPaymentResponse::AdyenResponse(response) => {
                get_adyen_response(response, is_manual_capture, item.http_code)?
            }
            AdyenPaymentResponse::AdyenRedirectResponse(response) => {
                get_redirection_response(response, item.http_code)?
            }
        };

        Ok(Self {
            status,
            response: error.map_or_else(|| Ok(payment_response_data), Err),
            ..item.data
        })
    }
}
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCaptureRequest {
    merchant_account: String,
    amount: Amount,
    reference: String,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for AdyenCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let auth_type = AdyenAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            merchant_account: auth_type.merchant_account,
            reference: item.payment_id.to_string(),
            amount: Amount {
                currency: item.request.currency.to_string(),
                value: item
                    .request
                    .amount_to_capture
                    .unwrap_or(item.request.amount),
            },
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCaptureResponse {
    merchant_account: String,
    payment_psp_reference: String,
    psp_reference: String,
    reference: String,
    status: String,
    amount: Amount,
}

impl TryFrom<types::PaymentsCaptureResponseRouterData<AdyenCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsCaptureResponseRouterData<AdyenCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let (status, amount_captured) = match item.response.status.as_str() {
            "received" => (
                storage_enums::AttemptStatus::Charged,
                Some(item.response.amount.value),
            ),
            _ => (storage_enums::AttemptStatus::Pending, None),
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.psp_reference),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            amount_captured,
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
            amount: Amount {
                currency: item.request.currency.to_string(),
                value: item.request.refund_amount,
            },
            merchant_refund_reason: item.request.reason.clone(),
            reference: item.request.refund_id.clone(),
        })
    }
}

// Refund Response Transform
impl<F> TryFrom<types::RefundsResponseRouterData<F, AdyenRefundResponse>>
    for types::RefundsRouterData<F>
{
    type Error = error_stack::Report<errors::ConnectorError>;
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
//     fn verify_transform_from_router_to_adyen_req() {
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
            result_code: match notif.success.as_str() {
                "true" => AdyenStatus::Authorised,
                _ => AdyenStatus::Refused,
            },
            amount: Some(Amount {
                value: notif.amount.value,
                currency: notif.amount.currency,
            }),
            refusal_reason: None,
            refusal_reason_code: None,
        }
    }
}
