//! Utility macros

#[allow(missing_docs)]
#[macro_export]
macro_rules! newtype_impl {
    ($is_pub:vis, $name:ident, $ty_path:path) => {
        impl core::ops::Deref for $name {
            type Target = $ty_path;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl core::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl From<$ty_path> for $name {
            fn from(ty: $ty_path) -> Self {
                Self(ty)
            }
        }

        impl $name {
            pub fn into_inner(self) -> $ty_path {
                self.0
            }
        }
    };
}

#[allow(missing_docs)]
#[macro_export]
macro_rules! newtype {
    ($is_pub:vis $name:ident = $ty_path:path) => {
        $is_pub struct $name(pub $ty_path);

        $crate::newtype_impl!($is_pub, $name, $ty_path);
    };

    ($is_pub:vis $name:ident = $ty_path:path, derives = ($($trt:path),*)) => {
        #[derive($($trt),*)]
        $is_pub struct $name(pub $ty_path);

        $crate::newtype_impl!($is_pub, $name, $ty_path);
    };
}

/// Use this to ensure that the corresponding
/// openapi route has been implemented in the openapi crate
#[macro_export]
macro_rules! openapi_route {
    ($route_name: ident) => {{
        #[cfg(feature = "openapi")]
        use openapi::routes::$route_name as _;

        $route_name
    }};
}

#[allow(missing_docs)]
#[macro_export]
macro_rules! fallback_reverse_lookup_not_found {
    ($a:expr,$b:expr) => {
        match $a {
            Ok(res) => res,
            Err(err) => {
                router_env::logger::error!(reverse_lookup_fallback = ?err);
                match err.current_context() {
                    errors::StorageError::ValueNotFound(_) => return $b,
                    errors::StorageError::DatabaseError(data_err) => {
                        match data_err.current_context() {
                            diesel_models::errors::DatabaseError::NotFound => return $b,
                            _ => return Err(err)
                        }
                    }
                    _=> return Err(err)
                }
            }
        };
    };
}

/// Collects names of all optional fields that are `None`.
/// This is typically useful for constructing error messages including a list of all missing fields.
#[macro_export]
macro_rules! collect_missing_value_keys {
    [$(($key:literal, $option:expr)),+] => {
        {
            let mut keys: Vec<&'static str> = Vec::new();
            $(
                if $option.is_none() {
                    keys.push($key);
                }
            )*
            keys
        }
    };
}

/// Implements the `ToSql` and `FromSql` traits on a type to allow it to be serialized/deserialized
/// to/from JSON data in the database.
#[macro_export]
macro_rules! impl_to_sql_from_sql_json {
    ($type:ty, $diesel_type:ty) => {
        #[allow(unused_qualifications)]
        impl diesel::serialize::ToSql<$diesel_type, diesel::pg::Pg> for $type {
            fn to_sql<'b>(
                &'b self,
                out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
            ) -> diesel::serialize::Result {
                let value = serde_json::to_value(self)?;

                // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
                // please refer to the diesel migration blog:
                // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
                <serde_json::Value as diesel::serialize::ToSql<
                                                                $diesel_type,
                                                                diesel::pg::Pg,
                                                            >>::to_sql(&value, &mut out.reborrow())
            }
        }

        #[allow(unused_qualifications)]
        impl diesel::deserialize::FromSql<$diesel_type, diesel::pg::Pg> for $type {
            fn from_sql(
                bytes: <diesel::pg::Pg as diesel::backend::Backend>::RawValue<'_>,
            ) -> diesel::deserialize::Result<Self> {
                let value = <serde_json::Value as diesel::deserialize::FromSql<
                    $diesel_type,
                    diesel::pg::Pg,
                >>::from_sql(bytes)?;
                Ok(serde_json::from_value(value)?)
            }
        }
    };
    ($type: ty) => {
        $crate::impl_to_sql_from_sql_json!($type, diesel::sql_types::Json);
        $crate::impl_to_sql_from_sql_json!($type, diesel::sql_types::Jsonb);
    };
}

