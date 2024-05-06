use std::collections::{HashMap, HashSet};

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token, Token,
};

mod kw {
    syn::custom_keyword!(imports);
    syn::custom_keyword!(key);
    syn::custom_keyword!(value);
    syn::custom_keyword!(map);
    syn::custom_keyword!(comparison);
    syn::custom_keyword!(number);
    syn::custom_keyword!(domain);
    syn::custom_keyword!(with);
    syn::custom_keyword!(identifier);
    syn::custom_keyword!(and);
    syn::custom_keyword!(description);
    syn::custom_keyword!(any);
    syn::custom_keyword!(rule);
    syn::custom_keyword!(under);
}

struct Imports {
    imports: kw::imports,
    brace: token::Brace,
    uses: Vec<syn::ItemUse>,
}

impl Parse for Imports {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let imports: kw::imports = input.parse()?;
        let content;
        let brace = syn::braced!(content in input);
        let mut uses = Vec::new();
        while !content.is_empty() {
            let use_item: syn::ItemUse = content.parse()?;
            uses.push(use_item)
        }

        Ok(Self {
            imports,
            brace,
            uses,
        })
    }
}

impl ToTokens for Imports {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.imports.to_tokens(tokens);
        self.brace.surround(tokens, |tokens| {
            for item_use in &self.uses {
                item_use.to_tokens(tokens)
            }
        });
    }
}

struct KeyType {
    key: kw::key,
    ty_kw: Token![type],
    eq: Token![=],
    ty: syn::Ident,
    semi: Token![;],
}

impl Parse for KeyType {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: kw::key = input.parse()?;
        let ty_kw: Token![type] = input.parse()?;
        let eq: Token![=] = input.parse()?;
        let ty: syn::Ident = input.parse()?;
        let semi: Token![;] = input.parse()?;

        Ok(Self {
            key,
            ty_kw,
            eq,
            ty,
            semi,
        })
    }
}

impl ToTokens for KeyType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.key.to_tokens(tokens);
        self.ty_kw.to_tokens(tokens);
        self.eq.to_tokens(tokens);
        self.ty.to_tokens(tokens);
        self.semi.to_tokens(tokens);
    }
}

struct ValueType {
    value: kw::value,
    ty_kw: Token![type],
    eq: Token![=],
    ty: syn::Ident,
    semi: Token![;],
}

impl Parse for ValueType {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let value: kw::value = input.parse()?;
        let ty_kw: Token![type] = input.parse()?;
        let eq: Token![=] = input.parse()?;
        let ty: syn::Ident = input.parse()?;
        let semi: Token![;] = input.parse()?;

        Ok(Self {
            value,
            ty_kw,
            eq,
            ty,
            semi,
        })
    }
}

impl ToTokens for ValueType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens);
        self.ty_kw.to_tokens(tokens);
        self.eq.to_tokens(tokens);
        self.ty.to_tokens(tokens);
        self.semi.to_tokens(tokens);
    }
}

enum ComparisonOperator {
    GreaterThan(Token![>]),
    LessThan(Token![<]),
    Equal(Token![=]),
    NotEqual(Token![!=]),
    GreaterThanEqual(Token![>=]),
    LessThanEqual(Token![<=]),
}

impl Parse for ComparisonOperator {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![>=]) {
            input.parse().map(Self::GreaterThanEqual)
        } else if lookahead.peek(Token![<=]) {
            input.parse().map(Self::LessThanEqual)
        } else if lookahead.peek(Token![!=]) {
            input.parse().map(Self::NotEqual)
        } else if lookahead.peek(Token![=]) {
            input.parse().map(Self::Equal)
        } else if lookahead.peek(Token![>]) {
            input.parse().map(Self::GreaterThan)
        } else if lookahead.peek(Token![<]) {
            input.parse().map(Self::LessThan)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for ComparisonOperator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::GreaterThan(gt) => gt.to_tokens(tokens),
            Self::LessThan(lt) => lt.to_tokens(tokens),
            Self::Equal(eq) => eq.to_tokens(tokens),
            Self::NotEqual(neq) => neq.to_tokens(tokens),
            Self::GreaterThanEqual(geq) => geq.to_tokens(tokens),
            Self::LessThanEqual(leq) => leq.to_tokens(tokens),
        }
        todo!()
    }
}

