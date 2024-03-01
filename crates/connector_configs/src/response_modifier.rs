use crate::common_config::{
    CardProvider, ConnectorApiIntegrationPayload, DashboardMetaData, DashboardPaymentMethodPayload,
    DashboardRequestPayload, GoogleApiModelData, GooglePayData, GpayDashboardPayLoad, Provider,
};

impl ConnectorApiIntegrationPayload {
    pub fn get_transformed_response_payload(response: Self) -> DashboardRequestPayload {
        let mut wallet_details: Vec<Provider> = Vec::new();
        let mut bank_redirect_details: Vec<Provider> = Vec::new();
        let mut pay_later_details: Vec<Provider> = Vec::new();
        let mut debit_details: Vec<CardProvider> = Vec::new();
        let mut credit_details: Vec<CardProvider> = Vec::new();
        let mut bank_transfer_details: Vec<Provider> = Vec::new();
        let mut crypto_details: Vec<Provider> = Vec::new();
        let mut bank_debit_details: Vec<Provider> = Vec::new();
        let mut reward_details: Vec<Provider> = Vec::new();
        let mut upi_details: Vec<Provider> = Vec::new();
        let mut voucher_details: Vec<Provider> = Vec::new();
        let mut gift_card_details: Vec<Provider> = Vec::new();
        let mut card_redirect_details: Vec<Provider> = Vec::new();

        if let Some(payment_methods_enabled) = response.payment_methods_enabled.clone() {
            for methods in payment_methods_enabled {
                match methods.payment_method {
                    api_models::enums::PaymentMethod::Card => {
                        if let Some(payment_method_types) = methods.payment_method_types {
                            for method_type in payment_method_types {
                                match method_type.payment_method_type {
                                    api_models::enums::PaymentMethodType::Credit => {
                                        if let Some(card_networks) = method_type.card_networks {
                                            for card in card_networks {
                                                credit_details.push(CardProvider {
                                                    payment_method_type: card,
                                                    accepted_currencies: method_type
                                                        .accepted_currencies
                                                        .clone(),
                                                    accepted_countries: method_type
                                                        .accepted_countries
                                                        .clone(),
                                                })
                                            }
                                        }
                                    }
                                    api_models::enums::PaymentMethodType::Debit => {
                                        if let Some(card_networks) = method_type.card_networks {
                                            for card in card_networks {
                                                // debit_details.push(card)
                                                debit_details.push(CardProvider {
                                                    payment_method_type: card,
                                                    accepted_currencies: method_type
                                                        .accepted_currencies
                                                        .clone(),
                                                    accepted_countries: method_type
                                                        .accepted_countries
                                                        .clone(),
                                                })
                                            }
                                        }
                                    }
                                    _ => (),
                                }
                            }
                        }
                    }
                    api_models::enums::PaymentMethod::Wallet => {
                        if let Some(payment_method_types) = methods.payment_method_types {
                            for method_type in payment_method_types {
                                // wallet_details.push(method_type.payment_method_type)
                                wallet_details.push(Provider {
                                    payment_method_type: method_type.payment_method_type,
                                    accepted_currencies: method_type.accepted_currencies.clone(),
                                    accepted_countries: method_type.accepted_countries.clone(),
                                })
                            }
                        }
                    }
                    api_models::enums::PaymentMethod::BankRedirect => {
                        if let Some(payment_method_types) = methods.payment_method_types {
                            for method_type in payment_method_types {
                                bank_redirect_details.push(Provider {
                                    payment_method_type: method_type.payment_method_type,
                                    accepted_currencies: method_type.accepted_currencies.clone(),
                                    accepted_countries: method_type.accepted_countries.clone(),
                                })
                            }
                        }
                    }
                    api_models::enums::PaymentMethod::PayLater => {
                        if let Some(payment_method_types) = methods.payment_method_types {
                            for method_type in payment_method_types {
                                pay_later_details.push(Provider {
                                    payment_method_type: method_type.payment_method_type,
                                    accepted_currencies: method_type.accepted_currencies.clone(),
                                    accepted_countries: method_type.accepted_countries.clone(),
                                })
                            }
                        }
                    }
                    api_models::enums::PaymentMethod::BankTransfer => {
                        if let Some(payment_method_types) = methods.payment_method_types {
                            for method_type in payment_method_types {
                                bank_transfer_details.push(Provider {
                                    payment_method_type: method_type.payment_method_type,
                                    accepted_currencies: method_type.accepted_currencies.clone(),
                                    accepted_countries: method_type.accepted_countries.clone(),
                                })
                            }
                        }
                    }
                    api_models::enums::PaymentMethod::Crypto => {
                        if let Some(payment_method_types) = methods.payment_method_types {
                            for method_type in payment_method_types {
                                crypto_details.push(Provider {
                                    payment_method_type: method_type.payment_method_type,
                                    accepted_currencies: method_type.accepted_currencies.clone(),
                                    accepted_countries: method_type.accepted_countries.clone(),
                                })
                            }
                        }
                    }
                    api_models::enums::PaymentMethod::BankDebit => {
                        if let Some(payment_method_types) = methods.payment_method_types {
                            for method_type in payment_method_types {
                                bank_debit_details.push(Provider {
                                    payment_method_type: method_type.payment_method_type,
                                    accepted_currencies: method_type.accepted_currencies.clone(),
                                    accepted_countries: method_type.accepted_countries.clone(),
                                })
                            }
                        }
                    }
                    api_models::enums::PaymentMethod::Reward => {
                        if let Some(payment_method_types) = methods.payment_method_types {
                            for method_type in payment_method_types {
                                reward_details.push(Provider {
                                    payment_method_type: method_type.payment_method_type,
                                    accepted_currencies: method_type.accepted_currencies.clone(),
                                    accepted_countries: method_type.accepted_countries.clone(),
                                })
                            }
                        }
                    }
                    api_models::enums::PaymentMethod::Upi => {
                        if let Some(payment_method_types) = methods.payment_method_types {
                            for method_type in payment_method_types {
                                upi_details.push(Provider {
                                    payment_method_type: method_type.payment_method_type,
                                    accepted_currencies: method_type.accepted_currencies.clone(),
                                    accepted_countries: method_type.accepted_countries.clone(),
                                })
                            }
                        }
                    }
                    api_models::enums::PaymentMethod::Voucher => {
                        if let Some(payment_method_types) = methods.payment_method_types {
                            for method_type in payment_method_types {
                                voucher_details.push(Provider {
                                    payment_method_type: method_type.payment_method_type,
                                    accepted_currencies: method_type.accepted_currencies.clone(),
                                    accepted_countries: method_type.accepted_countries.clone(),
                                })
                            }
                        }
                    }
                    api_models::enums::PaymentMethod::GiftCard => {
                        if let Some(payment_method_types) = methods.payment_method_types {
                            for method_type in payment_method_types {
                                gift_card_details.push(Provider {
                                    payment_method_type: method_type.payment_method_type,
                                    accepted_currencies: method_type.accepted_currencies.clone(),
                                    accepted_countries: method_type.accepted_countries.clone(),
                                })
                            }
                        }
                    }
                    api_models::enums::PaymentMethod::CardRedirect => {
                        if let Some(payment_method_types) = methods.payment_method_types {
                            for method_type in payment_method_types {
                                card_redirect_details.push(Provider {
                                    payment_method_type: method_type.payment_method_type,
                                    accepted_currencies: method_type.accepted_currencies.clone(),
                                    accepted_countries: method_type.accepted_countries.clone(),
                                })
                            }
                        }
                    }
                }
            }
        }

        let upi = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::Upi,
            payment_method_type: api_models::enums::PaymentMethod::Upi.to_string(),
            provider: Some(upi_details),
            card_provider: None,
        };

