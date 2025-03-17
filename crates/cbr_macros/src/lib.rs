


use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use rustc_hash::FxHashMap;
use rusty_money::iso;
use syn::{
    parse::{discouraged::Speculative, Parse, ParseStream},
    punctuated::Punctuated,
    token, Token,
};

mod kw {
    syn::custom_keyword!(branch);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    Money,
    Bip,
    ShortCircuit,
}

struct TypeckCtx {
    macros: FxHashMap<String, MacroInfo>,
}

impl TypeckCtx {
    fn new(macros: FxHashMap<String, MacroInfo>) -> Self {
        Self { macros }
    }

    fn get_macro(&self, name: &String) -> Option<&MacroInfo> {
        self.macros.get(name)
    }
}

struct MacroInfo {
    arg_names: Vec<String>,
    expr: Expr,
}

struct CompilationCtx {
    macros: FxHashMap<String, MacroInfo>,
}

impl CompilationCtx {
    fn get_macro(&self, name: &String) -> Option<&MacroInfo> {
        self.macros.get(name)
    }
}

trait CompileCostExpr {
    fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType>;
    fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream>;
}

#[derive(Debug, Clone)]
enum MoneyNodeType {
    Int(syn::LitInt),
    Float(syn::LitFloat),
}

impl ToTokens for MoneyNodeType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Int(int) => int.to_tokens(tokens),
            Self::Float(float) => float.to_tokens(tokens),
        }
    }
}

impl Parse for MoneyNodeType {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::LitInt) {
            input.parse().map(Self::Int)
        } else if lookahead.peek(syn::LitFloat) {
            input.parse().map(Self::Float)
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug, Clone)]
struct Money {
    currency: iso::Currency,
    minor: bool,
    currency_node: syn::Ident,
    money_node_type: MoneyNodeType,
}

impl CompileCostExpr for Money {
    fn typeck(&self, _ctx: &TypeckCtx) -> syn::Result<ValueType> {
        Ok(ValueType::Money)
    }

    fn compile(&self, _ctx: &CompilationCtx) -> syn::Result<TokenStream> {
        let currency = self.currency;
        let currency_str = self.currency.to_string();
        let currency_ident = quote::format_ident!("{}", currency_str);
        let minor = self.minor;
        let money_node_type = &self.money_node_type;
        match (money_node_type, minor) {
            (MoneyNodeType::Float(float), true) => Err(syn::Error::new_spanned(
                float,
                "minors should always be specified in integers",
            )),
            (MoneyNodeType::Int(int), true) => Ok(quote! {
                cbr_expr::Atom::Money(cbr_expr::Money{
                    val: #int,
                    currency: common_enums::Currency::#currency_ident,
                })
            }),
            (MoneyNodeType::Int(int), false) => {
                let val: i64 =
                    int.base10_parse::<i64>()? * i64::from(10_u32.pow(currency.exponent));
                Ok(quote! {
                    cbr_expr::Atom::Money(cbr_expr::Money{
                        val: #val,
                        currency: common_enums::Currency::#currency_ident,
                    })
                })
            }
            (MoneyNodeType::Float(float), false) => {
                let decimal: rust_decimal::Decimal = float
                    .base10_parse::<f64>()?
                    .try_into()
                    .map_err(|_| syn::Error::new_spanned(float, "float conversion error"))?;

                #[allow(clippy::as_conversions)]
                let val: i64 = decimal
                    .checked_mul(rust_decimal::Decimal::from(10_u32.pow(currency.exponent)))
                    .ok_or_else(|| {
                        syn::Error::new_spanned(float, "decimal multiplication not possible")
                    })?
                    .try_into()
                    .map_err(|_| {
                        syn::Error::new_spanned(self, "unable to get a well formed minor value")
                    })?;
                Ok(quote! {
                    cbr_expr::Atom::Money(cbr_expr::Money{
                        val: #val,
                        currency: common_enums::Currency::#currency_ident,
                    })
                })
            }
        }
    }
}

impl ToTokens for Money {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.currency_node.to_tokens(tokens);
    }
}

impl Parse for Money {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let money_node_type: MoneyNodeType = input.parse()?;
        let currency_node: syn::Ident = input.parse()?;
        let currency_str = currency_node.to_string();
        let is_minor_currency = currency_str.strip_prefix('m');
        match (money_node_type, is_minor_currency) {
            (MoneyNodeType::Float(float), Some(_)) => Err(syn::Error::new_spanned(
                float,
                "minors should always be specified in integers",
            )),
            (money_node_type @ MoneyNodeType::Int(_), None) => Ok(Self {
                currency: *iso::find(&currency_str)
                    .ok_or_else(|| syn::Error::new_spanned(&currency_node, "invalid currency"))?,
                minor: false,
                currency_node,
                money_node_type,
            }),
            (money_node_type @ MoneyNodeType::Int(_), Some(curr)) => Ok(Self {
                currency: *iso::find(curr)
                    .ok_or_else(|| syn::Error::new_spanned(&currency_node, "invalid currency"))?,
                minor: true,
                currency_node,
                money_node_type,
            }),
            (money_node_type @ MoneyNodeType::Float(_), None) => Ok(Self {
                currency: *iso::find(&currency_str)
                    .ok_or_else(|| syn::Error::new_spanned(&currency_node, "invalid currency"))?,
                minor: false,
                currency_node,
                money_node_type,
            }),
        }
    }
}

#[derive(Debug, Clone)]
enum BipNodeType {
    Int(syn::LitInt),
    Float(syn::LitFloat),
}

impl BipNodeType {
    fn get_value(&self) -> syn::Result<i64> {
        match self {
            Self::Int(int) => {
                let value = int.base10_parse::<i64>()?;

                if !(-100..=100).contains(&value) {
                    return Err(syn::Error::new_spanned(
                        int,
                        "percentage must be in the range [-100.00, 100.00]",
                    ));
                }

                Ok(value * 100)
            }

            Self::Float(flt) => {
                let value = flt.base10_parse::<f32>()?;

                if !(-100.0..=100.0).contains(&value) {
                    return Err(syn::Error::new_spanned(
                        flt,
                        "percentage must be in the range [-100.00, 100.00]",
                    ));
                }

                let rounded = (value * 100.0).round() / 100.0;
                #[allow(clippy::as_conversions)]
                let value: i64 = (rounded * 100.0).trunc() as i64;

                Ok(value)
            }
        }
    }
}

impl ToTokens for BipNodeType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Int(int) => int.to_tokens(tokens),
            Self::Float(float) => float.to_tokens(tokens),
        }
    }
}

impl Parse for BipNodeType {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::LitInt) {
            input.parse().map(Self::Int)
        } else if lookahead.peek(syn::LitFloat) {
            input.parse().map(Self::Float)
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug, Clone)]
struct Bip {
    value: i64,
    ast_node: BipNodeType,
    percent: Token![%],
}

impl CompileCostExpr for Bip {
    fn typeck(&self, _ctx: &TypeckCtx) -> syn::Result<ValueType> {
        Ok(ValueType::Bip)
    }