struct ComparisonMapperArm {
    comparison: ComparisonOperator,
    arrow: Token![->],
    body: syn::Expr,
}

impl Parse for ComparisonMapperArm {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let operator: ComparisonOperator = input.parse()?;
        let arrow: Token![->] = input.parse()?;
        let body: syn::Expr = input.parse()?;
        Ok(Self {
            comparison: operator,
            arrow,
            body,
        })
    }
}

impl ToTokens for ComparisonMapperArm {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.comparison.to_tokens(tokens);
        self.arrow.to_tokens(tokens);
        self.body.to_tokens(tokens);
    }
}

struct ComparisonMapper {
    map: kw::map,
    comparison: kw::comparison,
    brace: token::Brace,
    arms: Punctuated<ComparisonMapperArm, Token![,]>,
}

impl Parse for ComparisonMapper {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let map: kw::map = input.parse()?;
        let comparison: kw::comparison = input.parse()?;
        let content;
        let brace = syn::braced!(content in input);
        let arms = Punctuated::<ComparisonMapperArm, Token![,]>::parse_terminated(&content)?;
        Ok(Self {
            map,
            comparison,
            brace,
            arms,
        })
    }
}

impl ToTokens for ComparisonMapper {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.map.to_tokens(tokens);
        self.comparison.to_tokens(tokens);
        self.brace.surround(tokens, |tokens| {
            self.arms.to_tokens(tokens);
        });
    }
}

struct NumberMapper {
    map: kw::map,
    number: kw::number,
    paren: token::Paren,
    comparison_arg: syn::Ident,
    arg_comma: Token![,],
    number_arg: syn::Ident,
    equal: Token![=],
    brace: token::Brace,
    body: syn::Expr,
}

impl Parse for NumberMapper {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let map: kw::map = input.parse()?;
        let number: kw::number = input.parse()?;
        let content;
        let paren = syn::parenthesized!(content in input);
        let comparison_arg: syn::Ident = content.parse()?;
        let arg_comma: Token![,] = content.parse()?;
        let number_arg: syn::Ident = content.parse()?;
        let equal: Token![=] = input.parse()?;
        let body_content;
        let brace = syn::braced!(body_content in input);
        let body: syn::Expr = body_content.parse()?;

        Ok(Self {
            map,
            number,
            paren,
            comparison_arg,
            arg_comma,
            number_arg,
            equal,
            brace,
            body,
        })
    }
}

impl ToTokens for NumberMapper {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.map.to_tokens(tokens);
        self.number.to_tokens(tokens);
        self.paren.surround(tokens, |tokens| {
            self.comparison_arg.to_tokens(tokens);
            self.arg_comma.to_tokens(tokens);
            self.number_arg.to_tokens(tokens);
        });
        self.equal.to_tokens(tokens);
        self.brace.surround(tokens, |tokens| {
            self.body.to_tokens(tokens);
        });
    }
}

enum Mapper {
    Comparison(ComparisonMapper),
    Number(NumberMapper),
}

impl Parse for Mapper {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let forked = input.fork();
        let _map: kw::map = forked.parse()?;
        let lookahead = forked.lookahead1();
        if lookahead.peek(kw::number) {
            input.parse().map(Self::Number)
        } else if lookahead.peek(kw::comparison) {
            input.parse().map(Self::Comparison)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for Mapper {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Comparison(comp) => comp.to_tokens(tokens),
            Self::Number(num) => num.to_tokens(tokens),
        }
    }
}

