use common_enums::CountryAlpha2;
use hyperswitch_masking::ExposeOptionInterface;

use super::super::*;

const TEST_COUNTRY: CountryAlpha2 = CountryAlpha2::US;
const TEST_FIRST_NAME: &str = "John";
const TEST_LAST_NAME: &str = "Wheat Dough";
const TEST_FULL_NAME: &str = "John Wheat Dough";
const TEST_FIRST_NAME_SINGLE: &str = "John";

#[test]
fn test_wallet_payment_method_data_paypal() {
    let test_email: Email = Email::try_from("example@example.com".to_string()).unwrap();

    let paypal_wallet_payment_method_data =
        PaymentMethodData::Wallet(WalletData::PaypalRedirect(PaypalRedirection {
            email: Some(test_email.clone()),
            payer_id: None,
        }));

    let billing_address = paypal_wallet_payment_method_data
        .get_billing_address()
        .unwrap();

    assert_eq!(billing_address.email.unwrap(), test_email);

    assert!(billing_address.address.is_none());
    assert!(billing_address.phone.is_none());
}

#[test]
fn test_bank_redirect_payment_method_data_eps() {
    let test_email = Email::try_from("example@example.com".to_string()).unwrap();
    let test_first_name = Secret::new(String::from("Chaser"));

    let bank_redirect_billing = BankRedirectBilling {
        billing_name: Some(test_first_name.clone()),
        email: Some(test_email.clone()),
    };

    let eps_bank_redirect_payment_method_data =
        PaymentMethodData::BankRedirect(BankRedirectData::Eps {
            billing_details: Some(bank_redirect_billing),
            bank_name: None,
            country: Some(TEST_COUNTRY),
        });

    let billing_address = eps_bank_redirect_payment_method_data
        .get_billing_address()
        .unwrap();

    let address_details = billing_address.address.unwrap();

    assert_eq!(billing_address.email.unwrap(), test_email);
    assert_eq!(address_details.country.unwrap(), TEST_COUNTRY);
    assert_eq!(address_details.first_name.unwrap(), test_first_name);
    assert!(billing_address.phone.is_none());
}

#[test]
fn test_paylater_payment_method_data_klarna() {
    let test_email: Email = Email::try_from("example@example.com".to_string()).unwrap();

    let klarna_paylater_payment_method_data =
        PaymentMethodData::PayLater(PayLaterData::KlarnaRedirect {
            billing_email: Some(test_email.clone()),
            billing_country: Some(TEST_COUNTRY),
        });

    let billing_address = klarna_paylater_payment_method_data
        .get_billing_address()
        .unwrap();

    assert_eq!(billing_address.email.unwrap(), test_email);
    assert_eq!(
        billing_address.address.unwrap().country.unwrap(),
        TEST_COUNTRY
    );
    assert!(billing_address.phone.is_none());
}

#[test]
fn test_bank_debit_payment_method_data_ach() {
    let test_email = Email::try_from("example@example.com".to_string()).unwrap();
    let test_first_name = Secret::new(String::from("Chaser"));

    let bank_redirect_billing = BankDebitBilling {
        name: Some(test_first_name.clone()),
        address: None,
        email: Some(test_email.clone()),
    };

    let ach_bank_debit_payment_method_data =
        PaymentMethodData::BankDebit(BankDebitData::AchBankDebit {
            billing_details: Some(bank_redirect_billing),
            account_number: Secret::new("1234".to_string()),
            routing_number: Secret::new("1235".to_string()),
            bank_account_holder_name: None,
            bank_name: None,
            bank_type: None,
            bank_holder_type: None,
        });

    let billing_address = ach_bank_debit_payment_method_data
        .get_billing_address()
        .unwrap();

    let address_details = billing_address.address.unwrap();

    assert_eq!(billing_address.email.unwrap(), test_email);
    assert_eq!(address_details.first_name.unwrap(), test_first_name);
    assert!(billing_address.phone.is_none());
}

#[test]
fn test_card_payment_method_data() {
    let card_payment_method_data = PaymentMethodData::Card(Card {
        card_holder_name: Some(Secret::new(TEST_FIRST_NAME_SINGLE.into())),
        ..Default::default()
    });

    let billing_address = card_payment_method_data.get_billing_address();

    let billing_address = billing_address.unwrap();

    assert_eq!(
        billing_address.address.unwrap().first_name.expose_option(),
        Some(TEST_FIRST_NAME_SINGLE.into())
    );
}

#[test]
fn test_card_payment_method_data_empty() {
    let card_payment_method_data = PaymentMethodData::Card(Card::default());

    let billing_address = card_payment_method_data.get_billing_address();

    assert!(billing_address.is_none());
}

#[test]
fn test_card_payment_method_data_full_name() {
    let card_payment_method_data = PaymentMethodData::Card(Card {
        card_holder_name: Some(Secret::new(TEST_FULL_NAME.into())),
        ..Default::default()
    });

    let billing_details = card_payment_method_data.get_billing_address().unwrap();
    let billing_address = billing_details.address.unwrap();

    assert_eq!(
        billing_address.first_name.expose_option(),
        Some(TEST_FIRST_NAME.into())
    );

    assert_eq!(
        billing_address.last_name.expose_option(),
        Some(TEST_LAST_NAME.into())
    );
}

#[test]
fn test_card_payment_method_data_empty_string() {
    let card_payment_method_data = PaymentMethodData::Card(Card {
        card_holder_name: Some(Secret::new("".to_string())),
        ..Default::default()
    });

    let billing_details = card_payment_method_data.get_billing_address();

    assert!(billing_details.is_none());
}
