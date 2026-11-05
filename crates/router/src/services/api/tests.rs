#[test]
fn test_mime_essence() {
    assert_eq!(mime::APPLICATION_JSON.essence_str(), "application/json");
}
