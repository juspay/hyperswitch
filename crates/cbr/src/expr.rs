use std::fmt;

use common_enums::Currency;

use euclid::frontend::dir;

#[derive(Debug, Clone)]
pub struct BranchArm {
    pub patterns: Vec<dir::DirValue>,
    pub expr: CostExpr,
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub elems: Vec<BranchArm>,
    pub default: CostExpr,
}

#[derive(Debug, Clone)]
pub enum Atom {
    Key(dir::DirKey),
    Bip(i64),
    Money(Money),
}

#[derive(Debug, Clone)]
pub struct Money {
    pub val: i64,
    pub currency: Currency,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Add => '+',
                Self::Sub => '-',
                Self::Mul => '*',
                Self::Div => '/',
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct Binary {
    pub lhs: Box<CostExpr>,
    pub op: BinaryOp,
    pub rhs: Box<CostExpr>,
}

#[derive(Debug, Clone)]
pub enum ShortCircuitKind {
    Fail,
    Unsupported,
}

#[derive(Debug, Clone)]
pub struct ShortCircuit {
    pub kind: ShortCircuitKind,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum CostExpr {
    Atom(Atom),
    Binary(Binary),
    Neg(Box<CostExpr>),
    Branch(Box<Branch>),
    Directive(Box<Directive>),
    ShortCircuit(ShortCircuit),
}

pub use directives::Directive;

pub mod directives {
    use super::*;

    #[derive(Debug, Clone)]
    pub enum Directive {
        Min(Min),
        Max(Max),
        KeyEq(KeyEq),
        ValueEq(ValueEq),
    }

    #[derive(Debug, Clone, Copy)]
    pub enum DirectiveKind {
        Min,
        Max,
        KeyEq,
        ValueEq,
    }

    impl fmt::Display for DirectiveKind {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    Self::Min => "Min",
                    Self::Max => "Max",
                    Self::KeyEq => "KeyEq",
                    Self::ValueEq => "ValueEq",
                }
            )
        }
    }

    #[derive(Debug, Clone)]
    pub struct Min {
        pub expr: CostExpr,
        pub value: CostExpr,
    }

    #[derive(Debug, Clone)]
    pub struct Max {
        pub expr: CostExpr,
        pub value: CostExpr,
    }

    #[derive(Debug, Clone)]
    pub struct KeyEq {
        pub lhs: dir::DirKey,
        pub rhs: dir::DirKey,
        pub then_branch: CostExpr,
        pub else_branch: CostExpr,
    }

    #[derive(Debug, Clone)]
    pub struct ValueEq {
        pub key: dir::DirKey,
        pub value: dir::DirValue,
        pub then_branch: CostExpr,
        pub else_branch: CostExpr,
    }
}