    fn compile(&self, _ctx: &CompilationCtx) -> syn::Result<TokenStream> {
        let bip = self.value;
        Ok(quote! {
            cbr_expr::Atom::Bip(#bip)
        })
    }
}

impl ToTokens for Bip {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ast_node.to_tokens(tokens);
        self.percent.to_tokens(tokens);
    }
}

impl Parse for Bip {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let node: BipNodeType = input.parse()?;
        let percent = input.parse::<Token![%]>()?;

        Ok(Self {
            value: node.get_value()?,
            ast_node: node,
            percent,
        })
    }
}

#[derive(Debug, Clone)]
struct Macro {
    name: syn::Ident,
    params: Punctuated<MacroVar, Token![,]>,
    paren: token::Paren,
    eq: Token![=],
    expr: Box<Expr>,
    semi: Token![;],
}

impl ToTokens for Macro {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        self.paren.surround(tokens, |tokens| {
            self.params.to_tokens(tokens);
        });
        self.eq.to_tokens(tokens);
        self.expr.to_tokens(tokens);
        self.semi.to_tokens(tokens);
    }
}

impl Parse for Macro {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name: syn::Ident = input.parse()?;
        let params_stream;
        let paren = syn::parenthesized!(params_stream in input);
        let params = Punctuated::<MacroVar, Token![,]>::parse_terminated(&params_stream)?;
        let eq: Token![=] = input.parse()?;
        let expr: Box<Expr> = Box::new(Expr::parse_with_context(input, false)?);
        let semi: Token![;] = input.parse()?;

        Ok(Self {
            name,
            params,
            paren,
            eq,
            expr,
            semi,
        })
    }
}

#[derive(Debug, Clone)]
struct MacroInvocation {
    ident: syn::Ident,
    params: Punctuated<Expr, Token![,]>,
    paren: token::Paren,
}

impl ToTokens for MacroInvocation {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);
        self.paren.surround(tokens, |tokens| {
            self.params.to_tokens(tokens);
        });
    }
}

impl Parse for MacroInvocation {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        let in_paren;
        let paren = syn::parenthesized!(in_paren in input);
        let params = Punctuated::<Expr, Token![,]>::parse_terminated_with(&in_paren, |input| {
            Expr::parse_with_context(input, true)
        })?;

        Ok(Self {
            ident,
            params,
            paren,
        })
    }
}

impl MacroInvocation {
    fn expand(&self, info: &MacroInfo) -> syn::Result<Expr> {
        if info.arg_names.len() != self.params.len() {
            return Err(syn::Error::new_spanned(
                self,
                "invalid number of arguments to macro",
            ));
        }

        let argmap = FxHashMap::<String, &Expr>::from_iter(
            info.arg_names.iter().cloned().zip(self.params.iter()),
        );

        let mut expr = info.expr.clone();
        expr.expand_macro_vars(&argmap)?;
        Ok(expr)
    }
}

impl CompileCostExpr for MacroInvocation {
    fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
        let macro_info = ctx
            .get_macro(&self.ident.to_string())
            .ok_or_else(|| syn::Error::new_spanned(self, "unknown macro"))?;

        self.expand(macro_info)?.typeck(ctx)
    }

    fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
        let macro_info = ctx
            .get_macro(&self.ident.to_string())
            .ok_or_else(|| syn::Error::new_spanned(self, "unknown macro"))?;

        self.expand(macro_info)?.compile(ctx)
    }
}

#[derive(Debug, Clone)]
struct MacroVar {
    dollar: Token![$],
    ident: syn::Ident,
}

impl ToTokens for MacroVar {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.dollar.to_tokens(tokens);
        self.ident.to_tokens(tokens);
    }
}

impl Parse for MacroVar {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let dollar: Token![$] = input.parse()?;
        let ident: syn::Ident = input.parse()?;

        Ok(Self { dollar, ident })
    }
}

#[derive(Debug, Clone)]
struct Nested {
    expr: Box<Expr>,
    ast_paren: token::Paren,
}

impl CompileCostExpr for Nested {
    fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
        self.expr.typeck(ctx)
    }

    fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
        self.expr.compile(ctx)
    }
}

impl ToTokens for Nested {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ast_paren.surround(tokens, |tokens| {
            self.expr.to_tokens(tokens);
        });
    }
}

#[derive(Debug, Clone)]
enum Atom {
    Key(syn::Ident),
    Money(Money),
    Bip(Bip),
}

impl CompileCostExpr for Atom {
    fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
        match self {
            Self::Key(_) => Ok(ValueType::Money),
            Self::Money(money) => money.typeck(ctx),
            Self::Bip(bip) => bip.typeck(ctx),
        }
    }

    fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
        let atom = match self {
            Self::Key(ident) => quote! {
                cbr_expr::Atom::Key(
                    euclid_dir::DirKey::new(
                        euclid_dir::DirKeyKind::#ident,
                        None,
                    ),
                )
            },
            Self::Money(money) => money.compile(ctx)?,
            Self::Bip(bip) => bip.compile(ctx)?,
        };

        Ok(atom)
    }
}

impl Atom {
    fn from_ident(ident: syn::Ident) -> syn::Result<Self> {
        match ident.to_string().as_str() {
            "amount" => Ok(Self::Key(syn::Ident::new("PaymentAmount", ident.span()))),
            _ => Err(syn::Error::new(
                ident.span(),
                "invalid identifier encountered",
            )),
        }
    }
}

impl ToTokens for Atom {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Key(key) => key.to_tokens(tokens),
            Self::Money(money) => money.to_tokens(tokens),
            Self::Bip(bip) => bip.to_tokens(tokens),
        }
    }
}

impl Parse for Atom {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if (lookahead.peek(syn::LitInt) || lookahead.peek(syn::LitFloat)) && input.peek2(Token![%])
        {
            input.parse().map(Self::Bip)
        } else if (lookahead.peek(syn::LitFloat) || lookahead.peek(syn::LitInt))
            && input.peek2(syn::Ident)
        {
            input.parse().map(Self::Money)
        } else if lookahead.peek(syn::Ident) {
            input.parse().and_then(Self::from_ident)
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug, Clone)]
enum BinaryOp {
    Add(Token![+]),
    Sub(Token![-]),
    Mul(Token![*]),
    Div(Token![/]),
}

impl ToTokens for BinaryOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Add(add) => add.to_tokens(tokens),
            Self::Sub(sub) => sub.to_tokens(tokens),
            Self::Mul(mul) => mul.to_tokens(tokens),
            Self::Div(div) => div.to_tokens(tokens),
        }
    }
}

