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
/// 4.  **Tag Delimiter:** The macro invocation must specify a `tag_delimeter` literal,
///     which is the character used to separate the variant name from the field data in
///     the string representation (e.g., `tag_delimeter = ":",`).
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
/// 6.  **Error Type:** The crate using this macro must define an error enum accessible via
///     `$crate::errors::ParsingError`. This error enum must:
///     * Include a variant like `EnumParseFailure(&'static str)`.
///     * Implement `core::fmt::Debug`, `core::fmt::Display`, and `std::error::Error`.
///
/// # Serialization and Deserialization (`serde`)
///
/// When `serde` features are enabled and the necessary traits are derived or implemented,
/// this macro implements `Serialize` and `Deserialize` for the enum:
///
/// **Serialization:** An enum value like `MyEnum::VariantA { value: 123 }` (with `tag_delimeter = ":",`)
///     will be serialized into the string `"VariantA:123"`. If serializing to JSON, this results
///     in a JSON string: `"\"VariantA:123\""`.
/// **Deserialization:** The macro expects a string matching the format `"VariantName<delimiter>FieldValue"`.
///     It uses the enum's `FromStr` implementation internally. When deserializing from JSON, it
///     expects a JSON string containing the correctly formatted value (e.g., `"\"VariantA:123\""`).
///
/// # `Display` and `FromStr`
///
/// **`Display`:** Formats the enum into the `"VariantName<delimiter>FieldValue"` string.
///     `your_enum_value.to_string()` will produce this format.
/// **`FromStr`:** Parses the `"VariantName<delimiter>FieldValue"` string back into an enum value.
///     `"VariantA:123".parse::<MyEnum>()` will attempt this conversion.
///
/// # Example
///
/// ```rust
/// // Assume crate::errors::ParsingError exists and is compatible
/// # mod errors {
/// #   #[derive(Debug, PartialEq, Eq)]
/// #   pub enum ParsingError { EnumParseFailure(&'static str) }
/// #   impl core::fmt::Display for ParsingError {
/// #       fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
/// #           match self { ParsingError::EnumParseFailure(s) => write!(f, "{}", s) }
/// #       }
/// #   }
/// #   impl std::error::Error for ParsingError {}
/// # }
/// use std::str::FromStr;
/// use serde::{Serialize, Deserialize}; // Make serde traits visible if needed outside macro
///
/// // Use the macro to define the enum and implement traits
/// crate::impl_enum_str!(
///     tag_delimeter = ":",
///     #[derive(Debug, PartialEq, Clone)] // Add other derives as needed
///     pub enum Setting {
///         Timeout { duration_ms: u32 },
///         Username { name: String },
///         MaxRetries { count: u8 },
///     }
/// );
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Display / to_string()
///     let setting1 = Setting::Timeout { duration_ms: 5000 };
///     assert_eq!(setting1.to_string(), "Timeout:5000");
///
///     let setting2 = Setting::Username { name: "admin".to_string() };
///     assert_eq!(setting2.to_string(), "Username:admin");
///
///     // FromStr / parse()
///     let parsed_setting: Setting = "MaxRetries:3".parse()?;
///     assert_eq!(parsed_setting, Setting::MaxRetries { count: 3 });
///
///     let invalid_parse = "Username".parse::<Setting>();
///     assert!(invalid_parse.is_err());
///     assert!(invalid_parse.unwrap_err().to_string().contains("missing tag delimeter ':'"));
///
///     let data_parse_err = "Timeout:fast".parse::<Setting>();
///     assert!(data_parse_err.is_err());
///     assert!(data_parse_err.unwrap_err().to_string().contains("Failed to parse field data"));
///
///     // Serde (e.g., with JSON)
///     // Note: The custom Serialize impl produces a single string
///     let json_output = serde_json::to_string(&setting1)?;
///     assert_eq!(json_output, "\"Timeout:5000\""); // Note the outer quotes - it's a JSON string
///
///     Ok(())
/// }
///
/// # // Mock macro definition for doctest purposes
/// # #[macro_export] macro_rules! impl_enum_str { ($($tt:tt)*) => { $($tt)* } }
/// ```
#[macro_export]
macro_rules! impl_enum_str {
    (
        tag_delimeter = $tag_delim:literal,
        $(#[$enum_attr:meta])*
        pub enum $enum_name:ident {
            $(
                $(#[$variant_attr:meta])*
                $variant:ident {
                    $(#[$field_attr:meta])*
                    $field:ident : $field_ty:ty $(,)?
                }
            ),* $(,)? // Allow optional trailing comma after last variant
        }
    ) => {
        $(#[$enum_attr])*
        pub enum $enum_name {
            $(
                $(#[$variant_attr])*
                $variant {
                    $(#[$field_attr])*
                    $field : $field_ty
                }
            ),*
        }

        // Implement FromStr
        impl core::str::FromStr for $enum_name {
            type Err = $crate::errors::ParsingError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let Some((tag, associated_data)) = s.split_once($tag_delim) else {
                    return Err($crate::errors::ParsingError::EnumParseFailure(
                        concat!("Invalid format: missing tag delimeter '", $tag_delim, "'")
                    ));
                };

                match tag {
                    $(
                        stringify!($variant) => {
                            let parsed_field = associated_data.parse::<$field_ty>()
                                .map_err(|_| $crate::errors::ParsingError::EnumParseFailure(
                                    concat!("Failed to parse field data for variant '", stringify!($variant), "' as type '", stringify!($field_ty), "'")
                                ))?;

                            Ok($enum_name::$variant {
                                // Use the captured field name `$field`
                                $field: parsed_field
                            })
                        }
                    ),*
                    _ => Err($crate::errors::ParsingError::EnumParseFailure("Unknown variant tag")),
                }
            }
        }

        // Implement Serialize for the enum.
        impl ::serde::Serialize for $enum_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                let s = match self {
                    $(
                        // Destructure the single field using its captured name `$field`
                        $enum_name::$variant { $field, } => {
                            // Format using the variant name, delimiter, and the field's Display impl
                            format!("{}{}{}", stringify!($variant), $tag_delim, $field)
                        }
                    ),*
                };
                serializer.serialize_str(&s)
            }
        }

        // Implement Deserialize for the enum.
        impl<'de> ::serde::Deserialize<'de> for $enum_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                struct EnumVisitor;

                impl<'de> ::serde::de::Visitor<'de> for EnumVisitor {
                    type Value = $enum_name;

                    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        formatter.write_str(concat!("a string in the format VariantName", $tag_delim, "field_data"))
                    }

                    // We leverage the FromStr implementation for parsing the string
                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: ::serde::de::Error,
                    {
                        value.parse::<$enum_name>().map_err(::serde::de::Error::custom)
                    }

                    // Optionally implement visit_string for owned strings
                     fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
                    where
                        E: ::serde::de::Error,
                    {
                        self.visit_str(&value)
                    }
                }

                deserializer.deserialize_str(EnumVisitor)
            }
        }

        // Implement Display so that to_string() works.
        impl core::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    $(
                        // Destructure the single field using its captured name `$field`
                        $enum_name::$variant { $field, } => {
                            // Write using the variant name, delimiter, and the field's Display impl
                            write!(f, "{}{}{}", stringify!($variant), $tag_delim, $field)
                        }
                    ),*
                }
            }
        }
    };
}

