use std::collections::HashMap;

use api_models::{
    enums::{
        CountryAlpha2, FieldType,
        PaymentMethod::{BankRedirect, BankTransfer, Card, Wallet},
        PaymentMethodType, PayoutConnectors,
    },
    payment_methods::RequiredFieldInfo,
};

use crate::settings::{
    ConnectorFields, PaymentMethodType as PaymentMethodTypeInfo, PayoutRequiredFields,
    RequiredFieldFinal,
};

#[cfg(feature = "v1")]
impl Default for PayoutRequiredFields {
    fn default() -> Self {
        Self(HashMap::from([
            (
                Card,
                PaymentMethodTypeInfo(HashMap::from([
                    // Adyen
                    get_connector_payment_method_type_fields(
                        PayoutConnectors::Adyenplatform,
                        PaymentMethodType::Debit,
                    ),
                    get_connector_payment_method_type_fields(
                        PayoutConnectors::Adyenplatform,
                        PaymentMethodType::Credit,
                    ),
                ])),
            ),
            (
                BankTransfer,
                PaymentMethodTypeInfo(HashMap::from([
                    // Adyen
                    get_connector_payment_method_type_fields(
                        PayoutConnectors::Adyenplatform,
                        PaymentMethodType::SepaBankTransfer,
                    ),
                    // Ebanx
                    get_connector_payment_method_type_fields(
                        PayoutConnectors::Ebanx,
                        PaymentMethodType::Pix,
                    ),
                    // Wise
                    get_connector_payment_method_type_fields(
                        PayoutConnectors::Wise,
                        PaymentMethodType::Bacs,
                    ),
                ])),
            ),
            (
                Wallet,
                PaymentMethodTypeInfo(HashMap::from([
                    // Adyen
                    get_connector_payment_method_type_fields(
                        PayoutConnectors::Adyenplatform,
                        PaymentMethodType::Paypal,
                    ),
                ])),
            ),
            (
                // TODO: Refactor to support multiple connectors, each having its own set of required fields.
                BankRedirect,
                PaymentMethodTypeInfo(HashMap::from([{
                    let (pmt, mut gidadat_fields) = get_connector_payment_method_type_fields(
                        PayoutConnectors::Gigadat,
                        PaymentMethodType::Interac,
                    );

                    let (_, loonio_fields) = get_connector_payment_method_type_fields(
                        PayoutConnectors::Loonio,
                        PaymentMethodType::Interac,
                    );

                    gidadat_fields.fields.extend(loonio_fields.fields);

                    (pmt, gidadat_fields)
                }])),
            ),
        ]))
    }
}

fn get_billing_details_for_payment_method(
    connector: PayoutConnectors,
    payment_method_type: PaymentMethodType,
) -> HashMap<String, RequiredFieldInfo> {
    match connector {
        PayoutConnectors::Adyenplatform => {
            let mut fields = HashMap::from([
                (
                    "billing.address.line1".to_string(),
                    RequiredFieldInfo {
                        required_field: "billing.address.line1".to_string(),
                        display_name: "billing_address_line1".to_string(),
                        field_type: FieldType::Text,
                        value: None,
                    },
                ),
                (
                    "billing.address.line2".to_string(),
                    RequiredFieldInfo {
                        required_field: "billing.address.line2".to_string(),
                        display_name: "billing_address_line2".to_string(),
                        field_type: FieldType::Text,
                        value: None,
                    },
                ),
                (
                    "billing.address.city".to_string(),
                    RequiredFieldInfo {
                        required_field: "billing.address.city".to_string(),
                        display_name: "billing_address_city".to_string(),
                        field_type: FieldType::Text,
                        value: None,
                    },
                ),
                (
                    "billing.address.country".to_string(),
                    RequiredFieldInfo {
                        required_field: "billing.address.country".to_string(),
                        display_name: "billing_address_country".to_string(),
                        field_type: FieldType::UserAddressCountry {
                            options: get_countries_for_connector(connector)
                                .iter()
                                .map(|country| country.to_string())
                                .collect::<Vec<String>>(),
                        },
                        value: None,
                    },
                ),
            ]);

            // Add first_name for bank payouts only
            if payment_method_type == PaymentMethodType::SepaBankTransfer {
                fields.insert(
                    "billing.address.first_name".to_string(),
                    RequiredFieldInfo {
                        required_field: "billing.address.first_name".to_string(),
                        display_name: "billing_address_first_name".to_string(),
                        field_type: FieldType::Text,
                        value: None,
                    },
                );
            }

            fields
        }
        _ => get_billing_details(connector),
    }
}

