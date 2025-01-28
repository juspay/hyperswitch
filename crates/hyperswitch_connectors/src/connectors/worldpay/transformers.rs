use std::collections::HashMap;

use api_models::payments::{MandateIds, MandateReferenceId};
use base64::Engine;
use common_enums::enums;
use common_utils::{
    consts::BASE64_ENGINE, errors::CustomResult, ext_traits::OptionExt, pii, types::MinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    address,
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{Authorize, SetupMandate},
    router_request_types::{
        BrowserInformation, PaymentsAuthorizeData, ResponseId, SetupMandateRequestData,
    },
    router_response_types::{MandateReference, PaymentsResponseData, RedirectForm},
    types,
};
use hyperswitch_interfaces::{api, errors};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use super::{requests::*, response::*};
use crate::{
    types::ResponseRouterData,
    utils::{
        self, AddressData, ApplePay, CardData, ForeignTryFrom, PaymentsAuthorizeRequestData,
        PaymentsSetupMandateRequestData, RouterData as RouterDataTrait,
    },
};

#[derive(Debug, Serialize)]
pub struct WorldpayRouterData<T> {
    amount: i64,
    router_data: T,
}
impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, MinorUnit, T)> for WorldpayRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, minor_amount, item): (
            &api::CurrencyUnit,
            enums::Currency,
            MinorUnit,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: minor_amount.get_amount_as_i64(),
            router_data: item,
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorldpayConnectorMetadataObject {
    pub merchant_name: Option<Secret<String>>,
}

impl TryFrom<Option<&pii::SecretSerdeValue>> for WorldpayConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: Option<&pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.cloned())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

