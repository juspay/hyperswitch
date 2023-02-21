use std::str::FromStr;

use api_models::{self, payments};
use common_utils::{fp_utils, pii::Email};
use error_stack::{IntoReport, ResultExt};
use masking::ExposeInterface;
use serde::{Deserialize, Serialize};
use strum::EnumString;
use url::Url;
use uuid::Uuid;

use crate::{
    core::errors,
    pii::{self, ExposeOptionInterface, Secret},
    services,
    types::{self, api, storage::enums},
};

pub struct StripeAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for StripeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = item {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

// Stripe Types Definition
// PAYMENT
// PaymentIntentRequest

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StripeCaptureMethod {
    Manual,
    #[default]
    Automatic,
}

impl From<Option<enums::CaptureMethod>> for StripeCaptureMethod {
    fn from(item: Option<enums::CaptureMethod>) -> Self {
        match item {
            Some(p) => match p {
                enums::CaptureMethod::ManualMultiple => Self::Manual,
                enums::CaptureMethod::Manual => Self::Manual,
                enums::CaptureMethod::Automatic => Self::Automatic,
                enums::CaptureMethod::Scheduled => Self::Manual,
            },
            None => Self::Automatic,
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Auth3ds {
    #[default]
    Automatic,
    Any,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct PaymentIntentRequest {
    pub amount: i64, //amount in cents, hence passed as integer
    pub currency: String,
    pub statement_descriptor_suffix: Option<String>,
    #[serde(rename = "metadata[order_id]")]
    pub metadata_order_id: String,
    #[serde(rename = "metadata[txn_id]")]
    pub metadata_txn_id: String,
    #[serde(rename = "metadata[txn_uuid]")]
    pub metadata_txn_uuid: String,
    pub return_url: String,
    pub confirm: bool,
    pub mandate: Option<String>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub shipping: StripeShippingAddress,
    #[serde(flatten)]
    pub billing: StripeBillingAddress,
    #[serde(flatten)]
    pub payment_data: Option<StripePaymentMethodData>,
    pub capture_method: StripeCaptureMethod,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SetupIntentRequest {
    #[serde(rename = "metadata[order_id]")]
    pub metadata_order_id: String,
    #[serde(rename = "metadata[txn_id]")]
    pub metadata_txn_id: String,
    #[serde(rename = "metadata[txn_uuid]")]
    pub metadata_txn_uuid: String,
    pub confirm: bool,
    pub usage: Option<enums::FutureUsage>,
    pub off_session: Option<bool>,
    #[serde(flatten)]
    pub payment_data: StripePaymentMethodData,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripeCardData {
    #[serde(rename = "payment_method_types[]")]
    pub payment_method_types: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[card][number]")]
    pub payment_method_data_card_number: Secret<String, pii::CardNumber>,
    #[serde(rename = "payment_method_data[card][exp_month]")]
    pub payment_method_data_card_exp_month: Secret<String>,
    #[serde(rename = "payment_method_data[card][exp_year]")]
    pub payment_method_data_card_exp_year: Secret<String>,
    #[serde(rename = "payment_method_data[card][cvc]")]
    pub payment_method_data_card_cvc: Secret<String>,
    #[serde(rename = "payment_method_options[card][request_three_d_secure]")]
    pub payment_method_auth_type: Auth3ds,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct StripePayLaterData {
    #[serde(rename = "payment_method_types[]")]
    pub payment_method_types: StripePaymentMethodType,
    #[serde(rename = "payment_method_data[type]")]
    pub payment_method_data_type: StripePaymentMethodType,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StripePaymentMethodData {
    Card(StripeCardData),
    PayLater(StripePayLaterData),
    Bank,
    Wallet,
    Paypal,
}

#[derive(Debug, Eq, PartialEq, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodType {
    Card,
    Klarna,
    Affirm,
    AfterpayClearpay,
}

fn validate_shipping_address_against_payment_method(
    shipping_address: &StripeShippingAddress,
    payment_method: &StripePaymentMethodType,
) -> Result<(), errors::ConnectorError> {
    if let StripePaymentMethodType::AfterpayClearpay = payment_method {
        fp_utils::when(shipping_address.name.is_none(), || {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "shipping.address.first_name",
            })
        })?;

        fp_utils::when(shipping_address.line1.is_none(), || {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "shipping.address.line1",
            })
        })?;

        fp_utils::when(shipping_address.country.is_none(), || {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "shipping.address.country",
            })
        })?;

        fp_utils::when(shipping_address.zip.is_none(), || {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "shipping.address.zip",
            })
        })?;
    }
    Ok(())
}