// --- Tests ---
#[cfg(test)]
mod tests {
    #![allow(clippy::panic, clippy::expect_used)]
    use core::str::FromStr;

    use crate::{errors::ParsingError, impl_enum_str};

    impl_enum_str!(
        tag_delimeter = ":",
        #[derive(Debug, PartialEq, Clone)]
        pub enum TestEnum {
            VariantA { value: i32 },
            VariantB { text: String },
            VariantC { id: u64 },
        }
    );
    #[test]
    fn test_enum_from_str_int() {
        // Declaration: fn test_name()
        let input = "VariantA:42";
        let parsed = TestEnum::from_str(input).expect("Parsing 'VariantA:42' should succeed");
        assert_eq!(parsed, TestEnum::VariantA { value: 42 });
    }

    #[test]
    fn test_enum_from_str_string() {
        // Declaration: fn test_name()
        let input = "VariantB:hello world";
        let parsed =
            TestEnum::from_str(input).expect("Parsing 'VariantB:hello world' should succeed");
        assert_eq!(
            parsed,
            TestEnum::VariantB {
                text: "hello world".to_string()
            }
        );
    }

    #[test]
    fn test_enum_from_str_u64() {
        // Declaration: fn test_name()
        let input = "VariantC:123456789012345";
        let parsed =
            TestEnum::from_str(input).expect("Parsing 'VariantC:123456789012345' should succeed");
        assert_eq!(
            parsed,
            TestEnum::VariantC {
                id: 123456789012345
            }
        );
    }

