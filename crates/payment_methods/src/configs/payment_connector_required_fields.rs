use std::collections::{HashMap, HashSet};

use api_models::{
    enums::{self, Connector, FieldType},
    payment_methods::RequiredFieldInfo,
};

use crate::configs::settings::{
    BankRedirectConfig, ConnectorFields, Mandates, RequiredFieldFinal,
    SupportedConnectorsForMandate, SupportedPaymentMethodTypesForMandate,
    SupportedPaymentMethodsForMandate, ZeroMandates,
};
#[cfg(feature = "v1")]
use crate::configs::settings::{PaymentMethodType, RequiredFields};

impl Default for ZeroMandates {
    fn default() -> Self {
        Self {
            supported_payment_methods: SupportedPaymentMethodsForMandate(HashMap::new()),
        }
    }
}

impl Default for Mandates {
    fn default() -> Self {
        Self {
            supported_payment_methods: SupportedPaymentMethodsForMandate(HashMap::from([
                (
                    enums::PaymentMethod::PayLater,
                    SupportedPaymentMethodTypesForMandate(HashMap::from([(
                        enums::PaymentMethodType::Klarna,
                        SupportedConnectorsForMandate {
                            connector_list: HashSet::from([Connector::Adyen]),
                        },
                    )])),
                ),
                (
                    enums::PaymentMethod::Wallet,
                    SupportedPaymentMethodTypesForMandate(HashMap::from([
                        (
                            enums::PaymentMethodType::GooglePay,
                            SupportedConnectorsForMandate {
                                connector_list: HashSet::from([
                                    Connector::Stripe,
                                    Connector::Adyen,
                                    Connector::Globalpay,
                                    Connector::Multisafepay,
                                    Connector::Bankofamerica,
                                    Connector::Novalnet,
                                    Connector::Noon,
                                    Connector::Cybersource,
                                    Connector::Wellsfargo,
                                ]),
                            },
                        ),
                        (
                            enums::PaymentMethodType::ApplePay,
                            SupportedConnectorsForMandate {
                                connector_list: HashSet::from([
                                    Connector::Stripe,
                                    Connector::Adyen,
                                    Connector::Bankofamerica,
                                    Connector::Cybersource,
                                    Connector::Novalnet,
                                    Connector::Wellsfargo,
                                ]),
                            },
                        ),
                        (
                            enums::PaymentMethodType::SamsungPay,
                            SupportedConnectorsForMandate {
                                connector_list: HashSet::from([Connector::Cybersource]),
                            },
                        ),
                    ])),
                ),
                (
                    enums::PaymentMethod::Card,
                    SupportedPaymentMethodTypesForMandate(HashMap::from([
                        (
                            enums::PaymentMethodType::Credit,
                            SupportedConnectorsForMandate {
                                connector_list: HashSet::from([
                                    Connector::Aci,
                                    Connector::Adyen,
                                    Connector::Authorizedotnet,
                                    Connector::Globalpay,
                                    Connector::Worldpay,
                                    Connector::Fiuu,
                                    Connector::Multisafepay,
                                    Connector::Nexinets,
                                    Connector::Noon,
                                    Connector::Novalnet,
                                    Connector::Payme,
                                    Connector::Stripe,
                                    Connector::Bankofamerica,
                                    Connector::Cybersource,
                                    Connector::Wellsfargo,
                                ]),
                            },
                        ),
                        (
                            enums::PaymentMethodType::Debit,
                            SupportedConnectorsForMandate {
                                connector_list: HashSet::from([
                                    Connector::Aci,
                                    Connector::Adyen,
                                    Connector::Authorizedotnet,
                                    Connector::Globalpay,
                                    Connector::Worldpay,
                                    Connector::Fiuu,
                                    Connector::Multisafepay,
                                    Connector::Nexinets,
                                    Connector::Noon,
                                    Connector::Novalnet,
                                    Connector::Payme,
                                    Connector::Stripe,
                                ]),
                            },
                        ),
                    ])),
                ),
            ])),
            update_mandate_supported: SupportedPaymentMethodsForMandate(HashMap::default()),
        }
    }
}

