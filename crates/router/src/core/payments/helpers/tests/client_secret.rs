#[test]
fn test_client_secret_parse() {
    let client_secret1 = "pay_3TgelAms4RQec8xSStjF_secret_fc34taHLw1ekPgNh92qr";
    let client_secret2 = "pay_3Tgel__Ams4RQ_secret_ec8xSStjF_secret_fc34taHLw1ekPgNh92qr";
    let client_secret3 = "pay_3Tgel__Ams4RQ_secret_ec8xSStjF_secret__secret_fc34taHLw1ekPgNh92qr";

    assert_eq!(
        "pay_3TgelAms4RQec8xSStjF",
        super::super::get_payment_id_from_client_secret(client_secret1).unwrap()
    );
    assert_eq!(
        "pay_3Tgel__Ams4RQ_secret_ec8xSStjF",
        super::super::get_payment_id_from_client_secret(client_secret2).unwrap()
    );
    assert_eq!(
        "pay_3Tgel__Ams4RQ_secret_ec8xSStjF_secret_",
        super::super::get_payment_id_from_client_secret(client_secret3).unwrap()
    );
}
