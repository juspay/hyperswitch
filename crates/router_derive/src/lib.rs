//! Utility macros for the `router` crate.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use syn::parse_macro_input;

use crate::macros::diesel::DieselEnumMeta;

mod macros;

/// Uses the [`Debug`][Debug] implementation of a type to derive its [`Display`][Display]
/// implementation.
///
/// Causes a compilation error if the type doesn't implement the [`Debug`][Debug] trait.
///
/// [Debug]: ::core::fmt::Debug
/// [Display]: ::core::fmt::Display
///
/// # Example
///
/// ```
/// use router_derive::DebugAsDisplay;
///
/// #[derive(Debug, DebugAsDisplay)]
/// struct Point {
///     x: f32,
///     y: f32,
/// }
///
/// #[derive(Debug, DebugAsDisplay)]
/// enum Color {
///     Red,
///     Green,
///     Blue,
/// }
/// ```
#[proc_macro_derive(DebugAsDisplay)]
/// This function takes a proc_macro::TokenStream as input and uses the syn crate to parse it as a syn::DeriveInput. It then calls the debug_as_display_inner method from the macros module, passing the parsed input as a parameter. The result is then converted into a proc_macro::TokenStream and returned.
pub fn debug_as_display_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);

    let tokens =
        macros::debug_as_display_inner(&ast).unwrap_or_else(|error| error.to_compile_error());
    tokens.into()
}

/// Derives the boilerplate code required for using an enum with `diesel` and a PostgreSQL database.
/// The enum is required to implement (or derive) the [`ToString`][ToString] and the
/// [`FromStr`][FromStr] traits for this derive macro to be used.
///
/// Works in tandem with the [`diesel_enum`][diesel_enum] attribute macro to achieve the desired
/// results.
///
/// [diesel_enum]: macro@crate::diesel_enum
/// [FromStr]: ::core::str::FromStr
/// [ToString]: ::std::string::ToString
///
/// # Example
///
/// ```
/// use router_derive::diesel_enum;
///
/// // Deriving `FromStr` and `ToString` using the `strum` crate, you can also implement it
/// // yourself if required.
/// #[derive(strum::Display, strum::EnumString)]
/// #[derive(Debug)]
/// #[diesel_enum(storage_type = "pg_enum")]
/// enum Color {
///     Red,
///     Green,
///     Blue,
/// }
/// ```
#[proc_macro_derive(DieselEnum, attributes(storage_type))]
/// This method is a procedural macro that takes a `TokenStream` as input, parses it into a `DeriveInput` using the `syn` crate, and then calls the `diesel_enum_derive_inner` method from the `macros` module to generate the necessary code for deriving the `Enum` trait for a given enum. If the `diesel_enum_derive_inner` method returns an error, it is converted into a compile error. The resulting tokens are then converted back into a `TokenStream` and returned.
pub fn diesel_enum_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let tokens =
        macros::diesel_enum_derive_inner(&ast).unwrap_or_else(|error| error.to_compile_error());
    tokens.into()
}

/// Similar to [`DieselEnum`] but uses text when storing in the database, this is to avoid
/// making changes to the database when the enum variants are added or modified
///
/// # Example
/// [DieselEnum]: macro@crate::diesel_enum
///
/// ```
/// use router_derive::{diesel_enum};
///
/// // Deriving `FromStr` and `ToString` using the `strum` crate, you can also implement it
/// // yourself if required.
/// #[derive(strum::Display, strum::EnumString)]
/// #[derive(Debug)]
/// #[diesel_enum(storage_type = "text")]
/// enum Color {
///     Red,
///     Green,
///     Blue,
/// }
/// ```
#[proc_macro_derive(DieselEnumText)]
/// This method takes a proc_macro::TokenStream as input, parses it as a syn::DeriveInput, and then passes it to the diesel_enum_text_derive_inner method from the macros module. It then converts the resulting tokens into a proc_macro::TokenStream and returns it.
pub fn diesel_enum_derive_string(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let tokens = macros::diesel_enum_text_derive_inner(&ast)
        .unwrap_or_else(|error| error.to_compile_error());
    tokens.into()
}