fn fetch_payment_instrument(
    payment_method: PaymentMethodData,
    billing_address: Option<&address::Address>,
    mandate_ids: Option<MandateIds>,
) -> CustomResult<PaymentInstrument, errors::ConnectorError> {
    match payment_method {
        PaymentMethodData::Card(card) => Ok(PaymentInstrument::Card(CardPayment {
            raw_card_details: RawCardDetails {
                payment_type: PaymentType::Plain,
                expiry_date: ExpiryDate {
                    month: card.get_expiry_month_as_i8()?,
                    year: card.get_expiry_year_as_4_digit_i32()?,
                },
                card_number: card.card_number,
            },
            cvc: card.card_cvc,
            card_holder_name: billing_address.and_then(|address| address.get_optional_full_name()),
            billing_address: if let Some(address) =
                billing_address.and_then(|addr| addr.address.clone())
            {
                Some(BillingAddress {
                    address1: address.line1.get_required_value("line1").change_context(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "line1",
                        },
                    )?,
                    address2: address.line2,
                    address3: address.line3,
                    city: address.city.get_required_value("city").change_context(
                        errors::ConnectorError::MissingRequiredField { field_name: "city" },
                    )?,
                    state: address.state,
                    postal_code: address.zip.get_required_value("zip").change_context(
                        errors::ConnectorError::MissingRequiredField { field_name: "zip" },
                    )?,
                    country_code: address
                        .country
                        .get_required_value("country_code")
                        .change_context(errors::ConnectorError::MissingRequiredField {
                            field_name: "country_code",
                        })?,
                })
            } else {
                None
            },
        })),
        PaymentMethodData::CardDetailsForNetworkTransactionId(raw_card_details) => {
            Ok(PaymentInstrument::RawCardForNTI(RawCardDetails {
                payment_type: PaymentType::Plain,
                expiry_date: ExpiryDate {
                    month: raw_card_details.get_expiry_month_as_i8()?,
                    year: raw_card_details.get_expiry_year_as_4_digit_i32()?,
                },
                card_number: raw_card_details.card_number,
            }))
        }
        PaymentMethodData::MandatePayment => mandate_ids
            .and_then(|mandate_ids| {
                mandate_ids
                    .mandate_reference_id
                    .and_then(|mandate_id| match mandate_id {
                        MandateReferenceId::ConnectorMandateId(connector_mandate_id) => {
                            connector_mandate_id.get_connector_mandate_id().map(|href| {
                                PaymentInstrument::CardToken(CardToken {
                                    payment_type: PaymentType::Token,
                                    href,
                                    cvc: None,
                                })
                            })
                        }
                        _ => None,
                    })
            })
            .ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "connector_mandate_id",
                }
                .into(),
            ),
        PaymentMethodData::Wallet(wallet) => match wallet {
            WalletData::GooglePay(data) => Ok(PaymentInstrument::Googlepay(WalletPayment {
                payment_type: PaymentType::Encrypted,
                wallet_token: Secret::new(data.tokenization_data.token),
                ..WalletPayment::default()
            })),
            WalletData::ApplePay(data) => Ok(PaymentInstrument::Applepay(WalletPayment {
                payment_type: PaymentType::Encrypted,
                wallet_token: data.get_applepay_decoded_payment_data()?,
                ..WalletPayment::default()
            })),
            WalletData::AliPayQr(_)
            | WalletData::AliPayRedirect(_)
            | WalletData::AliPayHkRedirect(_)
            | WalletData::AmazonPay(_)
            | WalletData::AmazonPayRedirect(_)
            | WalletData::MomoRedirect(_)
            | WalletData::KakaoPayRedirect(_)
            | WalletData::GoPayRedirect(_)
            | WalletData::GcashRedirect(_)
            | WalletData::ApplePayRedirect(_)
            | WalletData::ApplePayThirdPartySdk(_)
            | WalletData::DanaRedirect {}
            | WalletData::GooglePayRedirect(_)
            | WalletData::GooglePayThirdPartySdk(_)
            | WalletData::MbWayRedirect(_)
            | WalletData::MobilePayRedirect(_)
            | WalletData::PaypalRedirect(_)
            | WalletData::PaypalSdk(_)
            | WalletData::Paze(_)
            | WalletData::SamsungPay(_)
            | WalletData::TwintRedirect {}
            | WalletData::VippsRedirect {}
            | WalletData::TouchNGoRedirect(_)
            | WalletData::WeChatPayRedirect(_)
            | WalletData::CashappQr(_)
            | WalletData::SwishQr(_)
            | WalletData::WeChatPayQr(_)
            | WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("worldpay"),
            )
            .into()),
        },
        PaymentMethodData::PayLater(_)
        | PaymentMethodData::BankRedirect(_)
        | PaymentMethodData::BankDebit(_)
        | PaymentMethodData::BankTransfer(_)
        | PaymentMethodData::Crypto(_)
        | PaymentMethodData::Reward
        | PaymentMethodData::RealTimePayment(_)
        | PaymentMethodData::MobilePayment(_)
        | PaymentMethodData::Upi(_)
        | PaymentMethodData::Voucher(_)
        | PaymentMethodData::CardRedirect(_)
        | PaymentMethodData::GiftCard(_)
        | PaymentMethodData::OpenBanking(_)
        | PaymentMethodData::CardToken(_)
        | PaymentMethodData::NetworkToken(_) => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("worldpay"),
        )
        .into()),
    }
}

impl TryFrom<(enums::PaymentMethod, Option<enums::PaymentMethodType>)> for PaymentMethod {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        src: (enums::PaymentMethod, Option<enums::PaymentMethodType>),
    ) -> Result<Self, Self::Error> {
        match (src.0, src.1) {
            (enums::PaymentMethod::Card, _) => Ok(Self::Card),
            (enums::PaymentMethod::Wallet, pmt) => {
                let pm = pmt.ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "payment_method_type",
                })?;
                match pm {
                    enums::PaymentMethodType::ApplePay => Ok(Self::ApplePay),
                    enums::PaymentMethodType::GooglePay => Ok(Self::GooglePay),
                    _ => Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("worldpay"),
                    )
                    .into()),
                }
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("worldpay"),
            )
            .into()),
        }
    }
}