fn infer_stripe_pay_later_issuer(
    issuer: &enums::PaymentIssuer,
    experience: &enums::PaymentExperience,
) -> Result<StripePaymentMethodType, errors::ConnectorError> {
    if &enums::PaymentExperience::RedirectToUrl == experience {
        match issuer {
            enums::PaymentIssuer::Klarna => Ok(StripePaymentMethodType::Klarna),
            enums::PaymentIssuer::Affirm => Ok(StripePaymentMethodType::Affirm),
            enums::PaymentIssuer::AfterpayClearpay => Ok(StripePaymentMethodType::AfterpayClearpay),
            _ => Err(errors::ConnectorError::NotSupported {
                payment_method: format!("{issuer} payments by {experience}"),
                connector: "stripe",
            }),
        }
    } else {
        Err(errors::ConnectorError::NotSupported {
            payment_method: format!("{issuer} payments by {experience}"),
            connector: "stripe",
        })
    }
}

impl TryFrom<(&api_models::payments::PayLaterData, StripePaymentMethodType)>
    for StripeBillingAddress
{
    type Error = errors::ConnectorError;

    fn try_from(
        (pay_later_data, pm_type): (&api_models::payments::PayLaterData, StripePaymentMethodType),
    ) -> Result<Self, Self::Error> {
        match (pay_later_data, pm_type) {
            (
                payments::PayLaterData::KlarnaRedirect {
                    billing_email,
                    billing_country,
                },
                StripePaymentMethodType::Klarna,
            ) => Ok(Self {
                email: Some(billing_email.to_owned()),
                country: Some(billing_country.to_owned()),
                ..Self::default()
            }),
            (payments::PayLaterData::AffirmRedirect {}, StripePaymentMethodType::Affirm) => {
                Ok(Self::default())
            }
            (
                payments::PayLaterData::AfterpayClearpayRedirect {
                    billing_email,
                    billing_name,
                },
                StripePaymentMethodType::AfterpayClearpay,
            ) => Ok(Self {
                email: Some(billing_email.to_owned()),
                name: Some(billing_name.to_owned()),
                ..Self::default()
            }),
            _ => Err(errors::ConnectorError::MismatchedPaymentData),
        }
    }
}

fn create_stripe_payment_method(
    issuer: Option<&enums::PaymentIssuer>,
    experience: Option<&enums::PaymentExperience>,
    payment_method: &api_models::payments::PaymentMethod,
    auth_type: enums::AuthenticationType,
) -> Result<
    (
        StripePaymentMethodData,
        StripePaymentMethodType,
        StripeBillingAddress,
    ),
    errors::ConnectorError,
