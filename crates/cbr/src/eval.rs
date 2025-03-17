use std::cmp::Ordering;

use common_enums::Currency;
use euclid::{frontend::dir, types};
use rust_decimal::Decimal;
use rustc_hash::{FxHashMap, FxHashSet};
use rusty_money::{self, Money};

use crate::{expr, utils};

#[derive(Debug, Clone)]
enum EvalAtom {
    EvalMoney(EvalMoney),
    Bip(i64),
}

#[derive(Debug, Clone)]
struct EvalMoney {
    pub val: Decimal,
    pub currency: Currency,
}

impl EvalMoney {
    pub fn new(val: Decimal, currency: Currency) -> Self {
        Self { val, currency }
    }
}

impl EvalAtom {
    fn add(
        ex_rates: &currency_conversion::ExchangeRates,
        lhs: Self,
        rhs: Self,
    ) -> Result<Self, CostEvaluationError> {
        Ok(match (lhs, rhs) {
            (Self::EvalMoney(l_money), Self::EvalMoney(r_money)) => {
                if l_money.currency == r_money.currency {
                    Self::EvalMoney(EvalMoney::new(
                        l_money
                            .val
                            .checked_add(r_money.val)
                            .ok_or(CostEvaluationError::ArithmeticFailure)?,
                        l_money.currency,
                    ))
                } else {
                    // Convert the rhs to lhs and then add them up
                    let redefined_money = currency_conversion::convert_decimal(
                        ex_rates,
                        r_money.currency,
                        l_money.currency,
                        r_money.val,
                    )
                    .map_err(CostEvaluationError::CurrencyConversionError)?;
                    Self::EvalMoney(EvalMoney::new(
                        l_money
                            .val
                            .checked_add(redefined_money)
                            .ok_or(CostEvaluationError::ArithmeticFailure)?,
                        l_money.currency,
                    ))
                }
            }
            (Self::Bip(l_bip), Self::Bip(r_bip)) => Self::Bip(l_bip + r_bip),
            (Self::EvalMoney(l_money), Self::Bip(r_bip)) => Self::EvalMoney(EvalMoney::new(
                l_money
                    .val
                    .checked_add(
                        l_money
                            .val
                            .checked_mul(Decimal::new(r_bip, 4))
                            .ok_or(CostEvaluationError::ArithmeticFailure)?,
                    )
                    .ok_or(CostEvaluationError::ArithmeticFailure)?,
                l_money.currency,
            )),
            (Self::Bip(l_bip), Self::EvalMoney(r_money)) => Self::EvalMoney(EvalMoney::new(
                r_money
                    .val
                    .checked_add(
                        r_money
                            .val
                            .checked_mul(Decimal::new(l_bip, 4))
                            .ok_or(CostEvaluationError::ArithmeticFailure)?,
                    )
                    .ok_or(CostEvaluationError::ArithmeticFailure)?,
                r_money.currency,
            )),
        })
    }