mod id_type {
    /// Defines an ID type.
    #[macro_export]
    macro_rules! id_type {
        ($type:ident, $doc:literal, $diesel_type:ty, $max_length:expr, $min_length:expr) => {
            #[doc = $doc]
            #[derive(
                Clone,
                Hash,
                PartialEq,
                Eq,
                serde::Serialize,
                serde::Deserialize,
                diesel::expression::AsExpression,
                utoipa::ToSchema,
            )]
            #[diesel(sql_type = $diesel_type)]
            #[schema(value_type = String)]
            pub struct $type($crate::id_type::LengthId<$max_length, $min_length>);
        };
        ($type:ident, $doc:literal) => {
            $crate::id_type!(
                $type,
                $doc,
                diesel::sql_types::Text,
                { $crate::consts::MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH },
                { $crate::consts::MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH }
            );
        };
    }

    /// Defines a Global Id type
    #[cfg(feature = "v2")]
    #[macro_export]
    macro_rules! global_id_type {
        ($type:ident, $doc:literal) => {
            #[doc = $doc]
            #[derive(
                Debug,
                Clone,
                Hash,
                PartialEq,
                Eq,
                serde::Serialize,
                serde::Deserialize,
                diesel::expression::AsExpression,
            )]
            #[diesel(sql_type = diesel::sql_types::Text)]
            pub struct $type($crate::id_type::global_id::GlobalId);
        };
    }

    /// Implements common methods on the specified ID type.
    #[macro_export]
    macro_rules! impl_id_type_methods {
        ($type:ty, $field_name:literal) => {
            impl $type {
                /// Get the string representation of the ID type.
                pub fn get_string_repr(&self) -> &str {
                    &self.0 .0 .0
                }
            }
        };
    }

    /// Implements the `Debug` trait on the specified ID type.
    #[macro_export]
    macro_rules! impl_debug_id_type {
        ($type:ty) => {
            impl core::fmt::Debug for $type {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_tuple(stringify!($type))
                        .field(&self.0 .0 .0)
                        .finish()
                }
            }
        };
    }

    /// Implements the `TryFrom<Cow<'static, str>>` trait on the specified ID type.
    #[macro_export]
    macro_rules! impl_try_from_cow_str_id_type {
        ($type:ty, $field_name:literal) => {
            impl TryFrom<std::borrow::Cow<'static, str>> for $type {
                type Error = error_stack::Report<$crate::errors::ValidationError>;

                fn try_from(value: std::borrow::Cow<'static, str>) -> Result<Self, Self::Error> {
                    use error_stack::ResultExt;

                    let merchant_ref_id = $crate::id_type::LengthId::from(value).change_context(
                        $crate::errors::ValidationError::IncorrectValueProvided {
                            field_name: $field_name,
                        },
                    )?;

                    Ok(Self(merchant_ref_id))
                }
            }
        };
    }

    /// Implements the `Default` trait on the specified ID type.
    #[macro_export]
    macro_rules! impl_default_id_type {
        ($type:ty, $prefix:literal) => {
            impl Default for $type {
                fn default() -> Self {
                    Self($crate::generate_ref_id_with_default_length($prefix))
                }
            }
        };
    }

    /// Implements the `GenerateId` trait on the specified ID type.
    #[macro_export]
    macro_rules! impl_generate_id_id_type {
        ($type:ty, $prefix:literal) => {
            impl $crate::id_type::GenerateId for $type {
                fn generate() -> Self {
                    Self($crate::generate_ref_id_with_default_length($prefix))
                }
            }
        };
    }

    /// Implements the `SerializableSecret` trait on the specified ID type.
    #[macro_export]
    macro_rules! impl_serializable_secret_id_type {
        ($type:ty) => {
            impl masking::SerializableSecret for $type {}
        };
    }

    /// Implements the `ToSql` and `FromSql` traits on the specified ID type.
    #[macro_export]
    macro_rules! impl_to_sql_from_sql_id_type {
        ($type:ty, $diesel_type:ty, $max_length:expr, $min_length:expr) => {
            impl<DB> diesel::serialize::ToSql<$diesel_type, DB> for $type
            where
                DB: diesel::backend::Backend,
                $crate::id_type::LengthId<$max_length, $min_length>:
                    diesel::serialize::ToSql<$diesel_type, DB>,
            {
                fn to_sql<'b>(
                    &'b self,
                    out: &mut diesel::serialize::Output<'b, '_, DB>,
                ) -> diesel::serialize::Result {
                    self.0.to_sql(out)
                }
            }

            impl<DB> diesel::deserialize::FromSql<$diesel_type, DB> for $type
            where
                DB: diesel::backend::Backend,
                $crate::id_type::LengthId<$max_length, $min_length>:
                    diesel::deserialize::FromSql<$diesel_type, DB>,
            {
                fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
                    $crate::id_type::LengthId::<$max_length, $min_length>::from_sql(value).map(Self)
                }
            }
        };
        ($type:ty) => {
            $crate::impl_to_sql_from_sql_id_type!(
                $type,
                diesel::sql_types::Text,
                { $crate::consts::MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH },
                { $crate::consts::MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH }
            );
        };
    }

    #[cfg(feature = "v2")]
    /// Implements the `ToSql` and `FromSql` traits on the specified Global ID type.
    #[macro_export]
    macro_rules! impl_to_sql_from_sql_global_id_type {
        ($type:ty, $diesel_type:ty) => {
            impl<DB> diesel::serialize::ToSql<$diesel_type, DB> for $type
            where
                DB: diesel::backend::Backend,
                $crate::id_type::global_id::GlobalId: diesel::serialize::ToSql<$diesel_type, DB>,
            {
                fn to_sql<'b>(
                    &'b self,
                    out: &mut diesel::serialize::Output<'b, '_, DB>,
                ) -> diesel::serialize::Result {
                    self.0.to_sql(out)
                }
            }

            impl<DB> diesel::deserialize::FromSql<$diesel_type, DB> for $type
            where
                DB: diesel::backend::Backend,
                $crate::id_type::global_id::GlobalId:
                    diesel::deserialize::FromSql<$diesel_type, DB>,
            {
                fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
                    $crate::id_type::global_id::GlobalId::from_sql(value).map(Self)
                }
            }
        };
        ($type:ty) => {
            $crate::impl_to_sql_from_sql_global_id_type!($type, diesel::sql_types::Text);
        };
    }

    /// Implements the `Queryable` trait on the specified ID type.
    #[macro_export]
    macro_rules! impl_queryable_id_type {
        ($type:ty, $diesel_type:ty) => {
            impl<DB> diesel::Queryable<$diesel_type, DB> for $type
            where
                DB: diesel::backend::Backend,
                Self: diesel::deserialize::FromSql<$diesel_type, DB>,
            {
                type Row = Self;

                fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
                    Ok(row)
                }
            }
        };
        ($type:ty) => {
            $crate::impl_queryable_id_type!($type, diesel::sql_types::Text);
        };
    }
}

