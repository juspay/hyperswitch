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
                    (
                        PaymentMethodType::Debit,
                        connectors(vec![(
                            PayoutConnectors::Adyenplatform,
                            fields(
                                vec![],
                                vec![],
                                adyen_billing_fields(PaymentMethodType::Debit, card_fields()),
                            ),
                        )]),
                    ),
                    (
                        PaymentMethodType::Credit,
                        connectors(vec![(
                            PayoutConnectors::Adyenplatform,
                            fields(
                                vec![],
                                vec![],
                                adyen_billing_fields(PaymentMethodType::Credit, card_fields()),
                            ),
                        )]),
                    ),
                ])),
            ),
            (
                BankTransfer,
                PaymentMethodTypeInfo(HashMap::from([
                    (
                        PaymentMethodType::SepaBankTransfer,
                        connectors(vec![
                            (
                                PayoutConnectors::Adyenplatform,
                                fields(
                                    vec![],
                                    vec![],
                                    adyen_billing_fields(
                                        PaymentMethodType::SepaBankTransfer,
                                        sepa_fields(),
                                    ),
                                ),
                            ),
                            (
                                PayoutConnectors::Deutschebank,
                                fields(vec![], vec![], sepa_deutschebank_fields()),
                            ),
                        ]),
                    ),
                    (
                        PaymentMethodType::Pix,
                        connectors(vec![
                            (
                                PayoutConnectors::Ebanx,
                                fields(vec![], vec![], pix_fields()),
                            ),
                            (
                                PayoutConnectors::Santander,
                                fields(vec![], vec![], pix_fields()),
                            ),
                        ]),
                    ),
                    (
                        PaymentMethodType::Bacs,
                        connectors(vec![(
                            PayoutConnectors::Wise,
                            fields(vec![], vec![], wise_billing_fields(bacs_fields())),
                        )]),
                    ),
                    (
                        PaymentMethodType::Ted,
                        connectors(vec![(
                            PayoutConnectors::Santander,
                            fields(vec![], vec![], ted_fields()),
                        )]),
                    ),
                ])),
            ),
            (
                Wallet,
                PaymentMethodTypeInfo(HashMap::from([(
                    PaymentMethodType::Paypal,
                    connectors(vec![(
                        PayoutConnectors::Adyenplatform,
                        fields(
                            vec![],
                            vec![],
                            adyen_billing_fields(PaymentMethodType::Paypal, paypal_fields()),
                        ),
                    )]),
                )])),
            ),
            (
                BankRedirect,
                PaymentMethodTypeInfo(HashMap::from([(
                    PaymentMethodType::Interac,
                    connectors(vec![
                        (
                            PayoutConnectors::Gigadat,
                            fields(vec![], vec![], gigadat_billing_fields(interac_fields())),
                        ),
                        (
                            PayoutConnectors::Loonio,
                            fields(vec![], vec![], loonio_billing_fields(interac_fields())),
                        ),
                    ]),
                )])),
            ),
        ]))
    }
}

#[cfg(feature = "v1")]
fn connectors(connectors: Vec<(PayoutConnectors, RequiredFieldFinal)>) -> ConnectorFields {
    ConnectorFields {
        fields: connectors.into_iter().map(|(c, f)| (c.into(), f)).collect(),
    }
}

#[cfg(feature = "v1")]
fn fields(
    mandate: Vec<(String, RequiredFieldInfo)>,
    non_mandate: Vec<(String, RequiredFieldInfo)>,
    common: Vec<(String, RequiredFieldInfo)>,
) -> RequiredFieldFinal {
    RequiredFieldFinal {
        mandate: mandate.into_iter().collect(),
        non_mandate: non_mandate.into_iter().collect(),
        common: common.into_iter().collect(),
    }
}

#[cfg(feature = "v1")]
fn card_fields() -> Vec<(String, RequiredFieldInfo)> {
    vec![
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
    ]
}

#[cfg(feature = "v1")]
fn bacs_fields() -> Vec<(String, RequiredFieldInfo)> {
    vec![
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
    ]
}

#[cfg(feature = "v1")]
fn pix_fields() -> Vec<(String, RequiredFieldInfo)> {
    vec![
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
    ]
}