    fn sub(
        ex_rates: &currency_conversion::ExchangeRates,
        lhs: Self,
        rhs: Self,
    ) -> Result<Self, CostEvaluationError> {
        Ok(match (lhs, rhs) {
            (Self::EvalMoney(l_money), Self::EvalMoney(r_money)) => {
                if l_money.currency == r_money.currency {
                    Self::EvalMoney(EvalMoney::new(l_money.val - r_money.val, l_money.currency))
                } else {
                    // Convert the currency and then add them up
                    let redefined_money = currency_conversion::convert_decimal(
                        ex_rates,
                        r_money.currency,
                        l_money.currency,
                        r_money.val,
                    )
                    .map_err(CostEvaluationError::CurrencyConversionError)?;
                    Self::EvalMoney(EvalMoney::new(
                        l_money
                            .val
                            .checked_sub(redefined_money)
                            .ok_or(CostEvaluationError::ArithmeticFailure)?,
                        l_money.currency,
                    ))
                }
            }
            (Self::Bip(l_bip), Self::Bip(r_bip)) => Self::Bip(l_bip - r_bip),
            (Self::EvalMoney(l_money), Self::Bip(r_bip)) => Self::EvalMoney(EvalMoney::new(
                l_money
                    .val
                    .checked_sub(
                        l_money
                            .val
                            .checked_mul(Decimal::new(r_bip, 4))
                            .ok_or(CostEvaluationError::ArithmeticFailure)?,
                    )
                    .ok_or(CostEvaluationError::ArithmeticFailure)?,
                l_money.currency,
            )),
            (Self::Bip(l_bip), Self::EvalMoney(r_money)) => Self::EvalMoney(EvalMoney::new(
                r_money
                    .val
                    .checked_sub(
                        r_money
                            .val
                            .checked_mul(Decimal::new(l_bip, 4))
                            .ok_or(CostEvaluationError::ArithmeticFailure)?,
                    )
                    .ok_or(CostEvaluationError::ArithmeticFailure)?,
                r_money.currency,
            )),
        })
    }

    fn mul(
        ex_rates: &currency_conversion::ExchangeRates,
        lhs: Self,
        rhs: Self,
    ) -> Result<Self, CostEvaluationError> {
        Ok(match (lhs, rhs) {
            (Self::EvalMoney(l_money), Self::EvalMoney(r_money)) => {
                if l_money.currency == r_money.currency {
                    Self::EvalMoney(EvalMoney::new(l_money.val * r_money.val, l_money.currency))
                } else {
                    // Convert the currency and then add them up
                    let redefined_money = currency_conversion::convert_decimal(
                        ex_rates,
                        r_money.currency,
                        l_money.currency,
                        r_money.val,
                    )
                    .map_err(CostEvaluationError::CurrencyConversionError)?;
                    Self::EvalMoney(EvalMoney::new(
                        l_money
                            .val
                            .checked_mul(redefined_money)
                            .ok_or(CostEvaluationError::ArithmeticFailure)?,
                        l_money.currency,
                    ))
                }
            }
            (Self::Bip(_), Self::Bip(_)) => Err(CostEvaluationError::BipMultiplication)?,
            (Self::EvalMoney(l_money), Self::Bip(r_bip)) => Self::EvalMoney(EvalMoney::new(
                l_money
                    .val
                    .checked_mul(Decimal::new(r_bip, 4))
                    .ok_or(CostEvaluationError::ArithmeticFailure)?,
                l_money.currency,
            )),
            (Self::Bip(l_bip), Self::EvalMoney(r_money)) => Self::EvalMoney(EvalMoney::new(
                r_money
                    .val
                    .checked_mul(Decimal::new(l_bip, 4))
                    .ok_or(CostEvaluationError::ArithmeticFailure)?,
                r_money.currency,
            )),
        })
    }

    fn div(
        ex_rates: &currency_conversion::ExchangeRates,
        lhs: Self,
        rhs: Self,
    ) -> Result<Self, CostEvaluationError> {
        Ok(match (lhs, rhs) {
            (Self::EvalMoney(l_money), Self::EvalMoney(r_money)) => {
                if l_money.currency == r_money.currency {
                    Self::EvalMoney(EvalMoney::new(l_money.val / r_money.val, l_money.currency))
                } else {
                    // Convert the currency and then add them up
                    let redefined_money = currency_conversion::convert_decimal(
                        ex_rates,
                        r_money.currency,
                        l_money.currency,
                        r_money.val,
                    )
                    .map_err(CostEvaluationError::CurrencyConversionError)?;
                    Self::EvalMoney(EvalMoney::new(
                        l_money
                            .val
                            .checked_div(redefined_money)
                            .ok_or(CostEvaluationError::ArithmeticFailure)?,
                        l_money.currency,
                    ))
                }
            }
            (Self::Bip(_), Self::Bip(_)) => Err(CostEvaluationError::BipMultiplication)?,
            (Self::EvalMoney(_), Self::Bip(_)) => Err(CostEvaluationError::MoneyBipDivision)?,
            (Self::Bip(_), Self::EvalMoney(_)) => Err(CostEvaluationError::MoneyBipDivision)?,
        })
    }