impl Parse for BinaryOp {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![+]) {
            input.parse().map(Self::Add)
        } else if lookahead.peek(Token![-]) {
            input.parse().map(Self::Sub)
        } else if lookahead.peek(Token![*]) {
            input.parse().map(Self::Mul)
        } else if lookahead.peek(Token![/]) {
            input.parse().map(Self::Div)
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Precedence {
    Base,
    Arithmetic,
    Term,
}

impl From<&Precedence> for u8 {
    fn from(value: &Precedence) -> Self {
        match value {
            Precedence::Base => 0,
            Precedence::Arithmetic => 1,
            Precedence::Term => 2,
        }
    }
}

impl Precedence {
    fn of(op: &BinaryOp) -> Self {
        match op {
            BinaryOp::Add(_) | BinaryOp::Sub(_) => Self::Arithmetic,
            BinaryOp::Mul(_) | BinaryOp::Div(_) => Self::Term,
        }
    }
}

impl PartialOrd for Precedence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let this: u8 = self.into();
        let other: u8 = other.into();
        Some(this.cmp(&other))
    }
}

#[derive(Debug, Clone)]
struct Binary {
    lhs: Box<Expr>,
    op: BinaryOp,
    rhs: Box<Expr>,
}

impl CompileCostExpr for Binary {
    fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
        let lhs_ty = self.lhs.typeck(ctx)?;
        let rhs_ty = self.rhs.typeck(ctx)?;

        let build_err = || {
            let mut err =
                syn::Error::new_spanned(&self.lhs, format!("this is of type '{lhs_ty:?}'"));
            err.combine(syn::Error::new_spanned(
                &self.rhs,
                format!("this is of type '{rhs_ty:?}'"),
            ));

            let (span, message) = match &self.op {
                BinaryOp::Add(add) => {
                    (add.span, format!("cannot add '{rhs_ty:?}' to '{lhs_ty:?}'"))
                }
                BinaryOp::Sub(sub) => (
                    sub.span,
                    format!("cannot subtract '{rhs_ty:?}' from '{lhs_ty:?}'"),
                ),
                BinaryOp::Mul(mul) => (
                    mul.span,
                    format!("cannot multiply '{lhs_ty:?}' with '{rhs_ty:?}'"),
                ),
                BinaryOp::Div(div) => (
                    div.span,
                    format!("cannot divide '{lhs_ty:?}' by '{rhs_ty:?}'"),
                ),
            };

            err.combine(syn::Error::new(span, message));
            Err(err)
        };

        let final_ty = match &self.op {
            BinaryOp::Add(_) => match (lhs_ty, rhs_ty) {
                (ValueType::ShortCircuit, other) => other,
                (other, ValueType::ShortCircuit) => other,
                (ValueType::Bip, ValueType::Bip) => ValueType::Bip,
                (ValueType::Bip, ValueType::Money)
                | (ValueType::Money, ValueType::Bip)
                | (ValueType::Money, ValueType::Money) => ValueType::Money,
            },
            BinaryOp::Sub(_) => match (lhs_ty, rhs_ty) {
                (ValueType::ShortCircuit, other) => other,
                (other, ValueType::ShortCircuit) => other,
                (ValueType::Bip, ValueType::Bip) => ValueType::Bip,
                (ValueType::Bip, ValueType::Money)
                | (ValueType::Money, ValueType::Bip)
                | (ValueType::Money, ValueType::Money) => ValueType::Money,
            },
            BinaryOp::Mul(_) => match (lhs_ty, rhs_ty) {
                (ValueType::ShortCircuit, other) => other,
                (other, ValueType::ShortCircuit) => other,
                (ValueType::Bip, ValueType::Bip) => build_err()?,
                (ValueType::Bip, ValueType::Money)
                | (ValueType::Money, ValueType::Bip)
                | (ValueType::Money, ValueType::Money) => ValueType::Money,
            },
            BinaryOp::Div(_) => match (lhs_ty, rhs_ty) {
                (ValueType::ShortCircuit, other) => other,
                (other, ValueType::ShortCircuit) => other,
                (ValueType::Bip, ValueType::Bip)
                | (ValueType::Money, ValueType::Bip)
                | (ValueType::Bip, ValueType::Money) => build_err()?,
                (ValueType::Money, ValueType::Money) => ValueType::Money,
            },
        };

        Ok(final_ty)
    }

    fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
        let lhs = self.lhs.compile(ctx)?;
        let rhs = self.rhs.compile(ctx)?;
        let op = match self.op {
            BinaryOp::Add(tok) => syn::Ident::new("Add", tok.span),
            BinaryOp::Sub(tok) => syn::Ident::new("Sub", tok.span),
            BinaryOp::Mul(tok) => syn::Ident::new("Mul", tok.span),
            BinaryOp::Div(tok) => syn::Ident::new("Div", tok.span),
        };

        Ok(quote! {
            cbr_expr::CostExpr::Binary(cbr_expr::Binary {
                lhs: Box::new(#lhs),
                op: cbr_expr::BinaryOp::#op,
                rhs: Box::new(#rhs),
            })
        })
    }
}

impl ToTokens for Binary {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.lhs.to_tokens(tokens);
        self.op.to_tokens(tokens);
        self.rhs.to_tokens(tokens);
    }
}

#[derive(Debug, Clone)]
struct UnaryNeg {
    minus: Token![-],
    expr: Box<Expr>,
}

impl CompileCostExpr for UnaryNeg {
    fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
        self.expr.typeck(ctx)
    }

    fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
        let sub_expr = self.expr.compile(ctx)?;
        Ok(quote! {
            cbr_expr::CostExpr::Neg(Box::new(#sub_expr))
        })
    }
}

impl ToTokens for UnaryNeg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.minus.to_tokens(tokens);
        self.expr.to_tokens(tokens);
    }
}

#[derive(Debug, Clone)]
enum BranchPathKind {
    Default,
    Elem,
}

#[derive(Debug, Clone)]
struct BranchPath {
    ident: syn::Ident,
    kind: BranchPathKind,
}

impl ToTokens for BranchPath {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);
    }
}

impl Parse for BranchPath {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        Ok(Self {
            kind: match ident.to_string().as_str() {
                "default" => BranchPathKind::Default,
                _ => BranchPathKind::Elem,
            },
            ident,
        })
    }
}

#[derive(Debug, Clone)]
struct BranchArm {
    pattern: Punctuated<BranchPath, Token![|]>,
    arrow: Token![=>],
    rhs: Box<Expr>,
}

impl ToTokens for BranchArm {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.pattern.to_tokens(tokens);
        self.arrow.to_tokens(tokens);
        self.rhs.to_tokens(tokens);
    }
}

impl Parse for BranchArm {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let elem = Punctuated::<BranchPath, Token![|]>::parse_separated_nonempty(input)?;
        let arrow: Token![=>] = input.parse()?;
        let rhs: Expr = input.parse()?;

        Ok(Self {
            pattern: elem,
            arrow,
            rhs: Box::new(rhs),
        })
    }
}

impl BranchArm {
    fn parse_with_context(
        input: ParseStream<'_>,
        allow_macro_invocation: bool,
    ) -> syn::Result<Self> {
        let elem = Punctuated::<BranchPath, Token![|]>::parse_separated_nonempty(input)?;
        let arrow: Token![=>] = input.parse()?;
        let rhs = Expr::parse_with_context(input, allow_macro_invocation)?;

        Ok(Self {
            pattern: elem,
            arrow,
            rhs: Box::new(rhs),
        })
    }
}