/// Create new generic list wrapper
#[macro_export]
macro_rules! create_list_wrapper {
    (
        $wrapper_name:ident,
        $type_name: ty,
        impl_functions: {
            $($function_def: tt)*
        }
    ) => {
        #[derive(Clone, Debug)]
        pub struct $wrapper_name(Vec<$type_name>);
        impl $wrapper_name {
            pub fn new(list: Vec<$type_name>) -> Self {
                Self(list)
            }
            pub fn with_capacity(size: usize) -> Self {
                Self(Vec::with_capacity(size))
            }
            $($function_def)*
        }
        impl std::ops::Deref for $wrapper_name {
            type Target = Vec<$type_name>;
            fn deref(&self) -> &<Self as std::ops::Deref>::Target {
                &self.0
            }
        }
        impl std::ops::DerefMut for $wrapper_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
        impl IntoIterator for $wrapper_name {
            type Item = $type_name;
            type IntoIter = std::vec::IntoIter<$type_name>;
            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }

        impl<'a> IntoIterator for &'a $wrapper_name {
            type Item = &'a $type_name;
            type IntoIter = std::slice::Iter<'a, $type_name>;
            fn into_iter(self) -> Self::IntoIter {
                self.0.iter()
            }
        }

        impl FromIterator<$type_name> for $wrapper_name {
            fn from_iter<T: IntoIterator<Item = $type_name>>(iter: T) -> Self {
                Self(iter.into_iter().collect())
            }
        }
    };
}