// Trait to abstract common functionality between Authorize and SetupMandate
trait WorldpayPaymentsRequestData {
    fn get_return_url(&self) -> Result<String, error_stack::Report<errors::ConnectorError>>;
    fn get_auth_type(&self) -> &enums::AuthenticationType;
    fn get_browser_info(&self) -> Option<&BrowserInformation>;
    fn get_payment_method_data(&self) -> &PaymentMethodData;
    fn get_setup_future_usage(&self) -> Option<enums::FutureUsage>;
    fn get_off_session(&self) -> Option<bool>;
    fn get_mandate_id(&self) -> Option<MandateIds>;
    fn get_currency(&self) -> enums::Currency;
    fn get_optional_billing_address(&self) -> Option<&address::Address>;
    fn get_connector_meta_data(&self) -> Option<&pii::SecretSerdeValue>;
    fn get_payment_method(&self) -> enums::PaymentMethod;
    fn get_payment_method_type(&self) -> Option<enums::PaymentMethodType>;
    fn get_connector_request_reference_id(&self) -> String;
    fn get_is_mandate_payment(&self) -> bool;
    fn get_settlement_info(&self, _amount: i64) -> Option<AutoSettlement> {
        None
    }
}

impl WorldpayPaymentsRequestData
    for RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
{
    fn get_return_url(&self) -> Result<String, error_stack::Report<errors::ConnectorError>> {
        self.request.get_router_return_url()
    }

    fn get_auth_type(&self) -> &enums::AuthenticationType {
        &self.auth_type
    }

    fn get_browser_info(&self) -> Option<&BrowserInformation> {
        self.request.browser_info.as_ref()
    }

    fn get_payment_method_data(&self) -> &PaymentMethodData {
        &self.request.payment_method_data
    }

    fn get_setup_future_usage(&self) -> Option<enums::FutureUsage> {
        self.request.setup_future_usage
    }

    fn get_off_session(&self) -> Option<bool> {
        self.request.off_session
    }

    fn get_mandate_id(&self) -> Option<MandateIds> {
        self.request.mandate_id.clone()
    }

    fn get_currency(&self) -> enums::Currency {
        self.request.currency
    }

    fn get_optional_billing_address(&self) -> Option<&address::Address> {
        self.get_optional_billing()
    }

    fn get_connector_meta_data(&self) -> Option<&pii::SecretSerdeValue> {
        self.connector_meta_data.as_ref()
    }

    fn get_payment_method(&self) -> enums::PaymentMethod {
        self.payment_method
    }

    fn get_payment_method_type(&self) -> Option<enums::PaymentMethodType> {
        self.request.payment_method_type
    }

    fn get_connector_request_reference_id(&self) -> String {
        self.connector_request_reference_id.clone()
    }

    fn get_is_mandate_payment(&self) -> bool {
        true
    }
}

impl WorldpayPaymentsRequestData
    for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
{
    fn get_return_url(&self) -> Result<String, error_stack::Report<errors::ConnectorError>> {
        self.request.get_complete_authorize_url()
    }

    fn get_auth_type(&self) -> &enums::AuthenticationType {
        &self.auth_type
    }

    fn get_browser_info(&self) -> Option<&BrowserInformation> {
        self.request.browser_info.as_ref()
    }

    fn get_payment_method_data(&self) -> &PaymentMethodData {
        &self.request.payment_method_data
    }

    fn get_setup_future_usage(&self) -> Option<enums::FutureUsage> {
        self.request.setup_future_usage
    }

    fn get_off_session(&self) -> Option<bool> {
        self.request.off_session
    }

    fn get_mandate_id(&self) -> Option<MandateIds> {
        self.request.mandate_id.clone()
    }

    fn get_currency(&self) -> enums::Currency {
        self.request.currency
    }

    fn get_optional_billing_address(&self) -> Option<&address::Address> {
        self.get_optional_billing()
    }

    fn get_connector_meta_data(&self) -> Option<&pii::SecretSerdeValue> {
        self.connector_meta_data.as_ref()
    }

    fn get_payment_method(&self) -> enums::PaymentMethod {
        self.payment_method
    }

    fn get_payment_method_type(&self) -> Option<enums::PaymentMethodType> {
        self.request.payment_method_type
    }

    fn get_connector_request_reference_id(&self) -> String {
        self.connector_request_reference_id.clone()
    }

    fn get_is_mandate_payment(&self) -> bool {
        self.request.is_mandate_payment()
    }

    fn get_settlement_info(&self, amount: i64) -> Option<AutoSettlement> {
        match (self.request.capture_method.unwrap_or_default(), amount) {
            (_, 0) => None,
            (enums::CaptureMethod::Automatic, _)
            | (enums::CaptureMethod::SequentialAutomatic, _) => Some(AutoSettlement { auto: true }),
            (enums::CaptureMethod::Manual, _) | (enums::CaptureMethod::ManualMultiple, _) => {
                Some(AutoSettlement { auto: false })
            }
            _ => None,
        }
    }
}