#[derive(Debug, Clone)]
struct Branch {
    branch: kw::branch,
    on: syn::Ident,
    body: Punctuated<BranchArm, Token![,]>,
    brace: token::Brace,
}

impl CompileCostExpr for Branch {
    fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
        let default_arm = self
            .body
            .last()
            .ok_or_else(|| syn::Error::new_spanned(self, "empty branch found"))?;

        let last_path = default_arm
            .pattern
            .last()
            .ok_or_else(|| syn::Error::new_spanned(self, "empty path found"))?;

        let last_path_is_default = matches!(last_path.kind, BranchPathKind::Default);

        if default_arm.pattern.len() > 1 {
            return Err(syn::Error::new_spanned(
                last_path,
                if last_path_is_default {
                    "default should be a standalone arm at the end of branch"
                } else {
                    "no default arm found at the end of branch"
                },
            ));
        }

        if !last_path_is_default {
            return Err(syn::Error::new_spanned(
                default_arm,
                "missing default arm at the end of branch",
            ));
        }

        let mut path_map: HashMap<String, proc_macro2::Span> = HashMap::new();

        for path in default_arm
            .pattern
            .iter()
            .take(default_arm.pattern.len() - 1)
        {
            if matches!(path.kind, BranchPathKind::Default) {
                return Err(syn::Error::new_spanned(path, "duplicate default arm found"));
            }

            if let Some(duplicate_span) = path_map.get(&path.ident.to_string()) {
                let mut err = syn::Error::new(*duplicate_span, "first path here");
                err.combine(syn::Error::new_spanned(path, "duplicate path found"));
                return Err(err);
            }

            path_map.insert(path.ident.to_string(), path.ident.span());
        }

        let default_ty = default_arm.rhs.typeck(ctx)?;

        let out_type = self.body.iter().take(self.body.len() - 1).try_fold(
            default_ty,
            |running_type, arm| {
                for path in arm.pattern.iter() {
                    if !matches!(path.kind, BranchPathKind::Elem) {
                        return Err(syn::Error::new_spanned(arm, "duplicate default arm found"));
                    }

                    if let Some(duplicate_span) = path_map.get(&path.ident.to_string()) {
                        let mut err = syn::Error::new(*duplicate_span, "first path here");
                        err.combine(syn::Error::new_spanned(path, "duplicate path found"));
                        return Err(err);
                    }

                    path_map.insert(path.ident.to_string(), path.ident.span());
                }

                let arm_type = arm.rhs.typeck(ctx)?;

                match (running_type, arm_type) {
                    (ValueType::ShortCircuit, arm_ty) => Ok(arm_ty),
                    (running_ty, ValueType::ShortCircuit) => Ok(running_ty),
                    (running_ty, arm_ty) if running_ty == arm_ty => Ok(arm_ty),
                    (running_ty, arm_ty) => {
                        let mut err = syn::Error::new_spanned(
                            &arm.rhs,
                            "branch arms have incompatible types",
                        );
                        err.combine(syn::Error::new_spanned(
                            &default_arm.pattern,
                            format!("this arm has type '{running_ty:?}'"),
                        ));
                        err.combine(syn::Error::new_spanned(
                            &arm.pattern,
                            format!("this arm has type '{arm_ty:?}'"),
                        ));

                        Err(err)
                    }
                }
            },
        )?;

        Ok(out_type)
    }

    fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
        let key = self.on.clone();
        let mut arms = Vec::<TokenStream>::with_capacity(self.body.len() - 1);
        for arm in self.body.iter().take(self.body.len() - 1) {
            let expr = arm.rhs.compile(ctx)?;
            let mut patterns = Vec::<TokenStream>::with_capacity(arm.pattern.len());
            for path in arm.pattern.iter() {
                let value = &path.ident;
                let pattern = quote! {
                    euclid_dir::DirValue::#key(euclid_dir_enums::#key::#value)
                };
                patterns.push(pattern);
            }

            arms.push(quote! {
                cbr_expr::BranchArm {
                    patterns: Vec::from_iter([
                        #(#patterns),*
                    ]),
                    expr: #expr,
                }
            });
        }

        let default_arm = self
            .body
            .last()
            .ok_or_else(|| syn::Error::new_spanned(self, "empty branch found"))?;

        let default_expr = default_arm.rhs.compile(ctx)?;

        Ok(quote! {
            cbr_expr::Branch {
                elems: Vec::from_iter([
                    #(#arms),*
                ]),
                default: #default_expr,
            }
        })
    }
}

impl ToTokens for Branch {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.branch.to_tokens(tokens);
        self.on.to_tokens(tokens);
        self.brace.surround(tokens, |tokens| {
            self.body.to_tokens(tokens);
        });
    }
}

impl Parse for Branch {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let branch: kw::branch = input.parse()?;
        let on: syn::Ident = input.parse()?;
        let in_brace;
        let brace = syn::braced!(in_brace in input);
        let body = Punctuated::<BranchArm, Token![,]>::parse_terminated(&in_brace)?;

        Ok(Self {
            branch,
            on,
            body,
            brace,
        })
    }
}

impl Branch {
    fn parse_with_context(
        input: ParseStream<'_>,
        allow_macro_invocation: bool,
    ) -> syn::Result<Self> {
        let branch: kw::branch = input.parse()?;
        let on: syn::Ident = input.parse()?;
        let in_brace;
        let brace = syn::braced!(in_brace in input);
        let body = {
            let mut punctuated = Punctuated::new();

            loop {
                if in_brace.is_empty() {
                    break;
                }

                let value = BranchArm::parse_with_context(&in_brace, allow_macro_invocation)?;
                punctuated.push_value(value);
                if in_brace.is_empty() {
                    break;
                }

                let punct = in_brace.parse()?;
                punctuated.push_punct(punct);
            }

            punctuated
        };

        Ok(Self {
            branch,
            on,
            body,
            brace,
        })
    }
}

#[derive(Debug, Clone)]
enum ShortCircuitKind {
    Fail(syn::Ident),
    Unsupported(syn::Ident),
}

impl ToTokens for ShortCircuitKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Fail(ident) => ident.to_tokens(tokens),
            Self::Unsupported(ident) => ident.to_tokens(tokens),
        }
    }
}

impl Parse for ShortCircuitKind {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        match ident.to_string().as_str() {
            "Fail" => Ok(Self::Fail(ident)),
            "Unsupported" => Ok(Self::Unsupported(ident)),
            invalid => Err(syn::Error::new_spanned(
                ident,
                format!("Invalid short circuit directive: {invalid}"),
            )),
        }
    }
}

impl CompileCostExpr for ShortCircuitKind {
    fn typeck(&self, _ctx: &TypeckCtx) -> syn::Result<ValueType> {
        Ok(ValueType::ShortCircuit)
    }