> {
    match payment_method {
        payments::PaymentMethod::Card(card_details) => {
            let payment_method_auth_type = match auth_type {
                enums::AuthenticationType::ThreeDs => Auth3ds::Any,
                enums::AuthenticationType::NoThreeDs => Auth3ds::Automatic,
            };
            Ok((
                StripePaymentMethodData::Card(StripeCardData {
                    payment_method_types: StripePaymentMethodType::Card,
                    payment_method_data_type: StripePaymentMethodType::Card,
                    payment_method_data_card_number: card_details.card_number.clone(),
                    payment_method_data_card_exp_month: card_details.card_exp_month.clone(),
                    payment_method_data_card_exp_year: card_details.card_exp_year.clone(),
                    payment_method_data_card_cvc: card_details.card_cvc.clone(),
                    payment_method_auth_type,
                }),
                StripePaymentMethodType::Card,
                StripeBillingAddress::default(),
            ))
        }
        payments::PaymentMethod::PayLater(pay_later_data) => {
            let pm_issuer = issuer.ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_issuer",
            })?;

            let pm_experience = experience.ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_experience",
            })?;

            let pm_type = infer_stripe_pay_later_issuer(pm_issuer, pm_experience)?;

            let billing_address =
                StripeBillingAddress::try_from((pay_later_data, pm_type.clone()))?;

            Ok((
                StripePaymentMethodData::PayLater(StripePayLaterData {
                    payment_method_types: pm_type.clone(),
                    payment_method_data_type: pm_type.clone(),
                }),
                pm_type,
                billing_address,
            ))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            "stripe does not support this payment method".to_string(),
        )),
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PaymentIntentRequest {
    type Error = errors::ConnectorError;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let metadata_order_id = item.payment_id.to_string();
        let metadata_txn_id = format!("{}_{}_{}", item.merchant_id, item.payment_id, "1");
        let metadata_txn_uuid = Uuid::new_v4().to_string(); //Fetch autogenerated txn_uuid from Database.

        let shipping_address = match item.address.shipping.clone() {
            Some(mut shipping) => StripeShippingAddress {
                city: shipping.address.as_mut().and_then(|a| a.city.take()),
                country: shipping.address.as_mut().and_then(|a| a.country.take()),
                line1: shipping.address.as_mut().and_then(|a| a.line1.take()),
                line2: shipping.address.as_mut().and_then(|a| a.line2.take()),
                zip: shipping.address.as_mut().and_then(|a| a.zip.take()),
                state: shipping.address.as_mut().and_then(|a| a.state.take()),
                name: shipping.address.as_mut().and_then(|a| {
                    a.first_name.as_ref().map(|first_name| {
                        format!(
                            "{} {}",
                            first_name.clone().expose(),
                            a.last_name.clone().expose_option().unwrap_or_default()
                        )
                        .into()
                    })
                }),
                phone: shipping.phone.map(|p| {
                    format!(
                        "{}{}",
                        p.country_code.unwrap_or_default(),
                        p.number.expose_option().unwrap_or_default()
                    )
                    .into()
                }),
            },
            None => StripeShippingAddress::default(),
        };

        let (payment_data, mandate, billing_address) = {
            match item
                .request
                .mandate_id
                .clone()
                .and_then(|mandate_ids| mandate_ids.connector_mandate_id)
            {
                None => {
                    let (payment_method_data, payment_method_type, billing_address) =
                        create_stripe_payment_method(
                            item.request.payment_issuer.as_ref(),
                            item.request.payment_experience.as_ref(),
                            &item.request.payment_method_data,
                            item.auth_type,
                        )?;

                    validate_shipping_address_against_payment_method(
                        &shipping_address,
                        &payment_method_type,
                    )?;

                    (Some(payment_method_data), None, billing_address)
                }
                Some(mandate_id) => (None, Some(mandate_id), StripeBillingAddress::default()),
            }
        };

        Ok(Self {
            amount: item.request.amount, //hopefully we don't loose some cents here
            currency: item.request.currency.to_string(), //we need to copy the value and not transfer ownership
            statement_descriptor_suffix: item.request.statement_descriptor_suffix.clone(),
            metadata_order_id,
            metadata_txn_id,
            metadata_txn_uuid,
            return_url: item
                .router_return_url
                .clone()
                .unwrap_or_else(|| "https://juspay.in/".to_string()),
            confirm: true, // Stripe requires confirm to be true if return URL is present

            description: item.description.clone(),
            shipping: shipping_address,
            billing: billing_address,
            capture_method: StripeCaptureMethod::from(item.request.capture_method),
            payment_data,
            mandate,
        })
    }
}

impl TryFrom<&types::VerifyRouterData> for SetupIntentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::VerifyRouterData) -> Result<Self, Self::Error> {
        let metadata_order_id = item.payment_id.to_string();
        let metadata_txn_id = format!("{}_{}_{}", item.merchant_id, item.payment_id, "1");
        let metadata_txn_uuid = Uuid::new_v4().to_string();

        //Only cards supported for mandates
        let pm_type = StripePaymentMethodType::Card;
        let payment_data = StripePaymentMethodData::try_from((
            item.request.payment_method_data.clone(),
            item.auth_type,
            pm_type,
        ))?;

        Ok(Self {
            confirm: true,
            metadata_order_id,
            metadata_txn_id,
            metadata_txn_uuid,
            payment_data,
            off_session: item.request.off_session,
            usage: item.request.setup_future_usage,
        })
    }
}

