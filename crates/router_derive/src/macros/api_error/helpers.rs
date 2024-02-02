use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::Parse, spanned::Spanned, DeriveInput, Field, Fields, LitStr, Token, TypePath, Variant,
};

use crate::macros::helpers::{get_metadata_inner, occurrence_error};

mod keyword {
    use syn::custom_keyword;

    // Enum metadata
    custom_keyword!(error_type_enum);

    // Variant metadata
    custom_keyword!(error_type);
    custom_keyword!(code);
    custom_keyword!(message);
    custom_keyword!(ignore);
}

enum EnumMeta {
    ErrorTypeEnum {
        keyword: keyword::error_type_enum,
        value: TypePath,
    },
}

impl Parse for EnumMeta {
        /// Parses the input stream and returns a result containing the parsed value or an error.
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(keyword::error_type_enum) {
            let keyword = input.parse()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            Ok(Self::ErrorTypeEnum { keyword, value })
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for EnumMeta {
        /// This method takes a reference to a TokenStream and modifies it by adding tokens
    /// based on the type of ErrorEnum. If the ErrorEnum is of type ErrorTypeEnum, it
    /// extracts the keyword from it and adds it to the TokenStream using the to_tokens method.
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::ErrorTypeEnum { keyword, .. } => keyword.to_tokens(tokens),
        }
    }
}

trait DeriveInputExt {
    /// Get all the error metadata associated with an enum.
    fn get_metadata(&self) -> syn::Result<Vec<EnumMeta>>;
}

impl DeriveInputExt for DeriveInput {
        /// Retrieves the metadata of the enum by parsing its attributes.
    fn get_metadata(&self) -> syn::Result<Vec<EnumMeta>> {
        get_metadata_inner("error", &self.attrs)
    }
}

pub(super) trait HasErrorTypeProperties {
    fn get_type_properties(&self) -> syn::Result<ErrorTypeProperties>;
}

#[derive(Clone, Debug, Default)]
pub(super) struct ErrorTypeProperties {
    pub error_type_enum: Option<TypePath>,
}

impl HasErrorTypeProperties for DeriveInput {
        /// Retrieves the properties of the error type, such as the error type enum keyword and value.
    /// If the error_type_enum attribute is not found, it returns a syn::Error.
    fn get_type_properties(&self) -> syn::Result<ErrorTypeProperties> {
        let mut output = ErrorTypeProperties::default();

        let mut error_type_enum_keyword = None;
        for meta in self.get_metadata()? {
            match meta {
                EnumMeta::ErrorTypeEnum { keyword, value } => {
                    if let Some(first_keyword) = error_type_enum_keyword {
                        return Err(occurrence_error(first_keyword, keyword, "error_type_enum"));
                    }

                    error_type_enum_keyword = Some(keyword);
                    output.error_type_enum = Some(value);
                }
            }
        }

        if output.error_type_enum.is_none() {
            return Err(syn::Error::new(
                self.span(),
                "error(error_type_enum) attribute not found",
            ));
        }

        Ok(output)
    }
}

enum VariantMeta {
    ErrorType {
        keyword: keyword::error_type,
        value: TypePath,
    },
    Code {
        keyword: keyword::code,
        value: LitStr,
    },
    Message {
        keyword: keyword::message,
        value: LitStr,
    },
    Ignore {
        keyword: keyword::ignore,
        value: LitStr,
    },
}

impl Parse for VariantMeta {
        /// Parses the input parse stream and returns a result based on the keyword lookahead.
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(keyword::error_type) {
            let keyword = input.parse()?;
            let _: Token![=] = input.parse()?;
            let value = input.parse()?;
            Ok(Self::ErrorType { keyword, value })
        } else if lookahead.peek(keyword::code) {
            let keyword = input.parse()?;
            let _: Token![=] = input.parse()?;
            let value = input.parse()?;
            Ok(Self::Code { keyword, value })
        } else if lookahead.peek(keyword::message) {
            let keyword = input.parse()?;
            let _: Token![=] = input.parse()?;
            let value = input.parse()?;
            Ok(Self::Message { keyword, value })
        } else if lookahead.peek(keyword::ignore) {
            let keyword = input.parse()?;
            let _: Token![=] = input.parse()?;
            let value = input.parse()?;
            Ok(Self::Ignore { keyword, value })
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for VariantMeta {
        /// Converts the enum variant to its corresponding tokens and appends them to the provided TokenStream.
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::ErrorType { keyword, .. } => keyword.to_tokens(tokens),
            Self::Code { keyword, .. } => keyword.to_tokens(tokens),
            Self::Message { keyword, .. } => keyword.to_tokens(tokens),
            Self::Ignore { keyword, .. } => keyword.to_tokens(tokens),
        }
    }
}