    fn compile(&self, _ctx: &CompilationCtx) -> syn::Result<TokenStream> {
        let ident = match self {
            Self::Fail(ident) => ident,
            Self::Unsupported(ident) => ident,
        };

        Ok(quote! {
            cbr_expr::ShortCircuitKind::#ident
        })
    }
}

#[derive(Debug, Clone)]
struct ShortCircuit {
    bang: Token![!],
    kind: ShortCircuitKind,
    paren: token::Paren,
    reason: syn::LitStr,
}

impl ToTokens for ShortCircuit {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.bang.to_tokens(tokens);
        self.kind.to_tokens(tokens);
        self.paren.surround(tokens, |tokens| {
            self.reason.to_tokens(tokens);
        });
    }
}

impl Parse for ShortCircuit {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let bang: Token![!] = input.parse()?;
        let kind: ShortCircuitKind = input.parse()?;
        let in_parens;
        let paren = syn::parenthesized!(in_parens in input);
        let reason: syn::LitStr = in_parens.parse()?;

        if !in_parens.is_empty() {
            return Err(syn::Error::new(
                in_parens.span(),
                "unexpected argument to short circuit",
            ));
        }

        Ok(Self {
            bang,
            kind,
            paren,
            reason,
        })
    }
}

impl CompileCostExpr for ShortCircuit {
    fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
        self.kind.typeck(ctx)
    }

    fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
        let kind = self.kind.compile(ctx)?;
        let reason = self.reason.clone();

        Ok(quote! {
            cbr_expr::CostExpr::ShortCircuit(cbr_expr::ShortCircuit {
                kind: #kind,
                reason: #reason.to_string(),
            })
        })
    }
}

mod directives {
    use std::ops::{Deref, DerefMut};

    use super::*;

    #[derive(Debug, Clone)]
    pub(super) enum Directive {
        Min(Min),
        Max(Max),
        KeyEq(KeyEq),
        ValueEq(ValueEq),
    }