#[derive(Clone, serde::Serialize)]
#[cfg_attr(feature = "v2", allow(dead_code))] // multiple variants are never constructed for v2
enum RequiredField {
    CardNumber,
    CardExpMonth,
    CardExpYear,
    CardCvc,
    CardNetwork,
    BillingUserFirstName,
    BillingUserLastName,
    /// display name and field type for billing first name
    BillingFirstName(&'static str, FieldType),
    /// display name and field type for billing last name
    BillingLastName(&'static str, FieldType),
    BillingEmail,
    Email,
    BillingPhone,
    BillingPhoneCountryCode,
    BillingAddressLine1,
    BillingAddressLine2,
    BillingAddressCity,
    BillingAddressState,
    BillingAddressZip,
    BillingCountries(Vec<&'static str>),
    BillingAddressCountries(Vec<&'static str>),
    ShippingFirstName,
    ShippingLastName,
    ShippingAddressCity,
    ShippingAddressState,
    ShippingAddressZip,
    ShippingCountries(Vec<&'static str>),
    ShippingAddressCountries(Vec<&'static str>),
    ShippingAddressLine1,
    ShippingAddressLine2,
    ShippingPhone,
    ShippingPhoneCountryCode,
    ShippingEmail,
    OpenBankingUkIssuer,
    OpenBankingCzechRepublicIssuer,
    OpenBankingPolandIssuer,
    OpenBankingSlovakiaIssuer,
    OpenBankingFpxIssuer,
    OpenBankingThailandIssuer,
    BanContactCardNumber,
    BanContactCardExpMonth,
    BanContactCardExpYear,
    IdealBankName,
    EpsBankName,
    EpsBankOptions(HashSet<enums::BankNames>),
    BlikCode,
    MifinityDateOfBirth,
    MifinityLanguagePreference(Vec<&'static str>),
    CryptoNetwork,
    CyptoPayCurrency(Vec<&'static str>),
    BoletoSocialSecurityNumber,
    UpiCollectVpaId,
    AchBankDebitAccountNumber,
    AchBankDebitRoutingNumber,
    AchBankDebitBankType(Vec<enums::BankType>),
    AchBankDebitBankAccountHolderName,
    SepaBankDebitIban,
    BacsBankDebitAccountNumber,
    BacsBankDebitSortCode,
    BecsBankDebitAccountNumber,
    BecsBankDebitBsbNumber,
    BecsBankDebitSortCode,
    PixKey,
    PixCnpj,
    PixCpf,
    PixSourceBankAccountId,
    GiftCardNumber,
    GiftCardCvc,
    DcbMsisdn,
    DcbClientUid,
    OrderDetailsProductName,
    Description,
}

impl RequiredField {
    fn to_tuple(&self) -> (String, RequiredFieldInfo) {
        match self {
            Self::CardNumber => (
                "payment_method_data.card.card_number".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.card.card_number".to_string(),
                    display_name: "card_number".to_string(),
                    field_type: FieldType::UserCardNumber,
                    value: None,
                },
            ),
            Self::CardExpMonth => (
                "payment_method_data.card.card_exp_month".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                    display_name: "card_exp_month".to_string(),
                    field_type: FieldType::UserCardExpiryMonth,
                    value: None,
                },
            ),
            Self::CardExpYear => (
                "payment_method_data.card.card_exp_year".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                    display_name: "card_exp_year".to_string(),
                    field_type: FieldType::UserCardExpiryYear,
                    value: None,
                },
            ),
            Self::CardCvc => (
                "payment_method_data.card.card_cvc".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.card.card_cvc".to_string(),
                    display_name: "card_cvc".to_string(),
                    field_type: FieldType::UserCardCvc,
                    value: None,
                },
            ),
            Self::CardNetwork => (
                "payment_method_data.card.card_network".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.card.card_network".to_string(),
                    display_name: "card_network".to_string(),
                    field_type: FieldType::UserCardNetwork,
                    value: None,
                },
            ),
            Self::BillingUserFirstName => (
                "billing.address.first_name".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                    display_name: "card_holder_name".to_string(),
                    field_type: FieldType::UserFullName,
                    value: None,
                },
            ),
            Self::BillingUserLastName => (
                "billing.address.last_name".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                    display_name: "card_holder_name".to_string(),
                    field_type: FieldType::UserFullName,
                    value: None,
                },
            ),
            Self::BillingFirstName(display_name, field_type) => (
                "billing.address.first_name".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                    display_name: display_name.to_string(),
                    field_type: field_type.clone(),
                    value: None,
                },
            ),
            Self::BillingLastName(display_name, field_type) => (
                "billing.address.last_name".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                    display_name: display_name.to_string(),
                    field_type: field_type.clone(),
                    value: None,
                },
            ),
            Self::Email => (
                "email".to_string(),
                RequiredFieldInfo {
                    required_field: "email".to_string(),
                    display_name: "email".to_string(),
                    field_type: FieldType::UserEmailAddress,
                    value: None,
                },
            ),
            Self::BillingEmail => (
                "billing.email".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.email".to_string(),
                    display_name: "email".to_string(),
                    field_type: FieldType::UserEmailAddress,
                    value: None,
                },
            ),
            Self::BillingPhone => (
                "billing.phone.number".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.phone.number".to_string(),
                    display_name: "phone".to_string(),
                    field_type: FieldType::UserPhoneNumber,
                    value: None,
                },
            ),
            Self::BillingPhoneCountryCode => (
                "billing.phone.country_code".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.phone.country_code".to_string(),
                    display_name: "dialing_code".to_string(),
                    field_type: FieldType::UserPhoneNumberCountryCode,
                    value: None,
                },
            ),
            Self::BillingAddressLine1 => (
                "billing.address.line1".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.address.line1".to_string(),
                    display_name: "line1".to_string(),
                    field_type: FieldType::UserAddressLine1,
                    value: None,
                },
            ),
            Self::BillingAddressLine2 => (
                "billing.address.line2".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.address.line2".to_string(),
                    display_name: "line2".to_string(),
                    field_type: FieldType::UserAddressLine2,
                    value: None,
                },
            ),
            Self::BillingAddressCity => (
                "billing.address.city".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.address.city".to_string(),
                    display_name: "city".to_string(),
                    field_type: FieldType::UserAddressCity,
                    value: None,
                },
            ),
            Self::BillingAddressState => (
                "billing.address.state".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.address.state".to_string(),
                    display_name: "state".to_string(),
                    field_type: FieldType::UserAddressState,
                    value: None,
                },
            ),
            Self::BillingAddressZip => (
                "billing.address.zip".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.address.zip".to_string(),
                    display_name: "zip".to_string(),
                    field_type: FieldType::UserAddressPincode,
                    value: None,
                },
            ),
            Self::BillingCountries(countries) => (
                "billing.address.country".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.address.country".to_string(),
                    display_name: "country".to_string(),
                    field_type: FieldType::UserCountry {
                        options: countries.iter().map(|c| c.to_string()).collect(),
                    },
                    value: None,
                },
            ),
            Self::BillingAddressCountries(countries) => (
                "billing.address.country".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.billing.address.country".to_string(),
                    display_name: "country".to_string(),
                    field_type: FieldType::UserAddressCountry {
                        options: countries.iter().map(|c| c.to_string()).collect(),
                    },
                    value: None,
                },
            ),
            Self::ShippingFirstName => (
                "shipping.address.first_name".to_string(),
                RequiredFieldInfo {
                    required_field: "shipping.address.first_name".to_string(),
                    display_name: "shipping_first_name".to_string(),
                    field_type: FieldType::UserShippingName,
                    value: None,
                },
            ),
            Self::ShippingLastName => (
                "shipping.address.last_name".to_string(),
                RequiredFieldInfo {
                    required_field: "shipping.address.last_name".to_string(),
                    display_name: "shipping_last_name".to_string(),
                    field_type: FieldType::UserShippingName,
                    value: None,
                },
            ),
            Self::ShippingAddressCity => (
                "shipping.address.city".to_string(),
                RequiredFieldInfo {
                    required_field: "shipping.address.city".to_string(),
                    display_name: "city".to_string(),
                    field_type: FieldType::UserShippingAddressCity,
                    value: None,
                },
            ),
            Self::ShippingAddressState => (
                "shipping.address.state".to_string(),
                RequiredFieldInfo {
                    required_field: "shipping.address.state".to_string(),
                    display_name: "state".to_string(),
                    field_type: FieldType::UserShippingAddressState,
                    value: None,
                },
            ),
            Self::ShippingAddressZip => (
                "shipping.address.zip".to_string(),
                RequiredFieldInfo {
                    required_field: "shipping.address.zip".to_string(),
                    display_name: "zip".to_string(),
                    field_type: FieldType::UserShippingAddressPincode,
                    value: None,
                },
            ),
            Self::ShippingCountries(countries) => (
                "shipping.address.country".to_string(),
                RequiredFieldInfo {
                    required_field: "shipping.address.country".to_string(),
                    display_name: "country".to_string(),
                    field_type: FieldType::UserCountry {
                        options: countries.iter().map(|c| c.to_string()).collect(),
                    },
                    value: None,
                },
            ),
            Self::ShippingAddressCountries(countries) => (
                "shipping.address.country".to_string(),
                RequiredFieldInfo {
                    required_field: "shipping.address.country".to_string(),
                    display_name: "country".to_string(),
                    field_type: FieldType::UserShippingAddressCountry {
                        options: countries.iter().map(|c| c.to_string()).collect(),
                    },
                    value: None,
                },
            ),
            Self::ShippingAddressLine1 => (
                "shipping.address.line1".to_string(),
                RequiredFieldInfo {
                    required_field: "shipping.address.line1".to_string(),
                    display_name: "line1".to_string(),
                    field_type: FieldType::UserShippingAddressLine1,
                    value: None,
                },
            ),
            Self::ShippingAddressLine2 => (
                "shipping.address.line2".to_string(),
                RequiredFieldInfo {
                    required_field: "shipping.address.line2".to_string(),
                    display_name: "line2".to_string(),
                    field_type: FieldType::UserShippingAddressLine2,
                    value: None,
                },
            ),
            Self::ShippingPhone => (
                "shipping.phone.number".to_string(),
                RequiredFieldInfo {
                    required_field: "shipping.phone.number".to_string(),
                    display_name: "phone_number".to_string(),
                    field_type: FieldType::UserPhoneNumber,
                    value: None,
                },
            ),
            Self::ShippingPhoneCountryCode => (
                "shipping.phone.country_code".to_string(),
                RequiredFieldInfo {
                    required_field: "shipping.phone.country_code".to_string(),
                    display_name: "dialing_code".to_string(),
                    field_type: FieldType::UserPhoneNumberCountryCode,
                    value: None,
                },
            ),
            Self::ShippingEmail => (
                "shipping.email".to_string(),
                RequiredFieldInfo {
                    required_field: "shipping.email".to_string(),
                    display_name: "email".to_string(),
                    field_type: FieldType::UserEmailAddress,
                    value: None,
                },
            ),
            Self::OpenBankingUkIssuer => (
                "payment_method_data.bank_redirect.open_banking_uk.issuer".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_redirect.open_banking_uk.issuer"
                        .to_string(),
                    display_name: "issuer".to_string(),
                    field_type: FieldType::UserBank,
                    value: None,
                },
            ),
            Self::OpenBankingCzechRepublicIssuer => (
                "payment_method_data.bank_redirect.open_banking_czech_republic.issuer".to_string(),
                RequiredFieldInfo {
                    required_field:
                        "payment_method_data.bank_redirect.open_banking_czech_republic.issuer"
                            .to_string(),
                    display_name: "issuer".to_string(),
                    field_type: FieldType::UserBank,
                    value: None,
                },
            ),
            Self::OpenBankingPolandIssuer => (
                "payment_method_data.bank_redirect.online_banking_poland.issuer".to_string(),
                RequiredFieldInfo {
                    required_field:
                        "payment_method_data.bank_redirect.online_banking_poland.issuer".to_string(),
                    display_name: "issuer".to_string(),
                    field_type: FieldType::UserBank,
                    value: None,
                },
            ),
            Self::OpenBankingSlovakiaIssuer => (
                "payment_method_data.bank_redirect.open_banking_slovakia.issuer".to_string(),
                RequiredFieldInfo {
                    required_field:
                        "payment_method_data.bank_redirect.open_banking_slovakia.issuer".to_string(),
                    display_name: "issuer".to_string(),
                    field_type: FieldType::UserBank,
                    value: None,
                },
            ),
            Self::OpenBankingFpxIssuer => (
                "payment_method_data.bank_redirect.open_banking_fpx.issuer".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_redirect.open_banking_fpx.issuer"
                        .to_string(),
                    display_name: "issuer".to_string(),
                    field_type: FieldType::UserBank,
                    value: None,
                },
            ),
            Self::OpenBankingThailandIssuer => (
                "payment_method_data.bank_redirect.open_banking_thailand.issuer".to_string(),
                RequiredFieldInfo {
                    required_field:
                        "payment_method_data.bank_redirect.open_banking_thailand.issuer".to_string(),
                    display_name: "issuer".to_string(),
                    field_type: FieldType::UserBank,
                    value: None,
                },
            ),
            Self::BanContactCardNumber => (
                "payment_method_data.bank_redirect.bancontact_card.card_number".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_redirect.bancontact_card.card_number"
                        .to_string(),
                    display_name: "card_number".to_string(),
                    field_type: FieldType::UserCardNumber,
                    value: None,
                },
            ),
            Self::BanContactCardExpMonth => (
                "payment_method_data.bank_redirect.bancontact_card.card_exp_month".to_string(),
                RequiredFieldInfo {
                    required_field:
                        "payment_method_data.bank_redirect.bancontact_card.card_exp_month"
                            .to_string(),
                    display_name: "card_exp_month".to_string(),
                    field_type: FieldType::UserCardExpiryMonth,
                    value: None,
                },
            ),
            Self::BanContactCardExpYear => (
                "payment_method_data.bank_redirect.bancontact_card.card_exp_year".to_string(),
                RequiredFieldInfo {
                    required_field:
                        "payment_method_data.bank_redirect.bancontact_card.card_exp_year"
                            .to_string(),
                    display_name: "card_exp_year".to_string(),
                    field_type: FieldType::UserCardExpiryYear,
                    value: None,
                },
            ),
            Self::IdealBankName => (
                "payment_method_data.bank_redirect.ideal.bank_name".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_redirect.ideal.bank_name".to_string(),
                    display_name: "bank_name".to_string(),
                    field_type: FieldType::UserBank,
                    value: None,
                },
            ),
            Self::EpsBankName => (
                "payment_method_data.bank_redirect.eps.bank_name".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_redirect.eps.bank_name".to_string(),
                    display_name: "bank_name".to_string(),
                    field_type: FieldType::UserBank,
                    value: None,
                },
            ),
            Self::EpsBankOptions(bank) => (
                "payment_method_data.bank_redirect.eps.bank_name".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_redirect.eps.bank_name".to_string(),
                    display_name: "bank_name".to_string(),
                    field_type: FieldType::UserBankOptions {
                        options: bank.iter().map(|bank| bank.to_string()).collect(),
                    },
                    value: None,
                },
            ),
            Self::BlikCode => (
                "payment_method_data.bank_redirect.blik.blik_code".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_redirect.blik.blik_code".to_string(),
                    display_name: "blik_code".to_string(),
                    field_type: FieldType::UserBlikCode,
                    value: None,
                },
            ),
            Self::MifinityDateOfBirth => (
                "payment_method_data.wallet.mifinity.date_of_birth".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.wallet.mifinity.date_of_birth".to_string(),
                    display_name: "date_of_birth".to_string(),
                    field_type: FieldType::UserDateOfBirth,
                    value: None,
                },
            ),
            Self::MifinityLanguagePreference(languages) => (
                "payment_method_data.wallet.mifinity.language_preference".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.wallet.mifinity.language_preference"
                        .to_string(),
                    display_name: "language_preference".to_string(),
                    field_type: FieldType::LanguagePreference {
                        options: languages.iter().map(|l| l.to_string()).collect(),
                    },
                    value: None,
                },
            ),
            Self::CryptoNetwork => (
                "payment_method_data.crypto.network".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.crypto.network".to_string(),
                    display_name: "network".to_string(),
                    field_type: FieldType::UserCryptoCurrencyNetwork,
                    value: None,
                },
            ),
            Self::CyptoPayCurrency(currencies) => (
                "payment_method_data.crypto.pay_currency".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.crypto.pay_currency".to_string(),
                    display_name: "currency".to_string(),
                    field_type: FieldType::UserCurrency {
                        options: currencies.iter().map(|c| c.to_string()).collect(),
                    },
                    value: None,
                },
            ),
            Self::BoletoSocialSecurityNumber => (
                "payment_method_data.voucher.boleto.social_security_number".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.voucher.boleto.social_security_number"
                        .to_string(),
                    display_name: "social_security_number".to_string(),
                    field_type: FieldType::UserSocialSecurityNumber,
                    value: None,
                },
            ),
            Self::UpiCollectVpaId => (
                "payment_method_data.upi.upi_collect.vpa_id".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.upi.upi_collect.vpa_id".to_string(),
                    display_name: "vpa_id".to_string(),
                    field_type: FieldType::UserVpaId,
                    value: None,
                },
            ),
            Self::AchBankDebitAccountNumber => (
                "payment_method_data.bank_debit.ach_bank_debit.account_number".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_debit.ach_bank_debit.account_number"
                        .to_string(),
                    display_name: "bank_account_number".to_string(),
                    field_type: FieldType::UserBankAccountNumber,
                    value: None,
                },
            ),
            Self::AchBankDebitRoutingNumber => (
                "payment_method_data.bank_debit.ach_bank_debit.routing_number".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_debit.ach_bank_debit.routing_number"
                        .to_string(),
                    display_name: "bank_routing_number".to_string(),
                    field_type: FieldType::UserBankRoutingNumber,
                    value: None,
                },
            ),
            Self::AchBankDebitBankType(bank_type) => (
                "payment_method_data.bank_debit.ach_bank_debit.bank_type".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_debit.ach_bank_debit.bank_type"
                        .to_string(),
                    display_name: "bank_type".to_string(),
                    field_type: FieldType::UserBankType {
                        options: bank_type.iter().map(|bt| bt.to_string()).collect(),
                    },
                    value: None,
                },
            ),
            Self::AchBankDebitBankAccountHolderName => (
                "payment_method_data.bank_debit.ach_bank_debit.bank_account_holder_name"
                    .to_string(),
                RequiredFieldInfo {
                    required_field:
                        "payment_method_data.bank_debit.ach_bank_debit.bank_account_holder_name"
                            .to_string(),
                    display_name: "bank_account_holder_name".to_string(),
                    field_type: FieldType::UserBankAccountHolderName,
                    value: None,
                },
            ),
            Self::SepaBankDebitIban => (
                "payment_method_data.bank_debit.sepa_bank_debit.iban".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_debit.sepa_bank_debit.iban"
                        .to_string(),
                    display_name: "iban".to_string(),
                    field_type: FieldType::UserIban,
                    value: None,
                },
            ),
            Self::BacsBankDebitAccountNumber => (
                "payment_method_data.bank_debit.bacs_bank_debit.account_number".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_debit.bacs_bank_debit.account_number"
                        .to_string(),
                    display_name: "bank_account_number".to_string(),
                    field_type: FieldType::UserBankAccountNumber,
                    value: None,
                },
            ),
            Self::BacsBankDebitSortCode => (
                "payment_method_data.bank_debit.bacs_bank_debit.sort_code".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_debit.bacs_bank_debit.sort_code"
                        .to_string(),
                    display_name: "bank_sort_code".to_string(),
                    field_type: FieldType::UserBankSortCode,
                    value: None,
                },
            ),
            Self::BecsBankDebitAccountNumber => (
                "payment_method_data.bank_debit.becs_bank_debit.account_number".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_debit.becs_bank_debit.account_number"
                        .to_string(),
                    display_name: "bank_account_number".to_string(),
                    field_type: FieldType::UserBankAccountNumber,
                    value: None,
                },
            ),
            Self::BecsBankDebitBsbNumber => (
                "payment_method_data.bank_debit.becs_bank_debit.bsb_number".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_debit.becs_bank_debit.bsb_number"
                        .to_string(),
                    display_name: "bsb_number".to_string(),
                    field_type: FieldType::UserBsbNumber,
                    value: None,
                },
            ),
            Self::BecsBankDebitSortCode => (
                "payment_method_data.bank_debit.becs_bank_debit.sort_code".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_debit.becs_bank_debit.sort_code"
                        .to_string(),
                    display_name: "bank_sort_code".to_string(),
                    field_type: FieldType::UserBankSortCode,
                    value: None,
                },
            ),
            Self::PixKey => (
                "payment_method_data.bank_transfer.pix.pix_key".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_transfer.pix.pix_key".to_string(),
                    display_name: "pix_key".to_string(),
                    field_type: FieldType::UserPixKey,
                    value: None,
                },
            ),
            Self::PixCnpj => (
                "payment_method_data.bank_transfer.pix.cnpj".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_transfer.pix.cnpj".to_string(),
                    display_name: "cnpj".to_string(),
                    field_type: FieldType::UserCnpj,
                    value: None,
                },
            ),
            Self::PixCpf => (
                "payment_method_data.bank_transfer.pix.cpf".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_transfer.pix.cpf".to_string(),
                    display_name: "cpf".to_string(),
                    field_type: FieldType::UserCpf,
                    value: None,
                },
            ),
            Self::PixSourceBankAccountId => (
                "payment_method_data.bank_transfer.pix.source_bank_account_id".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.bank_transfer.pix.source_bank_account_id"
                        .to_string(),
                    display_name: "source_bank_account_id".to_string(),
                    field_type: FieldType::UserSourceBankAccountId,
                    value: None,
                },
            ),
            Self::GiftCardNumber => (
                "payment_method_data.gift_card.number".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.gift_card.givex.number".to_string(),
                    display_name: "gift_card_number".to_string(),
                    field_type: FieldType::UserCardNumber,
                    value: None,
                },
            ),
            Self::GiftCardCvc => (
                "payment_method_data.gift_card.cvc".to_string(),
                RequiredFieldInfo {
                    required_field: "payment_method_data.gift_card.givex.cvc".to_string(),
                    display_name: "gift_card_cvc".to_string(),
                    field_type: FieldType::UserCardCvc,
                    value: None,
                },
            ),
            Self::DcbMsisdn => (
                "payment_method_data.mobile_payment.direct_carrier_billing.msisdn".to_string(),
                RequiredFieldInfo {
                    required_field:
                        "payment_method_data.mobile_payment.direct_carrier_billing.msisdn"
                            .to_string(),
                    display_name: "mobile_number".to_string(),
                    field_type: FieldType::UserMsisdn,
                    value: None,
                },
            ),
            Self::DcbClientUid => (
                "payment_method_data.mobile_payment.direct_carrier_billing.client_uid".to_string(),
                RequiredFieldInfo {
                    required_field:
                        "payment_method_data.mobile_payment.direct_carrier_billing.client_uid"
                            .to_string(),
                    display_name: "client_identifier".to_string(),
                    field_type: FieldType::UserClientIdentifier,
                    value: None,
                },
            ),
            Self::OrderDetailsProductName => (
                "order_details.0.product_name".to_string(),
                RequiredFieldInfo {
                    required_field: "order_details.0.product_name".to_string(),
                    display_name: "product_name".to_string(),
                    field_type: FieldType::OrderDetailsProductName,
                    value: None,
                },
            ),
            Self::Description => (
                "description".to_string(),
                RequiredFieldInfo {
                    required_field: "description".to_string(),
                    display_name: "description".to_string(),
                    field_type: FieldType::Text,
                    value: None,
                },
            ),
        }
    }
}