struct DomainDef {
    domain: kw::domain,
    domain_var: syn::Ident,
    with: kw::with,
    identifier: kw::identifier,
    identifier_value: syn::LitStr,
    and: kw::and,
    description: kw::description,
    description_value: syn::LitStr,
    semi: Token![;],
}

impl Parse for DomainDef {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let domain: kw::domain = input.parse()?;
        let domain_var: syn::Ident = input.parse()?;
        let with: kw::with = input.parse()?;
        let identifier: kw::identifier = input.parse()?;
        let identifier_value: syn::LitStr = input.parse()?;
        let and: kw::and = input.parse()?;
        let description: kw::description = input.parse()?;
        let description_value: syn::LitStr = input.parse()?;
        let semi: Token![;] = input.parse()?;

        Ok(Self {
            domain,
            domain_var,
            with,
            identifier,
            identifier_value,
            and,
            description,
            description_value,
            semi,
        })
    }
}

impl ToTokens for DomainDef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.domain.to_tokens(tokens);
        self.domain_var.to_tokens(tokens);
        self.with.to_tokens(tokens);
        self.identifier.to_tokens(tokens);
        self.identifier_value.to_tokens(tokens);
        self.and.to_tokens(tokens);
        self.description.to_tokens(tokens);
        self.description_value.to_tokens(tokens);
        self.semi.to_tokens(tokens);
    }
}

enum ValueKind {
    Enum(syn::Ident),
    Number(syn::LitInt),
}

impl Parse for ValueKind {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Ident) {
            input.parse().map(Self::Enum)
        } else if lookahead.peek(syn::LitInt) {
            input.parse().map(Self::Number)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for ValueKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Enum(en) => en.to_tokens(tokens),
            Self::Number(num) => num.to_tokens(tokens),
        }
    }
}

struct Comparison {
    key: syn::Ident,
    op: ComparisonOperator,
    value: ValueKind,
}

impl Parse for Comparison {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        let op: ComparisonOperator = input.parse()?;
        let value: ValueKind = input.parse()?;

        Ok(Self { key, op, value })
    }
}

impl ToTokens for Comparison {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.key.to_tokens(tokens);
        self.op.to_tokens(tokens);
        self.value.to_tokens(tokens);
    }
}

struct AnyClause {
    any: kw::any,
    key: syn::Ident,
}

impl Parse for AnyClause {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let any: kw::any = input.parse()?;
        let key: syn::Ident = input.parse()?;
        Ok(Self { any, key })
    }
}

impl ToTokens for AnyClause {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.any.to_tokens(tokens);
        self.key.to_tokens(tokens);
    }
}

struct NestedExpr {
    paren: token::Paren,
    expr: Box<AndChain>,
}

impl Parse for NestedExpr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        let paren = syn::parenthesized!(content in input);
        let expr: AndChain = content.parse()?;

        if !content.is_empty() {
            Err(syn::Error::new(content.span(), "Unexpected token"))
        } else {
            Ok(Self {
                paren,
                expr: Box::new(expr),
            })
        }
    }
}

impl ToTokens for NestedExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.paren.surround(tokens, |tokens| {
            self.expr.to_tokens(tokens);
        });
    }
}

enum Atom {
    Comparison(Comparison),
    AnyClause(AnyClause),
    Nested(NestedExpr),
}

impl Parse for Atom {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::any) {
            input.parse().map(Self::AnyClause)
        } else if lookahead.peek(syn::Ident) {
            input.parse().map(Self::Comparison)
        } else if lookahead.peek(token::Paren) {
            input.parse().map(Self::Nested)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for Atom {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Comparison(comp) => comp.to_tokens(tokens),
            Self::AnyClause(any) => any.to_tokens(tokens),
            Self::Nested(nested) => nested.to_tokens(tokens),
        }
    }
}

struct OrChain {
    members: Punctuated<Atom, Token![|]>,
}

impl Parse for OrChain {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            members: input.call(Punctuated::parse_separated_nonempty)?,
        })
    }
}

impl ToTokens for OrChain {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.members.to_tokens(tokens);
    }
}