    #[test]
    fn test_enum_to_string_int() {
        // Declaration: fn test_name()
        let value = TestEnum::VariantA { value: 99 };
        assert_eq!(value.to_string(), "VariantA:99");
    }

    #[test]
    fn test_enum_to_string_string() {
        // Declaration: fn test_name()
        let value = TestEnum::VariantB {
            text: "test string".to_string(),
        };
        assert_eq!(value.to_string(), "VariantB:test string");
    }

    #[test]
    fn test_enum_to_string_u64() {
        // Declaration: fn test_name()
        let value = TestEnum::VariantC { id: 98765 };
        assert_eq!(value.to_string(), "VariantC:98765");
    }

    #[test]
    fn test_enum_serialize_deserialize_string() {
        // Declaration: fn test_name()
        let value = TestEnum::VariantB {
            text: "test serialization".to_string(),
        };
        let serialized = serde_json::to_string(&value).expect("Serialization should succeed");
        assert_eq!(serialized, "\"VariantB:test serialization\"");
        let deserialized: TestEnum =
            serde_json::from_str(&serialized).expect("Deserialization should succeed");
        assert_eq!(value, deserialized);
    }

    #[test]
    fn test_enum_serialize_deserialize_int() {
        // Declaration: fn test_name()
        let value = TestEnum::VariantA { value: -10 };
        let serialized = serde_json::to_string(&value).expect("Serialization should succeed");
        assert_eq!(serialized, "\"VariantA:-10\"");
        let deserialized: TestEnum =
            serde_json::from_str(&serialized).expect("Deserialization should succeed");
        assert_eq!(value, deserialized);
    }

    #[test]
    fn test_enum_from_str_invalid_format() {
        // Declaration: fn test_name()
        let input = "InvalidFormat";
        let result = TestEnum::from_str(input);
        if let Err(e) = result {
            assert_eq!(
                e,
                ParsingError::EnumParseFailure("Invalid format: missing tag delimeter ':'")
            );
        } else {
            panic!("Parsing '{}' should have failed, but it succeeded.", input);
        }
    }

    #[test]
    fn test_enum_from_str_empty_data() {
        // Declaration: fn test_name()
        // Test empty data for i32 (expect error)
        let input_int = "VariantA:";
        let result_int = TestEnum::from_str(input_int);
        if let Err(ref e) = result_int {
            let msg = e.to_string();
            assert!(
                msg.contains("Failed to parse field data"),
                "Error message mismatch: '{}'",
                msg
            );
            assert!(
                msg.contains("VariantA"),
                "Error message mismatch: '{}'",
                msg
            );
        } else {
            panic!(
                "Parsing '{}' should have failed, but it succeeded.",
                input_int
            );
        }

        // Test empty data for String (expect success)
        let input_str = "VariantB:";
        let parsed_str = TestEnum::from_str(input_str)
            .expect("Parsing 'VariantB:' should succeed for String field");
        assert_eq!(
            parsed_str,
            TestEnum::VariantB {
                text: "".to_string()
            }
        );

        // Test empty data for u64 (expect error)
        let input_u64 = "VariantC:";
        let result_u64 = TestEnum::from_str(input_u64);
        if let Err(ref e) = result_u64 {
            let msg = e.to_string();
            assert!(
                msg.contains("Failed to parse field data"),
                "Error message mismatch: '{}'",
                msg
            );
            assert!(
                msg.contains("VariantC"),
                "Error message mismatch: '{}'",
                msg
            );
        } else {
            panic!(
                "Parsing '{}' should have failed, but it succeeded.",
                input_u64
            );
        }
    }

    #[test]
    fn test_enum_from_str_unknown_variant() {
        // Declaration: fn test_name()
        let input = "UnknownVariant:123";
        let result = TestEnum::from_str(input);
        if let Err(e) = result {
            assert_eq!(e, ParsingError::EnumParseFailure("Unknown variant tag"));
        } else {
            panic!("Parsing '{}' should have failed, but it succeeded.", input);
        }
    }
}
