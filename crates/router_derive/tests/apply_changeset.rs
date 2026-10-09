use router_derive::apply_changeset;

// A representative target struct with a mix of Option and non-Option fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
    pub count: i32,
    pub maybe_count: Option<i32>,
}

#[apply_changeset(target = Target)]
pub struct TargetUpdate {
    pub name: String,
    pub description: Option<String>,
    pub count: i32,
    pub maybe_count: Option<i32>,
}

#[test]
fn test_full_update() {
    let target = Target {
        id: 1,
        name: "original".to_string(),
        description: Some("original desc".to_string()),
        count: 10,
        maybe_count: Some(100),
    };

    let update = TargetUpdate {
        name: "updated".to_string(),
        description: Some("updated desc".to_string()),
        count: 20,
        maybe_count: Some(200),
    };

    let result = update.apply_changeset(target);

    assert_eq!(result.id, 1);
    assert_eq!(result.name, "updated");
    assert_eq!(result.description, Some("updated desc".to_string()));
    assert_eq!(result.count, 20);
    assert_eq!(result.maybe_count, Some(200));
}

#[test]
fn test_partial_update_with_none_options() {
    let target = Target {
        id: 1,
        name: "original".to_string(),
        description: Some("original desc".to_string()),
        count: 10,
        maybe_count: Some(100),
    };

    let update = TargetUpdate {
        name: "updated".to_string(),
        description: None,
        count: 20,
        maybe_count: None,
    };

    let result = update.apply_changeset(target.clone());

    assert_eq!(result.id, 1);
    assert_eq!(result.name, "updated");
    // Option fields with None should leave the original value intact
    assert_eq!(result.description, Some("original desc".to_string()));
    assert_eq!(result.count, 20);
    assert_eq!(result.maybe_count, Some(100));
}

#[test]
fn test_all_none_options() {
    let target = Target {
        id: 1,
        name: "original".to_string(),
        description: Some("original desc".to_string()),
        count: 10,
        maybe_count: Some(100),
    };

    let update = TargetUpdate {
        name: "updated".to_string(),
        description: None,
        count: 15,
        maybe_count: None,
    };

    let result = update.apply_changeset(target.clone());

    assert_eq!(result.id, 1);
    assert_eq!(result.name, "updated");
    assert_eq!(result.description, Some("original desc".to_string()));
    assert_eq!(result.count, 15);
    assert_eq!(result.maybe_count, Some(100));
}

#[test]
fn test_overwrite_some_with_none() {
    let target = Target {
        id: 1,
        name: "original".to_string(),
        description: Some("original desc".to_string()),
        count: 10,
        maybe_count: Some(100),
    };

    // Even though description is None, the macro treats Option specially:
    // None means "don't change", not "set to None".
    let update = TargetUpdate {
        name: "updated".to_string(),
        description: None,
        count: 20,
        maybe_count: None,
    };

    let result = update.apply_changeset(target.clone());

    // Original values should be preserved for Option fields when update is None
    assert_eq!(result.description, Some("original desc".to_string()));
    assert_eq!(result.maybe_count, Some(100));
}

#[test]
fn test_non_option_fields_always_overwritten() {
    let target = Target {
        id: 1,
        name: "original".to_string(),
        description: None,
        count: 10,
        maybe_count: None,
    };

    let update = TargetUpdate {
        name: "new name".to_string(),
        description: None,
        count: 99,
        maybe_count: None,
    };

    let result = update.apply_changeset(target);

    // Non-option fields are always overwritten regardless of value
    assert_eq!(result.name, "new name");
    assert_eq!(result.count, 99);
}

// Struct with only optional fields
#[derive(Debug, Clone, PartialEq)]
pub struct AllOptionalTarget {
    pub a: Option<String>,
    pub b: Option<i32>,
}

#[apply_changeset(target = AllOptionalTarget)]
pub struct AllOptionalUpdate {
    pub a: Option<String>,
    pub b: Option<i32>,
}

#[test]
fn test_all_optional_struct_some_updates() {
    let target = AllOptionalTarget {
        a: Some("a".to_string()),
        b: Some(1),
    };

    let update = AllOptionalUpdate {
        a: Some("new a".to_string()),
        b: Some(2),
    };

    let result = update.apply_changeset(target);

    assert_eq!(result.a, Some("new a".to_string()));
    assert_eq!(result.b, Some(2));
}

#[test]
fn test_all_optional_struct_none_updates() {
    let target = AllOptionalTarget {
        a: Some("a".to_string()),
        b: Some(1),
    };

    let update = AllOptionalUpdate { a: None, b: None };

    let result = update.apply_changeset(target.clone());

    // None means don't change for Option fields
    assert_eq!(result.a, Some("a".to_string()));
    assert_eq!(result.b, Some(1));
}

// Target has non-Option field, update struct has Option field for it.
#[derive(Debug, Clone, PartialEq)]
pub struct MismatchedTarget {
    pub id: u64,
    pub name: String,
    pub count: i32,
}