// PaymentIntentResponse

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeMetadata {
    pub order_id: String,
    pub txn_id: String,
    pub txn_uuid: String,
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
    #[serde(rename = "requires_action")]
    RequiresCustomerAction,
    #[serde(rename = "requires_payment_method")]
    RequiresPaymentMethod,
    RequiresConfirmation,
    Canceled,
    RequiresCapture,
}

impl From<StripePaymentStatus> for enums::AttemptStatus {
    fn from(item: StripePaymentStatus) -> Self {
        match item {
            StripePaymentStatus::Succeeded => Self::Charged,
            StripePaymentStatus::Failed => Self::Failure,
            StripePaymentStatus::Processing => Self::Authorizing,
            StripePaymentStatus::RequiresCustomerAction => Self::AuthenticationPending,
            StripePaymentStatus::RequiresPaymentMethod => Self::PaymentMethodAwaited,
            StripePaymentStatus::RequiresConfirmation => Self::ConfirmationAwaited,
            StripePaymentStatus::Canceled => Self::Voided,
            StripePaymentStatus::RequiresCapture => Self::Authorized,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct PaymentIntentResponse {
    pub id: String,
    pub object: String,
    pub amount: i64,
    pub amount_received: i64,
    pub amount_capturable: i64,
    pub currency: String,
    pub status: StripePaymentStatus,
    pub client_secret: Secret<String>,
    pub created: i32,
    pub customer: Option<String>,
    pub description: Option<String>,
    pub statement_descriptor: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: StripeMetadata,
    pub next_action: Option<StripeNextActionResponse>,
    pub payment_method_options: Option<StripePaymentMethodOptions>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct SetupIntentResponse {
    pub id: String,
    pub object: String,
    pub status: StripePaymentStatus, // Change to SetupStatus
    pub client_secret: Secret<String>,
    pub customer: Option<String>,
    pub statement_descriptor: Option<String>,
    pub statement_descriptor_suffix: Option<String>,
    pub metadata: StripeMetadata,
    pub next_action: Option<StripeNextActionResponse>,
    pub payment_method_options: Option<StripePaymentMethodOptions>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PaymentIntentResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaymentIntentResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data =
            item.response
                .next_action
                .map(|StripeNextActionResponse::RedirectToUrl(response)| {
                    services::RedirectForm::from((response.url, services::Method::Get))
                });

        let mandate_reference =
            item.response
                .payment_method_options
                .and_then(|payment_method_options| match payment_method_options {
                    StripePaymentMethodOptions::Card {
                        mandate_options, ..
                    } => mandate_options.map(|mandate_options| mandate_options.reference),
                    StripePaymentMethodOptions::Klarna {} => None,
                    StripePaymentMethodOptions::Affirm {} => None,
                    StripePaymentMethodOptions::AfterpayClearpay {} => None,
                });

        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            // client_secret: Some(item.response.client_secret.clone().as_str()),
            // description: item.response.description.map(|x| x.as_str()),
            // statement_descriptor_suffix: item.response.statement_descriptor_suffix.map(|x| x.as_str()),
            // three_ds_form,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference,
                connector_metadata: None,
            }),
            amount_captured: Some(item.response.amount_received),
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, SetupIntentResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, SetupIntentResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data =
            item.response
                .next_action
                .map(|StripeNextActionResponse::RedirectToUrl(response)| {
                    services::RedirectForm::from((response.url, services::Method::Get))
                });

        let mandate_reference =
            item.response
                .payment_method_options
                .and_then(|payment_method_options| match payment_method_options {
                    StripePaymentMethodOptions::Card {
                        mandate_options, ..
                    } => mandate_options.map(|mandate_option| mandate_option.reference),
                    StripePaymentMethodOptions::Klarna {} => None,
                    StripePaymentMethodOptions::Affirm {} => None,
                    StripePaymentMethodOptions::AfterpayClearpay {} => None,
                });

        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case", remote = "Self")]
pub enum StripeNextActionResponse {
    RedirectToUrl(StripeRedirectToUrlResponse),
}

// This impl is required because Stripe's response is of the below format, which is externally
// tagged, but also with an extra 'type' field specifying the enum variant name:
// "next_action": {
//   "redirect_to_url": { "return_url": "...", "url": "..." },
//   "type": "redirect_to_url"
// },
// Reference: https://github.com/serde-rs/serde/issues/1343#issuecomment-409698470
impl<'de> Deserialize<'de> for StripeNextActionResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(rename = "type")]
            _ignore: String,
            #[serde(flatten, with = "StripeNextActionResponse")]
            inner: StripeNextActionResponse,
        }
        Wrapper::deserialize(deserializer).map(|w| w.inner)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct StripeRedirectToUrlResponse {
    return_url: String,
    url: Url,
}

// REFUND :
// Type definition for Stripe RefundRequest

#[derive(Default, Debug, Serialize)]
pub struct RefundRequest {
    pub amount: Option<i64>, //amount in cents, hence passed as integer
    pub payment_intent: String,
    #[serde(rename = "metadata[order_id]")]
    pub metadata_order_id: String,
    #[serde(rename = "metadata[txn_id]")]
    pub metadata_txn_id: String,
    #[serde(rename = "metadata[txn_uuid]")]
    pub metadata_txn_uuid: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for RefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let amount = item.request.refund_amount;
        let metadata_txn_id = "Fetch txn_id from DB".to_string();
        let metadata_txn_uuid = "Fetch txn_id from DB".to_string();
        let payment_intent = item.request.connector_transaction_id.clone();
        Ok(Self {
            amount: Some(amount),
            payment_intent,
            metadata_order_id: item.payment_id.clone(),
            metadata_txn_id,
            metadata_txn_uuid,
        })
    }
}