/// Get the type name for a type
#[macro_export]
macro_rules! type_name {
    ($type:ty) => {
        std::any::type_name::<$type>()
            .rsplit("::")
            .nth(1)
            .unwrap_or_default();
    };
}

/// **Note** Creates an enum wrapper that implements `FromStr`, `Display`, `Serialize`, and `Deserialize`
/// based on a specific string representation format: `"VariantName<delimiter>FieldValue"`.
/// It handles parsing errors by returning a dedicated `Invalid` variant.
/// *Note*: The macro adds `Invalid,` automatically.
///
/// # Use Case
///
/// This macro is designed for scenarios where you need an enum, with each variant
/// holding a single piece of associated data, to be easily convertible to and from
/// a simple string format. This is useful for cases where enum is serialized to key value pairs
///
/// It avoids more complex serialization structures (like JSON objects `{"VariantName": value}`)
/// in favor of a plain string representation.
///
/// # Input Enum Format and Constraints
///
/// To use this macro, the enum definition must adhere to the following structure:
///
/// 1.  **Public Enum:** The enum must be declared as `pub enum EnumName { ... }`.
/// 2.  **Struct Variants Only:** All variants must be struct variants (using `{}`).
/// 3.  **Exactly One Field:** Each struct variant must contain *exactly one* named field.
///     * **Valid:** `VariantA { value: i32 }`
///     * **Invalid:** `VariantA(i32)` (tuple variant)
///     * **Invalid:** `VariantA` or `VariantA {}` (no field)
///     * **Invalid:** `VariantA { value: i32, other: bool }` (multiple fields)
/// 4.  **Tag Delimiter:** The macro invocation must specify a `tag_delimiter` literal,
///     which is the character used to separate the variant name from the field data in
///     the string representation (e.g., `tag_delimiter = ":",`).
/// 5.  **Field Type Requirements:** The type of the single field in each variant (`$field_ty`)
///     must implement:
///     * `core::str::FromStr`: To parse the field's data from the string part.
///       The `Err` type should ideally be convertible to a meaningful error, though the
///       macro currently uses a generic error message upon failure.
///     * `core::fmt::Display`: To convert the field's data into the string part.
///     * `serde::Serialize` and `serde::Deserialize<'de>`: Although the macro implements
///       custom `Serialize`/`Deserialize` for the *enum* using the string format, the field
///       type itself must satisfy these bounds if required elsewhere or by generic contexts.
///       The macro's implementations rely solely on `Display` and `FromStr` for the conversion.
/// 6.  **Error Type:** This macro uses `core::convert::Infallible` as it never fails but gives
///     `Self::Invalid` variant.
///
/// # Serialization and Deserialization (`serde`)
///
/// When `serde` features are enabled and the necessary traits are derived or implemented,
/// this macro implements `Serialize` and `Deserialize` for the enum:
///
/// **Serialization:** An enum value like `MyEnum::VariantA { value: 123 }` (with `tag_delimiter = ":",`)
///     will be serialized into the string `"VariantA:123"`. If serializing to JSON, this results
///     in a JSON string: `"\"VariantA:123\""`.
/// **Deserialization:** The macro expects a string matching the format `"VariantName<delimiter>FieldValue"`.
///     It uses the enum's `FromStr` implementation internally. When deserializing from JSON, it
///     expects a JSON string containing the correctly formatted value (e.g., `"\"VariantA:123\""`).
///
/// # `Display` and `FromStr`
///
/// **`Display`:** Formats valid variants to `"VariantName<delimiter>FieldValue"` and catch-all cases to `"Invalid"`.
/// **`FromStr`:** Parses `"VariantName<delimiter>FieldValue"` to the variant, or returns `Self::Invalid`
///   if the input string is malformed or `"Invalid"`.
///
/// # Example
///
/// ```rust
/// use std::str::FromStr;
///
/// crate::impl_enum_str!(
///     tag_delimiter = ":",
///     #[derive(Debug, PartialEq, Clone)] // Add other derives as needed
///     pub enum Setting {
///         Timeout { duration_ms: u32 },
///         Username { name: String },
///     }
/// );
/// // Note: The macro adds `Invalid,` automatically.
///
/// fn main() {
///     // Display
///     let setting1 = Setting::Timeout { duration_ms: 5000 };
///     assert_eq!(setting1.to_string(), "Timeout:5000");
///     assert_eq!(Setting::Invalid.to_string(), "Invalid");
///
///     // FromStr (returns Self, not Result)
///     let parsed_setting: Setting = "Username:admin".parse().expect("Valid parse"); // parse() itself doesn't panic
///     assert_eq!(parsed_setting, Setting::Username { name: "admin".to_string() });
///
///     let invalid_format: Setting = "Timeout".parse().expect("Parse always returns Self");
///     assert_eq!(invalid_format, Setting::Invalid); // Malformed input yields Invalid
///
///     let bad_data: Setting = "Timeout:fast".parse().expect("Parse always returns Self");
///     assert_eq!(bad_data, Setting::Invalid); // Bad field data yields Invalid
///
///     let unknown_tag: Setting = "Unknown:abc".parse().expect("Parse always returns Self");
///     assert_eq!(unknown_tag, Setting::Invalid); // Unknown tag yields Invalid
///
///     let explicit_invalid: Setting = "Invalid".parse().expect("Parse always returns Self");
///     assert_eq!(explicit_invalid, Setting::Invalid); // "Invalid" string yields Invalid
///
///     // Serde (requires derive Serialize/Deserialize on Setting)
///     // let json_output = serde_json::to_string(&setting1).unwrap();
///     // assert_eq!(json_output, "\"Timeout:5000\"");
///     // let invalid_json_output = serde_json::to_string(&Setting::Invalid).unwrap();
///     // assert_eq!(invalid_json_output, "\"Invalid\"");
///
///     // let deserialized: Setting = serde_json::from_str("\"Username:guest\"").unwrap();
///     // assert_eq!(deserialized, Setting::Username { name: "guest".to_string() });
///     // let deserialized_invalid: Setting = serde_json::from_str("\"Invalid\"").unwrap();
///     // assert_eq!(deserialized_invalid, Setting::Invalid);
///     // let deserialized_malformed: Setting = serde_json::from_str("\"TimeoutFast\"").unwrap();
///     // assert_eq!(deserialized_malformed, Setting::Invalid); // Malformed -> Invalid
/// }
///
/// # // Mock macro definition for doctest purposes
/// # #[macro_export] macro_rules! impl_enum_str { ($($tt:tt)*) => { $($tt)* } }
/// ```
#[macro_export]
macro_rules! impl_enum_str {
    (
        tag_delimiter = $tag_delim:literal,
        $(#[$enum_attr:meta])*
        pub enum $enum_name:ident {
            $(
                $(#[$variant_attr:meta])*
                $variant:ident {
                    $(#[$field_attr:meta])*
                    $field:ident : $field_ty:ty $(,)?
                }
            ),* $(,)?
        }
    ) => {
        $(#[$enum_attr])*
        pub enum $enum_name {
            $(
                $(#[$variant_attr])*
                $variant {
                    $(#[$field_attr])*
                    $field : $field_ty
                },
            )*
            /// Represents a parsing failure.
            Invalid, // Automatically add the Invalid variant
        }

        // Implement FromStr - now returns Self, not Result
        impl core::str::FromStr for $enum_name {
            // No associated error type needed
            type Err = core::convert::Infallible; // FromStr requires an Err type, use Infallible

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                // Check for explicit "Invalid" string first
                if s == "Invalid" {
                    #[cfg(feature = "logs")]
                    router_env::logger::warn!(
                        "Failed to parse {} enum from 'Invalid': explicit Invalid variant encountered",
                        stringify!($enum_name)
                    );
                    return Ok(Self::Invalid);
                }

                let Some((tag, associated_data)) = s.split_once($tag_delim) else {
                    // Missing delimiter -> Invalid
                    #[cfg(feature = "logs")]
                    router_env::logger::warn!(
                        "Failed to parse {} enum from '{}': missing delimiter",
                        stringify!($enum_name),
                        s
                    );
                    return Ok(Self::Invalid);
                };

                let result = match tag {
                    $(
                        stringify!($variant) => {
                            // Try to parse the field data
                            match associated_data.parse::<$field_ty>() {
                                Ok(parsed_field) => {
                                    // Success -> construct the variant
                                     Self::$variant { $field: parsed_field }
                                },
                                Err(_) => {
                                     // Field parse failure -> Invalid
                                     #[cfg(feature = "logs")]
                                     router_env::logger::warn!(
                                         "Failed to parse {} enum from '{}': field parse failure for variant '{}'",
                                         stringify!($enum_name),
                                         s,
                                         stringify!($variant)
                                     );
                                     Self::Invalid
                                }
                            }
                        }
                    ),*
                    // Unknown tag -> Invalid
                    _ => {
                        #[cfg(feature = "logs")]
                        router_env::logger::warn!(
                            "Failed to parse {} enum from '{}': unknown variant tag '{}'",
                            stringify!($enum_name),
                            s,
                            tag
                        );
                        Self::Invalid
                    },
                };
                Ok(result) // Always Ok because failure modes return Self::Invalid
            }
        }

        // Implement Serialize
        impl ::serde::Serialize for $enum_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                match self {
                    $(
                        Self::$variant { $field } => {
                            let s = format!("{}{}{}", stringify!($variant), $tag_delim, $field);
                            serializer.serialize_str(&s)
                        }
                    )*
                    // Handle Invalid variant
                    Self::Invalid => serializer.serialize_str("Invalid"),
                }
            }
        }

        // Implement Deserialize
        impl<'de> ::serde::Deserialize<'de> for $enum_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                struct EnumVisitor;

                impl<'de> ::serde::de::Visitor<'de> for EnumVisitor {
                    type Value = $enum_name;

                    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        formatter.write_str(concat!("a string like VariantName", $tag_delim, "field_data or 'Invalid'"))
                    }

                    // Leverage the FromStr implementation which now returns Self::Invalid on failure
                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: ::serde::de::Error,
                    {
                        // parse() now returns Result<Self, Infallible>
                        // We unwrap() the Ok because it's infallible.
                       Ok(value.parse::<$enum_name>().unwrap())
                    }

                     fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
                    where
                        E: ::serde::de::Error,
                    {
                        Ok(value.parse::<$enum_name>().unwrap())
                    }
                }

                deserializer.deserialize_str(EnumVisitor)
            }
        }

        // Implement Display
        impl core::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    $(
                        Self::$variant { $field } => {
                            write!(f, "{}{}{}", stringify!($variant), $tag_delim, $field)
                        }
                    )*
                     // Handle Invalid variant
                    Self::Invalid => write!(f, "Invalid"),
                }
            }
        }

        // Implement HasInvalidVariant trait
        impl $crate::types::HasInvalidVariant for $enum_name {
            fn is_invalid(&self) -> bool {
                matches!(self, Self::Invalid)
            }
        }
    };
}