    fn negate(atom: Self) -> Self {
        match atom {
            Self::EvalMoney(EvalMoney { val, currency }) => {
                Self::EvalMoney(EvalMoney::new(-val, currency))
            }
            Self::Bip(b) => Self::Bip(-b),
        }
    }

    fn ord(lhs: Self, rhs: Self) -> Result<Ordering, CostEvaluationError> {
        Ok(match (lhs, rhs) {
            (Self::EvalMoney(l_money), Self::EvalMoney(r_money)) => l_money.val.cmp(&r_money.val),
            (Self::EvalMoney(_), Self::Bip(_)) | (Self::Bip(_), Self::EvalMoney(_)) => {
                Err(CostEvaluationError::MoneyBipComparison)?
            }
            (Self::Bip(l_bip), Self::Bip(r_bip)) => l_bip.cmp(&r_bip),
        })
    }
}

pub struct CostEvalContext {
    enum_values: FxHashSet<dir::DirValue>,
    enum_keywise_values: FxHashMap<dir::DirKey, dir::DirValue>,
    num_keywise_values: FxHashMap<dir::DirKey, dir::DirValue>,
}

impl CostEvalContext {
    fn check_presence(&self, value: &dir::DirValue) -> bool {
        let key = value.get_key();
        match key.kind.get_type() {
            types::DataType::EnumVariant
            | types::DataType::StrValue
            | types::DataType::MetadataValue => self.enum_values.contains(value),
            types::DataType::Number => {
                let num_value = self
                    .num_keywise_values
                    .get(&key)
                    .and_then(|value| value.get_num_value());

                value
                    .get_num_value()
                    .zip(num_value)
                    .map_or(false, |(program_val, ctx_val)| {
                        let program_num = program_val.number;
                        let ctx_num = ctx_val.number;

                        match &program_val.refinement {
                            None => program_num == ctx_num,
                            Some(types::NumValueRefinement::NotEqual) => program_num != ctx_num,
                            Some(types::NumValueRefinement::GreaterThan) => ctx_num > program_num,
                            Some(types::NumValueRefinement::GreaterThanEqual) => {
                                ctx_num >= program_num
                            }
                            Some(types::NumValueRefinement::LessThan) => ctx_num < program_num,
                            Some(types::NumValueRefinement::LessThanEqual) => {
                                ctx_num <= program_num
                            }
                        }
                    })
            }
        }
    }

    fn get_value(&self, key: &dir::DirKey) -> Option<&dir::DirValue> {
        match key.get_type() {
            types::DataType::EnumVariant
            | types::DataType::StrValue
            | types::DataType::MetadataValue => self.enum_keywise_values.get(key),

            types::DataType::Number => self.num_keywise_values.get(key),
        }
    }

    fn check_key_key_equality(&self, key1: &dir::DirKey, key2: &dir::DirKey) -> bool {
        let key1_type = key1.get_type();
        let key2_type = key2.get_type();

        let values = match (key1_type, key2_type) {
            (types::DataType::EnumVariant, types::DataType::EnumVariant)
            | (types::DataType::StrValue, types::DataType::StrValue) => self
                .enum_keywise_values
                .get(key1)
                .cloned()
                .zip(self.enum_keywise_values.get(key2).cloned()),

            (types::DataType::Number, types::DataType::Number) => self
                .num_keywise_values
                .get(key1)
                .cloned()
                .zip(self.num_keywise_values.get(key2).cloned()),

            _ => return false,
        };

        values.map(|(v1, v2)| v1 == v2).unwrap_or_default()
    }