/// Derives the boilerplate code required for using an enum with `diesel` and a PostgreSQL database.
///
/// Storage Type can either be "text" or "db_enum"
/// Choosing text will store the enum as text in the database, whereas db_enum will map it to the
/// corresponding database enum
///
/// Works in tandem with the [`DieselEnum`][DieselEnum] derive macro to achieve the desired results.
/// The enum is required to implement (or derive) the [`ToString`][ToString] and the
/// [`FromStr`][FromStr] traits for the [`DieselEnum`][DieselEnum] derive macro to be used.
///
/// [DieselEnum]: crate::DieselEnum
/// [FromStr]: ::core::str::FromStr
/// [ToString]: ::std::string::ToString
///
/// # Example
///
/// ```
/// use router_derive::{diesel_enum};
///
/// // Deriving `FromStr` and `ToString` using the `strum` crate, you can also implement it
/// // yourself if required. (Required by the DieselEnum derive macro.)
/// #[derive(strum::Display, strum::EnumString)]
/// #[derive(Debug)]
/// #[diesel_enum(storage_type = "text")]
/// enum Color {
///     Red,
///     Green,
///     Blue,
/// }
/// ```
#[proc_macro_attribute]
/// This method is a procedural macro that takes in two token streams as input. It parses the first token stream as a custom metadata type called DieselEnumMeta, and the second token stream as a Rust enum type using the syn crate. It then calls a custom attribute macro from the macros::diesel module, passing in the parsed metadata and enum as arguments. If the attribute macro returns a Result containing an error, the error is converted to a compile error. The resulting token stream is returned as output.
pub fn diesel_enum(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args_parsed = parse_macro_input!(args as DieselEnumMeta);
    let item = syn::parse_macro_input!(item as syn::ItemEnum);

    macros::diesel::diesel_enum_attribute_macro(args_parsed, &item)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// A derive macro which generates the setter functions for any struct with fields
/// # Example
/// ```
/// use router_derive::Setter;
/// struct Test {
///     test:u32
/// }
/// ```
/// The above Example will expand to
/// ```
/// impl Test {
///     fn set_test(&mut self,val:u32)->&mut Self {
///         self.test = val;
///         self
///     }
/// }
/// ```
///

/// # Panics
///
/// Panics if a struct without named fields is provided as input to the macro
// FIXME: Remove allowed warnings, raise compile errors in a better manner instead of panicking
#[allow(clippy::panic, clippy::unwrap_used)]
#[proc_macro_derive(Setter, attributes(auth_based))]
/// Parses a given input token stream and generates a series of setter methods for a given struct, based on its fields. Each setter method allows for setting a specific field of the struct and returns a mutable reference to the struct itself.
pub fn setter(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let ident = &input.ident;

    // All the fields in the parent struct
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = input.data
    {
        named
    } else {
        // FIXME: Use `compile_error!()` instead
        panic!("You can't use this proc-macro on structs without fields");
    };

    // Methods in the build struct like if the struct is
    // Struct i {n: u32}
    // this will be
    // pub fn set_n(&mut self,n: u32)
    let build_methods = fields.iter().map(|f| {
        let name = f.ident.as_ref().unwrap();
        let method_name = format!("set_{name}");
        let method_ident = syn::Ident::new(&method_name, name.span());
        let ty = &f.ty;
        if check_if_auth_based_attr_is_present(f, "auth_based") {
            quote::quote! {
                /// Sets the value of the field {name} and returns a mutable reference to the struct if the `is_merchant_flow` flag is true. Otherwise, it only returns a mutable reference to the struct.
                pub fn #method_ident(&mut self, val:#ty, is_merchant_flow: bool)->&mut Self{
                    if is_merchant_flow {
                        self.#name = val;
                    }
                    self
                }
            }
        } else {
            quote::quote! {
                /// Sets the value of the field {name} and returns a mutable reference to the struct.
                pub fn #method_ident(&mut self, val:#ty)->&mut Self{
                    self.#name = val;
                    self
                }
            }
        }
    });

    let output = quote::quote! {
    #[automatically_derived]
    impl #ident {
            #(#build_methods)*
        }

    };

    output.into()
}

#[inline]
/// Checks if the given attribute identifier is present in the list of attributes for the given field.
fn check_if_auth_based_attr_is_present(f: &syn::Field, ident: &str) -> bool {
    for i in f.attrs.iter() {
        if i.path().is_ident(ident) {
            return true;
        }
    }
    false
}

