use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse::Parse, Field, LitInt, LitStr, Token, TypePath};

use crate::macros::helpers::{get_metadata_inner, occurrence_error};

mod keyword {
    use syn::custom_keyword;

    // Schema metadata
    custom_keyword!(value_type);
    custom_keyword!(min_length);
    custom_keyword!(max_length);
    custom_keyword!(example);
}

pub enum SchemaParameterVariant {
    ValueType {
        keyword: keyword::value_type,
        value: TypePath,
    },
    MinLength {
        keyword: keyword::min_length,
        value: LitInt,
    },
    MaxLength {
        keyword: keyword::max_length,
        value: LitInt,
    },
    Example {
        keyword: keyword::example,
        value: LitStr,
    },
}

impl Parse for SchemaParameterVariant {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(keyword::value_type) {
            let keyword = input.parse()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            Ok(Self::ValueType { keyword, value })
        } else if lookahead.peek(keyword::min_length) {
            let keyword = input.parse()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            Ok(Self::MinLength { keyword, value })
        } else if lookahead.peek(keyword::max_length) {
            let keyword = input.parse()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            Ok(Self::MaxLength { keyword, value })
        } else if lookahead.peek(keyword::example) {
            let keyword = input.parse()?;
            input.parse::<Token![=]>()?;
            let value = input.parse()?;
            Ok(Self::Example { keyword, value })
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for SchemaParameterVariant {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::ValueType { keyword, .. } => keyword.to_tokens(tokens),
            Self::MinLength { keyword, .. } => keyword.to_tokens(tokens),
            Self::MaxLength { keyword, .. } => keyword.to_tokens(tokens),
            Self::Example { keyword, .. } => keyword.to_tokens(tokens),
        }
    }
}

pub trait FieldExt {
    /// Get all the schema metadata associated with a field.
    fn get_schema_metadata(&self) -> syn::Result<Vec<SchemaParameterVariant>>;
}

impl FieldExt for Field {
    fn get_schema_metadata(&self) -> syn::Result<Vec<SchemaParameterVariant>> {
        get_metadata_inner("schema", &self.attrs)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SchemaParameters {
    pub value_type: Option<TypePath>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub example: Option<String>,
}

pub trait HasSchemaParameters {
    fn get_schema_parameters(&self) -> syn::Result<SchemaParameters>;
}

impl HasSchemaParameters for Field {
    fn get_schema_parameters(&self) -> syn::Result<SchemaParameters> {
        let mut output = SchemaParameters::default();

        let mut value_type_keyword = None;
        let mut min_length_keyword = None;
        let mut max_length_keyword = None;
        let mut example_keyword = None;

        for meta in self.get_schema_metadata()? {
            match meta {
                SchemaParameterVariant::ValueType { keyword, value } => {
                    if let Some(first_keyword) = value_type_keyword {
                        return Err(occurrence_error(first_keyword, keyword, "value_type"));
                    }

                    value_type_keyword = Some(keyword);
                    output.value_type = Some(value);
                }
                SchemaParameterVariant::MinLength { keyword, value } => {
                    if let Some(first_keyword) = min_length_keyword {
                        return Err(occurrence_error(first_keyword, keyword, "min_length"));
                    }

                    min_length_keyword = Some(keyword);
                    let min_length = value.base10_parse::<usize>()?;
                    output.min_length = Some(min_length);
                }
                SchemaParameterVariant::MaxLength { keyword, value } => {
                    if let Some(first_keyword) = max_length_keyword {
                        return Err(occurrence_error(first_keyword, keyword, "max_length"));
                    }

                    max_length_keyword = Some(keyword);
                    let max_length = value.base10_parse::<usize>()?;
                    output.max_length = Some(max_length);
                }
                SchemaParameterVariant::Example { keyword, value } => {
                    if let Some(first_keyword) = example_keyword {
                        return Err(occurrence_error(first_keyword, keyword, "example"));
                    }

                    example_keyword = Some(keyword);
                    output.example = Some(value.value());
                }
            }
        }

        Ok(output)
    }
}

/// Check if the field is applicable for running validations
#[derive(PartialEq)]
pub enum IsSchemaFieldApplicableForValidation {
    /// Not applicable for running validation checks
    Invalid,
    /// Applicable for running validation checks
    Valid,
    /// Applicable for validation but field is optional - this is needed for generating validation code only if the value of the field is present
    ValidOptional,
}

/// From implementation for checking if the field type is applicable for running schema validations
impl From<&syn::Type> for IsSchemaFieldApplicableForValidation {
    fn from(ty: &syn::Type) -> Self {
        if let syn::Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                let ident = &segment.ident;
                if ident == "String" || ident == "Url" {
                    return Self::Valid;
                }

                if ident == "Option" {
                    if let syn::PathArguments::AngleBracketed(generic_args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(syn::Type::Path(inner_path))) =
                            generic_args.args.first()
                        {
                            if let Some(inner_segment) = inner_path.path.segments.last() {
                                if inner_segment.ident == "String" || inner_segment.ident == "Url" {
                                    return Self::ValidOptional;
                                }
                            }
                        }
                    }
                }
            }
        }
        Self::Invalid
    }
}
