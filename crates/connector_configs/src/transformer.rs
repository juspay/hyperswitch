use std::str::FromStr;

use api_models::{
    enums::{
        Connector, PaymentMethod,
        PaymentMethodType::{self, AliPay, ApplePay, GooglePay, Klarna, Paypal, WeChatPay},
    },
    payment_methods, payments,
    refunds::MinorUnit,
};

use crate::common_config::{
    ApiModelMetaData, ConnectorApiIntegrationPayload, DashboardMetaData, DashboardRequestPayload,
    GoogleApiModelData, GooglePayData, PaymentMethodsEnabled, Provider,
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
            recurring_enabled: true,
            installment_payment_enabled: false,
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
                (Connector::Zen, GooglePay) | (Connector::Zen, ApplePay) => {
                    Some(api_models::enums::PaymentExperience::RedirectToUrl)
                }
                (Connector::Braintree, Paypal) | (Connector::Klarna, Klarna) => {
                    Some(api_models::enums::PaymentExperience::InvokeSdkClient)
                }
                (Connector::Globepay, AliPay)
                | (Connector::Globepay, WeChatPay)
                | (Connector::Stripe, WeChatPay) => {
                    Some(api_models::enums::PaymentExperience::DisplayQrCode)
                }
                (_, GooglePay) | (_, ApplePay) => {
                    Some(api_models::enums::PaymentExperience::InvokeSdkClient)
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
                recurring_enabled: true,
                installment_payment_enabled: false,
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
                                        recurring_enabled: true,
                                        installment_payment_enabled: false,
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
                    | PaymentMethod::CardRedirect => {
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

        let metadata = Self::transform_metedata(request);
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
            metadata,
        }
    }

    pub fn transform_metedata(request: Self) -> Option<ApiModelMetaData> {
        let default_metadata = DashboardMetaData {
            apple_pay_combined: None,
            google_pay: None,
            apple_pay: None,
            account_name: None,
            terminal_id: None,
            merchant_account_id: None,
            merchant_id: None,
            merchant_config_currency: None,
            endpoint_prefix: None,
            mcc: None,
            merchant_country_code: None,
            merchant_name: None,
            acquirer_bin: None,
            acquirer_merchant_id: None,
            acquirer_country_code: None,
            three_ds_requestor_name: None,
            three_ds_requestor_id: None,
            pull_mechanism_for_external_3ds_enabled: None,
            paypal_sdk: None,
            klarna_region: None,
            source_balance_account: None,
            brand_id: None,
            destination_account_number: None,
        };
        let meta_data = match request.metadata {
            Some(data) => data,
            None => default_metadata,
        };
        let google_pay = Self::get_google_pay_details(meta_data.clone(), request.connector);
        let account_name = meta_data.account_name.clone();
        let merchant_account_id = meta_data.merchant_account_id.clone();
        let merchant_id = meta_data.merchant_id.clone();
        let terminal_id = meta_data.terminal_id.clone();
        let endpoint_prefix = meta_data.endpoint_prefix.clone();
        let paypal_sdk = meta_data.paypal_sdk;
        let apple_pay = meta_data.apple_pay;
        let apple_pay_combined = meta_data.apple_pay_combined;
        let merchant_config_currency = meta_data.merchant_config_currency;
        let mcc = meta_data.mcc;
        let merchant_country_code = meta_data.merchant_country_code;
        let merchant_name = meta_data.merchant_name;
        let acquirer_bin = meta_data.acquirer_bin;
        let acquirer_merchant_id = meta_data.acquirer_merchant_id;
        let acquirer_country_code = meta_data.acquirer_country_code;
        let three_ds_requestor_name = meta_data.three_ds_requestor_name;
        let three_ds_requestor_id = meta_data.three_ds_requestor_id;
        let pull_mechanism_for_external_3ds_enabled =
            meta_data.pull_mechanism_for_external_3ds_enabled;
        let klarna_region = meta_data.klarna_region;
        let source_balance_account = meta_data.source_balance_account;
        let brand_id = meta_data.brand_id;
        let destination_account_number = meta_data.destination_account_number;

        Some(ApiModelMetaData {
            google_pay,
            apple_pay,
            account_name,
            merchant_account_id,
            terminal_id,
            merchant_id,
            merchant_config_currency,
            apple_pay_combined,
            endpoint_prefix,
            paypal_sdk,
            mcc,
            merchant_country_code,
            merchant_name,
            acquirer_bin,
            acquirer_merchant_id,
            acquirer_country_code,
            three_ds_requestor_name,
            three_ds_requestor_id,
            pull_mechanism_for_external_3ds_enabled,
            klarna_region,
            source_balance_account,
            brand_id,
            destination_account_number,
        })
    }

    fn get_custom_gateway_name(connector: Connector) -> String {
        match connector {
            Connector::Checkout => String::from("checkoutltd"),
            Connector::Nuvei => String::from("nuveidigital"),
            Connector::Authorizedotnet => String::from("authorizenet"),
            Connector::Globalpay => String::from("globalpayments"),
            Connector::Bankofamerica | Connector::Cybersource => String::from("cybersource"),
            _ => connector.to_string(),
        }
    }
    fn get_google_pay_details(
        meta_data: DashboardMetaData,
        connector: Connector,
    ) -> Option<GoogleApiModelData> {
        match meta_data.google_pay {
            Some(gpay_data) => {
                let google_pay_data = match gpay_data {
                    GooglePayData::Standard(data) => {
                        let token_parameter = payments::GpayTokenParameters {
                            gateway: Self::get_custom_gateway_name(connector),
                            gateway_merchant_id: data.gateway_merchant_id,
                            stripe_version: match connector {
                                Connector::Stripe => Some(String::from("2018-10-31")),
                                _ => None,
                            },
                            stripe_publishable_key: match connector {
                                Connector::Stripe => data.stripe_publishable_key,
                                _ => None,
                            },
                        };
                        let merchant_info = payments::GpayMerchantInfo {
                            merchant_name: data.merchant_name,
                            merchant_id: data.merchant_id,
                        };
                        let token_specification = payments::GpayTokenizationSpecification {
                            token_specification_type: String::from("PAYMENT_GATEWAY"),
                            parameters: token_parameter,
                        };
                        let allowed_payment_methods_parameters =
                            payments::GpayAllowedMethodsParameters {
                                allowed_auth_methods: vec![
                                    "PAN_ONLY".to_string(),
                                    "CRYPTOGRAM_3DS".to_string(),
                                ],
                                allowed_card_networks: vec![
                                    "AMEX".to_string(),
                                    "DISCOVER".to_string(),
                                    "INTERAC".to_string(),
                                    "JCB".to_string(),
                                    "MASTERCARD".to_string(),
                                    "VISA".to_string(),
                                ],
                                billing_address_required: None,
                                billing_address_parameters: None,
                                assurance_details_required: Some(false),
                            };
                        let allowed_payment_methods = payments::GpayAllowedPaymentMethods {
                            payment_method_type: String::from("CARD"),
                            parameters: allowed_payment_methods_parameters,
                            tokenization_specification: token_specification,
                        };
                        GoogleApiModelData::Standard(payments::GpayMetaData {
                            merchant_info,
                            allowed_payment_methods: vec![allowed_payment_methods],
                        })
                    }
                    GooglePayData::Zen(data) => GoogleApiModelData::Zen(data),
                };
                Some(google_pay_data)
            }
            _ => None,
        }
    }
}