/// Derives the [`Serialize`][Serialize] implementation for error responses that are returned by
/// the API server.
///
/// This macro can be only used with enums. In addition to deriving [`Serialize`][Serialize], this
/// macro provides three methods: `error_type()`, `error_code()` and `error_message()`. Each enum
/// variant must have three required fields:
///
/// - `error_type`: This must be an enum variant which is returned by the `error_type()` method.
/// - `code`: A string error code, returned by the `error_code()` method.
/// - `message`: A string error message, returned by the `error_message()` method. The message
///   provided will directly be passed to `format!()`.
///
/// The return type of the `error_type()` method is provided by the `error_type_enum` field
/// annotated to the entire enum. Thus, all enum variants provided to the `error_type` field must
/// be variants of the enum provided to `error_type_enum` field. In addition, the enum passed to
/// the `error_type_enum` field must implement [`Serialize`][Serialize].
///
/// **NOTE:** This macro does not implement the [`Display`][Display] trait.
///
/// # Example
///
/// ```
/// use router_derive::ApiError;
///
/// #[derive(Clone, Debug, serde::Serialize)]
/// enum ErrorType {
///     StartupError,
///     InternalError,
///     SerdeError,
/// }
///
/// #[derive(Debug, ApiError)]
/// #[error(error_type_enum = ErrorType)]
/// enum MyError {
///     #[error(error_type = ErrorType::StartupError, code = "E001", message = "Failed to read configuration")]
///     ConfigurationError,
///     #[error(error_type = ErrorType::InternalError, code = "E002", message = "A database error occurred")]
///     DatabaseError,
///     #[error(error_type = ErrorType::SerdeError, code = "E003", message = "Failed to deserialize object")]
///     DeserializationError,
///     #[error(error_type = ErrorType::SerdeError, code = "E004", message = "Failed to serialize object")]
///     SerializationError,
/// }
///
/// impl ::std::fmt::Display for MyError {
///     fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(), ::core::fmt::Error> {
///         f.write_str(&self.error_message())
///     }
/// }
/// ```
///
/// # The Generated `Serialize` Implementation
///
/// - For a simple enum variant with no fields, the generated [`Serialize`][Serialize]
/// implementation has only three fields, `type`, `code` and `message`:
///
/// ```
/// # use router_derive::ApiError;
/// # #[derive(Clone, Debug, serde::Serialize)]
/// # enum ErrorType {
/// #     StartupError,
/// # }
/// #[derive(Debug, ApiError)]
/// #[error(error_type_enum = ErrorType)]
/// enum MyError {
///     #[error(error_type = ErrorType::StartupError, code = "E001", message = "Failed to read configuration")]
///     ConfigurationError,
///     // ...
/// }
/// # impl ::std::fmt::Display for MyError {
/// #     fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(), ::core::fmt::Error> {
/// #         f.write_str(&self.error_message())
/// #     }
/// # }
///
/// let json = serde_json::json!({
///     "type": "StartupError",
///     "code": "E001",
///     "message": "Failed to read configuration"
/// });
/// assert_eq!(serde_json::to_value(MyError::ConfigurationError).unwrap(), json);
/// ```
///
/// - For an enum variant with named fields, the generated [`Serialize`][Serialize] implementation
/// includes three mandatory fields, `type`, `code` and `message`, and any other fields not
/// included in the message:
///
/// ```
/// # use router_derive::ApiError;
/// # #[derive(Clone, Debug, serde::Serialize)]
/// # enum ErrorType {
/// #     StartupError,
/// # }
/// #[derive(Debug, ApiError)]
/// #[error(error_type_enum = ErrorType)]
/// enum MyError {
///     #[error(error_type = ErrorType::StartupError, code = "E001", message = "Failed to read configuration file: {file_path}")]
///     ConfigurationError { file_path: String, reason: String },
///     // ...
/// }
/// # impl ::std::fmt::Display for MyError {
/// #     fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(), ::core::fmt::Error> {
/// #         f.write_str(&self.error_message())
/// #     }
/// # }
///
/// let json = serde_json::json!({
///     "type": "StartupError",
///     "code": "E001",
///     "message": "Failed to read configuration file: config.toml",
///     "reason": "File not found"
/// });
/// let error = MyError::ConfigurationError{
///     file_path: "config.toml".to_string(),
///     reason: "File not found".to_string(),
/// };
/// assert_eq!(serde_json::to_value(error).unwrap(), json);
/// ```
///
/// [Serialize]: https://docs.rs/serde/latest/serde/trait.Serialize.html
/// [Display]: ::core::fmt::Display
#[proc_macro_derive(ApiError, attributes(error))]
/// This method takes a proc_macro::TokenStream as input and parses it as a syn::DeriveInput. It then calls the api_error_derive_inner method from the macros module with the parsed input, and returns the resulting proc_macro::TokenStream.
pub fn api_error_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);

    let tokens =
        macros::api_error_derive_inner(&ast).unwrap_or_else(|error| error.to_compile_error());
    tokens.into()
}