// Type definition for Stripe Refund Response

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
    RequiresAction,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            self::RefundStatus::Succeeded => Self::Success,
            self::RefundStatus::Failed => Self::Failure,
            self::RefundStatus::Pending => Self::Pending,
            self::RefundStatus::RequiresAction => Self::ManualReview,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub object: String,
    pub amount: i64,
    pub currency: String,
    pub metadata: StripeMetadata,
    pub payment_intent: String,
    pub status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
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

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
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
pub struct ErrorDetails {
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct StripeShippingAddress {
    #[serde(rename = "shipping[address][city]")]
    pub city: Option<String>,
    #[serde(rename = "shipping[address][country]")]
    pub country: Option<String>,
    #[serde(rename = "shipping[address][line1]")]
    pub line1: Option<Secret<String>>,
    #[serde(rename = "shipping[address][line2]")]
    pub line2: Option<Secret<String>>,
    #[serde(rename = "shipping[address][postal_code]")]
    pub zip: Option<Secret<String>>,
    #[serde(rename = "shipping[address][state]")]
    pub state: Option<Secret<String>>,
    #[serde(rename = "shipping[name]")]
    pub name: Option<Secret<String>>,
    #[serde(rename = "shipping[phone]")]
    pub phone: Option<Secret<String>>,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct StripeBillingAddress {
    #[serde(rename = "payment_method_data[billing_details][email]")]
    pub email: Option<Secret<String, Email>>,
    #[serde(rename = "payment_method_data[billing_details][address][country]")]
    pub country: Option<String>,
    #[serde(rename = "payment_method_data[billing_details][name]")]
    pub name: Option<Secret<String>>,
}

#[derive(Debug, Clone, serde::Deserialize, Eq, PartialEq)]
pub struct StripeRedirectResponse {
    pub payment_intent: String,
    pub payment_intent_client_secret: String,
    pub source_redirect_slug: Option<String>,
    pub redirect_status: Option<StripePaymentStatus>,
    pub source_type: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Clone, Copy)]
pub struct CancelRequest {
    cancellation_reason: Option<CancellationReason>,
}

impl TryFrom<&types::PaymentsCancelRouterData> for CancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let cancellation_reason = match &item.request.cancellation_reason {
            Some(c) => Some(
                CancellationReason::from_str(c)
                    .into_report()
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?,
            ),
            None => None,
        };

