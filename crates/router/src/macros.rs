pub use common_utils::{collect_missing_value_keys, newtype};

#[macro_export]
macro_rules! get_formatted_date_time {
    ($date_format:tt) => {{
        let format = time::macros::format_description!($date_format);
        time::OffsetDateTime::now_utc()
            .format(&format)
            .change_context($crate::core::errors::ConnectorError::InvalidDateFormat)
    }};
}