struct AndChain {
    members: Punctuated<OrChain, Token![&]>,
}

impl Parse for AndChain {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            members: input.call(Punctuated::parse_separated_nonempty)?,
        })
    }
}

impl ToTokens for AndChain {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.members.to_tokens(tokens);
    }
}

enum Conclusion {
    Comparison(Comparison),
    AnyClause(AnyClause),
}

impl Parse for Conclusion {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::any) {
            input.parse().map(Self::AnyClause)
        } else if lookahead.peek(syn::Ident) {
            input.parse().map(Self::Comparison)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for Conclusion {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Comparison(comp) => comp.to_tokens(tokens),
            Self::AnyClause(any) => any.to_tokens(tokens),
        }
    }
}

struct Rule {
    rule: kw::rule,
    under: Option<(kw::under, syn::Ident)>,
    colon: Token![:],
    judgement: AndChain,
    arrow: Token![->],
    conclusion: Conclusion,
    semi: Token![;],
}

impl Parse for Rule {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let rule: kw::rule = input.parse()?;
        let under = if input.peek(kw::under) {
            let under: kw::under = input.parse()?;
            let domain_name: syn::Ident = input.parse()?;
            Some((under, domain_name))
        } else {
            None
        };
        let colon: Token![:] = input.parse()?;
        let judgement: AndChain = input.parse()?;
        let arrow: Token![->] = input.parse()?;
        let conclusion: Conclusion = input.parse()?;
        let semi: Token![;] = input.parse()?;
        Ok(Self {
            rule,
            under,
            colon,
            judgement,
            arrow,
            conclusion,
            semi,
        })
    }
}

impl ToTokens for Rule {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.rule.to_tokens(tokens);
        if let Some((under, domain_name)) = self.under.as_ref() {
            under.to_tokens(tokens);
            domain_name.to_tokens(tokens);
        }
        self.colon.to_tokens(tokens);
        self.judgement.to_tokens(tokens);
        self.arrow.to_tokens(tokens);
        self.conclusion.to_tokens(tokens);
        self.semi.to_tokens(tokens);
    }
}

enum TopLevelItem {
    KeyType(KeyType),
    ValueType(ValueType),
    Imports(Imports),
    Mapper(Mapper),
    DomainDef(DomainDef),
    Rule(Rule),
}

impl Parse for TopLevelItem {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::imports) {
            input.parse().map(Self::Imports)
        } else if lookahead.peek(kw::key) {
            input.parse().map(Self::KeyType)
        } else if lookahead.peek(kw::value) {
            input.parse().map(Self::ValueType)
        } else if lookahead.peek(kw::map) {
            input.parse().map(Self::Mapper)
        } else if lookahead.peek(kw::domain) {
            input.parse().map(Self::DomainDef)
        } else if lookahead.peek(kw::rule) {
            input.parse().map(Self::Rule)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for TopLevelItem {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::KeyType(key_ty) => key_ty.to_tokens(tokens),
            Self::ValueType(value_ty) => value_ty.to_tokens(tokens),
            Self::Imports(imports) => imports.to_tokens(tokens),
            Self::Mapper(mapper) => mapper.to_tokens(tokens),
            Self::DomainDef(domain) => domain.to_tokens(tokens),
            Self::Rule(rule) => rule.to_tokens(tokens),
        }
    }
}

struct GraphSpec {
    items: Vec<TopLevelItem>,
}

impl Parse for GraphSpec {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse()?);
        }
        Ok(Self { items })
    }
}

impl ToTokens for GraphSpec {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for item in &self.items {
            item.to_tokens(tokens);
        }
    }
}

struct GraphSpecCtx {
    key_type: KeyType,
    value_type: ValueType,
    imports: Vec<Imports>,
    mappers: Vec<Mapper>,
    domain_defs: Vec<DomainDef>,
    rules: Vec<Rule>,
}

impl TryFrom<GraphSpec> for GraphSpecCtx {
    type Error = syn::Error;