// --- Tests ---
#[cfg(test)]
mod tests {
    use serde_json::{json, Value as JsonValue};

    use crate::impl_enum_str;

    impl_enum_str!(
        tag_delimiter = ":",
        #[derive(Debug, PartialEq, Clone)]
        pub enum TestEnum {
            VariantA { value: i32 },
            VariantB { text: String },
            VariantC { id: u64 },
            VariantJson { data: JsonValue },
        } // Note: Invalid variant is added automatically by the macro
    );

    #[test]
    fn test_enum_from_str_ok() {
        // Success cases just parse directly
        let parsed_a: TestEnum = "VariantA:42".parse().unwrap(); // Unwrapping Infallible is fine
        assert_eq!(parsed_a, TestEnum::VariantA { value: 42 });

        let parsed_b: TestEnum = "VariantB:hello world".parse().unwrap();
        assert_eq!(
            parsed_b,
            TestEnum::VariantB {
                text: "hello world".to_string()
            }
        );

        let parsed_c: TestEnum = "VariantC:123456789012345".parse().unwrap();
        assert_eq!(
            parsed_c,
            TestEnum::VariantC {
                id: 123456789012345
            }
        );

        let parsed_json: TestEnum = r#"VariantJson:{"ok":true}"#.parse().unwrap();
        assert_eq!(
            parsed_json,
            TestEnum::VariantJson {
                data: json!({"ok": true})
            }
        );
    }