/// Derives the `core::payments::Operation` trait on a type with a default base
/// implementation.
///
/// ## Usage
/// On deriving, the conversion functions to be implemented need to be specified in an helper
/// attribute `#[operation(..)]`. To derive all conversion functions, use `#[operation(all)]`. To
/// derive specific conversion functions, pass the required identifiers to the attribute.
/// `#[operation(validate_request, get_tracker)]`. Available conversions are listed below :-
///
/// - validate_request
/// - get_tracker
/// - domain
/// - update_tracker
///
/// ## Example
/// ```
/// use router_derive::Operation;
///
/// #[derive(Operation)]
/// #[operation(all)]
/// struct Point {
///     x: u64,
///     y: u64
/// }
///
/// // The above will expand to this
/// const _: () = {
///     use crate::core::errors::RouterResult;
///     use crate::core::payments::{GetTracker, PaymentData, UpdateTracker, ValidateRequest};
///     impl crate::core::payments::Operation for Point {
///         fn to_validate_request(&self) -> RouterResult<&dyn ValidateRequest> {
///             Ok(self)
///         }
///         fn to_get_tracker(&self) -> RouterResult<&dyn GetTracker<PaymentData>> {
///             Ok(self)
///         }
///         fn to_domain(&self) -> RouterResult<&dyn Domain> {
///             Ok(self)
///         }
///         fn to_update_tracker(&self) -> RouterResult<&dyn UpdateTracker<PaymentData>> {
///             Ok(self)
///         }
///     }
///     impl crate::core::payments::Operation for &Point {
///         fn to_validate_request(&self) -> RouterResult<&dyn ValidateRequest> {
///             Ok(*self)
///         }
///         fn to_get_tracker(&self) -> RouterResult<&dyn GetTracker<PaymentData>> {
///             Ok(*self)
///         }
///         fn to_domain(&self) -> RouterResult<&dyn Domain> {
///             Ok(*self)
///         }
///         fn to_update_tracker(&self) -> RouterResult<&dyn UpdateTracker<PaymentData>> {
///             Ok(*self)
///         }
///     }
/// };
///
/// #[derive(Operation)]
/// #[operation(validate_request, get_tracker)]
/// struct Point3 {
///     x: u64,
///     y: u64,
///     z: u64
/// }
///
/// // The above will expand to this
/// const _: () = {
///     use crate::core::errors::RouterResult;
///     use crate::core::payments::{GetTracker, PaymentData, UpdateTracker, ValidateRequest};
///     impl crate::core::payments::Operation for Point3 {
///         fn to_validate_request(&self) -> RouterResult<&dyn ValidateRequest> {
///             Ok(self)
///         }
///         fn to_get_tracker(&self) -> RouterResult<&dyn GetTracker<PaymentData>> {
///             Ok(self)
///         }
///     }
///     impl crate::core::payments::Operation for &Point3 {
///         fn to_validate_request(&self) -> RouterResult<&dyn ValidateRequest> {
///             Ok(*self)
///         }
///         fn to_get_tracker(&self) -> RouterResult<&dyn GetTracker<PaymentData>> {
///             Ok(*self)
///         }
///     }
/// };
///
/// ```
///
/// The `const _: () = {}` allows us to import stuff with `use` without affecting the module
/// imports, since use statements are not allowed inside of impl blocks. This technique is
/// used by `diesel`.
#[proc_macro_derive(PaymentOperation, attributes(operation))]
/// This method takes a proc_macro::TokenStream as input and derives an operation based on the input. It first parses the input into a syn::DeriveInput, then calls the operation_derive_inner method from the macros::operation module to derive the operation. If an error occurs during the derivation process, it returns a compile error.
pub fn operation_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    macros::operation::operation_derive_inner(input)
        .unwrap_or_else(|err| err.to_compile_error().into())
}