// Dangling helper function to create ThreeDS request
fn create_three_ds_request<T: WorldpayPaymentsRequestData>(
    router_data: &T,
    is_mandate_payment: bool,
) -> Result<Option<ThreeDSRequest>, error_stack::Report<errors::ConnectorError>> {
    match (
        router_data.get_auth_type(),
        router_data.get_payment_method_data(),
    ) {
        // 3DS for NTI flow
        (_, PaymentMethodData::CardDetailsForNetworkTransactionId(_)) => Ok(None),
        // 3DS for regular payments
        (enums::AuthenticationType::ThreeDs, _) => {
            let browser_info = router_data.get_browser_info().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "browser_info",
                },
            )?;

            let accept_header = browser_info
                .accept_header
                .clone()
                .get_required_value("accept_header")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "accept_header",
                })?;

            let user_agent_header = browser_info
                .user_agent
                .clone()
                .get_required_value("user_agent")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "user_agent",
                })?;

            Ok(Some(ThreeDSRequest {
                three_ds_type: THREE_DS_TYPE.to_string(),
                mode: THREE_DS_MODE.to_string(),
                device_data: ThreeDSRequestDeviceData {
                    accept_header,
                    user_agent_header,
                    browser_language: browser_info.language.clone(),
                    browser_screen_width: browser_info.screen_width,
                    browser_screen_height: browser_info.screen_height,
                    browser_color_depth: browser_info.color_depth.map(|depth| depth.to_string()),
                    time_zone: browser_info.time_zone.map(|tz| tz.to_string()),
                    browser_java_enabled: browser_info.java_enabled,
                    browser_javascript_enabled: browser_info.java_script_enabled,
                    channel: Some(ThreeDSRequestChannel::Browser),
                },
                challenge: ThreeDSRequestChallenge {
                    return_url: router_data.get_return_url()?,
                    preference: if is_mandate_payment {
                        Some(ThreeDsPreference::ChallengeMandated)
                    } else {
                        None
                    },
                },
            }))
        }
        // Non 3DS
        _ => Ok(None),
    }
}

// Dangling helper function to determine token and agreement settings
fn get_token_and_agreement(
    payment_method_data: &PaymentMethodData,
    setup_future_usage: Option<enums::FutureUsage>,
    off_session: Option<bool>,
    mandate_ids: Option<MandateIds>,
) -> (Option<TokenCreation>, Option<CustomerAgreement>) {
    match (payment_method_data, setup_future_usage, off_session) {
        // CIT
        (PaymentMethodData::Card(_), Some(enums::FutureUsage::OffSession), _) => (
            Some(TokenCreation {
                token_type: TokenCreationType::Worldpay,
            }),
            Some(CustomerAgreement {
                agreement_type: CustomerAgreementType::Subscription,
                stored_card_usage: Some(StoredCardUsageType::First),
                scheme_reference: None,
            }),
        ),
        // MIT
        (PaymentMethodData::Card(_), _, Some(true)) => (
            None,
            Some(CustomerAgreement {
                agreement_type: CustomerAgreementType::Subscription,
                stored_card_usage: Some(StoredCardUsageType::Subsequent),
                scheme_reference: None,
            }),
        ),
        // NTI with raw card data
        (PaymentMethodData::CardDetailsForNetworkTransactionId(_), _, _) => (
            None,
            mandate_ids.and_then(|mandate_ids| {
                mandate_ids
                    .mandate_reference_id
                    .and_then(|mandate_id| match mandate_id {
                        MandateReferenceId::NetworkMandateId(network_transaction_id) => {
                            Some(CustomerAgreement {
                                agreement_type: CustomerAgreementType::Unscheduled,
                                scheme_reference: Some(network_transaction_id.into()),
                                stored_card_usage: None,
                            })
                        }
                        _ => None,
                    })
            }),
        ),
        _ => (None, None),
    }
}