        Ok(Self {
            cancellation_reason,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CancellationReason {
    Duplicate,
    Fraudulent,
    RequestedByCustomer,
    Abandoned,
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentMethodOptions {
    Card {
        mandate_options: Option<StripeMandateOptions>,
    },
    Klarna {},
    Affirm {},
    AfterpayClearpay {},
}
// #[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
// pub struct Card
#[derive(serde::Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct StripeMandateOptions {
    reference: String, // Extendable, But only important field to be captured
}
/// Represents the capture request body for stripe connector.
#[derive(Debug, Serialize, Clone, Copy)]
pub struct CaptureRequest {
    /// If amount_to_capture is None stripe captures the amount in the payment intent.
    amount_to_capture: Option<i64>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for CaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount_to_capture: item.request.amount_to_capture,
        })
    }
}

// #[cfg(test)]
// mod test_stripe_transformers {
//     use super::*;

//     #[test]
//     fn verify_transform_from_router_to_stripe_req() {
//         let router_req = PaymentsRequest {
//             amount: 100.0,
//             currency: "USD".to_string(),
//             ..Default::default()
//         };

//         let stripe_req = PaymentIntentRequest::from(router_req);

//         //metadata is generated everytime. So use the transformed struct to copy uuid

//         let stripe_req_expected = PaymentIntentRequest {
//             amount: 10000,
//             currency: "USD".to_string(),
//             statement_descriptor_suffix: None,
//             metadata_order_id: "Auto generate Order ID".to_string(),
//             metadata_txn_id: "Fetch from Merchant Account_Auto generate Order ID_1".to_string(),
//             metadata_txn_uuid: stripe_req.metadata_txn_uuid.clone(),
//             return_url: "Fetch Url from Merchant Account".to_string(),
//             confirm: false,
//             payment_method_types: "card".to_string(),
//             payment_method_data_type: "card".to_string(),
//             payment_method_data_card_number: None,
//             payment_method_data_card_exp_month: None,
//             payment_method_data_card_exp_year: None,
//             payment_method_data_card_cvc: None,
//             description: None,
//         };
//         assert_eq!(stripe_req_expected, stripe_req);
//     }
// }

#[derive(Debug, Deserialize)]
pub struct StripeWebhookDataObjectId {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct StripeWebhookDataId {
    pub object: StripeWebhookDataObjectId,
}

#[derive(Debug, Deserialize)]
pub struct StripeWebhookDataResource {
    pub object: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct StripeWebhookObjectResource {
    pub data: StripeWebhookDataResource,
}

#[derive(Debug, Deserialize)]
pub struct StripeWebhookObjectEventType {
    #[serde(rename = "type")]
    pub event_type: String,
}

#[derive(Debug, Deserialize)]
pub struct StripeWebhookObjectId {
    pub data: StripeWebhookDataId,
}

impl
    TryFrom<(
        api::PaymentMethod,
        enums::AuthenticationType,
        StripePaymentMethodType,
    )> for StripePaymentMethodData
{
    type Error = errors::ConnectorError;
    fn try_from(
        (pm_data, auth_type, pm_type): (
            api::PaymentMethod,
            enums::AuthenticationType,
            StripePaymentMethodType,
        ),
    ) -> Result<Self, Self::Error> {
        match pm_data {
            api::PaymentMethod::Card(ref ccard) => Ok(Self::Card({
                let payment_method_auth_type = match auth_type {
                    enums::AuthenticationType::ThreeDs => Auth3ds::Any,
                    enums::AuthenticationType::NoThreeDs => Auth3ds::Automatic,
                };
                StripeCardData {
                    payment_method_types: StripePaymentMethodType::Card,
                    payment_method_data_type: StripePaymentMethodType::Card,
                    payment_method_data_card_number: ccard.card_number.clone(),
                    payment_method_data_card_exp_month: ccard.card_exp_month.clone(),
                    payment_method_data_card_exp_year: ccard.card_exp_year.clone(),
                    payment_method_data_card_cvc: ccard.card_cvc.clone(),
                    payment_method_auth_type,
                }
            })),
            api::PaymentMethod::BankTransfer => Ok(Self::Bank),
            api::PaymentMethod::PayLater(_) => Ok(Self::PayLater(StripePayLaterData {
                payment_method_types: pm_type.clone(),
                payment_method_data_type: pm_type,
            })),
            api::PaymentMethod::Wallet(_) => Ok(Self::Wallet),
            api::PaymentMethod::Paypal => Ok(Self::Paypal),
        }
    }
}