    impl ToTokens for Directive {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            match self {
                Self::Min(min) => min.to_tokens(tokens),
                Self::Max(max) => max.to_tokens(tokens),
                Self::KeyEq(key_eq) => key_eq.to_tokens(tokens),
                Self::ValueEq(value_eq) => value_eq.to_tokens(tokens),
            }
        }
    }

    impl CompileCostExpr for Directive {
        fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
            match self {
                Self::Min(min) => min.typeck(ctx),
                Self::Max(max) => max.typeck(ctx),
                Self::KeyEq(key_eq) => key_eq.typeck(ctx),
                Self::ValueEq(value_eq) => value_eq.typeck(ctx),
            }
        }

        fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
            match self {
                Self::Min(min) => min.compile(ctx),
                Self::Max(max) => max.compile(ctx),
                Self::KeyEq(key_eq) => key_eq.compile(ctx),
                Self::ValueEq(value_eq) => value_eq.compile(ctx),
            }
        }
    }

    impl Directive {
        pub(super) fn parse_with_context(
            input: ParseStream<'_>,
            allow_macro_invocation: bool,
        ) -> syn::Result<Self> {
            let fork = input.fork();
            fork.parse::<Token![@]>()?;
            let ident: syn::Ident = fork.parse()?;

            match ident.to_string().as_str() {
                "min" => Min::parse_with_context(input, allow_macro_invocation).map(Self::Min),
                "max" => Max::parse_with_context(input, allow_macro_invocation).map(Self::Max),
                "keyeq" => {
                    KeyEq::parse_with_context(input, allow_macro_invocation).map(Self::KeyEq)
                }
                "valeq" => {
                    ValueEq::parse_with_context(input, allow_macro_invocation).map(Self::ValueEq)
                }
                _ => Err(syn::Error::new_spanned(ident, "unknown directive invoked")),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub(super) struct DirectiveArg<Sep, ArgTy> {
        sep: Option<Sep>,
        pub(super) arg_ty: ArgTy,
    }

    impl<Sep, ArgTy> Deref for DirectiveArg<Sep, ArgTy> {
        type Target = ArgTy;

        fn deref(&self) -> &Self::Target {
            &self.arg_ty
        }
    }

    impl<Sep, ArgTy> DerefMut for DirectiveArg<Sep, ArgTy> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.arg_ty
        }
    }

    impl<Sep, ArgTy> ToTokens for DirectiveArg<Sep, ArgTy>
    where
        Sep: ToTokens,
        ArgTy: ToTokens,
    {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.sep.to_tokens(tokens);
            self.arg_ty.to_tokens(tokens);
        }
    }

    impl<Sep, ArgTy> DirectiveArg<Sep, ArgTy>
    where
        Sep: Parse,
        ArgTy: Parse,
    {
        fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
            let sep: Sep = input.parse()?;
            let arg_ty: ArgTy = input.parse()?;

            Ok(Self {
                sep: Some(sep),
                arg_ty,
            })
        }

        fn parse_with<F>(input: ParseStream<'_>, parse_func: F) -> syn::Result<Self>
        where
            F: Fn(ParseStream<'_>) -> syn::Result<ArgTy>,
        {
            let sep: Sep = input.parse()?;
            let arg_ty = parse_func(input)?;

            Ok(Self {
                sep: Some(sep),
                arg_ty,
            })
        }

        fn parse_nosep(input: ParseStream<'_>) -> syn::Result<Self> {
            let arg_ty: ArgTy = input.parse()?;

            Ok(Self { sep: None, arg_ty })
        }

        fn parse_nosep_with<F>(input: ParseStream<'_>, parse_func: F) -> syn::Result<Self>
        where
            F: Fn(ParseStream<'_>) -> syn::Result<ArgTy>,
        {
            let arg_ty = parse_func(input)?;

            Ok(Self { sep: None, arg_ty })
        }
    }

    type Arg<T> = DirectiveArg<Token![,], T>;
    type ArgExpr = Arg<Box<Expr>>;
    type ArgIdent = Arg<syn::Ident>;
    type ArgIdentOrNumber = Arg<IdentOrNumber>;

    fn ensure_no_redundant_args(input: ParseStream<'_>) -> syn::Result<()> {
        if input.is_empty() {
            Ok(())
        } else {
            Err(syn::Error::new(
                input.span(),
                "unexpected extra argument found",
            ))
        }
    }

    #[derive(Debug, Clone)]
    pub(super) struct Min {
        at: Token![@],
        name: syn::Ident,
        paren: token::Paren,
        pub(super) expr: ArgExpr,
        pub(super) value: ArgExpr,
    }

    impl ToTokens for Min {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.at.to_tokens(tokens);
            self.name.to_tokens(tokens);
            self.paren.surround(tokens, |tokens| {
                self.expr.to_tokens(tokens);
                self.value.to_tokens(tokens);
            });
        }
    }

    impl CompileCostExpr for Min {
        fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
            let expr_type = self.expr.typeck(ctx)?;
            let value_type = self.value.typeck(ctx)?;

            if expr_type != value_type {
                let mut error =
                    syn::Error::new_spanned(&self.name, "lhs and rhs types don't match");
                error.combine(syn::Error::new_spanned(
                    &self.expr,
                    format!("this has type '{expr_type:?}'"),
                ));
                error.combine(syn::Error::new_spanned(
                    &self.value,
                    format!("this has type '{value_type:?}'"),
                ));
                Err(error)
            } else {
                Ok(expr_type)
            }
        }

        fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
            let expr = self.expr.compile(ctx)?;
            let value = self.value.compile(ctx)?;

            Ok(quote! {
                cbr_expr::CostExpr::Directive(Box::new(cbr_expr::Directive::Min(
                    cbr_expr::directives::Min {
                        expr: #expr,
                        value: #value,
                    }
                )))
            })
        }
    }

    impl Min {
        fn parse_with_context(
            input: ParseStream<'_>,
            allow_macro_invocation: bool,
        ) -> syn::Result<Self> {
            let at: Token![@] = input.parse()?;
            let name: syn::Ident = input.parse()?;
            let in_parens;
            let paren = syn::parenthesized!(in_parens in input);
            let expr = ArgExpr::parse_nosep_with(&in_parens, |inp| {
                Expr::parse_with_context(inp, allow_macro_invocation).map(Box::new)
            })?;
            let value = ArgExpr::parse_with(&in_parens, |inp| {
                Expr::parse_with_context(inp, allow_macro_invocation).map(Box::new)
            })?;

            ensure_no_redundant_args(&in_parens)?;

            Ok(Self {
                at,
                name,
                paren,
                expr,
                value,
            })
        }
    }

    #[derive(Debug, Clone)]
    pub(super) struct Max {
        at: Token![@],
        name: syn::Ident,
        paren: token::Paren,
        pub(super) expr: ArgExpr,
        pub(super) value: ArgExpr,
    }

    impl ToTokens for Max {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.at.to_tokens(tokens);
            self.name.to_tokens(tokens);
            self.paren.surround(tokens, |tokens| {
                self.expr.to_tokens(tokens);
                self.value.to_tokens(tokens);
            });
        }
    }

    impl CompileCostExpr for Max {
        fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
            let expr_type = self.expr.typeck(ctx)?;
            let value_type = self.value.typeck(ctx)?;

            if expr_type != value_type {
                let mut error =
                    syn::Error::new_spanned(&self.name, "lhs and rhs types don't match");
                error.combine(syn::Error::new_spanned(
                    &self.expr,
                    format!("this has type '{expr_type:?}'"),
                ));
                error.combine(syn::Error::new_spanned(
                    &self.value,
                    format!("this has type '{value_type:?}'"),
                ));
                Err(error)
            } else {
                Ok(expr_type)
            }
        }

        fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
            let expr = self.expr.compile(ctx)?;
            let value = self.value.compile(ctx)?;

            Ok(quote! {
                cbr_expr::CostExpr::Directive(Box::new(cbr_expr::Directive::Max(
                    cbr_expr::directives::Max {
                        expr: #expr,
                        value: #value,
                    }
                )))
            })
        }
    }

    impl Max {
        fn parse_with_context(
            input: ParseStream<'_>,
            allow_macro_invocation: bool,
        ) -> syn::Result<Self> {
            let at: Token![@] = input.parse()?;
            let name: syn::Ident = input.parse()?;
            let in_parens;
            let paren = syn::parenthesized!(in_parens in input);
            let expr = ArgExpr::parse_nosep_with(&in_parens, |inp| {
                Expr::parse_with_context(inp, allow_macro_invocation).map(Box::new)
            })?;
            let value = ArgExpr::parse_with(&in_parens, |inp| {
                Expr::parse_with_context(inp, allow_macro_invocation).map(Box::new)
            })?;

            ensure_no_redundant_args(&in_parens)?;

            Ok(Self {
                at,
                name,
                paren,
                expr,
                value,
            })
        }
    }

    #[derive(Debug, Clone)]
    pub(super) struct KeyEq {
        at: Token![@],
        name: syn::Ident,
        paren: token::Paren,
        lhs: ArgIdent,
        rhs: ArgIdent,
        pub(super) then_branch: ArgExpr,
        pub(super) else_branch: ArgExpr,
    }

    impl ToTokens for KeyEq {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.at.to_tokens(tokens);
            self.name.to_tokens(tokens);
            self.paren.surround(tokens, |tokens| {
                self.lhs.to_tokens(tokens);
                self.rhs.to_tokens(tokens);
                self.then_branch.to_tokens(tokens);
                self.else_branch.to_tokens(tokens);
            });
        }
    }

    impl CompileCostExpr for KeyEq {
        fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
            let then_type = self.then_branch.typeck(ctx)?;
            let else_type = self.else_branch.typeck(ctx)?;

            if then_type != else_type {
                let mut error =
                    syn::Error::new_spanned(&self.name, "then and else types don't match");
                error.combine(syn::Error::new_spanned(
                    &self.then_branch.arg_ty,
                    format!("this has type '{then_type:?}'"),
                ));
                error.combine(syn::Error::new_spanned(
                    &self.else_branch.arg_ty,
                    format!("this has type '{else_type:?}'"),
                ));
                Err(error)
            } else {
                Ok(then_type)
            }
        }

        fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
            let key1_ident = self.lhs.clone();
            let key2_ident = self.rhs.clone();
            let then_branch = self.then_branch.compile(ctx)?;
            let else_branch = self.else_branch.compile(ctx)?;

            Ok(quote! {
                cbr_expr::CostExpr::Directive(Box::new(cbr_expr::Directive::KeyEq(
                    cbr_expr::directives::KeyEq {
                        lhs: euclid_dir::DirKey::new(
                            euclid_dir::DirKeyKind::#key1_ident,
                            None
                        ),
                        rhs: euclid_dir::DirKey::new(
                            euclid_dir::DirKeyKind::#key2_ident,
                            None,
                        ),
                        then_branch: #then_branch,
                        else_branch: #else_branch,
                    }
                )))
            })
        }
    }

    impl KeyEq {
        fn parse_with_context(
            input: ParseStream<'_>,
            allow_macro_invocation: bool,
        ) -> syn::Result<Self> {
            let at: Token![@] = input.parse()?;
            let name: syn::Ident = input.parse()?;
            let in_parens;
            let paren = syn::parenthesized!(in_parens in input);
            let lhs = ArgIdent::parse_nosep(&in_parens)?;
            let rhs = ArgIdent::parse(&in_parens)?;
            let then_branch = ArgExpr::parse_with(&in_parens, |inp| {
                Expr::parse_with_context(inp, allow_macro_invocation).map(Box::new)
            })?;
            let else_branch = ArgExpr::parse_with(&in_parens, |inp| {
                Expr::parse_with_context(inp, allow_macro_invocation).map(Box::new)
            })?;

            ensure_no_redundant_args(&in_parens)?;

            Ok(Self {
                at,
                name,
                paren,
                lhs,
                rhs,
                then_branch,
                else_branch,
            })
        }
    }

    #[derive(Debug, Clone)]
    enum IdentOrNumber {
        Ident(syn::Ident),
        Number(syn::LitInt),
    }

    impl ToTokens for IdentOrNumber {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            match self {
                Self::Ident(ident) => ident.to_tokens(tokens),
                Self::Number(number) => number.to_tokens(tokens),
            }
        }
    }

    impl Parse for IdentOrNumber {
        fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
            let lookahead = input.lookahead1();
            if lookahead.peek(syn::Ident) {
                input.parse().map(Self::Ident)
            } else if lookahead.peek(syn::LitInt) {
                input.parse().map(Self::Number)
            } else {
                Err(lookahead.error())
            }
        }
    }

    #[derive(Debug, Clone)]
    pub(super) struct ValueEq {
        at: Token![@],
        name: syn::Ident,
        paren: token::Paren,
        lhs: ArgIdent,
        rhs: ArgIdentOrNumber,
        pub(super) then_branch: ArgExpr,
        pub(super) else_branch: ArgExpr,
    }

    impl ToTokens for ValueEq {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.at.to_tokens(tokens);
            self.name.to_tokens(tokens);
            self.paren.surround(tokens, |tokens| {
                self.lhs.to_tokens(tokens);
                self.rhs.to_tokens(tokens);
                self.then_branch.to_tokens(tokens);
                self.else_branch.to_tokens(tokens);
            });
        }
    }

    impl CompileCostExpr for ValueEq {
        fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
            let then_type = self.then_branch.typeck(ctx)?;
            let else_type = self.else_branch.typeck(ctx)?;

            if then_type != else_type {
                let mut error =
                    syn::Error::new_spanned(&self.name, "then and else types don't match");
                error.combine(syn::Error::new_spanned(
                    &self.then_branch.arg_ty,
                    format!("this has type '{then_type:?}'"),
                ));
                error.combine(syn::Error::new_spanned(
                    &self.else_branch.arg_ty,
                    format!("this has type '{else_type:?}'"),
                ));
                Err(error)
            } else {
                Ok(then_type)
            }
        }

        fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
            let key_ident = self.lhs.clone();
            let rhs_val = match &self.rhs.arg_ty {
                IdentOrNumber::Ident(value_ident) => quote! {
                    euclid_dir::DirValue::#key_ident(euclid_dir_enums::#key_ident::#value_ident)
                },

                IdentOrNumber::Number(num) => quote! {
                    euclid_dir::DirValue::#key_ident(euclid_types::NumValue {
                        number: #num,
                        refinement: None,
                    })
                },
            };
            let then_branch = self.then_branch.compile(ctx)?;
            let else_branch = self.else_branch.compile(ctx)?;

            Ok(quote! {
                cbr_expr::CostExpr::Directive(Box::new(cbr_expr::Directive::ValueEq(
                    cbr_expr::directives::ValueEq {
                        key: euclid_dir::DirKey::new(
                            euclid_dir::DirKeyKind::#key_ident,
                            None,
                        ),
                        value: #rhs_val,
                        then_branch: #then_branch,
                        else_branch: #else_branch,
                    }
                )))
            })
        }
    }

    impl ValueEq {
        fn parse_with_context(
            input: ParseStream<'_>,
            allow_macro_invocation: bool,
        ) -> syn::Result<Self> {
            let at: Token![@] = input.parse()?;
            let name: syn::Ident = input.parse()?;
            let in_parens;
            let paren = syn::parenthesized!(in_parens in input);
            let lhs = ArgIdent::parse_nosep(&in_parens)?;
            let rhs = ArgIdentOrNumber::parse(&in_parens)?;
            let then_branch = ArgExpr::parse_with(&in_parens, |inp| {
                Expr::parse_with_context(inp, allow_macro_invocation).map(Box::new)
            })?;
            let else_branch = ArgExpr::parse_with(&in_parens, |inp| {
                Expr::parse_with_context(inp, allow_macro_invocation).map(Box::new)
            })?;

            ensure_no_redundant_args(&in_parens)?;

            Ok(Self {
                at,
                name,
                paren,
                lhs,
                rhs,
                then_branch,
                else_branch,
            })
        }
    }
}

