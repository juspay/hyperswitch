//! Analysis for the Lowering logic in ast
//!
//!Certain functions that can be used to perform the complete lowering of ast to dir.
//!This includes lowering of enums, numbers, strings as well as Comparison logics.

use std::str::FromStr;

use crate::{
    dssa::types::{AnalysisError, AnalysisErrorType},
    enums::CollectVariants,
    frontend::{
        ast,
        dir::{self, enums as dir_enums, EuclidDirFilter},
    },
    types::{self, DataType},
};

/// lowers the provided key (enum variant) & value to the respective DirValue
///
/// For example
/// ```notrust
/// CardType = Visa
/// ```notrust
///
/// This serves for the purpose were we have the DirKey as an explicit Enum type and value as one
/// of the member of the same Enum.
/// So particularly it lowers a predefined Enum from DirKey to an Enum of DirValue.

macro_rules! lower_enum {
    ($key:ident, $value:ident) => {
        match $value {
            ast::ValueType::EnumVariant(ev) => Ok(vec![dir::DirValue::$key(
                dir_enums::$key::from_str(&ev).map_err(|_| AnalysisErrorType::InvalidVariant {
                    key: dir::DirKeyKind::$key.to_string(),
                    got: ev,
                    expected: dir_enums::$key::variants(),
                })?,
            )]),

            ast::ValueType::EnumVariantArray(eva) => eva
                .into_iter()
                .map(|ev| {
                    Ok(dir::DirValue::$key(
                        dir_enums::$key::from_str(&ev).map_err(|_| {
                            AnalysisErrorType::InvalidVariant {
                                key: dir::DirKeyKind::$key.to_string(),
                                got: ev,
                                expected: dir_enums::$key::variants(),
                            }
                        })?,
                    ))
                })
                .collect(),

            _ => Err(AnalysisErrorType::InvalidType {
                key: dir::DirKeyKind::$key.to_string(),
                expected: DataType::EnumVariant,
                got: $value.get_type(),
            }),
        }
    };
}

/// lowers the provided key for a numerical value
///
/// For example
/// ```notrust
/// payment_amount = 17052001
/// ```notrust
/// This is for the cases in which there are numerical values involved and they are lowered
/// accordingly on basis of the supplied key, currently payment_amount is the only key having this
/// use case

macro_rules! lower_number {
    ($key:ident, $value:ident, $comp:ident) => {
        match $value {
            ast::ValueType::Number(num) => Ok(vec![dir::DirValue::$key(types::NumValue {
                number: num,
                refinement: $comp.into(),
            })]),

            ast::ValueType::NumberArray(na) => na
                .into_iter()
                .map(|num| {
                    Ok(dir::DirValue::$key(types::NumValue {
                        number: num,
                        refinement: $comp.clone().into(),
                    }))
                })
                .collect(),

            ast::ValueType::NumberComparisonArray(nca) => nca
                .into_iter()
                .map(|nc| {
                    Ok(dir::DirValue::$key(types::NumValue {
                        number: nc.number,
                        refinement: nc.comparison_type.into(),
                    }))
                })
                .collect(),

            _ => Err(AnalysisErrorType::InvalidType {
                key: dir::DirKeyKind::$key.to_string(),
                expected: DataType::Number,
                got: $value.get_type(),
            }),
        }
    };
}

/// lowers the provided key & value to the respective DirValue
///
/// For example
/// ```notrust
/// card_bin = "123456"
/// ```notrust
///
/// This serves for the purpose were we have the DirKey as Card_bin and value as an arbitrary string
/// So particularly it lowers an arbitrary value to a predefined key.

macro_rules! lower_str {
    ($key:ident, $value:ident $(, $validation_closure:expr)?) => {
        match $value {
            ast::ValueType::StrValue(st) => {
                $($validation_closure(&st)?;)?
                Ok(vec![dir::DirValue::$key(types::StrValue { value: st })])
            }
            _ => Err(AnalysisErrorType::InvalidType {
                key: dir::DirKeyKind::$key.to_string(),
                expected: DataType::StrValue,
                got: $value.get_type(),
            }),
        }
    };
}