#[cfg(feature = "v1")]
fn ted_fields() -> Vec<(String, RequiredFieldInfo)> {
    vec![
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
            "payout_method_data.bank.bank_code".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.bank_code".to_string(),
                display_name: "bank_code".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "payout_method_data.bank.bank_account_type".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.bank_account_type".to_string(),
                display_name: "bank_account_type".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "payout_method_data.bank.tax_id".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.tax_id".to_string(),
                display_name: "tax_id".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "payout_method_data.bank.account_holder_name".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.account_holder_name".to_string(),
                display_name: "account_holder_name".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "payout_method_data.bank.bank_branch".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.bank_branch".to_string(),
                display_name: "bank_branch".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
    ]
}

#[cfg(feature = "v1")]
fn sepa_fields() -> Vec<(String, RequiredFieldInfo)> {
    vec![
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
    ]
}

#[cfg(feature = "v1")]
fn sepa_deutschebank_fields() -> Vec<(String, RequiredFieldInfo)> {
    vec![
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
            "payout_method_data.bank.account_holder_name".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.account_holder_name".to_string(),
                display_name: "account_holder_name".to_string(),
                field_type: FieldType::UserFullName,
                value: None,
            },
        ),
    ]
}

#[cfg(feature = "v1")]
fn paypal_fields() -> Vec<(String, RequiredFieldInfo)> {
    vec![(
        "payout_method_data.wallet.telephone_number".to_string(),
        RequiredFieldInfo {
            required_field: "payout_method_data.wallet.telephone_number".to_string(),
            display_name: "telephone_number".to_string(),
            field_type: FieldType::Text,
            value: None,
        },
    )]
}

#[cfg(feature = "v1")]
fn interac_fields() -> Vec<(String, RequiredFieldInfo)> {
    vec![(
        "payout_method_data.bank_redirect.email".to_string(),
        RequiredFieldInfo {
            required_field: "payout_method_data.bank_redirect.email".to_string(),
            display_name: "email".to_string(),
            field_type: FieldType::Text,
            value: None,
        },
    )]
}

#[cfg(feature = "v1")]
fn adyen_billing_fields(
    payment_method_type: PaymentMethodType,
    mut extra: Vec<(String, RequiredFieldInfo)>,
) -> Vec<(String, RequiredFieldInfo)> {
    let mut fields = vec![
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
                    options: adyen_countries().iter().map(|c| c.to_string()).collect(),
                },
                value: None,
            },
        ),
    ];

    if payment_method_type == PaymentMethodType::SepaBankTransfer {
        fields.push((
            "billing.address.first_name".to_string(),
            RequiredFieldInfo {
                required_field: "billing.address.first_name".to_string(),
                display_name: "billing_address_first_name".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ));
    }

    fields.append(&mut extra);
    fields
}

#[cfg(feature = "v1")]
fn wise_billing_fields(
    mut extra: Vec<(String, RequiredFieldInfo)>,
) -> Vec<(String, RequiredFieldInfo)> {
    let mut fields = vec![
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
                    options: vec![CountryAlpha2::US.to_string()],
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
    ];
    fields.append(&mut extra);
    fields
}

#[cfg(feature = "v1")]
fn gigadat_billing_fields(
    mut extra: Vec<(String, RequiredFieldInfo)>,
) -> Vec<(String, RequiredFieldInfo)> {
    let mut fields = vec![
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
    ];
    fields.append(&mut extra);
    fields
}

#[cfg(feature = "v1")]
fn loonio_billing_fields(
    mut extra: Vec<(String, RequiredFieldInfo)>,
) -> Vec<(String, RequiredFieldInfo)> {
    let mut fields = vec![
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
    ];
    fields.append(&mut extra);
    fields
}

#[cfg(feature = "v1")]
fn adyen_countries() -> Vec<CountryAlpha2> {
    vec![
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
        CountryAlpha2::HU,
        CountryAlpha2::NO,
        CountryAlpha2::PL,
        CountryAlpha2::SE,
        CountryAlpha2::GB,
        CountryAlpha2::CH,
    ]
}