#[derive(Debug, Clone)]
enum Expr {
    Atom(Atom),
    Binary(Binary),
    UnaryNeg(UnaryNeg),
    Nested(Nested),
    Branch(Branch),
    MacroVar(MacroVar),
    MacroInvocation(MacroInvocation),
    Directive(directives::Directive),
    ShortCircuit(ShortCircuit),
}

impl CompileCostExpr for Expr {
    fn typeck(&self, ctx: &TypeckCtx) -> syn::Result<ValueType> {
        match self {
            Self::Atom(atom) => atom.typeck(ctx),
            Self::Binary(binary) => binary.typeck(ctx),
            Self::UnaryNeg(neg) => neg.typeck(ctx),
            Self::Nested(nested) => nested.typeck(ctx),
            Self::Branch(branch) => branch.typeck(ctx),
            Self::MacroVar(var) => Err(syn::Error::new_spanned(var, "unexpanded macro var found")),
            Self::MacroInvocation(invocation) => invocation.typeck(ctx),
            Self::Directive(directive) => directive.typeck(ctx),
            Self::ShortCircuit(short_circuit) => short_circuit.typeck(ctx),
        }
    }

    fn compile(&self, ctx: &CompilationCtx) -> syn::Result<TokenStream> {
        Ok(match self {
            Self::Atom(atom) => {
                let tokens = atom.compile(ctx)?;
                quote! {
                    cbr_expr::CostExpr::Atom(#tokens)
                }
            }
            Self::Binary(bin) => bin.compile(ctx)?,
            Self::UnaryNeg(unary) => unary.compile(ctx)?,
            Self::Nested(nested) => nested.compile(ctx)?,
            Self::Branch(branch) => {
                let tokens = branch.compile(ctx)?;
                quote! {
                    cbr_expr::CostExpr::Branch(Box::new(#tokens))
                }
            }
            Self::MacroVar(var) => Err(syn::Error::new_spanned(var, "unexpanded macro var found"))?,
            Self::MacroInvocation(invocation) => invocation.compile(ctx)?,
            Self::Directive(directive) => directive.compile(ctx)?,
            Self::ShortCircuit(short_circuit) => short_circuit.compile(ctx)?,
        })
    }
}

impl ToTokens for Expr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Atom(atom) => atom.to_tokens(tokens),
            Self::Binary(bin) => bin.to_tokens(tokens),
            Self::UnaryNeg(neg) => neg.to_tokens(tokens),
            Self::Nested(nested) => nested.to_tokens(tokens),
            Self::Branch(branch) => branch.to_tokens(tokens),
            Self::MacroVar(var) => var.to_tokens(tokens),
            Self::MacroInvocation(invocation) => invocation.to_tokens(tokens),
            Self::Directive(directive) => directive.to_tokens(tokens),
            Self::ShortCircuit(short_circuit) => short_circuit.to_tokens(tokens),
        }
    }
}

