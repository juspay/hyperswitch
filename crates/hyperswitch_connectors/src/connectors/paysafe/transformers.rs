use std::collections::HashMap;

use cards::CardNumber;
use common_enums::{enums, Currency};
use common_utils::{
    id_type,
    pii::{Email, IpAddress, SecretSerdeValue},
    request::Method,
    types::MinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankRedirectData, GiftCardData, PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsPreProcessingData, PaymentsSyncData,
        ResponseId,
    },
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{consts, errors};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, to_connector_meta, BrowserInformationData, CardData, PaymentsAuthorizeRequestData,
        PaymentsCompleteAuthorizeRequestData, PaymentsPreProcessingRequestData, RouterData as _,
    },
};

pub struct PaysafeRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for PaysafeRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PaysafeConnectorMetadataObject {
    pub account_id: PaysafePaymentMethodDetails,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PaysafePaymentMethodDetails {
    pub card: Option<HashMap<Currency, CardAccountId>>,
    pub skrill: Option<HashMap<Currency, RedirectAccountId>>,
    pub interac: Option<HashMap<Currency, RedirectAccountId>>,
    pub pay_safe_card: Option<HashMap<Currency, RedirectAccountId>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CardAccountId {
    no_three_ds: Option<Secret<String>>,
    three_ds: Option<Secret<String>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RedirectAccountId {
    three_ds: Option<Secret<String>>,
}

impl TryFrom<&Option<SecretSerdeValue>> for PaysafeConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "merchant_connector_account.metadata",
            })?;
        Ok(metadata)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDs {
    pub merchant_url: String,
    pub device_channel: DeviceChannel,
    pub message_category: ThreeDsMessageCategory,
    pub authentication_purpose: ThreeDsAuthenticationPurpose,
    pub requestor_challenge_preference: ThreeDsChallengePreference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeviceChannel {
    Browser,
    Sdk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThreeDsMessageCategory {
    Payment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThreeDsAuthenticationPurpose {
    PaymentTransaction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThreeDsChallengePreference {
    ChallengeMandated,
    NoPreference,
    NoChallengeRequested,
    ChallengeRequested,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentHandleRequest {
    pub merchant_ref_num: String,
    pub amount: MinorUnit,
    pub settle_with_auth: bool,
    #[serde(flatten)]
    pub payment_method: PaysafePaymentMethod,
    pub currency_code: Currency,
    pub payment_type: PaysafePaymentType,
    pub transaction_type: TransactionType,
    pub return_links: Vec<ReturnLink>,
    pub account_id: Secret<String>,
    pub three_ds: Option<ThreeDs>,
    pub profile: Option<PaysafeProfile>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeProfile {
    pub first_name: Secret<String>,
    pub last_name: Secret<String>,
    pub email: Email,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum PaysafePaymentMethod {
    Card {
        card: PaysafeCard,
    },
    Skrill {
        skrill: SkrillWallet,
    },
    Interac {
        #[serde(rename = "interacEtransfer")]
        interac_etransfer: InteracBankRedirect,
    },
    PaysafeCard {
        #[serde(rename = "paysafecard")]
        pay_safe_card: PaysafeGiftCard,
    },
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkrillWallet {
    pub consumer_id: Email,
    pub country_code: Option<api_models::enums::CountryAlpha2>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InteracBankRedirect {
    pub consumer_id: Email,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeGiftCard {
    pub consumer_id: id_type::CustomerId,
}

#[derive(Debug, Serialize)]
pub struct ReturnLink {
    pub rel: LinkType,
    pub href: String,
    pub method: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    OnCompleted,
    OnFailed,
    OnCancelled,
    Default,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaysafePaymentType {
    Card,
    Skrill,
    InteracEtransfer,
    Paysafecard,
}

#[derive(Debug, Serialize)]
pub enum TransactionType {
    #[serde(rename = "PAYMENT")]
    Payment,
}

impl PaysafePaymentMethodDetails {
    pub fn get_no_three_ds_account_id(
        &self,
        currency: Currency,
    ) -> Result<Secret<String>, errors::ConnectorError> {
        self.card
            .as_ref()
            .and_then(|cards| cards.get(&currency))
            .and_then(|card| card.no_three_ds.clone())
            .ok_or(errors::ConnectorError::InvalidConnectorConfig {
                config: "Missing no_3ds account_id",
            })
    }

    pub fn get_three_ds_account_id(
        &self,
        currency: Currency,
    ) -> Result<Secret<String>, errors::ConnectorError> {
        self.card
            .as_ref()
            .and_then(|cards| cards.get(&currency))
            .and_then(|card| card.three_ds.clone())
            .ok_or(errors::ConnectorError::InvalidConnectorConfig {
                config: "Missing 3ds account_id",
            })
    }

    pub fn get_skrill_account_id(
        &self,
        currency: Currency,
    ) -> Result<Secret<String>, errors::ConnectorError> {
        self.skrill
            .as_ref()
            .and_then(|wallets| wallets.get(&currency))
            .and_then(|skrill| skrill.three_ds.clone())
            .ok_or(errors::ConnectorError::InvalidConnectorConfig {
                config: "Missing skrill account_id",
            })
    }

    pub fn get_interac_account_id(
        &self,
        currency: Currency,
    ) -> Result<Secret<String>, errors::ConnectorError> {
        self.interac
            .as_ref()
            .and_then(|redirects| redirects.get(&currency))
            .and_then(|interac| interac.three_ds.clone())
            .ok_or(errors::ConnectorError::InvalidConnectorConfig {
                config: "Missing interac account_id",
            })
    }

    pub fn get_paysafe_gift_card_account_id(
        &self,
        currency: Currency,
    ) -> Result<Secret<String>, errors::ConnectorError> {
        self.pay_safe_card
            .as_ref()
            .and_then(|gift_cards| gift_cards.get(&currency))
            .and_then(|pay_safe_card| pay_safe_card.three_ds.clone())
            .ok_or(errors::ConnectorError::InvalidConnectorConfig {
                config: "Missing paysafe gift card account_id",
            })
    }
}

impl TryFrom<&PaysafeRouterData<&PaymentsPreProcessingRouterData>> for PaysafePaymentHandleRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaysafeRouterData<&PaymentsPreProcessingRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "Card 3DS".to_string(),
                connector: "Paysafe",
            })?
        };
        let metadata: PaysafeConnectorMetadataObject =
            utils::to_connector_meta_from_secret(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "merchant_connector_account.metadata",
                })?;
        let currency = item.router_data.request.get_currency()?;
        match item.router_data.request.get_payment_method_data()?.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = PaysafeCard {
                    card_num: req_card.card_number.clone(),
                    card_expiry: PaysafeCardExpiry {
                        month: req_card.card_exp_month.clone(),
                        year: req_card.get_expiry_year_4_digit(),
                    },
                    cvv: if req_card.card_cvc.clone().expose().is_empty() {
                        None
                    } else {
                        Some(req_card.card_cvc.clone())
                    },
                    holder_name: item.router_data.get_optional_billing_full_name(),
                };

                let payment_method = PaysafePaymentMethod::Card { card: card.clone() };
                let account_id = metadata.account_id.get_no_three_ds_account_id(currency)?;
                let amount = item.amount;
                let payment_type = PaysafePaymentType::Card;
                let transaction_type = TransactionType::Payment;
                let redirect_url = item.router_data.request.get_router_return_url()?;
                let return_links = vec![
                    ReturnLink {
                        rel: LinkType::Default,
                        href: redirect_url.clone(),
                        method: Method::Get.to_string(),
                    },
                    ReturnLink {
                        rel: LinkType::OnCompleted,
                        href: redirect_url.clone(),
                        method: Method::Get.to_string(),
                    },
                    ReturnLink {
                        rel: LinkType::OnFailed,
                        href: redirect_url.clone(),
                        method: Method::Get.to_string(),
                    },
                    ReturnLink {
                        rel: LinkType::OnCancelled,
                        href: redirect_url.clone(),
                        method: Method::Get.to_string(),
                    },
                ];

                Ok(Self {
                    merchant_ref_num: item.router_data.connector_request_reference_id.clone(),
                    amount,
                    settle_with_auth: matches!(
                        item.router_data.request.capture_method,
                        Some(enums::CaptureMethod::Automatic) | None
                    ),
                    payment_method,
                    currency_code: currency,
                    payment_type,
                    transaction_type,
                    return_links,
                    account_id,
                    three_ds: None,
                    profile: None,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))?,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentHandleResponse {
    pub id: String,
    pub merchant_ref_num: String,
    pub payment_handle_token: Secret<String>,
    pub status: PaysafePaymentHandleStatus,
    pub links: Option<Vec<PaymentLink>>,
    pub error: Option<Error>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentLink {
    pub rel: String,
    pub href: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaysafePaymentHandleStatus {
    Initiated,
    Payable,
    #[default]
    Processing,
    Failed,
    Expired,
    Completed,
    Error,
}

impl TryFrom<PaysafePaymentHandleStatus> for common_enums::AttemptStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: PaysafePaymentHandleStatus) -> Result<Self, Self::Error> {
        match item {
            PaysafePaymentHandleStatus::Completed => Ok(Self::Authorized),
            PaysafePaymentHandleStatus::Failed
            | PaysafePaymentHandleStatus::Expired
            | PaysafePaymentHandleStatus::Error => Ok(Self::Failure),
            // We get an `Initiated` status, with a redirection link from the connector, which indicates that further action is required by the customer,
            PaysafePaymentHandleStatus::Initiated => Ok(Self::AuthenticationPending),
            PaysafePaymentHandleStatus::Payable | PaysafePaymentHandleStatus::Processing => {
                Ok(Self::Pending)
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaysafeMeta {
    pub payment_handle_token: Secret<String>,
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            PaysafePaymentHandleResponse,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            PaysafePaymentHandleResponse,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            preprocessing_id: Some(
                item.response
                    .payment_handle_token
                    .to_owned()
                    .peek()
                    .to_string(),
            ),
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
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<F, PaysafePaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            PaysafePaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: get_paysafe_payment_status(
                item.response.status,
                item.data.request.capture_method,
            ),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            PaysafePaymentHandleResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            PaysafePaymentHandleResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let url = match item.response.links.as_ref().and_then(|links| links.first()) {
            Some(link) => link.href.clone(),
            None => return Err(errors::ConnectorError::ResponseDeserializationFailed)?,
        };
        let redirection_data = Some(RedirectForm::Form {
            endpoint: url,
            method: Method::Get,
            form_fields: Default::default(),
        });
        let connector_metadata = serde_json::json!(PaysafeMeta {
            payment_handle_token: item.response.payment_handle_token.clone(),
        });
        Ok(Self {
            status: common_enums::AttemptStatus::try_from(item.response.status)?,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: Some(connector_metadata),
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentsRequest {
    pub merchant_ref_num: String,
    pub amount: MinorUnit,
    pub settle_with_auth: bool,
    pub payment_handle_token: Secret<String>,
    pub currency_code: Currency,
    pub customer_ip: Option<Secret<String, IpAddress>>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeCard {
    pub card_num: CardNumber,
    pub card_expiry: PaysafeCardExpiry,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvv: Option<Secret<String>>,
    pub holder_name: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaysafeCardExpiry {
    pub month: Secret<String>,
    pub year: Secret<String>,
}

impl TryFrom<&PaysafeRouterData<&PaymentsAuthorizeRouterData>> for PaysafePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaysafeRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_handle_token = Secret::new(item.router_data.get_preprocessing_id()?);
        let amount = item.amount;
        let customer_ip = Some(
            item.router_data
                .request
                .get_browser_info()?
                .get_ip_address()?,
        );

        Ok(Self {
            merchant_ref_num: item.router_data.connector_request_reference_id.clone(),
            payment_handle_token,
            amount,
            settle_with_auth: item.router_data.request.is_auto_capture()?,
            currency_code: item.router_data.request.currency,
            customer_ip,
        })
    }
}

impl TryFrom<&PaysafeRouterData<&PaymentsAuthorizeRouterData>> for PaysafePaymentHandleRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaysafeRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let metadata: PaysafeConnectorMetadataObject =
            utils::to_connector_meta_from_secret(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "merchant_connector_account.metadata",
                })?;
        let redirect_url_success = item.router_data.request.get_complete_authorize_url()?;
        let redirect_url = item.router_data.request.get_router_return_url()?;
        let return_links = vec![
            ReturnLink {
                rel: LinkType::Default,
                href: redirect_url.clone(),
                method: Method::Get.to_string(),
            },
            ReturnLink {
                rel: LinkType::OnCompleted,
                href: redirect_url_success.clone(),
                method: Method::Get.to_string(),
            },
            ReturnLink {
                rel: LinkType::OnFailed,
                href: redirect_url.clone(),
                method: Method::Get.to_string(),
            },
            ReturnLink {
                rel: LinkType::OnCancelled,
                href: redirect_url.clone(),
                method: Method::Get.to_string(),
            },
        ];
        let amount = item.amount;
        let currency_code = item.router_data.request.currency;
        let settle_with_auth = matches!(
            item.router_data.request.capture_method,
            Some(enums::CaptureMethod::Automatic) | None
        );
        let transaction_type = TransactionType::Payment;
        let (payment_method, payment_type, account_id, three_ds, profile) =
            match item.router_data.request.payment_method_data.clone() {
                PaymentMethodData::Card(req_card) => {
                    let card = PaysafeCard {
                        card_num: req_card.card_number.clone(),
                        card_expiry: PaysafeCardExpiry {
                            month: req_card.card_exp_month.clone(),
                            year: req_card.get_expiry_year_4_digit(),
                        },
                        cvv: if req_card.card_cvc.clone().expose().is_empty() {
                            None
                        } else {
                            Some(req_card.card_cvc.clone())
                        },
                        holder_name: item.router_data.get_optional_billing_full_name(),
                    };
                    let payment_method = PaysafePaymentMethod::Card { card: card.clone() };
                    let payment_type = PaysafePaymentType::Card;

                    let headers = item.router_data.header_payload.clone();
                    let platform = headers.as_ref().and_then(|h| h.x_client_platform.clone());
                    let device_channel = match platform {
                        Some(common_enums::ClientPlatform::Web)
                        | Some(common_enums::ClientPlatform::Unknown)
                        | None => DeviceChannel::Browser,
                        Some(common_enums::ClientPlatform::Ios)
                        | Some(common_enums::ClientPlatform::Android) => DeviceChannel::Sdk,
                    };

                    let account_id = metadata.account_id.get_three_ds_account_id(currency_code)?;
                    let three_ds = Some(ThreeDs {
                        merchant_url: item.router_data.request.get_router_return_url()?,
                        device_channel,
                        message_category: ThreeDsMessageCategory::Payment,
                        authentication_purpose: ThreeDsAuthenticationPurpose::PaymentTransaction,
                        requestor_challenge_preference:
                            ThreeDsChallengePreference::ChallengeMandated,
                    });

                    (payment_method, payment_type, account_id, three_ds, None)
                }

                PaymentMethodData::Wallet(WalletData::Skrill(_)) => {
                    let payment_method = PaysafePaymentMethod::Skrill {
                        skrill: SkrillWallet {
                            consumer_id: item.router_data.get_billing_email()?,
                            country_code: item.router_data.get_optional_billing_country(),
                        },
                    };
                    let payment_type = PaysafePaymentType::Skrill;
                    let account_id = metadata.account_id.get_skrill_account_id(currency_code)?;
                    (payment_method, payment_type, account_id, None, None)
                }
                PaymentMethodData::Wallet(_) => Err(errors::ConnectorError::NotImplemented(
                    "Payment Method".to_string(),
                ))?,

                PaymentMethodData::BankRedirect(BankRedirectData::Interac { .. }) => {
                    let payment_method = PaysafePaymentMethod::Interac {
                        interac_etransfer: InteracBankRedirect {
                            consumer_id: item.router_data.get_billing_email()?,
                        },
                    };
                    let payment_type = PaysafePaymentType::InteracEtransfer;
                    let account_id = metadata.account_id.get_interac_account_id(currency_code)?;
                    let profile = Some(PaysafeProfile {
                        first_name: item.router_data.get_billing_first_name()?,
                        last_name: item.router_data.get_billing_last_name()?,
                        email: item.router_data.get_billing_email()?,
                    });
                    (payment_method, payment_type, account_id, None, profile)
                }
                PaymentMethodData::BankRedirect(_) => Err(errors::ConnectorError::NotImplemented(
                    "Payment Method".to_string(),
                ))?,

                PaymentMethodData::GiftCard(gift_card_data) => match gift_card_data.as_ref() {
                    GiftCardData::PaySafeCard {} => {
                        let payment_method = PaysafePaymentMethod::PaysafeCard {
                            pay_safe_card: PaysafeGiftCard {
                                consumer_id: item.router_data.get_customer_id()?,
                            },
                        };
                        let payment_type = PaysafePaymentType::Paysafecard;
                        let account_id = metadata
                            .account_id
                            .get_paysafe_gift_card_account_id(currency_code)?;
                        (payment_method, payment_type, account_id, None, None)
                    }
                    _ => Err(errors::ConnectorError::NotImplemented(
                        "Payment Method".to_string(),
                    ))?,
                },

                _ => Err(errors::ConnectorError::NotImplemented(
                    "Payment Method".to_string(),
                ))?,
            };

        Ok(Self {
            merchant_ref_num: item.router_data.connector_request_reference_id.clone(),
            amount,
            settle_with_auth,
            payment_method,
            currency_code,
            payment_type,
            transaction_type,
            return_links,
            account_id,
            three_ds,
            profile,
        })
    }
}

impl TryFrom<&PaysafeRouterData<&PaymentsCompleteAuthorizeRouterData>> for PaysafePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaysafeRouterData<&PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let paysafe_meta: PaysafeMeta = to_connector_meta(
            item.router_data.request.connector_meta.clone(),
        )
        .change_context(errors::ConnectorError::InvalidConnectorConfig {
            config: "connector_metadata",
        })?;
        let payment_handle_token = paysafe_meta.payment_handle_token;
        let amount = item.amount;
        let customer_ip = Some(
            item.router_data
                .request
                .get_browser_info()?
                .get_ip_address()?,
        );

        Ok(Self {
            merchant_ref_num: item.router_data.connector_request_reference_id.clone(),
            payment_handle_token,
            amount,
            settle_with_auth: item.router_data.request.is_auto_capture()?,
            currency_code: item.router_data.request.currency,
            customer_ip,
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<F, PaysafePaymentsResponse, CompleteAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, CompleteAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            PaysafePaymentsResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: get_paysafe_payment_status(
                item.response.status,
                item.data.request.capture_method,
            ),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

pub struct PaysafeAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PaysafeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                username: api_key.to_owned(),
                password: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// Paysafe Payment Status
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaysafePaymentStatus {
    Received,
    Completed,
    Held,
    Failed,
    #[default]
    Pending,
    Cancelled,
    Processing,
}

pub fn get_paysafe_payment_status(
    status: PaysafePaymentStatus,
    capture_method: Option<common_enums::CaptureMethod>,
) -> common_enums::AttemptStatus {
    match status {
        PaysafePaymentStatus::Completed => match capture_method {
            Some(common_enums::CaptureMethod::Manual) => common_enums::AttemptStatus::Authorized,
            Some(common_enums::CaptureMethod::Automatic) | None => {
                common_enums::AttemptStatus::Charged
            }
            Some(common_enums::CaptureMethod::SequentialAutomatic)
            | Some(common_enums::CaptureMethod::ManualMultiple)
            | Some(common_enums::CaptureMethod::Scheduled) => {
                common_enums::AttemptStatus::Unresolved
            }
        },
        PaysafePaymentStatus::Failed => common_enums::AttemptStatus::Failure,
        PaysafePaymentStatus::Pending
        | PaysafePaymentStatus::Processing
        | PaysafePaymentStatus::Received
        | PaysafePaymentStatus::Held => common_enums::AttemptStatus::Pending,
        PaysafePaymentStatus::Cancelled => common_enums::AttemptStatus::Voided,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PaysafeSyncResponse {
    Payments(PaysafePaymentsSyncResponse),
    PaymentHandles(PaysafePaymentHandlesSyncResponse),
}

// Paysafe Payments Response Structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentsSyncResponse {
    pub payments: Vec<PaysafePaymentsResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentHandlesSyncResponse {
    pub payment_handles: Vec<PaysafePaymentHandleResponse>,
}

// Paysafe Payments Response Structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafePaymentsResponse {
    pub id: String,
    pub merchant_ref_num: Option<String>,
    pub status: PaysafePaymentStatus,
    pub error: Option<Error>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeSettlementResponse {
    pub merchant_ref_num: Option<String>,
    pub id: String,
    pub status: PaysafeSettlementStatus,
}

impl<F> TryFrom<ResponseRouterData<F, PaysafeSyncResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaysafeSyncResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = match &item.response {
            PaysafeSyncResponse::Payments(sync_response) => {
                let payment_response = sync_response
                    .payments
                    .first()
                    .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
                get_paysafe_payment_status(
                    payment_response.status,
                    item.data.request.capture_method,
                )
            }
            PaysafeSyncResponse::PaymentHandles(sync_response) => {
                let payment_handle_response = sync_response
                    .payment_handles
                    .first()
                    .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
                common_enums::AttemptStatus::try_from(payment_handle_response.status)?
            }
        };

        let response = if utils::is_payment_failure(status) {
            let (code, message, reason, connector_transaction_id) = match &item.response {
                PaysafeSyncResponse::Payments(sync_response) => {
                    let payment_response = sync_response
                        .payments
                        .first()
                        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
                    match &payment_response.error {
                        Some(err) => (
                            err.code.clone(),
                            err.message.clone(),
                            err.details
                                .as_ref()
                                .and_then(|d| d.first().cloned())
                                .or_else(|| Some(err.message.clone())),
                            payment_response.id.clone(),
                        ),
                        None => (
                            consts::NO_ERROR_CODE.to_string(),
                            consts::NO_ERROR_MESSAGE.to_string(),
                            None,
                            payment_response.id.clone(),
                        ),
                    }
                }
                PaysafeSyncResponse::PaymentHandles(sync_response) => {
                    let payment_handle_response = sync_response
                        .payment_handles
                        .first()
                        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
                    match &payment_handle_response.error {
                        Some(err) => (
                            err.code.clone(),
                            err.message.clone(),
                            err.details
                                .as_ref()
                                .and_then(|d| d.first().cloned())
                                .or_else(|| Some(err.message.clone())),
                            payment_handle_response.id.clone(),
                        ),
                        None => (
                            consts::NO_ERROR_CODE.to_string(),
                            consts::NO_ERROR_MESSAGE.to_string(),
                            None,
                            payment_handle_response.id.clone(),
                        ),
                    }
                }
            };

            Err(hyperswitch_domain_models::router_data::ErrorResponse {
                code,
                message,
                reason,
                attempt_status: None,
                connector_transaction_id: Some(connector_transaction_id),
                status_code: item.http_code,
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
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeCaptureRequest {
    pub merchant_ref_num: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<MinorUnit>,
}

impl TryFrom<&PaysafeRouterData<&PaymentsCaptureRouterData>> for PaysafeCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaysafeRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let amount = Some(item.amount);

        Ok(Self {
            merchant_ref_num: item.router_data.connector_request_reference_id.clone(),
            amount,
        })
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaysafeSettlementStatus {
    Received,
    Initiated,
    Completed,
    Expired,
    Failed,
    #[default]
    Pending,
    Cancelled,
}

impl From<PaysafeSettlementStatus> for common_enums::AttemptStatus {
    fn from(item: PaysafeSettlementStatus) -> Self {
        match item {
            PaysafeSettlementStatus::Completed
            | PaysafeSettlementStatus::Pending
            | PaysafeSettlementStatus::Received => Self::Charged,
            PaysafeSettlementStatus::Failed | PaysafeSettlementStatus::Expired => Self::Failure,
            PaysafeSettlementStatus::Cancelled => Self::Voided,
            PaysafeSettlementStatus::Initiated => Self::Pending,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, PaysafeSettlementResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaysafeSettlementResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<&PaysafeRouterData<&PaymentsCancelRouterData>> for PaysafeCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaysafeRouterData<&PaymentsCancelRouterData>) -> Result<Self, Self::Error> {
        let amount = Some(item.amount);

        Ok(Self {
            merchant_ref_num: item.router_data.connector_request_reference_id.clone(),
            amount,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VoidResponse {
    pub merchant_ref_num: Option<String>,
    pub id: String,
    pub status: PaysafeVoidStatus,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaysafeVoidStatus {
    Received,
    Completed,
    Held,
    Failed,
    #[default]
    Pending,
    Cancelled,
}

impl From<PaysafeVoidStatus> for common_enums::AttemptStatus {
    fn from(item: PaysafeVoidStatus) -> Self {
        match item {
            PaysafeVoidStatus::Completed
            | PaysafeVoidStatus::Pending
            | PaysafeVoidStatus::Received => Self::Voided,
            PaysafeVoidStatus::Failed | PaysafeVoidStatus::Held => Self::Failure,
            PaysafeVoidStatus::Cancelled => Self::Voided,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, VoidResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, VoidResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
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
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaysafeRefundRequest {
    pub merchant_ref_num: String,
    pub amount: MinorUnit,
}

impl<F> TryFrom<&PaysafeRouterData<&RefundsRouterData<F>>> for PaysafeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaysafeRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let amount = item.amount;

        Ok(Self {
            merchant_ref_num: item.router_data.request.refund_id.clone(),
            amount,
        })
    }
}

// Type definition for Refund Response

#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    Received,
    Initiated,
    Completed,
    Expired,
    Failed,
    #[default]
    Pending,
    Cancelled,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Received | RefundStatus::Completed => Self::Success,
            RefundStatus::Failed | RefundStatus::Cancelled | RefundStatus::Expired => Self::Failure,
            RefundStatus::Pending | RefundStatus::Initiated => Self::Pending,
        }
    }
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct PaysafeErrorResponse {
    pub error: Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub code: String,
    pub message: String,
    pub details: Option<Vec<String>>,
    #[serde(rename = "fieldErrors")]
    pub field_errors: Option<Vec<FieldError>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldError {
    pub field: Option<String>,
    pub error: String,
}