// Implementation for WorldpayPaymentsRequest using abstracted request
impl<T: WorldpayPaymentsRequestData> TryFrom<(&WorldpayRouterData<&T>, &Secret<String>)>
    for WorldpayPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(req: (&WorldpayRouterData<&T>, &Secret<String>)) -> Result<Self, Self::Error> {
        let (item, entity_id) = req;
        let worldpay_connector_metadata_object: WorldpayConnectorMetadataObject =
            WorldpayConnectorMetadataObject::try_from(item.router_data.get_connector_meta_data())?;

        let merchant_name = worldpay_connector_metadata_object.merchant_name.ok_or(
            errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata.merchant_name",
            },
        )?;

        let is_mandate_payment = item.router_data.get_is_mandate_payment();
        let three_ds = create_three_ds_request(item.router_data, is_mandate_payment)?;

        let (token_creation, customer_agreement) = get_token_and_agreement(
            item.router_data.get_payment_method_data(),
            item.router_data.get_setup_future_usage(),
            item.router_data.get_off_session(),
            item.router_data.get_mandate_id(),
        );

        Ok(Self {
            instruction: Instruction {
                settlement: item.router_data.get_settlement_info(item.amount),
                method: PaymentMethod::try_from((
                    item.router_data.get_payment_method(),
                    item.router_data.get_payment_method_type(),
                ))?,
                payment_instrument: fetch_payment_instrument(
                    item.router_data.get_payment_method_data().clone(),
                    item.router_data.get_optional_billing_address(),
                    item.router_data.get_mandate_id(),
                )?,
                narrative: InstructionNarrative {
                    line1: merchant_name.expose(),
                },
                value: PaymentValue {
                    amount: item.amount,
                    currency: item.router_data.get_currency(),
                },
                debt_repayment: None,
                three_ds,
                token_creation,
                customer_agreement,
            },
            merchant: Merchant {
                entity: entity_id.clone(),
                ..Default::default()
            },
            transaction_reference: item.router_data.get_connector_request_reference_id(),
            customer: None,
        })
    }
}

