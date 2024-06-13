pub fn add_attributes<T, U>(attributes: U) -> Vec<opentelemetry::KeyValue>
where
    T: Into<opentelemetry::Value>,
    U: IntoIterator<Item = (&'static str, T)>,
{
    attributes
        .into_iter()
        .map(|(key, value)| opentelemetry::KeyValue::new(key, value))
        .collect::<Vec<_>>()
}