#[cfg(feature = "v1")]
fn get_connector_payment_method_type_fields(
    connector: PayoutConnectors,
    payment_method_type: PaymentMethodType,
) -> (PaymentMethodType, ConnectorFields) {
    let mut common_fields = get_billing_details_for_payment_method(connector, payment_method_type);
    match payment_method_type {
        // Card
        PaymentMethodType::Debit => {
            common_fields.extend(get_card_fields());
            (
                payment_method_type,
                ConnectorFields {
                    fields: HashMap::from([(
                        connector.into(),
                        RequiredFieldFinal {
                            mandate: HashMap::new(),
                            non_mandate: HashMap::new(),
                            common: common_fields,
                        },
                    )]),
                },
            )
        }
        PaymentMethodType::Credit => {
            common_fields.extend(get_card_fields());
            (
                payment_method_type,
                ConnectorFields {
                    fields: HashMap::from([(
                        connector.into(),
                        RequiredFieldFinal {
                            mandate: HashMap::new(),
                            non_mandate: HashMap::new(),
                            common: common_fields,
                        },
                    )]),
                },
            )
        }

        // Banks
        PaymentMethodType::Bacs => {
            common_fields.extend(get_bacs_fields());
            (
                payment_method_type,
                ConnectorFields {
                    fields: HashMap::from([(
                        connector.into(),
                        RequiredFieldFinal {
                            mandate: HashMap::new(),
                            non_mandate: HashMap::new(),
                            common: common_fields,
                        },
                    )]),
                },
            )
        }
        PaymentMethodType::Pix => {
            common_fields.extend(get_pix_bank_transfer_fields());
            (
                payment_method_type,
                ConnectorFields {
                    fields: HashMap::from([(
                        connector.into(),
                        RequiredFieldFinal {
                            mandate: HashMap::new(),
                            non_mandate: HashMap::new(),
                            common: common_fields,
                        },
                    )]),
                },
            )
        }
        PaymentMethodType::SepaBankTransfer => {
            common_fields.extend(get_sepa_fields());
            (
                payment_method_type,
                ConnectorFields {
                    fields: HashMap::from([(
                        connector.into(),
                        RequiredFieldFinal {
                            mandate: HashMap::new(),
                            non_mandate: HashMap::new(),
                            common: common_fields,
                        },
                    )]),
                },
            )
        }

        // Wallets
        PaymentMethodType::Paypal => {
            common_fields.extend(get_paypal_fields());
            (
                payment_method_type,
                ConnectorFields {
                    fields: HashMap::from([(
                        connector.into(),
                        RequiredFieldFinal {
                            mandate: HashMap::new(),
                            non_mandate: HashMap::new(),
                            common: common_fields,
                        },
                    )]),
                },
            )
        }

        // Bank Redirect
        PaymentMethodType::Interac => {
            common_fields.extend(get_interac_fields());
            (
                payment_method_type,
                ConnectorFields {
                    fields: HashMap::from([(
                        connector.into(),
                        RequiredFieldFinal {
                            mandate: HashMap::new(),
                            non_mandate: HashMap::new(),
                            common: common_fields,
                        },
                    )]),
                },
            )
        }

        _ => (
            payment_method_type,
            ConnectorFields {
                fields: HashMap::new(),
            },
        ),
    }
}

fn get_card_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([
        (
            "payout_method_data.card.card_number".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.card.card_number".to_string(),
                display_name: "card_number".to_string(),
                field_type: FieldType::UserCardNumber,
                value: None,
            },
        ),
        (
            "payout_method_data.card.expiry_month".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.card.expiry_month".to_string(),
                display_name: "exp_month".to_string(),
                field_type: FieldType::UserCardExpiryMonth,
                value: None,
            },
        ),
        (
            "payout_method_data.card.expiry_year".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.card.expiry_year".to_string(),
                display_name: "exp_year".to_string(),
                field_type: FieldType::UserCardExpiryYear,
                value: None,
            },
        ),
        (
            "payout_method_data.card.card_holder_name".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.card.card_holder_name".to_string(),
                display_name: "card_holder_name".to_string(),
                field_type: FieldType::UserFullName,
                value: None,
            },
        ),
    ])
}

fn get_bacs_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([
        (
            "payout_method_data.bank.bank_sort_code".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.bank_sort_code".to_string(),
                display_name: "bank_sort_code".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "payout_method_data.bank.bank_account_number".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.bank_account_number".to_string(),
                display_name: "bank_account_number".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
    ])
}

fn get_pix_bank_transfer_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([
        (
            "payout_method_data.bank.bank_account_number".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.bank_account_number".to_string(),
                display_name: "bank_account_number".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "payout_method_data.bank.pix_key".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.pix_key".to_string(),
                display_name: "pix_key".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
    ])
}

fn get_sepa_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([
        (
            "payout_method_data.bank.iban".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.iban".to_string(),
                display_name: "iban".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "payout_method_data.bank.bic".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.bic".to_string(),
                display_name: "bic".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
    ])
}