// Define helper functions for common field groups
#[cfg_attr(feature = "v2", allow(dead_code))] // This function is not used in v2
fn card_basic() -> Vec<RequiredField> {
    vec![
        RequiredField::CardNumber,
        RequiredField::CardExpMonth,
        RequiredField::CardExpYear,
        RequiredField::CardCvc,
    ]
}

#[cfg_attr(feature = "v2", allow(dead_code))] // This function is not used in v2
fn full_name() -> Vec<RequiredField> {
    vec![
        RequiredField::BillingUserFirstName,
        RequiredField::BillingUserLastName,
    ]
}

#[cfg_attr(feature = "v2", allow(dead_code))] // This function is not used in v2
fn billing_name() -> Vec<RequiredField> {
    vec![
        RequiredField::BillingFirstName("billing_first_name", FieldType::UserBillingName),
        RequiredField::BillingLastName("billing_last_name", FieldType::UserBillingName),
    ]
}

#[cfg_attr(feature = "v2", allow(dead_code))] // This function is not used in v2
fn billing_email_billing_name() -> Vec<RequiredField> {
    vec![
        RequiredField::BillingEmail,
        RequiredField::BillingFirstName("billing_first_name", FieldType::UserBillingName),
        RequiredField::BillingLastName("billing_last_name", FieldType::UserBillingName),
    ]
}

#[cfg_attr(feature = "v2", allow(dead_code))] // This function is not used in v2
fn billing_email_billing_name_phone() -> Vec<RequiredField> {
    vec![
        RequiredField::BillingFirstName("billing_first_name", FieldType::UserBillingName),
        RequiredField::BillingLastName("billing_last_name", FieldType::UserBillingName),
        RequiredField::BillingEmail,
        RequiredField::BillingPhone,
        RequiredField::BillingPhoneCountryCode,
    ]
}

#[cfg_attr(feature = "v2", allow(dead_code))] // This function is not used in v2
fn email() -> Vec<RequiredField> {
    [RequiredField::Email].to_vec()
}

#[cfg_attr(feature = "v2", allow(dead_code))] // This function is not used in v2
fn billing_email() -> Vec<RequiredField> {
    [RequiredField::BillingEmail].to_vec()
}

#[cfg_attr(feature = "v2", allow(dead_code))] // This function is not used in v2
fn card_with_name() -> Vec<RequiredField> {
    [card_basic(), full_name()].concat()
}

#[cfg_attr(feature = "v2", allow(dead_code))] // This function is not used in v2
fn billing_email_name() -> Vec<RequiredField> {
    vec![
        RequiredField::BillingEmail,
        RequiredField::BillingUserFirstName,
        RequiredField::BillingUserLastName,
    ]
}

#[cfg_attr(feature = "v2", allow(dead_code))] // This function is not used in v2
fn billing_address() -> Vec<RequiredField> {
    vec![
        RequiredField::BillingAddressCity,
        RequiredField::BillingAddressState,
        RequiredField::BillingAddressZip,
        RequiredField::BillingAddressCountries(vec!["ALL"]),
        RequiredField::BillingAddressLine1,
    ]
}

/// Define the mandate, non-mandate, and common required fields for a connector
/// Eg: fields(vec![RequiredField::CardNumber], vec![RequiredField::BillingEmail], vec![RequiredField::BillingAddressCity])
#[cfg(feature = "v1")]
fn fields(
    mandate: Vec<RequiredField>,
    non_mandate: Vec<RequiredField>,
    common: Vec<RequiredField>,
) -> RequiredFieldFinal {
    let mandate_fields: HashMap<_, _> = mandate.iter().map(|f| f.to_tuple()).collect();
    let non_mandate_fields: HashMap<_, _> = non_mandate.iter().map(|f| f.to_tuple()).collect();
    let common_fields: HashMap<_, _> = common.iter().map(|f| f.to_tuple()).collect();
    RequiredFieldFinal {
        mandate: mandate_fields,
        non_mandate: non_mandate_fields,
        common: common_fields,
    }
}

#[cfg_attr(feature = "v2", allow(dead_code))] // This function is not used in v2
fn connectors(connectors: Vec<(Connector, RequiredFieldFinal)>) -> ConnectorFields {
    ConnectorFields {
        fields: connectors.into_iter().collect(),
    }
}

pub fn get_billing_required_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([
        RequiredField::BillingFirstName("billing_first_name", FieldType::UserBillingName)
            .to_tuple(),
        RequiredField::BillingLastName("billing_last_name", FieldType::UserBillingName).to_tuple(),
        RequiredField::BillingAddressCity.to_tuple(),
        RequiredField::BillingAddressState.to_tuple(),
        RequiredField::BillingAddressZip.to_tuple(),
        RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
        RequiredField::BillingAddressLine1.to_tuple(),
        RequiredField::BillingAddressLine2.to_tuple(),
        RequiredField::BillingPhone.to_tuple(),
        RequiredField::BillingPhoneCountryCode.to_tuple(),
        RequiredField::BillingEmail.to_tuple(),
    ])
}

pub fn get_shipping_required_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([
        RequiredField::ShippingFirstName.to_tuple(),
        RequiredField::ShippingLastName.to_tuple(),
        RequiredField::ShippingAddressCity.to_tuple(),
        RequiredField::ShippingAddressState.to_tuple(),
        RequiredField::ShippingAddressZip.to_tuple(),
        RequiredField::ShippingAddressCountries(vec!["ALL"]).to_tuple(),
        RequiredField::ShippingAddressLine1.to_tuple(),
        RequiredField::ShippingPhone.to_tuple(),
        RequiredField::ShippingPhoneCountryCode.to_tuple(),
        RequiredField::ShippingEmail.to_tuple(),
    ])
}