/// Generates different schemas with the ability to mark few fields as mandatory for certain schema
/// Usage
/// ```
/// #[derive(PolymorphicSchema)]
/// #[generate_schemas(PaymentsCreateRequest, PaymentsConfirmRequest)]
/// struct PaymentsRequest {
///     #[mandatory_in(PaymentsCreateRequest)]
///     amount: Option<u64>,
///     #[mandatory_in(PaymentsCreateRequest)]
///     currency: Option<String>,
///     payment_method: String,
/// }
/// ```
///
/// This will create two structs `PaymentsCreateRequest` and `PaymentsConfirmRequest` as follows
/// It will retain all the other attributes that are used in the original struct, and only consume
/// the #[mandatory_in] attribute to generate schemas
///
/// ```
/// #[derive(utoipa::ToSchema)]
/// struct PaymentsCreateRequest {
///     #[schema(required = true)]
///     amount: Option<u64>,
///
///     #[schema(required = true)]
///     currency: Option<String>,
///
///     payment_method: String,
/// }
///
/// #[derive(utoipa::ToSchema)]
/// struct PaymentsConfirmRequest {
///     amount: Option<u64>,
///     currency: Option<String>,
///     payment_method: String,
/// }
/// ```

#[proc_macro_derive(
    PolymorphicSchema,
    attributes(mandatory_in, generate_schemas, remove_in)
)]
/// This method takes a `proc_macro::TokenStream` as input and parses it as a `syn::DeriveInput`. 
/// It then calls the `polymorphic_macro_derive_inner` method from the `macros` module and 
/// processes the result. If there is an error, it is converted into a compile error. The final 
/// result is returned as a `proc_macro::TokenStream`.
pub fn polymorphic_schema(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    macros::polymorphic_macro_derive_inner(input)
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}

/// Implements the `Validate` trait to check if the config variable is present
/// Usage
/// ```
/// #[derive(ConfigValidate)]
/// struct Connectors {
///     pub stripe: ConnectorParams,
///     pub checkout: ConnectorParams
/// }
/// ```
///
/// This will call the `validate()` function for all the fields in the struct
///
/// ```
/// impl Connectors {
///      fn validate(&self) -> Result<(), ApplicationError> {
///         self.stripe.validate()?;
///         self.checkout.validate()?;
///      }
/// }
/// ```
#[proc_macro_derive(ConfigValidate)]
/// This method takes a `proc_macro::TokenStream` as input, parses it as a `syn::DeriveInput`,
/// and then passes it to the `macros::misc::validate_config` method to validate the configuration.
/// If validation fails, it converts the error into a compile error. Finally, it returns the result
/// as a `proc_macro::TokenStream`.
pub fn validate_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    macros::misc::validate_config(input)
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}

/// Generates the function to get the value out of enum variant
/// Usage
/// ```
/// #[derive(TryGetEnumVariant)]
/// #[error(RedisError(UnknownResult))]
/// enum Result {
///     Set(String),
///     Get(i32)
/// }
/// ```
///
/// This will generate the function to get `String` and `i32` out of the variants
///
/// ```
/// impl Result {
///     fn try_into_get(&self)-> Result<i32, RedisError> {
///         match self {
///             Self::Get(a) => Ok(a),
///             _=>Err(RedisError::UnknownResult)
///         }
///     }
///
///     fn try_into_set(&self)-> Result<String, RedisError> {
///         match self {
///             Self::Set(a) => Ok(a),
///             _=> Err(RedisError::UnknownResult)
///         }
///     }
/// }
#[proc_macro_derive(TryGetEnumVariant, attributes(error))]
/// This function takes a `proc_macro::TokenStream` as input and tries to parse it as a `syn::DeriveInput`.
/// It then calls the `try_get_enum_variant` function from the `macros::try_get_enum` module, passing the parsed input as an argument.
/// If successful, it returns the result as a `proc_macro::TokenStream`. If an error occurs, it converts the error into a compile error and returns it as a `proc_macro::TokenStream`.
pub fn try_get_enum_variant(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    macros::try_get_enum::try_get_enum_variant(input)
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}