    #[test]
    fn test_enum_from_str_failures_yield_invalid() {
        // Missing delimiter
        let parsed: TestEnum = "VariantA".parse().unwrap();
        assert_eq!(parsed, TestEnum::Invalid);

        // Unknown tag
        let parsed: TestEnum = "UnknownVariant:123".parse().unwrap();
        assert_eq!(parsed, TestEnum::Invalid);

        // Bad field data for i32
        let parsed: TestEnum = "VariantA:not_a_number".parse().unwrap();
        assert_eq!(parsed, TestEnum::Invalid);

        // Bad field data for JsonValue
        let parsed: TestEnum = r#"VariantJson:{"bad_json"#.parse().unwrap();
        assert_eq!(parsed, TestEnum::Invalid);

        // Empty field data for non-string (e.g., i32)
        let parsed: TestEnum = "VariantA:".parse().unwrap();
        assert_eq!(parsed, TestEnum::Invalid);

        // Empty field data for string IS valid for String type
        let parsed_str: TestEnum = "VariantB:".parse().unwrap();
        assert_eq!(
            parsed_str,
            TestEnum::VariantB {
                text: "".to_string()
            }
        );

        // Parsing the literal "Invalid" string
        let parsed_invalid_str: TestEnum = "Invalid".parse().unwrap();
        assert_eq!(parsed_invalid_str, TestEnum::Invalid);
    }