#[cfg(feature = "v1")]
impl RequiredFields {
    pub fn new(bank_config: &BankRedirectConfig) -> Self {
        let cards_required_fields = get_cards_required_fields();
        let mut debit_required_fields = cards_required_fields.clone();
        debit_required_fields.extend(HashMap::from([
            (
                Connector::Bankofamerica,
                fields(
                    vec![],
                    vec![],
                    [card_basic(), email(), full_name(), billing_address()].concat(),
                ),
            ),
            (
                Connector::Getnet,
                fields(
                    vec![],
                    vec![],
                    [card_basic(), vec![RequiredField::CardNetwork]].concat(),
                ),
            ),
        ]));
        Self(HashMap::from([
            (
                enums::PaymentMethod::Card,
                PaymentMethodType(HashMap::from([
                    (
                        enums::PaymentMethodType::Debit,
                        ConnectorFields {
                            fields: cards_required_fields.clone(),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Credit,
                        ConnectorFields {
                            fields: debit_required_fields.clone(),
                        },
                    ),
                ])),
            ),
            (
                enums::PaymentMethod::BankRedirect,
                PaymentMethodType(get_bank_redirect_required_fields(bank_config)),
            ),
            (
                enums::PaymentMethod::Wallet,
                PaymentMethodType(get_wallet_required_fields()),
            ),
            (
                enums::PaymentMethod::PayLater,
                PaymentMethodType(get_pay_later_required_fields()),
            ),
            (
                enums::PaymentMethod::Crypto,
                PaymentMethodType(HashMap::from([(
                    enums::PaymentMethodType::CryptoCurrency,
                    connectors(vec![(
                        Connector::Cryptopay,
                        fields(
                            vec![],
                            vec![
                                RequiredField::CyptoPayCurrency(vec![
                                    "BTC", "LTC", "ETH", "XRP", "XLM", "BCH", "ADA", "SOL", "SHIB",
                                    "TRX", "DOGE", "BNB", "USDT", "USDC", "DAI",
                                ]),
                                RequiredField::CryptoNetwork,
                            ],
                            vec![],
                        ),
                    )]),
                )])),
            ),
            (
                enums::PaymentMethod::Voucher,
                PaymentMethodType(get_voucher_required_fields()),
            ),
            (
                enums::PaymentMethod::Upi,
                PaymentMethodType(HashMap::from([(
                    enums::PaymentMethodType::UpiCollect,
                    connectors(vec![
                        (
                            Connector::Razorpay,
                            fields(
                                vec![],
                                vec![],
                                vec![
                                    RequiredField::UpiCollectVpaId,
                                    RequiredField::BillingEmail,
                                    RequiredField::BillingPhone,
                                    RequiredField::BillingPhoneCountryCode,
                                ],
                            ),
                        ),
                        (
                            Connector::Phonepe,
                            fields(
                                vec![],
                                vec![],
                                vec![
                                    RequiredField::UpiCollectVpaId,
                                    RequiredField::BillingEmail,
                                    RequiredField::BillingPhone,
                                    RequiredField::BillingPhoneCountryCode,
                                ],
                            ),
                        ),
                        (
                            Connector::Paytm,
                            fields(
                                vec![],
                                vec![],
                                vec![
                                    RequiredField::UpiCollectVpaId,
                                    RequiredField::BillingEmail,
                                    RequiredField::BillingPhone,
                                    RequiredField::BillingPhoneCountryCode,
                                ],
                            ),
                        ),
                    ]),
                )])),
            ),
            (
                enums::PaymentMethod::BankDebit,
                PaymentMethodType(get_bank_debit_required_fields()),
            ),
            (
                enums::PaymentMethod::BankTransfer,
                PaymentMethodType(get_bank_transfer_required_fields()),
            ),
            (
                enums::PaymentMethod::GiftCard,
                PaymentMethodType(HashMap::from([
                    (
                        enums::PaymentMethodType::PaySafeCard,
                        connectors(vec![(Connector::Adyen, fields(vec![], vec![], vec![]))]),
                    ),
                    (
                        enums::PaymentMethodType::Givex,
                        connectors(vec![(
                            Connector::Adyen,
                            fields(
                                vec![],
                                vec![RequiredField::GiftCardNumber, RequiredField::GiftCardCvc],
                                vec![],
                            ),
                        )]),
                    ),
                ])),
            ),
            (
                enums::PaymentMethod::CardRedirect,
                PaymentMethodType(HashMap::from([
                    (
                        enums::PaymentMethodType::Benefit,
                        connectors(vec![(
                            Connector::Adyen,
                            fields(
                                vec![],
                                vec![
                                    RequiredField::BillingFirstName(
                                        "first_name",
                                        FieldType::UserFullName,
                                    ),
                                    RequiredField::BillingLastName(
                                        "last_name",
                                        FieldType::UserFullName,
                                    ),
                                    RequiredField::BillingEmail,
                                    RequiredField::BillingPhone,
                                    RequiredField::BillingPhoneCountryCode,
                                ],
                                vec![],
                            ),
                        )]),
                    ),
                    (
                        enums::PaymentMethodType::Knet,
                        connectors(vec![(
                            Connector::Adyen,
                            fields(
                                vec![],
                                vec![
                                    RequiredField::BillingFirstName(
                                        "first_name",
                                        FieldType::UserFullName,
                                    ),
                                    RequiredField::BillingLastName(
                                        "last_name",
                                        FieldType::UserFullName,
                                    ),
                                    RequiredField::BillingEmail,
                                    RequiredField::BillingPhone,
                                    RequiredField::BillingPhoneCountryCode,
                                ],
                                vec![],
                            ),
                        )]),
                    ),
                    (
                        enums::PaymentMethodType::MomoAtm,
                        connectors(vec![(
                            Connector::Adyen,
                            fields(
                                vec![],
                                vec![
                                    RequiredField::BillingEmail,
                                    RequiredField::BillingPhone,
                                    RequiredField::BillingPhoneCountryCode,
                                ],
                                vec![],
                            ),
                        )]),
                    ),
                ])),
            ),
            (
                enums::PaymentMethod::MobilePayment,
                PaymentMethodType(HashMap::from([(
                    enums::PaymentMethodType::DirectCarrierBilling,
                    connectors(vec![(
                        Connector::Digitalvirgo,
                        RequiredFieldFinal {
                            mandate: HashMap::new(),
                            non_mandate: HashMap::new(),
                            common: HashMap::from([
                                RequiredField::DcbMsisdn.to_tuple(),
                                RequiredField::DcbClientUid.to_tuple(),
                                RequiredField::OrderDetailsProductName.to_tuple(),
                            ]),
                        },
                    )]),
                )])),
            ),
        ]))
    }
}

#[cfg(feature = "v1")]
impl Default for RequiredFields {
    fn default() -> Self {
        Self::new(&BankRedirectConfig::default())
    }
}

#[cfg(feature = "v1")]
fn get_cards_required_fields() -> HashMap<Connector, RequiredFieldFinal> {
    HashMap::from([
        (Connector::Aci, fields(vec![], vec![], card_with_name())),
        (Connector::Authipay, fields(vec![], vec![], card_basic())),
        (Connector::Adyen, fields(vec![], vec![], card_with_name())),
        (Connector::Airwallex, fields(vec![], card_basic(), vec![])),
        (
            Connector::Authorizedotnet,
            fields(vec![], vec![], card_basic()),
        ),
        (
            Connector::Bambora,
            fields(vec![], [card_with_name(), billing_email()].concat(), vec![]),
        ),
        (
            Connector::Bankofamerica,
            fields(
                vec![],
                vec![],
                [card_basic(), email(), full_name(), billing_address()].concat(),
            ),
        ),
        (
            Connector::Barclaycard,
            fields(
                vec![],
                vec![],
                [card_basic(), email(), full_name(), billing_address()].concat(),
            ),
        ),
        (Connector::Billwerk, fields(vec![], vec![], card_basic())),
        (
            Connector::Bluesnap,
            fields(
                vec![],
                [card_basic(), email(), full_name()].concat(),
                vec![],
            ),
        ),
        (Connector::Boku, fields(vec![], vec![], card_basic())),
        (Connector::Braintree, fields(vec![], vec![], card_basic())),
        (Connector::Celero, fields(vec![], vec![], card_basic())),
        (Connector::Checkout, fields(vec![], card_basic(), vec![])),
        (
            Connector::Coinbase,
            fields(vec![], vec![RequiredField::BillingUserFirstName], vec![]),
        ),
        (
            Connector::Cybersource,
            fields(
                vec![],
                vec![],
                [card_with_name(), billing_email(), billing_address()].concat(),
            ),
        ),
        (
            Connector::Datatrans,
            fields(vec![], vec![], [billing_email(), card_with_name()].concat()),
        ),
        (
            Connector::Deutschebank,
            fields(
                vec![],
                [
                    card_basic(),
                    email(),
                    billing_address(),
                    vec![
                        RequiredField::BillingFirstName("first_name", FieldType::UserFullName),
                        RequiredField::BillingLastName("last_name", FieldType::UserFullName),
                    ],
                ]
                .concat(),
                vec![],
            ),
        ),
        (
            Connector::Dlocal,
            fields(
                vec![],
                [
                    card_with_name(),
                    vec![RequiredField::BillingAddressCountries(vec!["ALL"])],
                ]
                .concat(),
                vec![],
            ),
        ),
        #[cfg(feature = "dummy_connector")]
        (
            Connector::DummyConnector1,
            fields(vec![], vec![], card_basic()),
        ),
        #[cfg(feature = "dummy_connector")]
        (
            Connector::DummyConnector2,
            fields(vec![], vec![], card_basic()),
        ),
        #[cfg(feature = "dummy_connector")]
        (
            Connector::DummyConnector3,
            fields(vec![], vec![], card_basic()),
        ),
        #[cfg(feature = "dummy_connector")]
        (
            Connector::DummyConnector4,
            fields(vec![], vec![], card_basic()),
        ),
        #[cfg(feature = "dummy_connector")]
        (
            Connector::DummyConnector5,
            fields(vec![], vec![], card_basic()),
        ),
        #[cfg(feature = "dummy_connector")]
        (
            Connector::DummyConnector6,
            fields(vec![], vec![], card_basic()),
        ),
        #[cfg(feature = "dummy_connector")]
        (
            Connector::DummyConnector7,
            fields(vec![], vec![], card_basic()),
        ),
        (
            Connector::Elavon,
            fields(vec![], [card_basic(), billing_email()].concat(), vec![]),
        ),
        (Connector::Fiserv, fields(vec![], card_basic(), vec![])),
        (
            Connector::Fiuu,
            fields(
                vec![
                    RequiredField::BillingEmail,
                    RequiredField::BillingUserFirstName,
                ],
                vec![],
                card_basic(),
            ),
        ),
        (Connector::Forte, fields(vec![], card_with_name(), vec![])),
        (Connector::Globalpay, fields(vec![], vec![], card_basic())),
        (
            Connector::Hipay,
            fields(
                vec![],
                vec![],
                [
                    vec![RequiredField::BillingEmail],
                    billing_address(),
                    card_with_name(),
                ]
                .concat(),
            ),
        ),
        (
            Connector::Helcim,
            fields(
                vec![],
                [
                    card_with_name(),
                    vec![
                        RequiredField::BillingAddressZip,
                        RequiredField::BillingAddressLine1,
                    ],
                ]
                .concat(),
                vec![],
            ),
        ),
        (Connector::Iatapay, fields(vec![], vec![], vec![])),
        (Connector::Mollie, fields(vec![], card_with_name(), vec![])),
        (Connector::Moneris, fields(vec![], card_basic(), vec![])),
        (
            Connector::Multisafepay,
            fields(
                vec![],
                vec![],
                [
                    card_with_name(),
                    vec![
                        RequiredField::BillingAddressLine1,
                        RequiredField::BillingAddressLine2,
                        RequiredField::BillingAddressCity,
                        RequiredField::BillingAddressZip,
                        RequiredField::BillingAddressCountries(vec!["ALL"]),
                    ],
                ]
                .concat(),
            ),
        ),
        (Connector::Nexinets, fields(vec![], vec![], card_basic())),
        (
            Connector::Nexixpay,
            RequiredFieldFinal {
                mandate: HashMap::new(),
                non_mandate: HashMap::new(),
                common: HashMap::from([
                    RequiredField::CardNumber.to_tuple(),
                    RequiredField::CardExpMonth.to_tuple(),
                    RequiredField::CardExpYear.to_tuple(),
                    RequiredField::BillingFirstName("first_name", FieldType::UserFullName)
                        .to_tuple(),
                    RequiredField::BillingLastName("last_name", FieldType::UserFullName).to_tuple(),
                ]),
            },
        ),
        (
            Connector::Nmi,
            fields(
                vec![],
                [card_with_name(), vec![RequiredField::BillingAddressZip]].concat(),
                vec![],
            ),
        ),
        (Connector::Noon, fields(vec![], vec![], card_with_name())),
        (
            Connector::Novalnet,
            fields(
                vec![],
                vec![],
                [
                    vec![
                        RequiredField::BillingFirstName("first_name", FieldType::UserFullName),
                        RequiredField::BillingLastName("last_name", FieldType::UserFullName),
                    ],
                    billing_email(),
                ]
                .concat(),
            ),
        ),
        (
            Connector::Zift,
            fields(vec![], vec![], [card_with_name()].concat()),
        ),
        (
            Connector::Nuvei,
            fields(
                vec![],
                vec![],
                [
                    card_basic(),
                    vec![
                        RequiredField::BillingEmail,
                        RequiredField::BillingCountries(vec!["ALL"]),
                        RequiredField::BillingFirstName("first_name", FieldType::UserFullName),
                        RequiredField::BillingLastName("last_name", FieldType::UserFullName),
                    ],
                ]
                .concat(),
            ),
        ),
        (
            Connector::Paybox,
            fields(
                vec![],
                vec![],
                [
                    email(),
                    card_with_name(),
                    vec![
                        RequiredField::BillingAddressLine1,
                        RequiredField::BillingAddressCity,
                        RequiredField::BillingAddressZip,
                        RequiredField::BillingAddressCountries(vec!["ALL"]),
                    ],
                ]
                .concat(),
            ),
        ),
        (
            Connector::Paysafe,
            fields(
                vec![
                    RequiredField::BillingAddressCountries(vec!["ALL"]),
                    RequiredField::BillingEmail,
                    RequiredField::BillingAddressZip,
                    RequiredField::BillingAddressState,
                ],
                vec![],
                vec![],
            ),
        ),
        (
            Connector::Payload,
            fields(
                vec![],
                vec![],
                [
                    email(),
                    card_with_name(),
                    vec![
                        RequiredField::BillingAddressLine1,
                        RequiredField::BillingAddressCity,
                        RequiredField::BillingAddressZip,
                        RequiredField::BillingAddressState,
                        RequiredField::BillingAddressCountries(vec!["ALL"]),
                    ],
                ]
                .concat(),
            ),
        ),
        (
            Connector::Payme,
            fields(vec![], vec![], [email(), card_with_name()].concat()),
        ),
        (Connector::Paypal, fields(vec![], card_basic(), vec![])),
        (Connector::Payu, fields(vec![], card_basic(), vec![])),
        (
            Connector::Peachpayments,
            fields(vec![], vec![], card_with_name()),
        ),
        (
            Connector::Powertranz,
            fields(vec![], card_with_name(), vec![]),
        ),
        (Connector::Rapyd, fields(vec![], card_with_name(), vec![])),
        (Connector::Redsys, fields(vec![], card_basic(), vec![])),
        (Connector::Shift4, fields(vec![], card_basic(), vec![])),
        (Connector::Silverflow, fields(vec![], vec![], card_basic())),
        (Connector::Square, fields(vec![], vec![], card_basic())),
        (Connector::Stax, fields(vec![], card_with_name(), vec![])),
        (Connector::Stripe, fields(vec![], vec![], card_basic())),
        (
            Connector::Trustpay,
            fields(
                vec![],
                [
                    card_with_name(),
                    vec![
                        RequiredField::BillingAddressLine1,
                        RequiredField::BillingAddressCity,
                        RequiredField::BillingAddressZip,
                        RequiredField::BillingAddressCountries(vec!["ALL"]),
                    ],
                ]
                .concat(),
                vec![],
            ),
        ),
        (
            Connector::Trustpayments,
            fields(vec![], vec![], card_basic()),
        ),
        (
            Connector::Tesouro,
            fields(
                vec![],
                vec![],
                vec![
                    RequiredField::CardNumber,
                    RequiredField::CardExpMonth,
                    RequiredField::CardExpYear,
                    RequiredField::CardCvc,
                ],
            ),
        ),
        (Connector::Tsys, fields(vec![], card_basic(), vec![])),
        (
            Connector::Wellsfargo,
            fields(
                vec![],
                vec![],
                [card_with_name(), email(), billing_address()].concat(),
            ),
        ),
        (
            Connector::Worldline,
            fields(
                vec![],
                [
                    card_basic(),
                    vec![RequiredField::BillingAddressCountries(vec!["ALL"])],
                ]
                .concat(),
                vec![],
            ),
        ),
        (
            Connector::Worldpay,
            fields(
                vec![],
                vec![],
                vec![
                    RequiredField::CardNumber,
                    RequiredField::CardExpMonth,
                    RequiredField::CardExpYear,
                    RequiredField::BillingUserFirstName,
                ],
            ),
        ),
        (
            Connector::Worldpayxml,
            fields(vec![], card_with_name(), vec![]),
        ),
        (
            Connector::Worldpayvantiv,
            fields(vec![], card_basic(), vec![]),
        ),
        (
            Connector::Xendit,
            fields(
                vec![],
                vec![],
                [
                    card_basic(),
                    vec![
                        RequiredField::BillingEmail,
                        RequiredField::BillingPhone,
                        RequiredField::BillingPhoneCountryCode,
                        RequiredField::BillingUserFirstName,
                        RequiredField::BillingUserLastName,
                        RequiredField::BillingAddressCountries(vec!["ID,PH"]),
                    ],
                ]
                .concat(),
            ),
        ),
        (
            Connector::Zen,
            RequiredFieldFinal {
                mandate: HashMap::new(),
                non_mandate: HashMap::from([
                    RequiredField::CardNumber.to_tuple(),
                    RequiredField::CardExpMonth.to_tuple(),
                    RequiredField::CardExpYear.to_tuple(),
                    RequiredField::Email.to_tuple(),
                ]),
                common: HashMap::new(),
            },
        ),
    ])
}

#[cfg(feature = "v1")]
fn get_bank_redirect_required_fields(
    bank_config: &BankRedirectConfig,
) -> HashMap<enums::PaymentMethodType, ConnectorFields> {
    HashMap::from([
        (
            enums::PaymentMethodType::OpenBankingUk,
            connectors(vec![
                (Connector::Volt, fields(vec![], billing_name(), vec![])),
                (
                    Connector::Adyen,
                    fields(vec![], vec![RequiredField::OpenBankingUkIssuer], vec![]),
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Trustly,
            connectors(vec![
                (Connector::Adyen, fields(vec![], vec![], vec![])),
                (
                    Connector::Airwallex,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingUserFirstName.to_tuple(),
                            RequiredField::BillingUserLastName.to_tuple(),
                            RequiredField::BillingAddressCountries(vec![
                                "DE", "DK", "EE", "ES", "FI", "GB", "LV", "LT", "NL", "PL", "PT",
                                "SE", "SK",
                            ])
                            .to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::OnlineBankingCzechRepublic,
            connectors(vec![(
                Connector::Adyen,
                fields(
                    vec![],
                    vec![RequiredField::OpenBankingCzechRepublicIssuer],
                    vec![],
                ),
            )]),
        ),
        (
            enums::PaymentMethodType::OnlineBankingFinland,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], vec![RequiredField::BillingEmail], vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::OnlineBankingPoland,
            connectors(vec![(
                Connector::Adyen,
                fields(
                    vec![],
                    vec![
                        RequiredField::OpenBankingPolandIssuer,
                        RequiredField::BillingEmail,
                    ],
                    vec![],
                ),
            )]),
        ),
        (
            enums::PaymentMethodType::OnlineBankingSlovakia,
            connectors(vec![(
                Connector::Adyen,
                fields(
                    vec![],
                    vec![RequiredField::OpenBankingSlovakiaIssuer],
                    vec![],
                ),
            )]),
        ),
        (
            enums::PaymentMethodType::OnlineBankingFpx,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], vec![RequiredField::OpenBankingFpxIssuer], vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::OnlineBankingThailand,
            connectors(vec![(
                Connector::Adyen,
                fields(
                    vec![],
                    vec![RequiredField::OpenBankingThailandIssuer],
                    vec![],
                ),
            )]),
        ),
        (
            enums::PaymentMethodType::Bizum,
            connectors(vec![(Connector::Adyen, fields(vec![], vec![], vec![]))]),
        ),
        (
            enums::PaymentMethodType::Przelewy24,
            connectors(vec![(
                Connector::Stripe,
                fields(vec![], vec![RequiredField::BillingEmail], vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::BancontactCard,
            connectors(vec![
                (Connector::Mollie, fields(vec![], vec![], vec![])),
                (
                    Connector::Stripe,
                    fields(
                        vec![RequiredField::BillingEmail],
                        vec![],
                        vec![
                            RequiredField::BillingFirstName(
                                "billing_name",
                                FieldType::UserFullName,
                            ),
                            RequiredField::BillingLastName("billing_name", FieldType::UserFullName),
                        ],
                    ),
                ),
                (
                    Connector::Adyen,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BanContactCardNumber.to_tuple(),
                            RequiredField::BanContactCardExpMonth.to_tuple(),
                            RequiredField::BanContactCardExpYear.to_tuple(),
                            RequiredField::BillingUserFirstName.to_tuple(),
                            RequiredField::BillingUserLastName.to_tuple(),
                        ]),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Giropay,
            connectors(vec![
                (
                    Connector::Aci,
                    fields(
                        vec![],
                        vec![RequiredField::BillingCountries(vec!["DE"])],
                        vec![],
                    ),
                ),
                (Connector::Adyen, fields(vec![], vec![], vec![])),
                (
                    Connector::Globalpay,
                    fields(
                        vec![],
                        vec![],
                        vec![RequiredField::BillingAddressCountries(vec!["DE"])],
                    ),
                ),
                (Connector::Mollie, fields(vec![], vec![], vec![])),
                (
                    Connector::Nuvei,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::Email.to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["DE"]).to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
                (
                    Connector::Paypal,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingCountries(vec!["DE"]).to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
                (
                    Connector::Stripe,
                    fields(
                        vec![],
                        vec![
                            RequiredField::BillingFirstName(
                                "billing_name",
                                FieldType::UserBillingName,
                            ),
                            RequiredField::BillingLastName(
                                "billing_name",
                                FieldType::UserBillingName,
                            ),
                        ],
                        vec![],
                    ),
                ),
                (Connector::Shift4, fields(vec![], vec![], vec![])),
                (
                    Connector::Trustpay,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["DE"]).to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Ideal,
            connectors(vec![
                (
                    Connector::Aci,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::IdealBankName.to_tuple(),
                            RequiredField::BillingCountries(vec!["NL"]).to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
                (
                    Connector::Adyen,
                    fields(vec![], vec![], vec![RequiredField::IdealBankName]),
                ),
                (Connector::Globalpay, fields(vec![], vec![], vec![])),
                (Connector::Mollie, fields(vec![], vec![], vec![])),
                (Connector::Nexinets, fields(vec![], vec![], vec![])),
                (Connector::Airwallex, fields(vec![], vec![], vec![])),
                (
                    Connector::Nuvei,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::Email.to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["NL"]).to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
                (
                    Connector::Shift4,
                    fields(
                        vec![],
                        vec![RequiredField::BillingCountries(vec!["NL"])],
                        vec![],
                    ),
                ),
                (
                    Connector::Paypal,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingCountries(vec!["NL"]).to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
                (
                    Connector::Stripe,
                    RequiredFieldFinal {
                        mandate: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_name",
                                FieldType::UserFullName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName("billing_name", FieldType::UserFullName)
                                .to_tuple(),
                            RequiredField::BillingEmail.to_tuple(),
                        ]),
                        non_mandate: HashMap::new(),
                        common: HashMap::new(),
                    },
                ),
                (
                    Connector::Trustpay,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["NL"]).to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Sofort,
            connectors(vec![
                (
                    Connector::Aci,
                    fields(
                        vec![],
                        vec![RequiredField::BillingCountries(vec![
                            "ES", "GB", "SE", "AT", "NL", "DE", "CH", "BE", "FR", "FI", "IT", "PL",
                        ])],
                        vec![],
                    ),
                ),
                (Connector::Adyen, fields(vec![], vec![], vec![])),
                (
                    Connector::Globalpay,
                    fields(
                        vec![],
                        vec![],
                        vec![RequiredField::BillingAddressCountries(vec![
                            "AT", "BE", "DE", "ES", "IT", "NL",
                        ])],
                    ),
                ),
                (Connector::Mollie, fields(vec![], vec![], vec![])),
                (Connector::Nexinets, fields(vec![], vec![], vec![])),
                (
                    Connector::Nuvei,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::Email.to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressCountries(vec![
                                "ES", "GB", "IT", "DE", "FR", "AT", "BE", "NL", "BE", "SK",
                            ])
                            .to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
                (
                    Connector::Paypal,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingCountries(vec![
                                "ES", "GB", "AT", "NL", "DE", "BE",
                            ])
                            .to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
                (Connector::Shift4, fields(vec![], vec![], vec![])),
                (
                    Connector::Stripe,
                    RequiredFieldFinal {
                        mandate: HashMap::from([RequiredField::BillingEmail.to_tuple()]),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingCountries(vec!["ES", "AT", "NL", "DE", "BE"])
                                .to_tuple(),
                            RequiredField::BillingFirstName(
                                "account_holder_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "account_holder_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Trustpay,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec![
                                "ES", "GB", "SE", "AT", "NL", "DE", "CH", "BE", "FR", "FI", "IT",
                                "PL",
                            ])
                            .to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Eps,
            connectors(vec![
                (
                    Connector::Adyen,
                    fields(vec![], vec![], vec![RequiredField::EpsBankName]),
                ),
                (
                    Connector::Stripe,
                    fields(
                        vec![],
                        vec![
                            RequiredField::BillingFirstName(
                                "billing_name",
                                FieldType::UserFullName,
                            ),
                            RequiredField::EpsBankOptions(
                                bank_config
                                    .0
                                    .get(&enums::PaymentMethodType::Eps)
                                    .and_then(|connector_bank_names| {
                                        connector_bank_names.0.get("stripe")
                                    })
                                    .map(|bank_names| bank_names.banks.clone())
                                    .unwrap_or_default(),
                            ),
                            RequiredField::BillingLastName("billing_name", FieldType::UserFullName),
                        ],
                        vec![],
                    ),
                ),
                (
                    Connector::Aci,
                    fields(
                        vec![],
                        vec![RequiredField::BillingCountries(vec!["AT"])],
                        vec![],
                    ),
                ),
                (
                    Connector::Globalpay,
                    fields(
                        vec![],
                        vec![],
                        vec![RequiredField::BillingAddressCountries(vec!["AT"])],
                    ),
                ),
                (Connector::Mollie, fields(vec![], vec![], vec![])),
                (
                    Connector::Paypal,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_name",
                                FieldType::UserFullName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName("billing_name", FieldType::UserFullName)
                                .to_tuple(),
                            RequiredField::BillingCountries(vec!["AT"]).to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
                (
                    Connector::Trustpay,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["AT"]).to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
                (Connector::Shift4, fields(vec![], vec![], vec![])),
                (
                    Connector::Nuvei,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::Email.to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["AT"]).to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Blik,
            connectors(vec![
                (
                    Connector::Adyen,
                    fields(vec![], vec![], vec![RequiredField::BlikCode]),
                ),
                (
                    Connector::Stripe,
                    fields(vec![], vec![], vec![RequiredField::BlikCode]),
                ),
                (
                    Connector::Trustpay,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::Email.to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Airwallex,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingUserFirstName.to_tuple(),
                            RequiredField::BillingUserLastName.to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Interac,
            connectors(vec![
                (
                    Connector::Paysafe,
                    fields(vec![], vec![RequiredField::BillingEmail], vec![]),
                ),
                (
                    Connector::Gigadat,
                    fields(
                        vec![],
                        vec![
                            RequiredField::BillingEmail,
                            RequiredField::BillingUserFirstName,
                            RequiredField::BillingUserLastName,
                            RequiredField::BillingPhone,
                            RequiredField::BillingPhoneCountryCode,
                        ],
                        vec![],
                    ),
                ),
                (
                    Connector::Loonio,
                    fields(
                        vec![],
                        vec![
                            RequiredField::BillingEmail,
                            RequiredField::BillingUserFirstName,
                            RequiredField::BillingUserLastName,
                        ],
                        vec![],
                    ),
                ),
            ]),
        ),
    ])
}

#[cfg(feature = "v1")]
fn get_wallet_required_fields() -> HashMap<enums::PaymentMethodType, ConnectorFields> {
    HashMap::from([
        (
            enums::PaymentMethodType::ApplePay,
            connectors(vec![
                (Connector::Stripe, fields(vec![], vec![], vec![])),
                (Connector::Adyen, fields(vec![], vec![], vec![])),
                (
                    Connector::Nuvei,
                    fields(
                        vec![],
                        vec![],
                        vec![
                            RequiredField::BillingEmail,
                            RequiredField::BillingCountries(vec!["ALL"]),
                            RequiredField::BillingFirstName("first_name", FieldType::UserFullName),
                            RequiredField::BillingLastName("last_name", FieldType::UserFullName),
                        ],
                    ),
                ),
                (
                    Connector::Bankofamerica,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::Email.to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressState.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Cybersource,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingEmail.to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressState.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Novalnet,
                    fields(vec![], vec![], vec![RequiredField::BillingEmail]),
                ),
                (
                    Connector::Paysafe,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Wellsfargo,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::Email.to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressState.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                            RequiredField::ShippingFirstName.to_tuple(),
                            RequiredField::ShippingLastName.to_tuple(),
                            RequiredField::ShippingAddressCity.to_tuple(),
                            RequiredField::ShippingAddressState.to_tuple(),
                            RequiredField::ShippingAddressZip.to_tuple(),
                            RequiredField::ShippingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::ShippingAddressLine1.to_tuple(),
                        ]),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::SamsungPay,
            connectors(vec![(
                Connector::Cybersource,
                RequiredFieldFinal {
                    mandate: HashMap::new(),
                    non_mandate: HashMap::new(),
                    common: HashMap::from([
                        RequiredField::BillingEmail.to_tuple(),
                        RequiredField::BillingFirstName(
                            "billing_first_name",
                            FieldType::UserBillingName,
                        )
                        .to_tuple(),
                        RequiredField::BillingLastName(
                            "billing_last_name",
                            FieldType::UserBillingName,
                        )
                        .to_tuple(),
                        RequiredField::BillingAddressCity.to_tuple(),
                        RequiredField::BillingAddressState.to_tuple(),
                        RequiredField::BillingAddressZip.to_tuple(),
                        RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                        RequiredField::BillingAddressLine1.to_tuple(),
                    ]),
                },
            )]),
        ),
        (
            enums::PaymentMethodType::GooglePay,
            connectors(vec![
                (Connector::Adyen, fields(vec![], vec![], vec![])),
                (
                    Connector::Bankofamerica,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::Email.to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressState.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Barclaycard,
                    fields(vec![], vec![], [full_name(), billing_address()].concat()),
                ),
                (Connector::Bluesnap, fields(vec![], vec![], vec![])),
                (Connector::Noon, fields(vec![], vec![], vec![])),
                (
                    Connector::Novalnet,
                    fields(vec![], vec![], vec![RequiredField::BillingEmail]),
                ),
                (
                    Connector::Nuvei,
                    fields(
                        vec![],
                        vec![],
                        vec![
                            RequiredField::BillingEmail,
                            RequiredField::BillingCountries(vec!["ALL"]),
                            RequiredField::BillingFirstName("first_name", FieldType::UserFullName),
                            RequiredField::BillingLastName("last_name", FieldType::UserFullName),
                        ],
                    ),
                ),
                (Connector::Airwallex, fields(vec![], vec![], vec![])),
                (Connector::Authorizedotnet, fields(vec![], vec![], vec![])),
                (Connector::Checkout, fields(vec![], vec![], vec![])),
                (Connector::Globalpay, fields(vec![], vec![], vec![])),
                (
                    Connector::Multisafepay,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressState.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                            RequiredField::BillingAddressLine2.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Cybersource,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingEmail.to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressState.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                        ]),
                    },
                ),
                (Connector::Payu, fields(vec![], vec![], vec![])),
                (Connector::Rapyd, fields(vec![], vec![], vec![])),
                (Connector::Stripe, fields(vec![], vec![], vec![])),
                (Connector::Trustpay, fields(vec![], vec![], vec![])),
                (
                    Connector::Wellsfargo,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::Email.to_tuple(),
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressState.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                            RequiredField::ShippingFirstName.to_tuple(),
                            RequiredField::ShippingLastName.to_tuple(),
                            RequiredField::ShippingAddressCity.to_tuple(),
                            RequiredField::ShippingAddressState.to_tuple(),
                            RequiredField::ShippingAddressZip.to_tuple(),
                            RequiredField::ShippingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::ShippingAddressLine1.to_tuple(),
                        ]),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::WeChatPay,
            connectors(vec![
                (Connector::Stripe, fields(vec![], vec![], vec![])),
                (Connector::Adyen, fields(vec![], vec![], vec![])),
            ]),
        ),
        (
            enums::PaymentMethodType::AliPay,
            connectors(vec![
                (Connector::Stripe, fields(vec![], vec![], vec![])),
                (Connector::Adyen, fields(vec![], vec![], vec![])),
            ]),
        ),
        (
            enums::PaymentMethodType::AliPayHk,
            connectors(vec![(Connector::Adyen, fields(vec![], vec![], vec![]))]),
        ),
        (
            enums::PaymentMethodType::AmazonPay,
            connectors(vec![
                (Connector::Stripe, fields(vec![], vec![], vec![])),
                (
                    Connector::Amazonpay,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::ShippingFirstName.to_tuple(),
                            RequiredField::ShippingLastName.to_tuple(),
                            RequiredField::ShippingAddressLine1.to_tuple(),
                            RequiredField::ShippingAddressCity.to_tuple(),
                            RequiredField::ShippingAddressState.to_tuple(),
                            RequiredField::ShippingAddressZip.to_tuple(),
                            RequiredField::ShippingPhone.to_tuple(),
                        ]),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Cashapp,
            connectors(vec![(Connector::Stripe, fields(vec![], vec![], vec![]))]),
        ),
        (
            enums::PaymentMethodType::MbWay,
            connectors(vec![(
                Connector::Adyen,
                fields(
                    vec![],
                    vec![
                        RequiredField::BillingPhone,
                        RequiredField::BillingPhoneCountryCode,
                    ],
                    vec![],
                ),
            )]),
        ),
        (
            enums::PaymentMethodType::KakaoPay,
            connectors(vec![(Connector::Adyen, fields(vec![], vec![], vec![]))]),
        ),
        (
            enums::PaymentMethodType::Twint,
            connectors(vec![(Connector::Adyen, fields(vec![], vec![], vec![]))]),
        ),
        (
            enums::PaymentMethodType::Gcash,
            connectors(vec![(Connector::Adyen, fields(vec![], vec![], vec![]))]),
        ),
        (
            enums::PaymentMethodType::Vipps,
            connectors(vec![(Connector::Adyen, fields(vec![], vec![], vec![]))]),
        ),
        (
            enums::PaymentMethodType::Dana,
            connectors(vec![(Connector::Adyen, fields(vec![], vec![], vec![]))]),
        ),
        (
            enums::PaymentMethodType::Momo,
            connectors(vec![(Connector::Adyen, fields(vec![], vec![], vec![]))]),
        ),
        (
            enums::PaymentMethodType::Swish,
            connectors(vec![(Connector::Adyen, fields(vec![], vec![], vec![]))]),
        ),
        (
            enums::PaymentMethodType::TouchNGo,
            connectors(vec![(Connector::Adyen, fields(vec![], vec![], vec![]))]),
        ),
        (
            // Added shipping fields for the SDK flow to accept it from wallet directly,
            // this won't show up in SDK in payment's sheet but will be used in the background
            enums::PaymentMethodType::Paypal,
            connectors(vec![
                (
                    Connector::Adyen,
                    fields(vec![], vec![], vec![RequiredField::BillingEmail]),
                ),
                (Connector::Braintree, fields(vec![], vec![], vec![])),
                (
                    Connector::Novalnet,
                    fields(vec![], vec![], vec![RequiredField::BillingEmail]),
                ),
                (
                    Connector::Paypal,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::ShippingFirstName.to_tuple(),
                            RequiredField::ShippingLastName.to_tuple(),
                            RequiredField::ShippingAddressCity.to_tuple(),
                            RequiredField::ShippingAddressState.to_tuple(),
                            RequiredField::ShippingAddressZip.to_tuple(),
                            RequiredField::ShippingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::ShippingAddressLine1.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Airwallex,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingUserFirstName.to_tuple(),
                            RequiredField::BillingUserLastName.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Mifinity,
            connectors(vec![(
                Connector::Mifinity,
                fields(
                    vec![],
                    vec![],
                    vec![
                        RequiredField::MifinityDateOfBirth,
                        RequiredField::BillingAddressLine1,
                        RequiredField::BillingAddressCity,
                        RequiredField::BillingFirstName("first_name", FieldType::UserFullName),
                        RequiredField::BillingLastName("last_name", FieldType::UserFullName),
                        RequiredField::BillingPhone,
                        RequiredField::BillingPhoneCountryCode,
                        RequiredField::BillingCountries(vec![
                            "BR", "CN", "SG", "MY", "DE", "CH", "DK", "GB", "ES", "AD", "GI", "FI",
                            "FR", "GR", "HR", "IT", "JP", "MX", "AR", "CO", "CL", "PE", "VE", "UY",
                            "PY", "BO", "EC", "GT", "HN", "SV", "NI", "CR", "PA", "DO", "CU", "PR",
                            "NL", "NO", "PL", "PT", "SE", "RU", "TR", "TW", "HK", "MO", "AX", "AL",
                            "DZ", "AS", "AO", "AI", "AG", "AM", "AW", "AU", "AT", "AZ", "BS", "BH",
                            "BD", "BB", "BE", "BZ", "BJ", "BM", "BT", "BQ", "BA", "BW", "IO", "BN",
                            "BG", "BF", "BI", "KH", "CM", "CA", "CV", "KY", "CF", "TD", "CX", "CC",
                            "KM", "CG", "CK", "CI", "CW", "CY", "CZ", "DJ", "DM", "EG", "GQ", "ER",
                            "EE", "ET", "FK", "FO", "FJ", "GF", "PF", "TF", "GA", "GM", "GE", "GH",
                            "GL", "GD", "GP", "GU", "GG", "GN", "GW", "GY", "HT", "HM", "VA", "IS",
                            "IN", "ID", "IE", "IM", "IL", "JE", "JO", "KZ", "KE", "KI", "KW", "KG",
                            "LA", "LV", "LB", "LS", "LI", "LT", "LU", "MK", "MG", "MW", "MV", "ML",
                            "MT", "MH", "MQ", "MR", "MU", "YT", "FM", "MD", "MC", "MN", "ME", "MS",
                            "MA", "MZ", "NA", "NR", "NP", "NC", "NZ", "NE", "NG", "NU", "NF", "MP",
                            "OM", "PK", "PW", "PS", "PG", "PH", "PN", "QA", "RE", "RO", "RW", "BL",
                            "SH", "KN", "LC", "MF", "PM", "VC", "WS", "SM", "ST", "SA", "SN", "RS",
                            "SC", "SL", "SX", "SK", "SI", "SB", "SO", "ZA", "GS", "KR", "LK", "SR",
                            "SJ", "SZ", "TH", "TL", "TG", "TK", "TO", "TT", "TN", "TM", "TC", "TV",
                            "UG", "UA", "AE", "UZ", "VU", "VN", "VG", "VI", "WF", "EH", "ZM",
                        ]),
                        RequiredField::BillingEmail,
                        RequiredField::MifinityLanguagePreference(vec![
                            "BR", "PT_BR", "CN", "ZH_CN", "DE", "DK", "DA", "DA_DK", "EN", "ES",
                            "FI", "FR", "GR", "EL", "EL_GR", "HR", "IT", "JP", "JA", "JA_JP", "LA",
                            "ES_LA", "NL", "NO", "PL", "PT", "RU", "SV", "SE", "SV_SE", "ZH", "TW",
                            "ZH_TW",
                        ]),
                    ],
                ),
            )]),
        ),
        (
            enums::PaymentMethodType::Skrill,
            connectors(vec![
                (
                    Connector::Airwallex,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingUserFirstName.to_tuple(),
                            RequiredField::BillingUserLastName.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::BillingEmail.to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
                (
                    Connector::Paysafe,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::BillingEmail.to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
            ]),
        ),
    ])
}

#[cfg(feature = "v1")]
fn get_pay_later_required_fields() -> HashMap<enums::PaymentMethodType, ConnectorFields> {
    HashMap::from([
        (
            enums::PaymentMethodType::AfterpayClearpay,
            connectors(vec![
                (
                    Connector::Stripe,
                    fields(
                        vec![],
                        vec![
                            RequiredField::BillingEmail,
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            ),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            ),
                            RequiredField::BillingAddressLine1,
                            RequiredField::BillingAddressCity,
                            RequiredField::BillingAddressZip,
                            RequiredField::BillingAddressCountries(vec![
                                "GB", "AU", "CA", "US", "NZ",
                            ]),
                            RequiredField::BillingAddressState,
                            RequiredField::ShippingFirstName,
                            RequiredField::ShippingLastName,
                            RequiredField::ShippingAddressCity,
                            RequiredField::ShippingAddressState,
                            RequiredField::ShippingAddressZip,
                            RequiredField::ShippingAddressCountries(vec!["ALL"]),
                            RequiredField::ShippingAddressLine1,
                        ],
                        vec![],
                    ),
                ),
                (
                    Connector::Adyen,
                    fields(
                        vec![],
                        vec![
                            RequiredField::BillingEmail,
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            ),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            ),
                            RequiredField::BillingAddressLine1,
                            RequiredField::BillingAddressLine2,
                            RequiredField::BillingAddressCity,
                            RequiredField::BillingAddressZip,
                            RequiredField::BillingAddressCountries(vec![
                                "GB", "AU", "CA", "US", "NZ",
                            ]),
                            RequiredField::BillingAddressState,
                            RequiredField::ShippingAddressCity,
                            RequiredField::ShippingAddressZip,
                            RequiredField::ShippingAddressCountries(vec![
                                "GB", "AU", "CA", "US", "NZ",
                            ]),
                            RequiredField::ShippingAddressLine1,
                            RequiredField::ShippingAddressLine2,
                        ],
                        vec![],
                    ),
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Flexiti,
            connectors(vec![(
                Connector::Flexiti,
                RequiredFieldFinal {
                    mandate: HashMap::new(),
                    non_mandate: HashMap::from([
                        RequiredField::BillingUserFirstName.to_tuple(),
                        RequiredField::BillingUserLastName.to_tuple(),
                        RequiredField::BillingAddressCity.to_tuple(),
                        RequiredField::BillingAddressState.to_tuple(),
                        RequiredField::BillingAddressZip.to_tuple(),
                        RequiredField::BillingAddressLine1.to_tuple(),
                        RequiredField::BillingAddressLine2.to_tuple(),
                        RequiredField::BillingAddressState.to_tuple(),
                        RequiredField::ShippingFirstName.to_tuple(),
                        RequiredField::ShippingLastName.to_tuple(),
                        RequiredField::ShippingAddressLine1.to_tuple(),
                        RequiredField::ShippingAddressLine2.to_tuple(),
                        RequiredField::ShippingAddressZip.to_tuple(),
                        RequiredField::ShippingAddressCity.to_tuple(),
                        RequiredField::ShippingAddressState.to_tuple(),
                    ]),
                    common: HashMap::new(),
                },
            )]),
        ),
        (
            enums::PaymentMethodType::Klarna,
            connectors(vec![
                (
                    Connector::Stripe,
                    fields(
                        vec![],
                        vec![
                            RequiredField::BillingAddressCountries(vec![
                                "AU", "AT", "BE", "CA", "CZ", "DK", "FI", "FR", "GR", "DE", "IE",
                                "IT", "NL", "NZ", "NO", "PL", "PT", "RO", "ES", "SE", "CH", "GB",
                                "US",
                            ]),
                            RequiredField::BillingEmail,
                        ],
                        vec![],
                    ),
                ),
                (
                    Connector::Adyen,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingAddressCountries(vec!["ALL"]).to_tuple(),
                            RequiredField::BillingEmail.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Klarna,
                    fields(
                        vec![],
                        vec![],
                        vec![RequiredField::BillingAddressCountries(vec![
                            "AU", "AT", "BE", "CA", "CZ", "DK", "FI", "FR", "DE", "GR", "IE", "IT",
                            "NL", "NZ", "NO", "PL", "PT", "ES", "SE", "CH", "GB", "US",
                        ])],
                    ),
                ),
                (
                    Connector::Airwallex,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([RequiredField::BillingAddressCountries(vec![
                            "AT", "BE", "FI", "FR", "DE", "GR", "IE", "IT", "NL", "PT", "ES", "DK",
                            "NO", "PL", "SE", "CH", "GB", "CZ", "US",
                        ])
                        .to_tuple()]),
                        common: HashMap::new(),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Affirm,
            connectors(vec![
                (Connector::Stripe, fields(vec![], vec![], vec![])),
                (
                    Connector::Adyen,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingUserFirstName.to_tuple(),
                            RequiredField::BillingUserLastName.to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressState.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["US"]).to_tuple(),
                            RequiredField::BillingPhone.to_tuple(),
                            RequiredField::BillingPhoneCountryCode.to_tuple(),
                            RequiredField::BillingEmail.to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                            RequiredField::BillingAddressLine2.to_tuple(),
                            RequiredField::ShippingAddressLine1.to_tuple(),
                            RequiredField::ShippingAddressLine2.to_tuple(),
                            RequiredField::ShippingAddressZip.to_tuple(),
                            RequiredField::ShippingAddressCity.to_tuple(),
                            RequiredField::ShippingCountries(vec!["US"]).to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::PayBright,
            connectors(vec![(
                Connector::Adyen,
                RequiredFieldFinal {
                    mandate: HashMap::new(),
                    non_mandate: HashMap::from([
                        RequiredField::BillingUserFirstName.to_tuple(),
                        RequiredField::BillingUserLastName.to_tuple(),
                        RequiredField::BillingAddressCity.to_tuple(),
                        RequiredField::BillingAddressState.to_tuple(),
                        RequiredField::BillingAddressZip.to_tuple(),
                        RequiredField::BillingAddressCountries(vec!["CA"]).to_tuple(),
                        RequiredField::BillingPhone.to_tuple(),
                        RequiredField::BillingPhoneCountryCode.to_tuple(),
                        RequiredField::BillingEmail.to_tuple(),
                        RequiredField::BillingAddressLine1.to_tuple(),
                        RequiredField::BillingAddressLine2.to_tuple(),
                        RequiredField::ShippingAddressCity.to_tuple(),
                        RequiredField::ShippingAddressZip.to_tuple(),
                        RequiredField::ShippingAddressCountries(vec!["ALL"]).to_tuple(),
                        RequiredField::ShippingAddressLine1.to_tuple(),
                        RequiredField::ShippingAddressLine2.to_tuple(),
                    ]),
                    common: HashMap::new(),
                },
            )]),
        ),
        (
            enums::PaymentMethodType::Walley,
            connectors(vec![(
                Connector::Adyen,
                fields(
                    vec![],
                    vec![
                        RequiredField::BillingPhone,
                        RequiredField::BillingAddressCountries(vec!["DK", "FI", "NO", "SE"]),
                        RequiredField::BillingPhoneCountryCode,
                        RequiredField::BillingEmail,
                    ],
                    vec![],
                ),
            )]),
        ),
        (
            enums::PaymentMethodType::Alma,
            connectors(vec![(
                Connector::Adyen,
                RequiredFieldFinal {
                    mandate: HashMap::new(),
                    non_mandate: HashMap::from([
                        RequiredField::BillingFirstName(
                            "billing_first_name",
                            FieldType::UserBillingName,
                        )
                        .to_tuple(),
                        RequiredField::BillingLastName(
                            "billing_last_name",
                            FieldType::UserBillingName,
                        )
                        .to_tuple(),
                        RequiredField::BillingAddressCity.to_tuple(),
                        RequiredField::BillingAddressState.to_tuple(),
                        RequiredField::BillingAddressZip.to_tuple(),
                        RequiredField::BillingAddressCountries(vec!["FR"]).to_tuple(),
                        RequiredField::BillingPhone.to_tuple(),
                        RequiredField::BillingPhoneCountryCode.to_tuple(),
                        RequiredField::BillingEmail.to_tuple(),
                        RequiredField::BillingAddressLine1.to_tuple(),
                        RequiredField::BillingAddressLine2.to_tuple(),
                    ]),
                    common: HashMap::new(),
                },
            )]),
        ),
        (
            enums::PaymentMethodType::Atome,
            connectors(vec![
                (
                    Connector::Adyen,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingUserFirstName.to_tuple(),
                            RequiredField::BillingUserLastName.to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressState.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["MY", "SG"]).to_tuple(),
                            RequiredField::BillingPhone.to_tuple(),
                            RequiredField::BillingPhoneCountryCode.to_tuple(),
                            RequiredField::BillingEmail.to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                            RequiredField::BillingAddressLine2.to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
                (
                    Connector::Airwallex,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingPhone.to_tuple(),
                            RequiredField::BillingPhoneCountryCode.to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
            ]),
        ),
    ])
}

#[cfg(feature = "v1")]
fn get_voucher_required_fields() -> HashMap<enums::PaymentMethodType, ConnectorFields> {
    HashMap::from([
        (
            enums::PaymentMethodType::Boleto,
            connectors(vec![
                (
                    Connector::Adyen,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BoletoSocialSecurityNumber.to_tuple(),
                            RequiredField::BillingUserFirstName.to_tuple(),
                            RequiredField::BillingUserLastName.to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressState.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["BR"]).to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                            RequiredField::BillingAddressLine2.to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
                (Connector::Zen, fields(vec![], vec![], vec![])),
            ]),
        ),
        (
            enums::PaymentMethodType::Alfamart,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::Indomaret,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::Oxxo,
            connectors(vec![(Connector::Adyen, fields(vec![], vec![], vec![]))]),
        ),
        (
            enums::PaymentMethodType::SevenEleven,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name_phone(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::Lawson,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name_phone(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::MiniStop,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name_phone(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::FamilyMart,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name_phone(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::Seicomart,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name_phone(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::PayEasy,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name_phone(), vec![]),
            )]),
        ),
    ])
}

#[cfg(feature = "v1")]
fn get_bank_debit_required_fields() -> HashMap<enums::PaymentMethodType, ConnectorFields> {
    HashMap::from([
        (
            enums::PaymentMethodType::Ach,
            connectors(vec![
                (
                    Connector::Stripe,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::AchBankDebitAccountNumber.to_tuple(),
                            RequiredField::AchBankDebitRoutingNumber.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Adyen,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingFirstName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::AchBankDebitAccountNumber.to_tuple(),
                            RequiredField::AchBankDebitRoutingNumber.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Dwolla,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::from([
                            RequiredField::BillingFirstName(
                                "first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName("last_name", FieldType::UserBillingName)
                                .to_tuple(),
                            RequiredField::AchBankDebitAccountNumber.to_tuple(),
                            RequiredField::AchBankDebitRoutingNumber.to_tuple(),
                            RequiredField::AchBankDebitBankAccountHolderName.to_tuple(),
                            RequiredField::AchBankDebitBankType(vec![
                                enums::BankType::Checking,
                                enums::BankType::Savings,
                            ])
                            .to_tuple(),
                        ]),
                        common: HashMap::new(),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Sepa,
            connectors(vec![
                (
                    Connector::Stripe,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::SepaBankDebitIban.to_tuple(),
                            RequiredField::BillingEmail.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Adyen,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingFirstName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::SepaBankDebitIban.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Deutschebank,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingFirstName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingEmail.to_tuple(),
                            RequiredField::SepaBankDebitIban.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Inespay,
                    fields(vec![], vec![], vec![RequiredField::SepaBankDebitIban]),
                ),
                (
                    Connector::Nordea,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),

                        non_mandate: HashMap::new(),

                        common: HashMap::from([
                            RequiredField::BillingAddressCountries(vec!["DK,FI,NO,SE"]).to_tuple(),
                            RequiredField::SepaBankDebitIban.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Novalnet,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::SepaBankDebitIban.to_tuple(),
                        ]),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Bacs,
            connectors(vec![
                (
                    Connector::Stripe,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BacsBankDebitAccountNumber.to_tuple(),
                            RequiredField::BacsBankDebitSortCode.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["UK"]).to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Adyen,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingFirstName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BacsBankDebitAccountNumber.to_tuple(),
                            RequiredField::BacsBankDebitSortCode.to_tuple(),
                        ]),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::Becs,
            connectors(vec![
                (
                    Connector::Stripe,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BecsBankDebitAccountNumber.to_tuple(),
                            RequiredField::BecsBankDebitBsbNumber.to_tuple(),
                            RequiredField::BillingEmail.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Adyen,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingFirstName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BillingLastName(
                                "owner_name",
                                FieldType::UserBillingName,
                            )
                            .to_tuple(),
                            RequiredField::BecsBankDebitAccountNumber.to_tuple(),
                            RequiredField::BecsBankDebitSortCode.to_tuple(),
                        ]),
                    },
                ),
            ]),
        ),
    ])
}

#[cfg(feature = "v1")]
fn get_bank_transfer_required_fields() -> HashMap<enums::PaymentMethodType, ConnectorFields> {
    HashMap::from([
        (
            enums::PaymentMethodType::Multibanco,
            connectors(vec![(
                Connector::Stripe,
                fields(vec![], vec![RequiredField::BillingEmail], vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::LocalBankTransfer,
            connectors(vec![(
                Connector::Zsl,
                fields(
                    vec![],
                    vec![
                        RequiredField::BillingAddressCountries(vec!["CN"]),
                        RequiredField::BillingAddressCity,
                    ],
                    vec![],
                ),
            )]),
        ),
        (
            enums::PaymentMethodType::Ach,
            connectors(vec![(
                Connector::Checkbook,
                fields(
                    vec![],
                    vec![],
                    vec![
                        RequiredField::BillingUserFirstName,
                        RequiredField::BillingUserLastName,
                        RequiredField::BillingEmail,
                        RequiredField::Description,
                    ],
                ),
            )]),
        ),
        (
            enums::PaymentMethodType::Pix,
            connectors(vec![
                (
                    Connector::Itaubank,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::PixKey.to_tuple(),
                            RequiredField::PixCnpj.to_tuple(),
                            RequiredField::PixCpf.to_tuple(),
                            RequiredField::BillingUserFirstName.to_tuple(),
                            RequiredField::BillingUserLastName.to_tuple(),
                        ]),
                    },
                ),
                (Connector::Adyen, fields(vec![], vec![], vec![])),
                (
                    Connector::Santander,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingUserFirstName.to_tuple(),
                            RequiredField::BillingUserLastName.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Calida,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::BillingUserFirstName.to_tuple(),
                            RequiredField::BillingUserLastName.to_tuple(),
                            RequiredField::BillingCountries(vec![
                                "AT", "BE", "BG", "HR", "CY", "CZ", "DK", "EE", "FI", "FR", "DE",
                                "GR", "HU", "IE", "IT", "LV", "LT", "LU", "MT", "NL", "PL", "PT",
                                "RO", "SK", "SI", "ES", "SE", "IS", "LI", "NO",
                            ])
                            .to_tuple(),
                            RequiredField::BillingAddressCity.to_tuple(),
                            RequiredField::BillingAddressLine1.to_tuple(),
                            RequiredField::BillingAddressZip.to_tuple(),
                        ]),
                    },
                ),
                (
                    Connector::Facilitapay,
                    RequiredFieldFinal {
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                        common: HashMap::from([
                            RequiredField::PixSourceBankAccountId.to_tuple(),
                            RequiredField::BillingAddressCountries(vec!["BR"]).to_tuple(),
                            RequiredField::BillingUserFirstName.to_tuple(),
                            RequiredField::BillingUserLastName.to_tuple(),
                            RequiredField::PixCpf.to_tuple(),
                        ]),
                    },
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::PermataBankTransfer,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::BcaBankTransfer,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::BniVa,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::BriVa,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::CimbVa,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::DanamonVa,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::MandiriVa,
            connectors(vec![(
                Connector::Adyen,
                fields(vec![], billing_email_billing_name(), vec![]),
            )]),
        ),
        (
            enums::PaymentMethodType::SepaBankTransfer,
            connectors(vec![
                (
                    Connector::Stripe,
                    fields(
                        vec![],
                        vec![],
                        vec![
                            RequiredField::BillingEmail,
                            RequiredField::BillingUserFirstName,
                            RequiredField::BillingUserLastName,
                            RequiredField::BillingAddressCountries(vec![
                                "BE", "DE", "ES", "FR", "IE", "NL",
                            ]),
                        ],
                    ),
                ),
                (
                    Connector::Trustpay,
                    fields(
                        vec![],
                        vec![],
                        vec![
                            RequiredField::Email,
                            RequiredField::BillingFirstName(
                                "billing_first_name",
                                FieldType::UserBillingName,
                            ),
                            RequiredField::BillingLastName(
                                "billing_last_name",
                                FieldType::UserBillingName,
                            ),
                            RequiredField::BillingAddressLine1,
                            RequiredField::BillingAddressCity,
                            RequiredField::BillingAddressZip,
                            RequiredField::BillingAddressCountries(vec!["ALL"]),
                        ],
                    ),
                ),
            ]),
        ),
        (
            enums::PaymentMethodType::InstantBankTransfer,
            connectors(vec![(
                Connector::Trustpay,
                fields(
                    vec![],
                    vec![],
                    vec![
                        RequiredField::Email,
                        RequiredField::BillingFirstName(
                            "billing_first_name",
                            FieldType::UserBillingName,
                        ),
                        RequiredField::BillingLastName(
                            "billing_last_name",
                            FieldType::UserBillingName,
                        ),
                        RequiredField::BillingAddressLine1,
                        RequiredField::BillingAddressCity,
                        RequiredField::BillingAddressZip,
                        RequiredField::BillingAddressCountries(vec!["ALL"]),
                    ],
                ),
            )]),
        ),
        (
            enums::PaymentMethodType::InstantBankTransferFinland,
            connectors(vec![(
                Connector::Trustpay,
                fields(
                    vec![],
                    vec![],
                    vec![
                        RequiredField::Email,
                        RequiredField::BillingFirstName(
                            "billing_first_name",
                            FieldType::UserBillingName,
                        ),
                        RequiredField::BillingLastName(
                            "billing_last_name",
                            FieldType::UserBillingName,
                        ),
                        RequiredField::BillingAddressLine1,
                        RequiredField::BillingAddressCity,
                        RequiredField::BillingAddressZip,
                        RequiredField::BillingAddressCountries(vec!["FI"]),
                    ],
                ),
            )]),
        ),
        (
            enums::PaymentMethodType::InstantBankTransferPoland,
            connectors(vec![(
                Connector::Trustpay,
                fields(
                    vec![],
                    vec![],
                    vec![
                        RequiredField::Email,
                        RequiredField::BillingFirstName(
                            "billing_first_name",
                            FieldType::UserBillingName,
                        ),
                        RequiredField::BillingLastName(
                            "billing_last_name",
                            FieldType::UserBillingName,
                        ),
                        RequiredField::BillingAddressLine1,
                        RequiredField::BillingAddressCity,
                        RequiredField::BillingAddressZip,
                        RequiredField::BillingAddressCountries(vec!["PL"]),
                    ],
                ),
            )]),
        ),
        (
            enums::PaymentMethodType::Bacs,
            connectors(vec![(
                Connector::Stripe,
                fields(vec![], vec![], billing_email_name()),
            )]),
        ),
    ])
}

#[test]
fn test_required_fields_to_json() {
    // Test billing fields
    let billing_fields = get_billing_required_fields();
    // let billing_json = serde_json::to_string_pretty(&billing_fields)?;

    // Verify billing fields have expected entries
    assert!(billing_fields.contains_key("billing.address.first_name"));
    assert!(billing_fields.contains_key("billing.address.last_name"));
    assert!(billing_fields.contains_key("billing.address.city"));
    assert!(billing_fields.contains_key("billing.address.zip"));
    assert!(billing_fields.contains_key("billing.email"));

    // Verify specific billing field properties
    let billing_first_name = billing_fields.get("billing.address.first_name").unwrap();
    assert_eq!(billing_first_name.display_name, "billing_first_name");
    assert!(matches!(
        billing_first_name.field_type,
        FieldType::UserBillingName
    ));

    // Test shipping fields
    let shipping_fields = get_shipping_required_fields();
    // let shipping_json = serde_json::to_string_pretty(&shipping_fields)?;

    // Verify shipping fields have expected entries
    assert!(shipping_fields.contains_key("shipping.address.first_name"));
    assert!(shipping_fields.contains_key("shipping.address.last_name"));
    assert!(shipping_fields.contains_key("shipping.address.city"));
    assert!(shipping_fields.contains_key("shipping.address.zip"));
    assert!(shipping_fields.contains_key("shipping.email"));

    // Verify specific shipping field properties
    let shipping_address_line1 = shipping_fields.get("shipping.address.line1").unwrap();
    assert_eq!(shipping_address_line1.display_name, "line1");
    assert!(matches!(
        shipping_address_line1.field_type,
        FieldType::UserShippingAddressLine1
    ));

    #[cfg(feature = "v1")]
    {
        let default_fields = RequiredFields::default();
        // let default_json = serde_json::to_string_pretty(&default_fields.0)?;

        // Check default fields for payment methods
        assert!(default_fields.0.contains_key(&enums::PaymentMethod::Card));
        assert!(default_fields.0.contains_key(&enums::PaymentMethod::Wallet));

        // Verify card payment method types
        if let Some(card_method) = default_fields.0.get(&enums::PaymentMethod::Card) {
            assert!(card_method
                .0
                .contains_key(&enums::PaymentMethodType::Credit));
            assert!(card_method.0.contains_key(&enums::PaymentMethodType::Debit));
        }

        // Verify specific connector fields
        if let Some(card_method) = default_fields.0.get(&enums::PaymentMethod::Card) {
            if let Some(credit_type) = card_method.0.get(&enums::PaymentMethodType::Credit) {
                // Check if Stripe connector exists
                assert!(credit_type.fields.contains_key(&Connector::Stripe));

                // Verify Stripe required fields
                if let Some(stripe_fields) = credit_type.fields.get(&Connector::Stripe) {
                    // Check that card_basic fields are in "common" fields for Stripe
                    assert!(stripe_fields
                        .common
                        .contains_key("payment_method_data.card.card_number"));
                    assert!(stripe_fields
                        .common
                        .contains_key("payment_method_data.card.card_exp_month"));
                    assert!(stripe_fields
                        .common
                        .contains_key("payment_method_data.card.card_exp_year"));
                    assert!(stripe_fields
                        .common
                        .contains_key("payment_method_data.card.card_cvc"));
                }
            }
        }
        // print the result of default required fields as json in new file
        serde_json::to_writer_pretty(
            std::fs::File::create("default_required_fields.json").unwrap(),
            &default_fields,
        )
        .unwrap();
    }
}