trait VariantExt {
    /// Get all the error metadata associated with an enum variant.
    fn get_metadata(&self) -> syn::Result<Vec<VariantMeta>>;
}

impl VariantExt for Variant {
        /// Retrieves the metadata for the given error variant by parsing the attributes of the enum.
    /// 
    /// # Arguments
    /// 
    /// * `self` - A reference to the current instance of the enum.
    /// 
    /// # Returns
    /// 
    /// A Result containing a vector of VariantMeta, representing the metadata of the error variant.
    /// 
    fn get_metadata(&self) -> syn::Result<Vec<VariantMeta>> {
        get_metadata_inner("error", &self.attrs)
    }
}

pub(super) trait HasErrorVariantProperties {
    fn get_variant_properties(&self) -> syn::Result<ErrorVariantProperties>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct ErrorVariantProperties {
    pub error_type: Option<TypePath>,
    pub code: Option<LitStr>,
    pub message: Option<LitStr>,
    pub ignore: std::collections::HashSet<String>,
}

impl HasErrorVariantProperties for Variant {
        /// Retrieves the properties of an error variant by parsing its metadata. This method iterates through the metadata of the error variant and populates an ErrorVariantProperties struct with the error type, code, message, and ignore keywords. If there are multiple occurrences of the same keyword within the metadata, it returns an error. The method returns a Result containing the populated ErrorVariantProperties on success, or a syn::Error on failure.
    fn get_variant_properties(&self) -> syn::Result<ErrorVariantProperties> {
        let mut output = ErrorVariantProperties::default();

        let mut error_type_keyword = None;
        let mut code_keyword = None;
        let mut message_keyword = None;
        let mut ignore_keyword = None;
        for meta in self.get_metadata()? {
            match meta {
                VariantMeta::ErrorType { keyword, value } => {
                    if let Some(first_keyword) = error_type_keyword {
                        return Err(occurrence_error(first_keyword, keyword, "error_type"));
                    }

                    error_type_keyword = Some(keyword);
                    output.error_type = Some(value);
                }
                VariantMeta::Code { keyword, value } => {
                    if let Some(first_keyword) = code_keyword {
                        return Err(occurrence_error(first_keyword, keyword, "code"));
                    }

                    code_keyword = Some(keyword);
                    output.code = Some(value);
                }
                VariantMeta::Message { keyword, value } => {
                    if let Some(first_keyword) = message_keyword {
                        return Err(occurrence_error(first_keyword, keyword, "message"));
                    }

                    message_keyword = Some(keyword);
                    output.message = Some(value);
                }
                VariantMeta::Ignore { keyword, value } => {
                    if let Some(first_keyword) = ignore_keyword {
                        return Err(occurrence_error(first_keyword, keyword, "ignore"));
                    }
                    ignore_keyword = Some(keyword);
                    output.ignore = value
                        .value()
                        .replace(' ', "")
                        .split(',')
                        .map(ToString::to_string)
                        .collect();
                }
            }
        }

        Ok(output)
    }
}

/// This function takes a reference to a Variant and a string representing an attribute, and returns a syn::Error with a message indicating that the specified attribute must be specified for the given variant.
fn missing_attribute_error(variant: &Variant, attr: &str) -> syn::Error {
    syn::Error::new_spanned(variant, format!("{attr} must be specified"))
}

/// Checks if the given variant has all the required attributes and returns an error if any attribute is missing.
pub(super) fn check_missing_attributes(
    variant: &Variant,
    variant_properties: &ErrorVariantProperties,
) -> syn::Result<()> {
    if variant_properties.error_type.is_none() {
        return Err(missing_attribute_error(variant, "error_type"));
    }
    if variant_properties.code.is_none() {
        return Err(missing_attribute_error(variant, "code"));
    }
    if variant_properties.message.is_none() {
        return Err(missing_attribute_error(variant, "message"));
    }

    Ok(())
}

/// Get all the fields not used in the error message.
pub(super) fn get_unused_fields(
    fields: &Fields,
    message: &str,
    ignore: &std::collections::HashSet<String>,
) -> Vec<Field> {
    let fields = match fields {
        syn::Fields::Unit => Vec::new(),
        syn::Fields::Unnamed(_) => Vec::new(),
        syn::Fields::Named(fields) => fields.named.iter().cloned().collect(),
    };
    fields
        .iter()
        .filter(|&field| {
            // Safety: Named fields are guaranteed to have an identifier.
            #[allow(clippy::unwrap_used)]
            let field_name = format!("{}", field.ident.as_ref().unwrap());
            !message.contains(&field_name) && !ignore.contains(&field_name)
        })
        .cloned()
        .collect()
}