    #[test]
    fn test_enum_display_and_serialize() {
        // Display valid
        let value_a = TestEnum::VariantA { value: 99 };
        assert_eq!(value_a.to_string(), "VariantA:99");
        // Serialize valid
        let json_a = serde_json::to_string(&value_a).expect("Serialize A failed");
        assert_eq!(json_a, "\"VariantA:99\""); // Serializes to JSON string

        // Display Invalid
        let value_invalid = TestEnum::Invalid;
        assert_eq!(value_invalid.to_string(), "Invalid");
        // Serialize Invalid
        let json_invalid = serde_json::to_string(&value_invalid).expect("Serialize Invalid failed");
        assert_eq!(json_invalid, "\"Invalid\""); // Serializes to JSON string "Invalid"
    }

    #[test]
    fn test_enum_deserialize() {
        // Deserialize valid
        let input_a = "\"VariantA:123\"";
        let deserialized_a: TestEnum = serde_json::from_str(input_a).expect("Deserialize A failed");
        assert_eq!(deserialized_a, TestEnum::VariantA { value: 123 });

        // Deserialize explicit "Invalid"
        let input_invalid = "\"Invalid\"";
        let deserialized_invalid: TestEnum =
            serde_json::from_str(input_invalid).expect("Deserialize Invalid failed");
        assert_eq!(deserialized_invalid, TestEnum::Invalid);

        // Deserialize malformed string (according to macro rules) -> Invalid
        let input_malformed = "\"VariantA_no_delimiter\"";
        let deserialized_malformed: TestEnum =
            serde_json::from_str(input_malformed).expect("Deserialize malformed should succeed");
        assert_eq!(deserialized_malformed, TestEnum::Invalid);

        // Deserialize string with bad field data -> Invalid
        let input_bad_data = "\"VariantA:not_a_number\"";
        let deserialized_bad_data: TestEnum =
            serde_json::from_str(input_bad_data).expect("Deserialize bad data should succeed");
        assert_eq!(deserialized_bad_data, TestEnum::Invalid);
    }
}
