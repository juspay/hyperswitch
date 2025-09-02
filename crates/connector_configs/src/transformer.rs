use std::str::FromStr;

use api_models::{
    enums::{
        Connector, PaymentMethod,
        PaymentMethodType::{self, AliPay, ApplePay, GooglePay, Klarna, Paypal, WeChatPay},
    },
    payment_methods,
    refunds::MinorUnit,
};

use crate::common_config::{
    ConnectorApiIntegrationPayload, DashboardRequestPayload, PaymentMethodsEnabled, Provider,
};

impl DashboardRequestPayload {
    pub fn transform_card(
        payment_method_type: PaymentMethodType,
        card_provider: Vec<api_models::enums::CardNetwork>,
    ) -> payment_methods::RequestPaymentMethodTypes {
        payment_methods::RequestPaymentMethodTypes {
            payment_method_type,
            card_networks: Some(card_provider),
            minimum_amount: Some(MinorUnit::zero()),
            maximum_amount: Some(MinorUnit::new(68607706)),
            recurring_enabled: Some(true),
            installment_payment_enabled: Some(false),
            accepted_currencies: None,
            accepted_countries: None,
            payment_experience: None,
        }
    }

    pub fn get_payment_experience(
        connector: Connector,
        payment_method_type: PaymentMethodType,
        payment_method: PaymentMethod,
        payment_experience: Option<api_models::enums::PaymentExperience>,
    ) -> Option<api_models::enums::PaymentExperience> {
        match payment_method {
            PaymentMethod::BankRedirect => None,
            _ => match (connector, payment_method_type) {
                #[cfg(feature = "dummy_connector")]
                (Connector::DummyConnector4, _) | (Connector::DummyConnector7, _) => {
                    Some(api_models::enums::PaymentExperience::RedirectToUrl)
                }
                (Connector::Paypal, Paypal) => payment_experience,
                (Connector::Klarna, Klarna) => payment_experience,
                (Connector::Zen, GooglePay) | (Connector::Zen, ApplePay) => {
                    Some(api_models::enums::PaymentExperience::RedirectToUrl)
                }
                (Connector::Braintree, Paypal) => {
                    Some(api_models::enums::PaymentExperience::InvokeSdkClient)
                }
                (Connector::Globepay, AliPay)
                | (Connector::Globepay, WeChatPay)
                | (Connector::Stripe, WeChatPay) => {
                    Some(api_models::enums::PaymentExperience::DisplayQrCode)
                }
                (_, GooglePay)
                | (_, ApplePay)
                | (_, PaymentMethodType::SamsungPay)
                | (_, PaymentMethodType::Paze)
                | (_, PaymentMethodType::AmazonPay) => {
                    Some(api_models::enums::PaymentExperience::InvokeSdkClient)
                }
                (_, PaymentMethodType::DirectCarrierBilling) => {
                    Some(api_models::enums::PaymentExperience::CollectOtp)
                }
                (_, PaymentMethodType::Cashapp) | (_, PaymentMethodType::Swish) => {
                    Some(api_models::enums::PaymentExperience::DisplayQrCode)
                }
                _ => Some(api_models::enums::PaymentExperience::RedirectToUrl),
            },
        }
    }

    pub fn transform_payment_method(
        connector: Connector,
        provider: Vec<Provider>,
        payment_method: PaymentMethod,
    ) -> Vec<payment_methods::RequestPaymentMethodTypes> {
        let mut payment_method_types = Vec::new();
        for method_type in provider {
            let data = payment_methods::RequestPaymentMethodTypes {
                payment_method_type: method_type.payment_method_type,
                card_networks: None,
                minimum_amount: Some(MinorUnit::zero()),
                maximum_amount: Some(MinorUnit::new(68607706)),
                recurring_enabled: Some(true),
                installment_payment_enabled: Some(false),
                accepted_currencies: method_type.accepted_currencies,
                accepted_countries: method_type.accepted_countries,
                payment_experience: Self::get_payment_experience(
                    connector,
                    method_type.payment_method_type,
                    payment_method,
                    method_type.payment_experience,
                ),
            };
            payment_method_types.push(data)
        }
        payment_method_types
    }

    pub fn create_connector_request(
        request: Self,
        api_response: ConnectorApiIntegrationPayload,
    ) -> ConnectorApiIntegrationPayload {
        let mut card_payment_method_types = Vec::new();
        let mut payment_method_enabled = Vec::new();

        if let Some(payment_methods_enabled) = request.payment_methods_enabled.clone() {
            for payload in payment_methods_enabled {
                match payload.payment_method {
                    PaymentMethod::Card => {
                        if let Some(card_provider) = payload.card_provider {
                            let payment_type =
                                PaymentMethodType::from_str(&payload.payment_method_type)
                                    .map_err(|_| "Invalid key received".to_string());

                            if let Ok(payment_type) = payment_type {
                                for method in card_provider {
                                    let data = payment_methods::RequestPaymentMethodTypes {
                                        payment_method_type: payment_type,
                                        card_networks: Some(vec![method.payment_method_type]),
                                        minimum_amount: Some(MinorUnit::zero()),
                                        maximum_amount: Some(MinorUnit::new(68607706)),
                                        recurring_enabled: Some(true),
                                        installment_payment_enabled: Some(false),
                                        accepted_currencies: method.accepted_currencies,
                                        accepted_countries: method.accepted_countries,
                                        payment_experience: None,
                                    };
                                    card_payment_method_types.push(data)
                                }
                            }
                        }
                    }

                    PaymentMethod::BankRedirect
                    | PaymentMethod::Wallet
                    | PaymentMethod::PayLater
                    | PaymentMethod::BankTransfer
                    | PaymentMethod::Crypto
                    | PaymentMethod::BankDebit
                    | PaymentMethod::Reward
                    | PaymentMethod::RealTimePayment
                    | PaymentMethod::Upi
                    | PaymentMethod::Voucher
                    | PaymentMethod::GiftCard
                    | PaymentMethod::OpenBanking
                    | PaymentMethod::CardRedirect
                    | PaymentMethod::MobilePayment => {
                        if let Some(provider) = payload.provider {
                            let val = Self::transform_payment_method(
                                request.connector,
                                provider,
                                payload.payment_method,
                            );
                            if !val.is_empty() {
                                let methods = PaymentMethodsEnabled {
                                    payment_method: payload.payment_method,
                                    payment_method_types: Some(val),
                                };
                                payment_method_enabled.push(methods);
                            }
                        }
                    }
                };
            }
            if !card_payment_method_types.is_empty() {
                let card = PaymentMethodsEnabled {
                    payment_method: PaymentMethod::Card,
                    payment_method_types: Some(card_payment_method_types),
                };
                payment_method_enabled.push(card);
            }
        }

        ConnectorApiIntegrationPayload {
            connector_type: api_response.connector_type,
            profile_id: api_response.profile_id,
            connector_name: api_response.connector_name,
            connector_label: api_response.connector_label,
            merchant_connector_id: api_response.merchant_connector_id,
            disabled: api_response.disabled,
            test_mode: api_response.test_mode,
            payment_methods_enabled: Some(payment_method_enabled),
            connector_webhook_details: api_response.connector_webhook_details,
            metadata: request.metadata,
        }
    }
}