        let voucher: DashboardPaymentMethodPayload = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::Voucher,
            payment_method_type: api_models::enums::PaymentMethod::Voucher.to_string(),
            provider: Some(voucher_details),
            card_provider: None,
        };

        let gift_card: DashboardPaymentMethodPayload = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::GiftCard,
            payment_method_type: api_models::enums::PaymentMethod::GiftCard.to_string(),
            provider: Some(gift_card_details),
            card_provider: None,
        };

        let reward = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::Reward,
            payment_method_type: api_models::enums::PaymentMethod::Reward.to_string(),
            provider: Some(reward_details),
            card_provider: None,
        };

        let wallet = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::Wallet,
            payment_method_type: api_models::enums::PaymentMethod::Wallet.to_string(),
            provider: Some(wallet_details),
            card_provider: None,
        };
        let bank_redirect = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::BankRedirect,
            payment_method_type: api_models::enums::PaymentMethod::BankRedirect.to_string(),
            provider: Some(bank_redirect_details),
            card_provider: None,
        };

        let bank_debit = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::BankDebit,
            payment_method_type: api_models::enums::PaymentMethod::BankDebit.to_string(),
            provider: Some(bank_debit_details),
            card_provider: None,
        };

        let bank_transfer = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::BankTransfer,
            payment_method_type: api_models::enums::PaymentMethod::BankTransfer.to_string(),
            provider: Some(bank_transfer_details),
            card_provider: None,
        };

        let crypto = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::Crypto,
            payment_method_type: api_models::enums::PaymentMethod::Crypto.to_string(),
            provider: Some(crypto_details),
            card_provider: None,
        };

        let card_redirect = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::CardRedirect,
            payment_method_type: api_models::enums::PaymentMethod::CardRedirect.to_string(),
            provider: Some(card_redirect_details),
            card_provider: None,
        };
        let pay_later = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::PayLater,
            payment_method_type: api_models::enums::PaymentMethod::PayLater.to_string(),
            provider: Some(pay_later_details),
            card_provider: None,
        };
        let debit_details = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::Card,
            payment_method_type: api_models::enums::PaymentMethodType::Debit.to_string(),
            provider: None,
            card_provider: Some(debit_details),
        };
        let credit_details = DashboardPaymentMethodPayload {
            payment_method: api_models::enums::PaymentMethod::Card,
            payment_method_type: api_models::enums::PaymentMethodType::Credit.to_string(),
            provider: None,
            card_provider: Some(credit_details),
        };

        let google_pay = Self::get_google_pay_metadata_response(response.clone());
        let account_name = match response.metadata.clone() {
            Some(meta_data) => meta_data.account_name,
            _ => None,
        };

        let merchant_account_id = match response.metadata.clone() {
            Some(meta_data) => meta_data.merchant_account_id,
            _ => None,
        };
        let merchant_id = match response.metadata.clone() {
            Some(meta_data) => meta_data.merchant_id,
            _ => None,
        };
        let terminal_id = match response.metadata.clone() {
            Some(meta_data) => meta_data.terminal_id,
            _ => None,
        };
        let endpoint_prefix = match response.metadata.clone() {
            Some(meta_data) => meta_data.endpoint_prefix,
            _ => None,
        };
        let apple_pay = match response.metadata.clone() {
            Some(meta_data) => meta_data.apple_pay,
            _ => None,
        };
        let apple_pay_combined = match response.metadata.clone() {
            Some(meta_data) => meta_data.apple_pay_combined,
            _ => None,
        };
        let merchant_config_currency = match response.metadata.clone() {
            Some(meta_data) => meta_data.merchant_config_currency,
            _ => None,
        };

        let meta_data = DashboardMetaData {
            merchant_config_currency,
            merchant_account_id,
            apple_pay,
            apple_pay_combined,
            google_pay,
            account_name,
            terminal_id,
            merchant_id,
            endpoint_prefix,
        };

        DashboardRequestPayload {
            connector: response.connector_name,
            payment_methods_enabled: Some(vec![
                upi,
                voucher,
                reward,
                wallet,
                bank_redirect,
                bank_debit,
                bank_transfer,
                crypto,
                card_redirect,
                pay_later,
                debit_details,
                credit_details,
                gift_card,
            ]),
            metadata: Some(meta_data),
        }
    }

    pub fn get_google_pay_metadata_response(response: Self) -> Option<GooglePayData> {
        match response.metadata {
            Some(meta_data) => {
                match meta_data.google_pay {
                    Some(google_pay) => match google_pay {
                        GoogleApiModelData::Standard(standard_data) => {
                            let data = standard_data.allowed_payment_methods.first().map(
                                |allowed_pm| {
                                    allowed_pm.tokenization_specification.parameters.clone()
                                },
                            )?;
                            Some(GooglePayData::Standard(GpayDashboardPayLoad {
                                gateway_merchant_id: data.gateway_merchant_id,
                                stripe_version: data.stripe_version,
                                stripe_publishable_key: data.stripe_publishable_key,
                                merchant_name: standard_data.merchant_info.merchant_name,
                                merchant_id: standard_data.merchant_info.merchant_id,
                            }))
                        }
                        GoogleApiModelData::Zen(data) => Some(GooglePayData::Zen(data)),
                    },
                    None => None,
                }
            }
            None => None,
        }
    }
}