macro_rules! lower_metadata {
    ($key:ident, $value:ident) => {
        match $value {
            ast::ValueType::MetadataVariant(md) => {
                Ok(vec![dir::DirValue::$key(types::MetadataValue {
                    key: md.key,
                    value: md.value,
                })])
            }
            _ => Err(AnalysisErrorType::InvalidType {
                key: dir::DirKeyKind::$key.to_string(),
                expected: DataType::MetadataValue,
                got: $value.get_type(),
            }),
        }
    };
}
/// lowers the comparison operators for different subtle value types present
/// by throwing required errors for comparisons that can't be performed for a certain value type
/// for example
/// can't have greater/less than operations on enum types

fn lower_comparison_inner<O: EuclidDirFilter>(
    comp: ast::Comparison,
) -> Result<Vec<dir::DirValue>, AnalysisErrorType> {
    let key_enum = dir::DirKeyKind::from_str(comp.lhs.as_str())
        .map_err(|_| AnalysisErrorType::InvalidKey(comp.lhs.clone()))?;

    if !O::is_key_allowed(&key_enum) {
        return Err(AnalysisErrorType::InvalidKey(key_enum.to_string()));
    }

    match (&comp.comparison, &comp.value) {
        (
            ast::ComparisonType::LessThan
            | ast::ComparisonType::GreaterThan
            | ast::ComparisonType::GreaterThanEqual
            | ast::ComparisonType::LessThanEqual,
            ast::ValueType::EnumVariant(_),
        ) => {
            Err(AnalysisErrorType::InvalidComparison {
                operator: comp.comparison.clone(),
                value_type: DataType::EnumVariant,
            })?;
        }

        (
            ast::ComparisonType::LessThan
            | ast::ComparisonType::GreaterThan
            | ast::ComparisonType::GreaterThanEqual
            | ast::ComparisonType::LessThanEqual,
            ast::ValueType::NumberArray(_),
        ) => {
            Err(AnalysisErrorType::InvalidComparison {
                operator: comp.comparison.clone(),
                value_type: DataType::Number,
            })?;
        }

        (
            ast::ComparisonType::LessThan
            | ast::ComparisonType::GreaterThan
            | ast::ComparisonType::GreaterThanEqual
            | ast::ComparisonType::LessThanEqual,
            ast::ValueType::EnumVariantArray(_),
        ) => {
            Err(AnalysisErrorType::InvalidComparison {
                operator: comp.comparison.clone(),
                value_type: DataType::EnumVariant,
            })?;
        }

        (
            ast::ComparisonType::LessThan
            | ast::ComparisonType::GreaterThan
            | ast::ComparisonType::GreaterThanEqual
            | ast::ComparisonType::LessThanEqual,
            ast::ValueType::NumberComparisonArray(_),
        ) => {
            Err(AnalysisErrorType::InvalidComparison {
                operator: comp.comparison.clone(),
                value_type: DataType::Number,
            })?;
        }

        _ => {}
    }

    let value = comp.value;
    let comparison = comp.comparison;

    match key_enum {
        dir::DirKeyKind::PaymentMethod => lower_enum!(PaymentMethod, value),

        dir::DirKeyKind::CardType => lower_enum!(CardType, value),

        dir::DirKeyKind::CardNetwork => lower_enum!(CardNetwork, value),

        dir::DirKeyKind::PayLaterType => lower_enum!(PayLaterType, value),

        dir::DirKeyKind::WalletType => lower_enum!(WalletType, value),

        dir::DirKeyKind::BankDebitType => lower_enum!(BankDebitType, value),

        dir::DirKeyKind::BankRedirectType => lower_enum!(BankRedirectType, value),

        dir::DirKeyKind::CryptoType => lower_enum!(CryptoType, value),

        dir::DirKeyKind::PaymentType => lower_enum!(PaymentType, value),

        dir::DirKeyKind::MandateType => lower_enum!(MandateType, value),

        dir::DirKeyKind::MandateAcceptanceType => lower_enum!(MandateAcceptanceType, value),

        dir::DirKeyKind::RewardType => lower_enum!(RewardType, value),

        dir::DirKeyKind::PaymentCurrency => lower_enum!(PaymentCurrency, value),

        dir::DirKeyKind::AuthenticationType => lower_enum!(AuthenticationType, value),

        dir::DirKeyKind::CaptureMethod => lower_enum!(CaptureMethod, value),

        dir::DirKeyKind::BusinessCountry => lower_enum!(BusinessCountry, value),

        dir::DirKeyKind::BillingCountry => lower_enum!(BillingCountry, value),

        dir::DirKeyKind::SetupFutureUsage => lower_enum!(SetupFutureUsage, value),

        dir::DirKeyKind::UpiType => lower_enum!(UpiType, value),

        dir::DirKeyKind::VoucherType => lower_enum!(VoucherType, value),

        dir::DirKeyKind::GiftCardType => lower_enum!(GiftCardType, value),

        dir::DirKeyKind::BankTransferType => lower_enum!(BankTransferType, value),

        dir::DirKeyKind::CardRedirectType => lower_enum!(CardRedirectType, value),

        dir::DirKeyKind::CardBin => {
            let validation_closure = |st: &String| -> Result<(), AnalysisErrorType> {
                if st.len() == 6 && st.chars().all(|x| x.is_ascii_digit()) {
                    Ok(())
                } else {
                    Err(AnalysisErrorType::InvalidValue {
                        key: dir::DirKeyKind::CardBin,
                        value: st.clone(),
                        message: Some("Expected 6 digits".to_string()),
                    })
                }
            };
            lower_str!(CardBin, value, validation_closure)
        }

        dir::DirKeyKind::BusinessLabel => lower_str!(BusinessLabel, value),

        dir::DirKeyKind::MetaData => lower_metadata!(MetaData, value),

        dir::DirKeyKind::PaymentAmount => lower_number!(PaymentAmount, value, comparison),

        dir::DirKeyKind::Connector => Err(AnalysisErrorType::InvalidKey(
            dir::DirKeyKind::Connector.to_string(),
        )),
    }
}