    fn try_from(value: GraphSpec) -> Result<Self, Self::Error> {
        let mut key_type: Option<KeyType> = None;
        let mut value_type: Option<ValueType> = None;
        let mut imports = Vec::new();
        let mut mappers = Vec::new();
        let mut domain_defs = Vec::new();
        let mut rules = Vec::new();

        for item in value.items {
            match item {
                TopLevelItem::KeyType(key_ty) => {
                    if let Some(ref original_kt) = key_type {
                        let mut err =
                            syn::Error::new_spanned(key_ty, "duplicate key type specified");
                        err.combine(syn::Error::new_spanned(
                            original_kt,
                            "original key type specified here",
                        ));
                        return Err(err);
                    } else {
                        key_type = Some(key_ty);
                    }
                }
                TopLevelItem::ValueType(value_ty) => {
                    if let Some(ref original_vt) = value_type {
                        let mut err =
                            syn::Error::new_spanned(value_ty, "duplicate value type specified");
                        err.combine(syn::Error::new_spanned(
                            original_vt,
                            "original value type specified here",
                        ));
                        return Err(err);
                    } else {
                        value_type = Some(value_ty);
                    }
                }
                TopLevelItem::Imports(imp) => imports.push(imp),
                TopLevelItem::Mapper(mapper) => mappers.push(mapper),
                TopLevelItem::DomainDef(domain_def) => domain_defs.push(domain_def),
                TopLevelItem::Rule(rule) => rules.push(rule),
            }
        }

        let final_key_type =
            key_type.ok_or_else(|| syn::Error::new(Span::call_site(), "no key type specified"))?;
        let final_value_type = value_type
            .ok_or_else(|| syn::Error::new(Span::call_site(), "no value type specified"))?;

        Ok(Self {
            key_type: final_key_type,
            value_type: final_value_type,
            imports,
            mappers,
            domain_defs,
            rules,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum EdgeRelation {
    Positive,
    Negative,
}

impl From<EdgeRelation> for TokenStream {
    fn from(value: EdgeRelation) -> Self {
        match value {
            EdgeRelation::Positive => quote! { cgraph::Relation::Positive },
            EdgeRelation::Negative => quote! { cgraph::Relation::Negative },
        }
    }
}

impl ToTokens for EdgeRelation {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        TokenStream::from(*self).to_tokens(tokens);
    }
}

type CompileResult<T> = Result<T, syn::Error>;

struct CompileCtx {
    graph_ident: syn::Ident,
    key_ty_ident: syn::Ident,
    val_ty_ident: syn::Ident,
    ident_idx: usize,
    domain_name2ident: HashMap<String, syn::Ident>,
    domain_identifiers: HashSet<String>,
}

impl CompileCtx {
    fn new(key_ty_ident: syn::Ident, val_ty_ident: syn::Ident) -> Self {
        Self {
            graph_ident: quote::format_ident!("_graph"),
            key_ty_ident,
            val_ty_ident,
            ident_idx: 0,
            domain_name2ident: HashMap::new(),
            domain_identifiers: HashSet::new(),
        }
    }

    fn next_ident(&mut self) -> syn::Ident {
        let idx = self.ident_idx;
        self.ident_idx += 1;
        quote::format_ident!("_{idx}")
    }

    fn compile_comparison(
        &mut self,
        comp: &Comparison,
    ) -> CompileResult<(syn::Ident, EdgeRelation, TokenStream)> {
        if !matches!(
            comp.op,
            ComparisonOperator::Equal(_) | ComparisonOperator::NotEqual(_)
        ) {
            return Err(syn::Error::new_spanned(&comp.op, "operator not supported"));
        }

        match &comp.value {
            ValueKind::Number(_) => Err(syn::Error::new_spanned(
                &comp.value,
                "numbers are not supported yet",
            )),
            ValueKind::Enum(enum_val) => {
                let node_ident = self.next_ident();
                let graph = &self.graph_ident;
                let val_ty = &self.val_ty_ident;
                let key = &comp.key;
                let code = quote! {
                    let #node_ident = #graph.make_value_node(
                        cgraph::NodeValue::Value(#val_ty::#key(#key::#enum_val)),
                        None,
                        None::<()>,
                    );
                };

                let relation = if matches!(comp.op, ComparisonOperator::NotEqual(_)) {
                    EdgeRelation::Negative
                } else {
                    EdgeRelation::Positive
                };
                Ok((node_ident, relation, code))
            }
        }
    }

    fn compile_any_clause(
        &mut self,
        any: &AnyClause,
    ) -> CompileResult<(syn::Ident, EdgeRelation, TokenStream)> {
        let node_ident = self.next_ident();
        let graph = &self.graph_ident;
        let key_ty = &self.key_ty_ident;
        let key = &any.key;
        let code = quote! {
            let #node_ident = #graph.make_value_node(
                cgraph::NodeValue::Key(#key_ty::#key),
                None,
                None::<()>,
            );
        };

        Ok((node_ident, EdgeRelation::Positive, code))
    }

    fn compile_atom(
        &mut self,
        atom: &Atom,
        domain: Option<&syn::Ident>,
    ) -> CompileResult<(syn::Ident, EdgeRelation, TokenStream)> {
        match atom {
            Atom::Comparison(comp) => self.compile_comparison(comp),
            Atom::AnyClause(any) => self.compile_any_clause(any),
            Atom::Nested(nested) => self
                .compile_and_chain(&nested.expr, domain)
                .map(|(id, ts)| (id, EdgeRelation::Positive, ts)),
        }
    }

    fn compile_or_chain(
        &mut self,
        chain: &OrChain,
        domain: Option<&syn::Ident>,
    ) -> CompileResult<(syn::Ident, TokenStream)> {
        let mut token_streams = Vec::with_capacity(chain.members.len());
        let mut args = Vec::with_capacity(chain.members.len());
        for member in chain.members.iter() {
            let (ident, relation, ts) = self.compile_atom(member, domain)?;
            token_streams.push(ts);
            args.push(quote! { (#ident, #relation, cgraph::Strength::Strong) });
        }

        let node_ident = self.next_ident();
        let domain_tokens = domain.map_or_else(|| quote! { None }, |id| quote! { Some(#id) });
        let graph = &self.graph_ident;
        let code = quote! {
            #(#token_streams)*
            let #node_ident = #graph.make_any_aggregator(
                &[#(#args),*],
                None,
                None::<()>,
                #domain_tokens,
            ).expect("any aggregator construction");
        };

        Ok((node_ident, code))
    }

    fn compile_and_chain(
        &mut self,
        chain: &AndChain,
        domain: Option<&syn::Ident>,
    ) -> CompileResult<(syn::Ident, TokenStream)> {
        let mut token_streams = Vec::with_capacity(chain.members.len());
        let mut args = Vec::with_capacity(chain.members.len());
        for member in chain.members.iter() {
            let (ident, ts) = self.compile_or_chain(member, domain)?;
            token_streams.push(ts);
            args.push(quote! { (#ident, cgraph::Relation::Positive, cgraph::Strength::Strong) });
        }

        let node_ident = self.next_ident();
        let domain_tokens = domain.map_or_else(|| quote! { None }, |id| quote! { Some(#id) });
        let graph = &self.graph_ident;
        let code = quote! {
            #(#token_streams)*
            let #node_ident = #graph.make_all_aggregator(
                &[#(#args),*],
                None,
                None::<()>,
                #domain_tokens,
            ).expect("all aggregator construction");
        };

        Ok((node_ident, code))
    }

    fn compile_conclusion(
        &mut self,
        conclusion: &Conclusion,
    ) -> CompileResult<(syn::Ident, TokenStream)> {
        let (ident, relation, ts) = match conclusion {
            Conclusion::AnyClause(any) => self.compile_any_clause(any)?,
            Conclusion::Comparison(comp) => self.compile_comparison(comp)?,
        };

        if matches!(relation, EdgeRelation::Negative) {
            Err(syn::Error::new_spanned(
                conclusion,
                "cannot use a negative condition in the conclusion",
            ))
        } else {
            Ok((ident, ts))
        }
    }

    fn compile_rule(&mut self, rule: &Rule) -> CompileResult<TokenStream> {
        let domain = rule
            .under
            .as_ref()
            .map(|(_, name_ident)| {
                self.domain_name2ident
                    .get(&name_ident.to_string())
                    .cloned()
                    .ok_or_else(|| syn::Error::new_spanned(name_ident, "unknown domain"))
            })
            .transpose()?;

        let (judgement_ident, judgement_ts) =
            self.compile_and_chain(&rule.judgement, domain.as_ref())?;

        let (conclusion_ident, conclusion_ts) = self.compile_conclusion(&rule.conclusion)?;

        let domain_tokens = domain.map_or_else(|| quote! { None }, |id| quote! { Some(#id) });
        let graph = &self.graph_ident;
        let code = quote! {
            #judgement_ts
            #conclusion_ts
            #graph.make_edge(
                #judgement_ident,
                #conclusion_ident,
                cgraph::Strength::Normal,
                cgraph::Relation::Positive,
                #domain_tokens,
            ).expect("judgement to conclusion edge creation");
        };

        Ok(code)
    }

    fn compile_domain_def(&mut self, domain_def: &DomainDef) -> CompileResult<TokenStream> {
        if self
            .domain_name2ident
            .contains_key(&domain_def.domain_var.to_string())
        {
            return Err(syn::Error::new_spanned(
                domain_def,
                format!(
                    "domain with the name '{}' already exists",
                    domain_def.domain_var.to_string()
                ),
            ));
        }

        if self
            .domain_identifiers
            .contains(&domain_def.identifier_value.value())
        {
            return Err(syn::Error::new_spanned(
                domain_def,
                "domain with the given identifier already exists",
            ));
        }

        let domain_ident = self.next_ident();
        let graph = &self.graph_ident;
        let domain_identifier = &domain_def.identifier_value;
        let domain_description = &domain_def.description_value;
        let code = quote! {
            let #domain_ident = #graph.make_domain(#domain_identifier, #domain_description)
                .expect("domain");
        };

        self.domain_name2ident
            .insert(domain_def.domain_var.to_string(), domain_ident);
        self.domain_identifiers
            .insert(domain_def.identifier_value.value());

        Ok(code)
    }

    fn compile(mut self, spec: GraphSpecCtx) -> CompileResult<TokenStream> {
        let mut uses = Vec::new();
        for import in spec.imports {
            uses.extend(import.uses);
        }

        let mut domain_defs = Vec::with_capacity(spec.domain_defs.len());
        for domain_def in spec.domain_defs {
            domain_defs.push(self.compile_domain_def(&domain_def)?);
        }

        let mut rule_defs = Vec::with_capacity(spec.rules.len());
        for rule in spec.rules {
            rule_defs.push(self.compile_rule(&rule)?);
        }

        let graph = self.graph_ident;
        Ok(quote! {{
            #(#uses)*
            let mut #graph = cgraph::ConstraintGraphBuilder::new();
            #(#domain_defs)*
            #(#rule_defs)*
            #graph.build()
        }})
    }
}

pub(crate) fn constraint_graph_inner(
    ts: proc_macro::TokenStream,
) -> Result<proc_macro::TokenStream, syn::Error> {
    let graph_spec: GraphSpec = syn::parse(ts)?;
    let graph_spec_ctx: GraphSpecCtx = graph_spec.try_into()?;
    let compile_ctx = CompileCtx::new(
        graph_spec_ctx.key_type.ty.clone(),
        graph_spec_ctx.value_type.ty.clone(),
    );
    let compiled = compile_ctx.compile(graph_spec_ctx)?;
    Ok(compiled.into())
}