fn get_paypal_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([(
        "payout_method_data.wallet.telephone_number".to_string(),
        RequiredFieldInfo {
            required_field: "payout_method_data.wallet.telephone_number".to_string(),
            display_name: "telephone_number".to_string(),
            field_type: FieldType::Text,
            value: None,
        },
    )])
}

fn get_interac_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([(
        "payout_method_data.bank_redirect.email".to_string(),
        RequiredFieldInfo {
            required_field: "payout_method_data.bank_redirect.email".to_string(),
            display_name: "email".to_string(),
            field_type: FieldType::Text,
            value: None,
        },
    )])
}

fn get_countries_for_connector(connector: PayoutConnectors) -> Vec<CountryAlpha2> {
    match connector {
        PayoutConnectors::Adyenplatform => vec![
            CountryAlpha2::ES,
            CountryAlpha2::SK,
            CountryAlpha2::AT,
            CountryAlpha2::NL,
            CountryAlpha2::DE,
            CountryAlpha2::BE,
            CountryAlpha2::FR,
            CountryAlpha2::FI,
            CountryAlpha2::PT,
            CountryAlpha2::IE,
            CountryAlpha2::EE,
            CountryAlpha2::LT,
            CountryAlpha2::LV,
            CountryAlpha2::IT,
            CountryAlpha2::CZ,
            CountryAlpha2::DE,
            CountryAlpha2::HU,
            CountryAlpha2::NO,
            CountryAlpha2::PL,
            CountryAlpha2::SE,
            CountryAlpha2::GB,
            CountryAlpha2::CH,
        ],
        PayoutConnectors::Stripe => vec![CountryAlpha2::US],
        _ => vec![],
    }
}

fn get_billing_details(connector: PayoutConnectors) -> HashMap<String, RequiredFieldInfo> {
    match connector {
        PayoutConnectors::Adyen => HashMap::from([
            (
                "billing.address.line1".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.line1".to_string(),
                    display_name: "billing_address_line1".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
            (
                "billing.address.line2".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.line2".to_string(),
                    display_name: "billing_address_line2".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
            (
                "billing.address.city".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.city".to_string(),
                    display_name: "billing_address_city".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
            (
                "billing.address.zip".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.zip".to_string(),
                    display_name: "billing_address_zip".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
            (
                "billing.address.country".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.country".to_string(),
                    display_name: "billing_address_country".to_string(),
                    field_type: FieldType::UserAddressCountry {
                        options: get_countries_for_connector(connector)
                            .iter()
                            .map(|country| country.to_string())
                            .collect::<Vec<String>>(),
                    },
                    value: None,
                },
            ),
            (
                "billing.address.first_name".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.first_name".to_string(),
                    display_name: "billing_address_first_name".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
            (
                "billing.address.last_name".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.last_name".to_string(),
                    display_name: "billing_address_last_name".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
        ]),
        PayoutConnectors::Wise => HashMap::from([
            (
                "billing.address.line1".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.line1".to_string(),
                    display_name: "billing_address_line1".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
            (
                "billing.address.city".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.city".to_string(),
                    display_name: "billing_address_city".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
            (
                "billing.address.state".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.state".to_string(),
                    display_name: "billing_address_state".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
            (
                "billing.address.zip".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.zip".to_string(),
                    display_name: "billing_address_zip".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
            (
                "billing.address.country".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.country".to_string(),
                    display_name: "billing_address_country".to_string(),
                    field_type: FieldType::UserAddressCountry {
                        options: get_countries_for_connector(connector)
                            .iter()
                            .map(|country| country.to_string())
                            .collect::<Vec<String>>(),
                    },
                    value: None,
                },
            ),
            (
                "billing.address.first_name".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.first_name".to_string(),
                    display_name: "billing_address_first_name".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
        ]),
        PayoutConnectors::Loonio => HashMap::from([
            (
                "billing.address.first_name".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.first_name".to_string(),
                    display_name: "billing_address_first_name".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
            (
                "billing.address.last_name".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.last_name".to_string(),
                    display_name: "billing_address_last_name".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
        ]),
        PayoutConnectors::Gigadat => HashMap::from([
            (
                "billing.address.first_name".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.first_name".to_string(),
                    display_name: "billing_address_first_name".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
            (
                "billing.address.last_name".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.address.last_name".to_string(),
                    display_name: "billing_address_last_name".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
            (
                "billing.phone.number".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.phone.number".to_string(),
                    display_name: "phone".to_string(),
                    field_type: FieldType::UserPhoneNumber,
                    value: None,
                },
            ),
            (
                "billing.phone.country_code".to_string(),
                RequiredFieldInfo {
                    required_field: "billing.phone.country_code".to_string(),
                    display_name: "dialing_code".to_string(),
                    field_type: FieldType::UserPhoneNumberCountryCode,
                    value: None,
                },
            ),
        ]),
        _ => HashMap::from([]),
    }
}
