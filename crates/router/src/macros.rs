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

#[macro_export]
macro_rules! get_payment_link_config_value_based_on_priority {
    ($config:expr, $business_config:expr, $field:ident, $default:expr) => {
        $config
            .as_ref()
            .and_then(|pc_config| pc_config.theme_config.$field.clone())
            .or_else(|| {
                $business_config
                    .as_ref()
                    .and_then(|business_config| business_config.$field.clone())
            })
            .unwrap_or($default)
    };
}

#[macro_export]
macro_rules! get_payment_link_config_value {
    ($config:expr, $business_config:expr, $(($field:ident, $default:expr)),*) => {
        (
            $(get_payment_link_config_value_based_on_priority!($config, $business_config, $field, $default)),*
        )
    };
    ($config:expr, $business_config:expr, $(($field:ident)),*) => {
        (
            $(
                $config
                    .as_ref()
                    .and_then(|pc_config| pc_config.theme_config.$field.clone())
                    .or_else(|| {
                        $business_config
                            .as_ref()
                            .and_then(|business_config| business_config.$field.clone())
                    })
            ),*
        )
    };
    ($config:expr, $business_config:expr, $(($field:ident $(, $transform:expr)?)),* $(,)?) => {
        (
            $(
                $config
                    .as_ref()
                    .and_then(|pc_config| pc_config.theme_config.$field.clone())
                    .or_else(|| {
                        $business_config
                            .as_ref()
                            .and_then(|business_config| {
                                let value = business_config.$field.clone();
                                $(let value = value.map($transform);)?
                                value
                            })
                    })
            ),*
        )
    };

}
