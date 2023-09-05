#[cfg(feature = "kv_store")]
/// Generates hscan field pattern. Suppose the field is pa_1234_ref_1211 it will generate
/// pa_1234_ref_*
pub fn generate_hscan_pattern_for_refund(sk: &str) -> String {
    sk.split('_')
        .take(3)
        .chain(["*"])
        .collect::<Vec<&str>>()
        .join("_")
}

#[cfg(feature = "kv_store")]
/// Generates hscan field pattern. Suppose the field is pa_1234 it will generate
/// pa_*
pub fn generate_hscan_pattern_for_attempt(sk: &str) -> String {
    sk.split('_')
        .take(1)
        .chain(["*"])
        .collect::<Vec<&str>>()
        .join("_")
}
