pub fn display_serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: std::fmt::Display,
    S: serde::ser::Serializer,
{
    serializer.serialize_str(&format!("{}", value))
}
