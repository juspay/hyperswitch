use super::*;

#[test]
fn validate_id_length_constraint() {
    let payment_id =
        "abcdefghijlkmnopqrstuvwzyzabcdefghijknlmnopsjkdnfjsknfkjsdnfspoig".to_string(); //length = 65

    let result = validate_id(payment_id, "payment_id");
    assert!(result.is_err());
}

#[test]
fn validate_id_proper_response() {
    let payment_id = "abcdefghijlkmnopqrstjhbjhjhkhbhgcxdfxvmhb".to_string();

    let result = validate_id(payment_id.clone(), "payment_id");
    assert!(result.is_ok());
    let result = result.unwrap_or_default();
    assert_eq!(result, payment_id);
}

#[test]
fn test_generate_id() {
    let generated_id = generate_id(consts::ID_LENGTH, "ref");
    assert_eq!(generated_id.len(), consts::ID_LENGTH + 4)
}

#[test]
fn test_filter_objects_based_on_profile_id_list() {
    #[derive(PartialEq, Debug, Clone)]
    struct Object {
        profile_id: Option<common_utils::id_type::ProfileId>,
    }

    impl Object {
        pub fn new(profile_id: &'static str) -> Self {
            Self {
                profile_id: Some(
                    common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from(profile_id))
                        .expect("invalid profile ID"),
                ),
            }
        }
    }

    impl GetProfileId for Object {
        fn get_profile_id(&self) -> Option<&common_utils::id_type::ProfileId> {
            self.profile_id.as_ref()
        }
    }

    fn new_profile_id(profile_id: &'static str) -> common_utils::id_type::ProfileId {
        common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from(profile_id))
            .expect("invalid profile ID")
    }

    // non empty object_list and profile_id_list
    let object_list = vec![
        Object::new("p1"),
        Object::new("p2"),
        Object::new("p2"),
        Object::new("p4"),
        Object::new("p5"),
    ];
    let profile_id_list = vec![
        new_profile_id("p1"),
        new_profile_id("p2"),
        new_profile_id("p3"),
    ];
    let filtered_list =
        filter_objects_based_on_profile_id_list(Some(profile_id_list), object_list.clone());
    let expected_result = vec![Object::new("p1"), Object::new("p2"), Object::new("p2")];
    assert_eq!(filtered_list, expected_result);

    // non empty object_list and empty profile_id_list
    let empty_profile_id_list = vec![];
    let filtered_list =
        filter_objects_based_on_profile_id_list(Some(empty_profile_id_list), object_list.clone());
    let expected_result = vec![];
    assert_eq!(filtered_list, expected_result);

    // non empty object_list and None profile_id_list
    let profile_id_list_as_none = None;
    let filtered_list =
        filter_objects_based_on_profile_id_list(profile_id_list_as_none, object_list);
    let expected_result = vec![
        Object::new("p1"),
        Object::new("p2"),
        Object::new("p2"),
        Object::new("p4"),
        Object::new("p5"),
    ];
    assert_eq!(filtered_list, expected_result);
}