impl Parse for Expr {
    #[inline]
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Self::parse_with_context(input, true)
    }
}

impl Expr {
    fn parse_atomic(input: ParseStream<'_>, allow_macro_invocation: bool) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(token::Paren) {
            let inner;
            let paren_token = syn::parenthesized!(inner in input);

            Ok(Self::Nested(Nested {
                expr: Box::new(Self::parse_with_context(&inner, allow_macro_invocation)?),
                ast_paren: paren_token,
            }))
        } else if lookahead.peek(kw::branch) {
            Branch::parse_with_context(input, allow_macro_invocation).map(Self::Branch)
        } else if lookahead.peek(syn::LitInt) || lookahead.peek(syn::LitFloat) {
            input.parse().map(Self::Atom)
        } else if lookahead.peek(Token![@]) {
            directives::Directive::parse_with_context(input, allow_macro_invocation)
                .map(Self::Directive)
        } else if lookahead.peek(Token![!]) {
            input.parse().map(Self::ShortCircuit)
        } else if !allow_macro_invocation && lookahead.peek(Token![$]) {
            input.parse().map(Self::MacroVar)
        } else if lookahead.peek(syn::Ident) {
            let fork = input.fork();
            let ident: syn::Ident = fork.parse()?;

            if fork.peek(token::Paren) && !allow_macro_invocation {
                Err(syn::Error::new_spanned(
                    ident,
                    "macro invocations not allowed in this context",
                ))
            } else if fork.peek(token::Paren) && allow_macro_invocation {
                input.parse().map(Self::MacroInvocation)
            } else {
                input.parse().map(Self::Atom)
            }
        } else {
            Err(lookahead.error())
        }
    }

    fn parse_unary(input: ParseStream<'_>, allow_macro_invocation: bool) -> syn::Result<Self> {
        if input.peek(Token![-]) {
            let minus = input.parse::<Token![-]>()?;
            Ok(Self::UnaryNeg(UnaryNeg {
                minus,
                expr: Box::new(Self::parse_unary(input, allow_macro_invocation)?),
            }))
        } else {
            Self::parse_atomic(input, allow_macro_invocation)
        }
    }

    /// Parses a binary expression using "Pratt Parsing" which is sensitive to operator precedence
    fn parse_expr_inner(
        input: ParseStream<'_>,
        mut lhs: Self,
        base: Precedence,
        allow_macro_invocation: bool,
    ) -> syn::Result<Self> {
        loop {
            if input.is_empty() {
                break;
            }

            let op = match input.fork().parse::<BinaryOp>() {
                Ok(op) => op,
                Err(_) => break,
            };

            let prec = Precedence::of(&op);
            if prec < base {
                break;
            }

            input.parse::<BinaryOp>()?;
            let mut rhs = Self::parse_unary(input, allow_macro_invocation)?;
            loop {
                let next = if let Ok(op) = input.fork().parse::<BinaryOp>() {
                    Precedence::of(&op)
                } else {
                    Precedence::Base
                };

                if next > prec {
                    rhs = Self::parse_expr_inner(input, rhs, next, allow_macro_invocation)?;
                } else {
                    break;
                }
            }

            lhs = Self::Binary(Binary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            });
        }

        Ok(lhs)
    }

    fn parse_with_context(
        input: ParseStream<'_>,
        allow_macro_invocation: bool,
    ) -> syn::Result<Self> {
        let unary = Self::parse_unary(input, allow_macro_invocation)?;
        Self::parse_expr_inner(input, unary, Precedence::Base, allow_macro_invocation)
    }

    fn expand_macro_vars(&mut self, argmap: &FxHashMap<String, &Self>) -> syn::Result<()> {
        match self {
            Self::Atom(_) | Self::ShortCircuit(_) => {}

            Self::Binary(bin) => {
                bin.lhs.expand_macro_vars(argmap)?;
                bin.rhs.expand_macro_vars(argmap)?;
            }

            Self::UnaryNeg(unary) => {
                unary.expr.expand_macro_vars(argmap)?;
            }

            Self::Nested(nested) => {
                nested.expr.expand_macro_vars(argmap)?;
            }

            Self::Branch(branch) => {
                for arm in branch.body.iter_mut() {
                    arm.rhs.expand_macro_vars(argmap)?;
                }
            }

            Self::MacroVar(var) => {
                let expr = argmap
                    .get(&var.ident.to_string())
                    .ok_or_else(|| syn::Error::new_spanned(var, "invalid macro variable"))?;

                *self = (*expr).clone();
            }

            Self::MacroInvocation(invocation) => {
                Err(syn::Error::new_spanned(
                    invocation,
                    "invalid nested macro invocation found",
                ))?;
            }

            Self::Directive(directive) => match directive {
                directives::Directive::Min(min) => {
                    min.expr.expand_macro_vars(argmap)?;
                    min.value.expand_macro_vars(argmap)?;
                }

                directives::Directive::Max(max) => {
                    max.expr.expand_macro_vars(argmap)?;
                    max.value.expand_macro_vars(argmap)?;
                }

                directives::Directive::KeyEq(key_eq) => {
                    key_eq.then_branch.expand_macro_vars(argmap)?;
                    key_eq.else_branch.expand_macro_vars(argmap)?;
                }

                directives::Directive::ValueEq(value_eq) => {
                    value_eq.then_branch.expand_macro_vars(argmap)?;
                    value_eq.else_branch.expand_macro_vars(argmap)?;
                }
            },
        }

        Ok(())
    }
}

struct Program {
    macros: Vec<Macro>,
    body: Expr,
}

impl ToTokens for Program {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for cost_macro in &self.macros {
            cost_macro.to_tokens(tokens);
        }
        self.body.to_tokens(tokens);
    }
}

impl Parse for Program {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut macros = Vec::<Macro>::new();

        loop {
            let fork = input.fork();
            if let Ok(mac) = fork.parse() {
                macros.push(mac);
                input.advance_to(&fork);
            } else {
                break;
            };
        }

        let expr: Expr = input.parse()?;

        Ok(Self { macros, body: expr })
    }
}

impl From<Macro> for MacroInfo {
    fn from(value: Macro) -> Self {
        Self {
            arg_names: value
                .params
                .iter()
                .map(|var| var.ident.to_string())
                .collect(),
            expr: *value.expr,
        }
    }
}

fn costexpr_inner(ts: proc_macro::TokenStream) -> syn::Result<TokenStream> {
    let program = syn::parse::<Program>(ts)?;
    let mut macros: FxHashMap<String, MacroInfo> = FxHashMap::default();
    for mac in program.macros {
        macros.insert(mac.name.to_string(), mac.into());
    }

    let typeck_ctx = TypeckCtx::new(macros);
    program.body.typeck(&typeck_ctx)?;

    let compile_ctx = CompilationCtx {
        macros: typeck_ctx.macros,
    };
    let compiled = program.body.compile(&compile_ctx)?;
    Ok(quote! {{ #compiled }})
}

#[proc_macro]
pub fn costexpr(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match costexpr_inner(ts) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