/// returns all the comparison values by matching them appropriately to ComparisonTypes and in turn
/// calls the lower_comparison_inner function
fn lower_comparison<O: EuclidDirFilter>(
    comp: ast::Comparison,
) -> Result<dir::DirComparison, AnalysisError> {
    let metadata = comp.metadata.clone();
    let logic = match &comp.comparison {
        ast::ComparisonType::Equal => dir::DirComparisonLogic::PositiveDisjunction,
        ast::ComparisonType::NotEqual => dir::DirComparisonLogic::NegativeConjunction,
        ast::ComparisonType::LessThan => dir::DirComparisonLogic::PositiveDisjunction,
        ast::ComparisonType::LessThanEqual => dir::DirComparisonLogic::PositiveDisjunction,
        ast::ComparisonType::GreaterThanEqual => dir::DirComparisonLogic::PositiveDisjunction,
        ast::ComparisonType::GreaterThan => dir::DirComparisonLogic::PositiveDisjunction,
    };
    let values = lower_comparison_inner::<O>(comp).map_err(|etype| AnalysisError {
        error_type: etype,
        metadata: metadata.clone(),
    })?;

    Ok(dir::DirComparison {
        values,
        logic,
        metadata,
    })
}

/// lowers the if statement accordingly with a condition and following nested if statements (if
/// present)
fn lower_if_statement<O: EuclidDirFilter>(
    stmt: ast::IfStatement,
) -> Result<dir::DirIfStatement, AnalysisError> {
    Ok(dir::DirIfStatement {
        condition: stmt
            .condition
            .into_iter()
            .map(lower_comparison::<O>)
            .collect::<Result<_, _>>()?,
        nested: stmt
            .nested
            .map(|n| n.into_iter().map(lower_if_statement::<O>).collect())
            .transpose()?,
    })
}

/// lowers the rules supplied accordingly to DirRule struct by specifying the rule_name,
/// connector_selection and statements that are a bunch of if statements
pub fn lower_rule<O: EuclidDirFilter>(
    rule: ast::Rule<O>,
) -> Result<dir::DirRule<O>, AnalysisError> {
    Ok(dir::DirRule {
        name: rule.name,
        connector_selection: rule.connector_selection,
        statements: rule
            .statements
            .into_iter()
            .map(lower_if_statement::<O>)
            .collect::<Result<_, _>>()?,
    })
}

/// uses the above rules and lowers the whole ast Program into DirProgram by specifying
/// default_selection that is ast ConnectorSelection, a vector of DirRules and clones the metadata
/// whatever comes in the ast_program
pub fn lower_program<O: EuclidDirFilter>(
    program: ast::Program<O>,
) -> Result<dir::DirProgram<O>, AnalysisError> {
    Ok(dir::DirProgram {
        default_selection: program.default_selection,
        rules: program
            .rules
            .into_iter()
            .map(lower_rule)
            .collect::<Result<_, _>>()?,
        metadata: program.metadata,
    })
}
