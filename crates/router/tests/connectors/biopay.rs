use router::connector::Biopay;

#[test]
fn biopay_connector_exists() {
    let _connector = Biopay::new();
}