#[apply_changeset(target = MismatchedTarget)]
pub struct MismatchedUpdate {
    pub id: u64,
    pub name: Option<String>,
    pub count: Option<i32>,
}

#[test]
fn test_optional_update_for_non_optional_target_some() {
    let target = MismatchedTarget {
        id: 1,
        name: "original".to_string(),
        count: 10,
    };

    let update = MismatchedUpdate {
        id: 2,
        name: Some("updated".to_string()),
        count: Some(20),
    };

    let result = update.apply_changeset(target);

    assert_eq!(result.id, 2);
    assert_eq!(result.name, "updated");
    assert_eq!(result.count, 20);
}

#[test]
fn test_optional_update_for_non_optional_target_none() {
    let target = MismatchedTarget {
        id: 1,
        name: "original".to_string(),
        count: 10,
    };

    let update = MismatchedUpdate {
        id: 2,
        name: None,
        count: None,
    };

    let result = update.apply_changeset(target);

    assert_eq!(result.id, 2);
    // Because the update field is Option<T>, the macro's special handling applies:
    // None means "don't change", so the target keeps its original value.
    assert_eq!(result.name, "original");
    assert_eq!(result.count, 10);
}

// Edge case: target initially has None, update provides Some
#[test]
fn test_none_to_some_transition() {
    let target = AllOptionalTarget { a: None, b: None };

    let update = AllOptionalUpdate {
        a: Some("now present".to_string()),
        b: Some(42),
    };

    let result = update.apply_changeset(target);

    assert_eq!(result.a, Some("now present".to_string()));
    assert_eq!(result.b, Some(42));
}

// Target field is Option<T> but update struct uses Option<Option<T>> to distinguish:
//   None         → don't change
//   Some(None)   → explicitly set target to None
//   Some(Some(v))→ set target to Some(v)
#[derive(Debug, Clone, PartialEq)]
pub struct NestedOptionalTarget {
    pub id: u64,
    pub nickname: Option<String>,
}

#[apply_changeset(target = NestedOptionalTarget)]
pub struct NestedOptionalUpdate {
    pub id: u64,
    pub nickname: Option<Option<String>>,
}

#[test]
fn test_nested_option_none_means_no_change() {
    let target = NestedOptionalTarget {
        id: 1,
        nickname: Some("alice".to_string()),
    };

    let update = NestedOptionalUpdate {
        id: 2,
        nickname: None,
    };

    let result = update.apply_changeset(target);

    assert_eq!(result.id, 2);
    // None means "don't change" for Option<Option<T>>
    assert_eq!(result.nickname, Some("alice".to_string()));
}

#[test]
fn test_nested_option_some_none_sets_none() {
    let target = NestedOptionalTarget {
        id: 1,
        nickname: Some("alice".to_string()),
    };

    let update = NestedOptionalUpdate {
        id: 2,
        nickname: Some(None),
    };

    let result = update.apply_changeset(target);

    assert_eq!(result.id, 2);
    // Some(None) explicitly sets the target field to None
    assert_eq!(result.nickname, None);
}

#[test]
fn test_nested_option_some_some_sets_value() {
    let target = NestedOptionalTarget {
        id: 1,
        nickname: Some("alice".to_string()),
    };

    let update = NestedOptionalUpdate {
        id: 2,
        nickname: Some(Some("bob".to_string())),
    };

    let result = update.apply_changeset(target);

    assert_eq!(result.id, 2);
    // Some(Some(v)) sets the target to Some(v)
    assert_eq!(result.nickname, Some("bob".to_string()));
}

// Target has Option<T> fields, update struct has non-Option T fields.
// The macro should automatically wrap the assigned value in Some(...).
#[derive(Debug, Clone, PartialEq)]
pub struct OptionTargetWithNonOptionalUpdate {
    pub name: Option<String>,
    pub count: Option<i32>,
}

#[apply_changeset(target = OptionTargetWithNonOptionalUpdate)]
pub struct NonOptionalToOptionUpdate {
    pub name: String,
    pub count: i32,
}

#[test]
fn test_non_optional_update_for_optional_target_none_start() {
    let target = OptionTargetWithNonOptionalUpdate {
        name: None,
        count: None,
    };

    let update = NonOptionalToOptionUpdate {
        name: "new name".to_string(),
        count: 42,
    };

    let result = update.apply_changeset(target);

    assert_eq!(result.name, Some("new name".to_string()));
    assert_eq!(result.count, Some(42));
}

#[test]
fn test_non_optional_update_for_optional_target_some_start() {
    let target = OptionTargetWithNonOptionalUpdate {
        name: Some("original".to_string()),
        count: Some(10),
    };

    let update = NonOptionalToOptionUpdate {
        name: "updated".to_string(),
        count: 20,
    };

    let result = update.apply_changeset(target);

    assert_eq!(result.name, Some("updated".to_string()));
    assert_eq!(result.count, Some(20));
}
