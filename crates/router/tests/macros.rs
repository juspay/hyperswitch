#[cfg(test)]
mod flat_struct_test {
    #![allow(clippy::unwrap_used)]
    use std::collections::HashMap;

    use router_derive::FlatStruct;
    use serde::Serialize;

    #[test]
    fn test_flat_struct() {
        #[derive(FlatStruct, Serialize)]
        struct User {
            address: Address,
        }

        #[derive(Serialize)]
        struct Address {
            line1: String,
            zip: String,
            city: String,
        }

        let line1 = "1397".to_string();
        let zip = "Some street".to_string();
        let city = "941222".to_string();

        let address = Address {
            line1: line1.clone(),
            zip: zip.clone(),
            city: city.clone(),
        };
        let user = User { address };
        let flat_user_map = user.flat_struct();

        let mut required_map = HashMap::new();
        required_map.insert("address.line1".to_string(), line1);
        required_map.insert("address.zip".to_string(), zip);
        required_map.insert("address.city".to_string(), city);

        assert_eq!(flat_user_map, required_map);
    }
}
