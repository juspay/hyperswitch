use api_models::payments;
use common_utils::pii;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::router_data::KlarnaSdkResponse;
use hyperswitch_domain_models::router_data::KlarnaCheckoutResponse;

use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, AddressData, AddressDetailsData, PaymentsAuthorizeRequestData, RouterData,
    },
    core::errors,
    types::{self, api, storage::enums, transformers::ForeignFrom, domain},
};

#[derive(Debug, Serialize)]
pub struct KlarnaRouterData<T> {
    amount: i64,
    router_data: T,
}

impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, i64, T)> for KlarnaRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        (_currency_unit, _currency, amount, router_data): (
            &api::CurrencyUnit,
            enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data,
        })
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

#[derive(Default, Debug, Serialize)]
pub struct KlarnaPaymentsRequest {
    auto_capture: bool,
    order_lines: Vec<OrderLines>,
    order_amount: i64,
    purchase_country: enums::CountryAlpha2,
    purchase_currency: enums::Currency,
    merchant_reference1: Option<String>,
    merchant_reference2: Option<String>,
    shipping_address: Option<KlarnaShippingAddress>,
    order_tax_amount: Option<i64>,
    // merchant_urls: Option<MerchantURLs>,

}

// #[derive(Default, Debug, Serialize)]
// pub enum KlarnaPaymentsRequest{
//     SDK (KlarnaSDKRequest),
//     Checkout (KlarnaCheckoutRequest),
// }

// #[derive(Debug)]
// pub struct KlarnaSDKRequest {
//     auto_capture: bool,
//     order_lines: Vec<OrderLines>,
//     order_amount: i64,
//     purchase_country: enums::CountryAlpha2,
//     purchase_currency: enums::Currency,
//     merchant_reference1: Option<String>,
//     merchant_reference2: Option<String>,
//     shipping_address: Option<KlarnaShippingAddress>,
// }

// pub struct KlarnaCheckoutRequest {
//     auto_capture: bool,
//     order_lines: Vec<OrderLines>,
//     order_amount: i64,
//     purchase_country: enums::CountryAlpha2,
//     purchase_currency: enums::Currency,
//     // merchant_urls: Option<String>,

//     merchant_reference1: Option<String>,
//     merchant_reference2: Option<String>,
//     shipping_address: Option<KlarnaShippingAddress>,
// }