pub struct WorldpayAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) entity_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for WorldpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            // TODO: Remove this later, kept purely for backwards compatibility
            ConnectorAuthType::BodyKey { api_key, key1 } => {
                let auth_key = format!("{}:{}", key1.peek(), api_key.peek());
                let auth_header = format!("Basic {}", BASE64_ENGINE.encode(auth_key));
                Ok(Self {
                    api_key: Secret::new(auth_header),
                    entity_id: Secret::new("default".to_string()),
                })
            }
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => {
                let auth_key = format!("{}:{}", key1.peek(), api_key.peek());
                let auth_header = format!("Basic {}", BASE64_ENGINE.encode(auth_key));
                Ok(Self {
                    api_key: Secret::new(auth_header),
                    entity_id: api_secret.clone(),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

impl From<PaymentOutcome> for enums::AttemptStatus {
    fn from(item: PaymentOutcome) -> Self {
        match item {
            PaymentOutcome::Authorized => Self::Authorized,
            PaymentOutcome::SentForSettlement => Self::Charged,
            PaymentOutcome::ThreeDsDeviceDataRequired => Self::DeviceDataCollectionPending,
            PaymentOutcome::ThreeDsAuthenticationFailed => Self::AuthenticationFailed,
            PaymentOutcome::ThreeDsChallenged => Self::AuthenticationPending,
            PaymentOutcome::SentForCancellation => Self::VoidInitiated,
            PaymentOutcome::SentForPartialRefund | PaymentOutcome::SentForRefund => {
                Self::AutoRefunded
            }
            PaymentOutcome::Refused | PaymentOutcome::FraudHighRisk => Self::Failure,
            PaymentOutcome::ThreeDsUnavailable => Self::AuthenticationFailed,
        }
    }
}

impl From<PaymentOutcome> for enums::RefundStatus {
    fn from(item: PaymentOutcome) -> Self {
        match item {
            PaymentOutcome::SentForPartialRefund | PaymentOutcome::SentForRefund => Self::Success,
            PaymentOutcome::Refused
            | PaymentOutcome::FraudHighRisk
            | PaymentOutcome::Authorized
            | PaymentOutcome::SentForSettlement
            | PaymentOutcome::ThreeDsDeviceDataRequired
            | PaymentOutcome::ThreeDsAuthenticationFailed
            | PaymentOutcome::ThreeDsChallenged
            | PaymentOutcome::SentForCancellation
            | PaymentOutcome::ThreeDsUnavailable => Self::Failure,
        }
    }
}

impl From<&EventType> for enums::AttemptStatus {
    fn from(value: &EventType) -> Self {
        match value {
            EventType::SentForAuthorization => Self::Authorizing,
            EventType::SentForSettlement => Self::Charged,
            EventType::Settled => Self::Charged,
            EventType::Authorized => Self::Authorized,
            EventType::Refused
            | EventType::SettlementFailed
            | EventType::Expired
            | EventType::Cancelled
            | EventType::Error => Self::Failure,
            EventType::SentForRefund
            | EventType::RefundFailed
            | EventType::Refunded
            | EventType::Unknown => Self::Pending,
        }
    }
}

impl From<EventType> for enums::RefundStatus {
    fn from(value: EventType) -> Self {
        match value {
            EventType::Refunded | EventType::SentForRefund => Self::Success,
            EventType::RefundFailed => Self::Failure,
            EventType::Authorized
            | EventType::Cancelled
            | EventType::Settled
            | EventType::Refused
            | EventType::Error
            | EventType::SentForSettlement
            | EventType::SentForAuthorization
            | EventType::SettlementFailed
            | EventType::Expired
            | EventType::Unknown => Self::Pending,
        }
    }
}

impl<F, T>
    ForeignTryFrom<(
        ResponseRouterData<F, WorldpayPaymentsResponse, T, PaymentsResponseData>,
        Option<String>,
    )> for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        item: (
            ResponseRouterData<F, WorldpayPaymentsResponse, T, PaymentsResponseData>,
            Option<String>,
        ),
    ) -> Result<Self, Self::Error> {
        let (router_data, optional_correlation_id) = item;
        let (description, redirection_data, mandate_reference, network_txn_id, error) = router_data
            .response
            .other_fields
            .as_ref()
            .map(|other_fields| match other_fields {
                WorldpayPaymentResponseFields::AuthorizedResponse(res) => (
                    res.description.clone(),
                    None,
                    res.token.as_ref().map(|mandate_token| MandateReference {
                        connector_mandate_id: Some(mandate_token.href.clone().expose()),
                        payment_method_id: Some(mandate_token.token_id.clone()),
                        mandate_metadata: None,
                        connector_mandate_request_reference_id: None,
                    }),
                    res.scheme_reference.clone(),
                    None,
                ),
                WorldpayPaymentResponseFields::DDCResponse(res) => (
                    None,
                    Some(RedirectForm::WorldpayDDCForm {
                        endpoint: res.device_data_collection.url.clone(),
                        method: common_utils::request::Method::Post,
                        collection_id: Some("SessionId".to_string()),
                        form_fields: HashMap::from([
                            (
                                "Bin".to_string(),
                                res.device_data_collection.bin.clone().expose(),
                            ),
                            (
                                "JWT".to_string(),
                                res.device_data_collection.jwt.clone().expose(),
                            ),
                        ]),
                    }),
                    None,
                    None,
                    None,
                ),
                WorldpayPaymentResponseFields::ThreeDsChallenged(res) => (
                    None,
                    Some(RedirectForm::Form {
                        endpoint: res.challenge.url.to_string(),
                        method: common_utils::request::Method::Post,
                        form_fields: HashMap::from([(
                            "JWT".to_string(),
                            res.challenge.jwt.clone().expose(),
                        )]),
                    }),
                    None,
                    None,
                    None,
                ),
                WorldpayPaymentResponseFields::RefusedResponse(res) => (
                    None,
                    None,
                    None,
                    None,
                    Some((res.refusal_code.clone(), res.refusal_description.clone())),
                ),
                WorldpayPaymentResponseFields::FraudHighRisk(_) => (None, None, None, None, None),
            })
            .unwrap_or((None, None, None, None, None));
        let worldpay_status = router_data.response.outcome.clone();
        let optional_error_message = match worldpay_status {
            PaymentOutcome::ThreeDsAuthenticationFailed => {
                Some("3DS authentication failed from issuer".to_string())
            }
            PaymentOutcome::ThreeDsUnavailable => {
                Some("3DS authentication unavailable from issuer".to_string())
            }
            PaymentOutcome::FraudHighRisk => Some("Transaction marked as high risk".to_string()),
            _ => None,
        };
        let status = enums::AttemptStatus::from(worldpay_status.clone());
        let response = match (optional_error_message, error) {
            (None, None) => Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::foreign_try_from((
                    router_data.response,
                    optional_correlation_id.clone(),
                ))?,
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(mandate_reference),
                connector_metadata: None,
                network_txn_id: network_txn_id.map(|id| id.expose()),
                connector_response_reference_id: optional_correlation_id.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            (Some(reason), _) => Err(ErrorResponse {
                code: worldpay_status.to_string(),
                message: reason.clone(),
                reason: Some(reason),
                status_code: router_data.http_code,
                attempt_status: Some(status),
                connector_transaction_id: optional_correlation_id,
            }),
            (_, Some((code, message))) => Err(ErrorResponse {
                code,
                message: message.clone(),
                reason: Some(message),
                status_code: router_data.http_code,
                attempt_status: Some(status),
                connector_transaction_id: optional_correlation_id,
            }),
        };
        Ok(Self {
            status,
            description,
            response,
            ..router_data.data
        })
    }
}

impl TryFrom<(&types::PaymentsCaptureRouterData, MinorUnit)> for WorldpayPartialRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(req: (&types::PaymentsCaptureRouterData, MinorUnit)) -> Result<Self, Self::Error> {
        let (item, amount) = req;
        Ok(Self {
            reference: item.payment_id.clone().replace("_", "-"),
            value: PaymentValue {
                amount: amount.get_amount_as_i64(),
                currency: item.request.currency,
            },
        })
    }
}

