/// Serializes the given value into a string representation using the Display trait, and then uses the provided serializer to serialize the string.
/// 
/// # Arguments
/// 
/// * `value` - The value to be serialized.
/// * `serializer` - The serializer to use for the serialization.
/// 
/// # Returns
/// 
/// The result of the serialization, which is either the serialized string representation if successful, or an error if the serialization fails.
pub fn display_serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: std::fmt::Display,
    S: serde::ser::Serializer,
{
    serializer.serialize_str(&format!("{}", value))
}
