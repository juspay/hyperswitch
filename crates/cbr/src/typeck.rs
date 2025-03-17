use crate::expr;
use euclid::frontend::dir;
use euclid::types::DataType;

#[derive(Debug, thiserror::Error)]
pub enum CostExprTypeckError {
    #[error("invalid key type received (key: '{}')", .0.kind)]
    InvalidKeyType(dir::DirKey),

    #[error("invalid binary operation '{lhs_type:?} {op} {rhs_type:?}'")]
    InvalidBinaryOp {
        lhs_type: Type,
        op: expr::BinaryOp,
        rhs_type: Type,
    },

    #[error("branch arms have incompatible types '{type1:?}' and '{type2:?}'")]
    IncompatibleBranchArms { type1: Type, type2: Type },

    #[error("incompatible '{directive}' directive types '{type1:?}' and '{type2:?}' (message: {message:?})")]
    IncompatibleDirectiveTypes {
        type1: Type,
        type2: Type,
        directive: expr::directives::DirectiveKind,
        message: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Money,
    Bip,
    ShortCircuit,
}

trait Typeck {
    fn typeck(&self) -> Result<Type, CostExprTypeckError>;
}

impl Typeck for expr::Atom {
    fn typeck(&self) -> Result<Type, CostExprTypeckError> {
        match self {
            Self::Bip(_) => Ok(Type::Bip),
            Self::Money(_) => Ok(Type::Money),
            Self::Key(key) => {
                let the_type = key.get_type();
                if let DataType::Number = the_type {
                    Ok(Type::Money)
                } else {
                    Err(CostExprTypeckError::InvalidKeyType(key.clone()))
                }
            }
        }
    }
}

impl Typeck for expr::Binary {
    fn typeck(&self) -> Result<Type, CostExprTypeckError> {
        let lhs_ty = self.lhs.typeck()?;
        let rhs_ty = self.rhs.typeck()?;

        let build_err = || {
            Err::<Type, _>(CostExprTypeckError::InvalidBinaryOp {
                lhs_type: lhs_ty,
                op: self.op,
                rhs_type: rhs_ty,
            })
        };

        Ok(match self.op {
            expr::BinaryOp::Add => match (lhs_ty, rhs_ty) {
                (Type::ShortCircuit, other) => other,
                (other, Type::ShortCircuit) => other,
                (Type::Bip, Type::Bip) => Type::Bip,
                (Type::Bip, Type::Money)
                | (Type::Money, Type::Bip)
                | (Type::Money, Type::Money) => Type::Money,
            },

            expr::BinaryOp::Sub => match (lhs_ty, rhs_ty) {
                (Type::ShortCircuit, other) => other,
                (other, Type::ShortCircuit) => other,
                (Type::Bip, Type::Bip) => Type::Bip,
                (Type::Bip, Type::Money)
                | (Type::Money, Type::Bip)
                | (Type::Money, Type::Money) => Type::Money,
            },

            expr::BinaryOp::Mul => match (lhs_ty, rhs_ty) {
                (Type::ShortCircuit, other) => other,
                (other, Type::ShortCircuit) => other,
                (Type::Bip, Type::Bip) => build_err()?,
                (Type::Bip, Type::Money)
                | (Type::Money, Type::Bip)
                | (Type::Money, Type::Money) => build_err()?,
            },

            expr::BinaryOp::Div => match (lhs_ty, rhs_ty) {
                (Type::ShortCircuit, other) => other,
                (other, Type::ShortCircuit) => other,
                (Type::Bip, Type::Bip) | (Type::Bip, Type::Money) | (Type::Money, Type::Bip) => {
                    build_err()?
                }
                (Type::Money, Type::Money) => Type::Money,
            },
        })
    }
}

impl Typeck for expr::BranchArm {
    fn typeck(&self) -> Result<Type, CostExprTypeckError> {
        self.expr.typeck()
    }
}

impl Typeck for expr::Branch {
    fn typeck(&self) -> Result<Type, CostExprTypeckError> {
        let default_ty = self.default.typeck()?;
        self.elems.iter().try_fold(default_ty, |running_ty, arm| {
            let arm_type = arm.typeck()?;

            match (running_ty, arm_type) {
                (Type::ShortCircuit, arm_ty) => Ok(arm_ty),
                (running_ty, Type::ShortCircuit) => Ok(running_ty),
                (running_ty, arm_ty) if running_ty == arm_ty => Ok(arm_ty),
                (running_ty, arm_ty) => Err(CostExprTypeckError::IncompatibleBranchArms {
                    type1: running_ty,
                    type2: arm_ty,
                }),
            }
        })
    }
}

impl Typeck for expr::directives::Min {
    fn typeck(&self) -> Result<Type, CostExprTypeckError> {
        let expr_ty = self.expr.typeck()?;
        let value_ty = self.expr.typeck()?;

        if expr_ty == value_ty {
            Ok(value_ty)
        } else {
            Err(CostExprTypeckError::IncompatibleDirectiveTypes {
                type1: expr_ty,
                type2: value_ty,
                directive: expr::directives::DirectiveKind::Min,
                message: None,
            })
        }
    }
}

impl Typeck for expr::directives::Max {
    fn typeck(&self) -> Result<Type, CostExprTypeckError> {
        let expr_ty = self.expr.typeck()?;
        let value_ty = self.value.typeck()?;

        if expr_ty == value_ty {
            Ok(value_ty)
        } else {
            Err(CostExprTypeckError::IncompatibleDirectiveTypes {
                type1: expr_ty,
                type2: value_ty,
                directive: expr::directives::DirectiveKind::Max,
                message: None,
            })
        }
    }
}

impl Typeck for expr::directives::KeyEq {
    fn typeck(&self) -> Result<Type, CostExprTypeckError> {
        let then_ty = self.then_branch.typeck()?;
        let else_ty = self.else_branch.typeck()?;

        if then_ty == else_ty {
            Ok(then_ty)
        } else {
            Err(CostExprTypeckError::IncompatibleDirectiveTypes {
                type1: then_ty,
                type2: else_ty,
                directive: expr::directives::DirectiveKind::KeyEq,
                message: None,
            })
        }
    }
}

impl Typeck for expr::directives::ValueEq {
    fn typeck(&self) -> Result<Type, CostExprTypeckError> {
        let then_ty = self.then_branch.typeck()?;
        let else_ty = self.else_branch.typeck()?;

        if then_ty == else_ty {
            Ok(then_ty)
        } else {
            Err(CostExprTypeckError::IncompatibleDirectiveTypes {
                type1: then_ty,
                type2: else_ty,
                directive: expr::directives::DirectiveKind::ValueEq,
                message: None,
            })
        }
    }
}

impl Typeck for expr::Directive {
    fn typeck(&self) -> Result<Type, CostExprTypeckError> {
        match self {
            Self::Min(min) => min.typeck(),
            Self::Max(max) => max.typeck(),
            Self::KeyEq(key_eq) => key_eq.typeck(),
            Self::ValueEq(value_eq) => value_eq.typeck(),
        }
    }
}

impl Typeck for expr::ShortCircuit {
    fn typeck(&self) -> Result<Type, CostExprTypeckError> {
        Ok(Type::ShortCircuit)
    }
}

impl Typeck for expr::CostExpr {
    fn typeck(&self) -> Result<Type, CostExprTypeckError> {
        match self {
            Self::Atom(atom) => atom.typeck(),
            Self::Binary(binary) => binary.typeck(),
            Self::Neg(expr) => expr.typeck(),
            Self::Branch(branch) => branch.typeck(),
            Self::Directive(directive) => directive.typeck(),
            Self::ShortCircuit(sc) => sc.typeck(),
        }
    }
}