    fn check_key_value_equality(&self, key: &dir::DirKey, value: &dir::DirValue) -> bool {
        self.get_value(key)
            .map(|ctx_value| ctx_value == value)
            .unwrap_or_default()
    }

    pub fn from_dir_values(vals: impl IntoIterator<Item = dir::DirValue>) -> Self {
        let mut enum_values = FxHashSet::default();
        let mut enum_keywise_values = FxHashMap::default();
        let mut num_keywise_values = FxHashMap::default();

        for dir_val in vals {
            let key = dir_val.get_key();
            match key.get_type() {
                types::DataType::EnumVariant
                | types::DataType::StrValue
                | types::DataType::MetadataValue => {
                    enum_values.insert(dir_val.clone());
                    enum_keywise_values.insert(key, dir_val);
                }
                types::DataType::Number => {
                    num_keywise_values.insert(key, dir_val);
                }
            }
        }

        Self {
            enum_values,
            enum_keywise_values,
            num_keywise_values,
        }
    }

    fn get_currency_value(&self) -> Result<Currency, CostEvaluationError> {
        Ok(
            match self
                .enum_keywise_values
                .get(&dir::DirKey::new(dir::DirKeyKind::PaymentCurrency, None))
            {
                Some(dir::DirValue::PaymentCurrency(currency)) => *currency,
                _ => {
                    return Err(CostEvaluationError::CurrencyNotFound(
                        currency_conversion::CurrencyConversionError::DecimalMultiplicationFailed,
                    ));
                }
            },
        )
    }

