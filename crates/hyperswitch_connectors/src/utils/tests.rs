use crate::utils;
#[test]
fn test_image_data_source_url() {
    let qr_image_data_source_url = utils::QrImage::new_from_data("Hyperswitch".to_string());
    assert!(qr_image_data_source_url.is_ok());
}