#[derive(Debug, Deserialize, Serialize)]
pub struct KlarnaPaymentsResponse {
    order_id: String,
    fraud_status: KlarnaFraudStatus,
    authorized_payment_method: Option<AuthorizedPaymentMethod>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthorizedPaymentMethod {
    #[serde(rename = "type")]
    payment_type: String,
}

impl From<AuthorizedPaymentMethod> for types::AdditionalPaymentMethodConnectorResponse {
    fn from(item: AuthorizedPaymentMethod) -> Self {
        match item.payment_type.as_str(){
            "klarna_sdk"=> Self::PayLater {
                klarna_sdk: Some(KlarnaSdkResponse {
                    payment_type: Some(item.payment_type),
                }),
                klarna_checkout: None,
            },
            "klarna_checkout" => Self::PayLater {
                klarna_checkout: Some(KlarnaCheckoutResponse {
                    payment_type: Some(item.payment_type),
                }),
                klarna_sdk: None,
            },
            _ => Self::PayLater { 
                klarna_sdk: None, 
                klarna_checkout: None 
            },
        }

        // Self::PayLater {
            // klarna_sdk: Some(KlarnaSdkResponse {
            //     payment_type: Some(item.payment_type),
            // }),
        // }
    }
}

#[derive(Debug, Serialize)]
pub struct KlarnaSessionRequest {
    intent: KlarnaSessionIntent,
    purchase_country: enums::CountryAlpha2,
    purchase_currency: enums::Currency,
    order_amount: i64,
    order_lines: Vec<OrderLines>,
    shipping_address: Option<KlarnaShippingAddress>,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct MerchantURLs {
    terms: String,
    checkout: String,
    confirmation: String,
    push: String,
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
        println!("1 request of auth {:?} ",request);
        println!("2 item {:?} ",item);


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
                        total_amount: i64::from(data.quantity) * (data.amount),
                        tax_rate:None,
                        total_tax_amount:None
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

impl TryFrom<types::PaymentsSessionResponseRouterData<KlarnaSessionResponse>>
    for types::PaymentsSessionRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsSessionResponseRouterData<KlarnaSessionResponse>,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;
        Ok(Self {
            response: Ok(types::PaymentsResponseData::SessionResponse {
                session_token: api::SessionToken::Klarna(Box::new(
                    payments::KlarnaSessionTokenResponse {
                        session_token: response.client_token.clone().expose(),
                        session_id: response.session_id.clone(),
                    },
                )),
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
        println!("klarna auth request {:?} ",request);
        println!("klarna auth item {:?} ",item);

        let payment_method_data = request.payment_method_data.clone();
        let payment_experience = request.payment_experience;
        let payment_method_type = request.payment_method_type;
        // match request.order_details.clone() {
        // match payment_method_data {
        //     domain::PaymentMethodData::PayLater(domain::PayLaterData::KlarnaSdk { token }) => {
        //         match (payment_experience, payment_method_type) {
        //             (
        //                 common_enums::PaymentExperience::InvokeSdkClient,
        //                 common_enums::PaymentMethodType::Klarna,
        //             ) => {
        //                 println!("klarna payments called");

                        
        //                 Ok(KlarnaPaymentsRequest::SDK(KlarnaSDKRequest{
        //                     purchase_country: item.router_data.get_billing_country()?,
        //                     purchase_currency: request.currency,
        //                     order_amount: item.amount,
        //                     order_lines: order_details
        //                         .iter()
        //                         .map(|data| OrderLines {
        //                             name: data.product_name.clone(),
        //                             quantity: data.quantity,
        //                             unit_price: data.amount,
        //                             total_amount: i64::from(data.quantity) * (data.amount),
        //                         })          
        //                         .collect(),
        //                     merchant_reference1: Some(
        //                         item.router_data.connector_request_reference_id.clone(),
        //                     ),
        //                     merchant_reference2: item
        //                         .router_data
        //                         .request
        //                         .merchant_order_reference_id
        //                         .clone(),
        //                     auto_capture: request.is_auto_capture()?,
        //                     shipping_address: get_address_info(
        //                         item.router_data.get_optional_shipping(),
        //                     )
        //                     .transpose()?,
        //                 }))
        //             }
        //             (None, None) => todo!(),
        //             (None, Some(_)) => todo!(),
        //             (Some(_), None) => todo!(),
        //             (Some(_), Some(_)) => todo!(),
        //         }
        //     }
        //     domain::PaymentMethodData::PayLater(domain::PayLaterData::KlarnaCheckout {}) => {
        //         match (payment_experience, payment_method_type) {
        //             (
        //                 common_enums::PaymentExperience::InvokeSdkClient,
        //                 common_enums::PaymentMethodType::KlarnaCheckout,
        //             ) => {
        //                 println!("klarna SDK called");
        //                 Ok(KlarnaPaymentsRequest::Checkout(KlarnaCheckoutRequest{
        //                     purchase_country: item.router_data.get_billing_country()?,
        //                     purchase_currency: request.currency,
        //                     order_amount: item.amount,
        //                     order_lines: order_details
        //                         .iter()
        //                         .map(|data| OrderLines {
        //                             name: data.product_name.clone(),
        //                             quantity: data.quantity,
        //                             unit_price: data.amount,
        //                             total_amount: i64::from(data.quantity) * (data.amount),
        //                         })          
        //                         .collect(),
                            
        //                     merchant_urls: item
        //                         .router_data
        //                         .request
        //                         .merchant_urls
        //                         .clone(),
        //                     auto_capture: request.is_auto_capture()?,
        //                     shipping_address: get_address_info(
        //                         item.router_data.get_optional_shipping(),
        //                     )
        //                     .transpose()?,
        //                 }))
        //             }
        //             (None, None) => todo!(),
        //             (None, Some(_)) => todo!(),
        //             (Some(_), None) => todo!(),
        //             (Some(_), Some(_)) => todo!(),
        //         }
        //     }
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(card) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardDetailsForNetworkTransactionId(card_details_for_network_transaction_id) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardRedirect(card_redirect_data) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::Wallet(wallet_data) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::PayLater(pay_later_data) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankRedirect(bank_redirect_data) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankDebit(bank_debit_data) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankTransfer(bank_transfer_data) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::Crypto(crypto_data) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::MandatePayment => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::Reward => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::RealTimePayment(real_time_payment_data) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::Upi(upi_data) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::Voucher(voucher_data) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::GiftCard(gift_card_data) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardToken(card_token) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::OpenBanking(open_banking_data) => todo!(),
        //     hyperswitch_domain_models::payment_method_data::PaymentMethodData::NetworkToken(network_token_data) => todo!(),
        

        
        // }

        match payment_method_data {
            domain::PaymentMethodData::PayLater(domain::PayLaterData::KlarnaSdk { token }) => {
                match request.order_details.clone() {
                   Some(order_details)=> {
                        println!("klarna payments called");

                        Ok(Self{
                            purchase_country: item.router_data.get_billing_country()?,
                            purchase_currency: request.currency,
                            order_amount: item.amount,
                            order_tax_amount:None,
                            order_lines: order_details
                                .iter()
                                .map(|data| OrderLines {
                                    name: data.product_name.clone(),
                                    quantity: data.quantity,
                                    unit_price: data.amount,
                                    total_amount: i64::from(data.quantity) * (data.amount),
                                    total_tax_amount:None,
                                    tax_rate:None
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
                            // merchant_urls:None,
                            auto_capture: request.is_auto_capture()?,
                            shipping_address: get_address_info(
                                item.router_data.get_optional_shipping(),
                            )
                            .transpose()?,
                        })
                   
                    }
                    None => todo!(),
                    // None => todo!(),
                }
            }
            domain::PaymentMethodData::PayLater(domain::PayLaterData::KlarnaCheckout {}) => {
                match request.order_details.clone() {

                    Some(order_details)=> {
                        println!("klarna Checkout called, order details: {:?}", order_details);
                      
                        Ok(Self{
                            purchase_country: item.router_data.get_billing_country()?,
                            purchase_currency: request.currency,
                            order_amount: item.amount,
                            order_tax_amount: Some(item.amount),
                            order_lines: order_details
                                .iter()
                                .map(|data| OrderLines {
                                    name: data.product_name.clone(),
                                    quantity: data.quantity,
                                    unit_price: data.amount,
                                    total_tax_amount:data.total_tax_amount,
                                    tax_rate:data.tax_rate,
                                    total_amount: i64::from(data.quantity) * (data.amount),

                                    // total_amount: i64::from(data.quantity) * (data.amount),
                                })          
                                .collect(),
                            
                            // merchant_urls: request.merchant_urls,
                            // merchant_urls:None,
                            merchant_reference1: None,
                            merchant_reference2: None,
                            auto_capture: request.is_auto_capture()?,
                            shipping_address: get_address_info(
                                item.router_data.get_optional_shipping(),
                            )
                            .transpose()?,
                        })
                    
                    
                    }
                    None => todo!(),
                }
            }
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(card) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardDetailsForNetworkTransactionId(card_details_for_network_transaction_id) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardRedirect(card_redirect_data) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::Wallet(wallet_data) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::PayLater(pay_later_data) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankRedirect(bank_redirect_data) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankDebit(bank_debit_data) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankTransfer(bank_transfer_data) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::Crypto(crypto_data) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::MandatePayment => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::Reward => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::RealTimePayment(real_time_payment_data) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::Upi(upi_data) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::Voucher(voucher_data) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::GiftCard(gift_card_data) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardToken(card_token) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::OpenBanking(open_banking_data) => todo!(),
            hyperswitch_domain_models::payment_method_data::PaymentMethodData::NetworkToken(network_token_data) => todo!(),
        }

        
        
        
        
        // match request.order_details.clone() {
        //     Some(order_details) => Ok(Self {
        //         purchase_country: item.router_data.get_billing_country()?,
        //         purchase_currency: request.currency,
        //         order_amount: item.amount,
        //         order_lines: order_details
        //             .iter()
        //             .map(|data| OrderLines {
        //                 name: data.product_name.clone(),
        //                 quantity: data.quantity,
        //                 unit_price: data.amount,
        //                 total_amount: i64::from(data.quantity) * (data.amount),
        //             })
        //             .collect(),
        //         merchant_reference1: Some(item.router_data.connector_request_reference_id.clone()),
        //         merchant_reference2: item.router_data.request.merchant_order_reference_id.clone(),
        //         auto_capture: request.is_auto_capture()?,
        //         shipping_address: get_address_info(item.router_data.get_optional_shipping())
        //             .transpose()?,
        //     }),
        //     None => Err(report!(errors::ConnectorError::MissingRequiredField {
        //         field_name: "order_details"
        //     })),
        // }
    
    }
}


fn get_address_info(
    address: Option<&payments::Address>,
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

impl TryFrom<types::PaymentsResponseRouterData<KlarnaPaymentsResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsResponseRouterData<KlarnaPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        println!("in response of auth ");
        let connector_response = types::ConnectorResponseData::with_additional_payment_method_data(
            match item.response.authorized_payment_method {
                Some(authorized_payment_method) => {
                    types::AdditionalPaymentMethodConnectorResponse::from(authorized_payment_method)
                }
                None => {
                    types::AdditionalPaymentMethodConnectorResponse::PayLater { klarna_sdk: None, klarna_checkout: todo!() }
                }
            },
        );
        println!("2 response of auth {:?} ",connector_response);
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.order_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.order_id.clone()),
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            status: enums::AttemptStatus::foreign_from((
                item.response.fraud_status,
                item.data.request.is_auto_capture()?,
            )),
            connector_response: Some(connector_response),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct OrderLines {
    name: String,
    quantity: u16,
    unit_price: i64,
    total_amount: i64,
    tax_rate:Option<i64>,
    total_tax_amount:Option<i64>,
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

impl TryFrom<&types::ConnectorAuthType> for KlarnaAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
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

impl ForeignFrom<(KlarnaFraudStatus, bool)> for enums::AttemptStatus {
    fn foreign_from((klarna_status, is_auto_capture): (KlarnaFraudStatus, bool)) -> Self {
        match klarna_status {
            KlarnaFraudStatus::Accepted => {
                if is_auto_capture {
                    Self::Charged
                } else {
                    Self::Authorized
                }
            }
            KlarnaFraudStatus::Pending => Self::Pending,
            KlarnaFraudStatus::Rejected => Self::Failure,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlarnaPsyncResponse {
    pub order_id: String,
    pub status: KlarnaPaymentStatus,
    pub klarna_reference: Option<String>,
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

impl<F, T>
    TryFrom<types::ResponseRouterData<F, KlarnaPsyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, KlarnaPsyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.order_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item
                    .response
                    .klarna_reference
                    .or(Some(item.response.order_id)),
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct KlarnaCaptureRequest {
    captured_amount: i64,
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
    TryFrom<
        types::ResponseRouterData<
            F,
            KlarnaCaptureResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            KlarnaCaptureResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
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
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(resource_id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: Some(connector_meta),
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            status,
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct KlarnaRefundRequest {
    refunded_amount: i64,
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

impl TryFrom<types::RefundsResponseRouterData<api::Execute, KlarnaRefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, KlarnaRefundResponse>,
    ) -> Result<Self, Self::Error> {
        // https://docs.klarna.com/api/ordermanagement/#operation/refundOrder
        // If 201 status code, then Refund is Successful, other status codes are handled by the error handler
        let status = if item.http_code == 201 {
            enums::RefundStatus::Pending
        } else {
            enums::RefundStatus::Failure
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
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

impl TryFrom<types::RefundsResponseRouterData<api::RSync, KlarnaRefundSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, KlarnaRefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        // https://docs.klarna.com/api/ordermanagement/#operation/get
        // If 200 status code, then Refund is Successful, other status codes are handled by the error handler
        let status = if item.http_code == 200 {
            enums::RefundStatus::Success
        } else {
            enums::RefundStatus::Failure
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
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