    fn get_key_value(&self, key: &dir::DirKey) -> Result<i64, CostEvaluationError> {
        Ok(self
            .num_keywise_values
            .get(key)
            .ok_or(CostEvaluationError::KeyAbsentInContext(key.clone()))?
            .get_num_value()
            .ok_or(CostEvaluationError::NumValueNotFoundForKey(key.clone()))?
            .number
            .get_amount_as_i64())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CostEvaluationError {
    #[error("bips cannot be multiplied")]
    BipMultiplication,
    #[error("failure upoun processing of decimals")]
    ArithmeticFailure,
    #[error("bips cannot be divided")]
    BipDivision,
    #[error("cannot divide Money with bip")]
    MoneyBipDivision,
    #[error("cannot compare money with a bip")]
    MoneyBipComparison,
    #[error("the key '{}' was not found in the context", .0.kind)]
    KeyAbsentInContext(dir::DirKey),
    #[error("num value not found for key '{}'", .0.kind)]
    NumValueNotFoundForKey(dir::DirKey),
    #[error("error performing currency conversion")]
    CurrencyConversionError(currency_conversion::CurrencyConversionError),
    #[error("currency not found")]
    CurrencyNotFound(currency_conversion::CurrencyConversionError),
    #[error("got a bip as evaluation result which is not allowed")]
    BipAsResult,
    #[error("cost expr short circuit (kind: '{:?}') (reason: '{}')", .0.kind, .0.reason)]
    ShortCircuit(expr::ShortCircuit),
}

impl CostEvaluationError {
    pub fn is_short_circuit(&self) -> bool {
        matches!(self, Self::ShortCircuit(_))
    }
}

fn evaluate_directive(
    exchange_rates: &currency_conversion::ExchangeRates,
    directive: &expr::Directive,
    ctx: &CostEvalContext,
) -> Result<EvalAtom, CostEvaluationError> {
    match directive {
        expr::Directive::Min(min) => {
            let expr = evaluate_inner(exchange_rates, &min.expr, ctx)?;
            let value = evaluate_inner(exchange_rates, &min.value, ctx)?;
            Ok(match EvalAtom::ord(value.clone(), expr.clone())? {
                Ordering::Less | Ordering::Equal => value,
                Ordering::Greater => expr,
            })
        }

        expr::Directive::Max(max) => {
            let expr = evaluate_inner(exchange_rates, &max.expr, ctx)?;
            let value = evaluate_inner(exchange_rates, &max.value, ctx)?;

            Ok(match EvalAtom::ord(value.clone(), expr.clone())? {
                Ordering::Greater | Ordering::Equal => value,
                Ordering::Less => expr,
            })
        }

        expr::Directive::KeyEq(key_eq) => {
            if ctx.check_key_key_equality(&key_eq.lhs, &key_eq.rhs) {
                evaluate_inner(exchange_rates, &key_eq.then_branch, ctx)
            } else {
                evaluate_inner(exchange_rates, &key_eq.else_branch, ctx)
            }
        }

        expr::Directive::ValueEq(value_eq) => {
            if ctx.check_key_value_equality(&value_eq.key, &value_eq.value) {
                evaluate_inner(exchange_rates, &value_eq.then_branch, ctx)
            } else {
                evaluate_inner(exchange_rates, &value_eq.else_branch, ctx)
            }
        }
    }
}

fn evaluate_inner(
    exchange_rates: &currency_conversion::ExchangeRates,
    expr: &expr::CostExpr,
    ctx: &CostEvalContext,
) -> Result<EvalAtom, CostEvaluationError> {
    match expr {
        expr::CostExpr::Atom(atom) => Ok(match atom {
            expr::Atom::Money(money) => EvalAtom::EvalMoney(EvalMoney::new(
                *Money::from_minor(
                    money.val,
                    &utils::get_iso_curr(&money.currency)
                        .map_err(CostEvaluationError::CurrencyConversionError)?,
                )
                .amount(),
                money.currency,
            )),
            expr::Atom::Bip(bip) => EvalAtom::Bip(*bip),
            expr::Atom::Key(key) => EvalAtom::EvalMoney(EvalMoney {
                val: ctx.get_key_value(key)?.into(),
                currency: ctx.get_currency_value()?,
            }),
        }),

        expr::CostExpr::Binary(bin) => {
            let lhs_val = evaluate_inner(exchange_rates, &bin.lhs, ctx)?;
            let rhs_val = evaluate_inner(exchange_rates, &bin.rhs, ctx)?;

            let bin_func = match bin.op {
                expr::BinaryOp::Add => EvalAtom::add,
                expr::BinaryOp::Sub => EvalAtom::sub,
                expr::BinaryOp::Mul => EvalAtom::mul,
                expr::BinaryOp::Div => EvalAtom::div,
            };

            bin_func(exchange_rates, lhs_val, rhs_val)
        }

        expr::CostExpr::Neg(sub) => {
            let atom = evaluate_inner(exchange_rates, sub, ctx)?;
            Ok(EvalAtom::negate(atom))
        }

        expr::CostExpr::Branch(branch) => {
            let selected_arm = branch
                .elems
                .iter()
                .find(|arm| arm.patterns.iter().any(|pat| ctx.check_presence(pat)))
                .map(|arm| &arm.expr);

            evaluate_inner(
                exchange_rates,
                if let Some(nested_expr) = selected_arm {
                    nested_expr
                } else {
                    &branch.default
                },
                ctx,
            )
        }

        expr::CostExpr::Directive(directive) => evaluate_directive(exchange_rates, directive, ctx),

        expr::CostExpr::ShortCircuit(short_circuit) => {
            Err(CostEvaluationError::ShortCircuit(short_circuit.clone()))
        }
    }
}

pub fn evaluate(
    exchange_rates: &currency_conversion::ExchangeRates,
    expr: &expr::CostExpr,
    ctx: &CostEvalContext,
) -> Result<Decimal, CostEvaluationError> {
    let result = evaluate_inner(exchange_rates, expr, ctx)?;
    match result {
        EvalAtom::EvalMoney(res) => Ok(res.val),
        EvalAtom::Bip(_) => Err(CostEvaluationError::BipAsResult),
    }
}
