//! The facade's hasher with NO runtime hook: the feature compiled in, deja
//! idle. This is the shape of a release build serving ordinary traffic, where
//! predictable keys would be a hash-flooding hole.
#![cfg(feature = "deja")]

use common_utils::collections::HashMap;

fn rendered() -> String {
    let mut map: HashMap<u32, ()> = HashMap::default();
    for key in 0..32 {
        map.insert(key, ());
    }
    format!("{map:?}")
}

#[test]
fn a_correlation_with_deja_idle_keeps_random_keys() {
    assert!(deja::runtime_mode_is_disabled());
    let _correlation = deja::test_support::recording_correlation("corr-1");
    assert_ne!(rendered(), rendered());
}