impl<F> TryFrom<(&types::RefundsRouterData<F>, MinorUnit)> for WorldpayPartialRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(req: (&types::RefundsRouterData<F>, MinorUnit)) -> Result<Self, Self::Error> {
        let (item, amount) = req;
        Ok(Self {
            reference: item.request.refund_id.clone().replace("_", "-"),
            value: PaymentValue {
                amount: amount.get_amount_as_i64(),
                currency: item.request.currency,
            },
        })
    }
}

impl TryFrom<WorldpayWebhookEventType> for WorldpayEventResponse {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(event: WorldpayWebhookEventType) -> Result<Self, Self::Error> {
        Ok(Self {
            last_event: event.event_details.event_type,
            links: None,
        })
    }
}

impl ForeignTryFrom<(WorldpayPaymentsResponse, Option<String>)> for ResponseIdStr {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        item: (WorldpayPaymentsResponse, Option<String>),
    ) -> Result<Self, Self::Error> {
        get_resource_id(item.0, item.1, |id| Self { id })
    }
}

impl ForeignTryFrom<(WorldpayPaymentsResponse, Option<String>)> for ResponseId {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        item: (WorldpayPaymentsResponse, Option<String>),
    ) -> Result<Self, Self::Error> {
        get_resource_id(item.0, item.1, Self::ConnectorTransactionId)
    }
}

impl TryFrom<&types::PaymentsCompleteAuthorizeRouterData> for WorldpayCompleteAuthorizationRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCompleteAuthorizeRouterData) -> Result<Self, Self::Error> {
        let params = item
            .request
            .redirect_response
            .as_ref()
            .and_then(|redirect_response| redirect_response.params.as_ref())
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;
        serde_urlencoded::from_str::<Self>(params.peek())
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
    }
}
