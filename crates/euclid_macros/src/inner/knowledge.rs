use std::{hash::Hash, rc::Rc};

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use rustc_hash::{FxHashMap, FxHashSet};
use syn::{parse::Parse, Token};

mod strength {
    syn::custom_punctuation!(Normal, ->);
    syn::custom_punctuation!(Strong, ->>);
}

mod kw {
    syn::custom_keyword!(any);
    syn::custom_keyword!(not);
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum Comparison {
    LessThan,
    Equal,
    GreaterThan,
    GreaterThanEqual,
    LessThanEqual,
}

impl ToString for Comparison {
    fn to_string(&self) -> String {
        match self {
            Self::LessThan => "< ".to_string(),
            Self::Equal => String::new(),
            Self::GreaterThanEqual => ">= ".to_string(),
            Self::LessThanEqual => "<= ".to_string(),
            Self::GreaterThan => "> ".to_string(),
        }
    }
}

impl Parse for Comparison {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(Token![>]) {
            input.parse::<Token![>]>()?;
            Ok(Self::GreaterThan)
        } else if input.peek(Token![<]) {
            input.parse::<Token![<]>()?;
            Ok(Self::LessThan)
        } else if input.peek(Token!(<=)) {
            input.parse::<Token![<=]>()?;
            Ok(Self::LessThanEqual)
        } else if input.peek(Token!(>=)) {
            input.parse::<Token![>=]>()?;
            Ok(Self::GreaterThanEqual)
        } else {
            Ok(Self::Equal)
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum ValueType {
    Any,
    EnumVariant(String),
    Number { number: i64, comparison: Comparison },
}

impl ValueType {
    fn to_string(&self, key: &str) -> String {
        match self {
            Self::Any => format!("{key}(any)"),
            Self::EnumVariant(s) => format!("{key}({s})"),
            Self::Number { number, comparison } => {
                format!("{}({}{})", key, comparison.to_string(), number)
            }
        }
    }
}

impl Parse for ValueType {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Ident) {
            let ident: syn::Ident = input.parse()?;
            Ok(Self::EnumVariant(ident.to_string()))
        } else if lookahead.peek(Token![>])
            || lookahead.peek(Token![<])
            || lookahead.peek(syn::LitInt)
        {
            let comparison: Comparison = input.parse()?;
            let number: syn::LitInt = input.parse()?;
            let num_val = number.base10_parse::<i64>()?;
            Ok(Self::Number {
                number: num_val,
                comparison,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Atom {
    key: String,
    value: ValueType,
}

impl ToString for Atom {
    fn to_string(&self) -> String {
        self.value.to_string(&self.key)
    }
}

impl Parse for Atom {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let maybe_any: syn::Ident = input.parse()?;
        if maybe_any == "any" {
            let actual_key: syn::Ident = input.parse()?;
            Ok(Self {
                key: actual_key.to_string(),
                value: ValueType::Any,
            })
        } else {
            let content;
            syn::parenthesized!(content in input);
            let value: ValueType = content.parse()?;
            Ok(Self {
                key: maybe_any.to_string(),
                value,
            })
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, strum::Display)]
enum Strength {
    Normal,
    Strong,
}

impl Parse for Strength {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(strength::Strong) {
            input.parse::<strength::Strong>()?;
            Ok(Self::Strong)
        } else if lookahead.peek(strength::Normal) {
            input.parse::<strength::Normal>()?;
            Ok(Self::Normal)
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, strum::Display)]
enum Relation {
    Positive,
    Negative,
}

enum AtomType {
    Value {
        relation: Relation,
        atom: Rc<Atom>,
    },

    InAggregator {
        key: String,
        values: Vec<String>,
        relation: Relation,
    },
}

fn parse_atom_type_inner(
    input: syn::parse::ParseStream<'_>,
    key: syn::Ident,
    relation: Relation,
) -> syn::Result<AtomType> {
    let result = if input.peek(Token![in]) {
        input.parse::<Token![in]>()?;

        let bracketed;
        syn::bracketed!(bracketed in input);

        let mut values = Vec::<String>::new();
        let first: syn::Ident = bracketed.parse()?;
        values.push(first.to_string());
        while !bracketed.is_empty() {
            bracketed.parse::<Token![,]>()?;
            let next: syn::Ident = bracketed.parse()?;
            values.push(next.to_string());
        }

        AtomType::InAggregator {
            key: key.to_string(),
            values,
            relation,
        }
    } else if input.peek(kw::any) {
        input.parse::<kw::any>()?;
        AtomType::Value {
            relation,
            atom: Rc::new(Atom {
                key: key.to_string(),
                value: ValueType::Any,
            }),
        }
    } else {
        let value: ValueType = input.parse()?;
        AtomType::Value {
            relation,
            atom: Rc::new(Atom {
                key: key.to_string(),
                value,
            }),
        }
    };

    Ok(result)
}

impl Parse for AtomType {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        let content;
        syn::parenthesized!(content in input);

        let relation = if content.peek(kw::not) {
            content.parse::<kw::not>()?;
            Relation::Negative
        } else {
            Relation::Positive
        };

        let result = parse_atom_type_inner(&content, key, relation)?;

        if !content.is_empty() {
            Err(content.error("Unexpected input received after atom value"))
        } else {
            Ok(result)
        }
    }
}

fn parse_rhs_atom(input: syn::parse::ParseStream<'_>) -> syn::Result<Atom> {
    let key: syn::Ident = input.parse()?;
    let content;
    syn::parenthesized!(content in input);

    let lookahead = content.lookahead1();

    let value_type = if lookahead.peek(kw::any) {
        content.parse::<kw::any>()?;
        ValueType::Any
    } else if lookahead.peek(syn::Ident) {
        let variant = content.parse::<syn::Ident>()?;
        ValueType::EnumVariant(variant.to_string())
    } else {
        return Err(lookahead.error());
    };

    if !content.is_empty() {
        Err(content.error("Unexpected input received after atom value"))
    } else {
        Ok(Atom {
            key: key.to_string(),
            value: value_type,
        })
    }
}

struct Rule {
    lhs: Vec<AtomType>,
    strength: Strength,
    rhs: Rc<Atom>,
}

impl Parse for Rule {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let first_atom: AtomType = input.parse()?;
        let mut lhs: Vec<AtomType> = vec![first_atom];

        while input.peek(Token![&]) {
            input.parse::<Token![&]>()?;
            let and_atom: AtomType = input.parse()?;
            lhs.push(and_atom);
        }

        let strength: Strength = input.parse()?;

        let rhs: Rc<Atom> = Rc::new(parse_rhs_atom(input)?);

        input.parse::<Token![;]>()?;

        Ok(Self { lhs, strength, rhs })
    }
}

#[derive(Clone)]
enum Scope {
    Crate,
    Extern,
}

impl Parse for Scope {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![crate]) {
            input.parse::<Token![crate]>()?;
            Ok(Self::Crate)
        } else if lookahead.peek(Token![extern]) {
            input.parse::<Token![extern]>()?;
            Ok(Self::Extern)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToString for Scope {
    fn to_string(&self) -> String {
        match self {
            Self::Crate => "crate".to_string(),
            Self::Extern => "euclid".to_string(),
        }
    }
}

#[derive(Clone)]
struct Program {
    rules: Vec<Rc<Rule>>,
    scope: Scope,
}

impl Parse for Program {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let scope: Scope = input.parse()?;
        let mut rules: Vec<Rc<Rule>> = Vec::new();

        while !input.is_empty() {
            rules.push(Rc::new(input.parse::<Rule>()?));
        }

        Ok(Self { rules, scope })
    }
}

struct GenContext {
    next_idx: usize,
    next_node_idx: usize,
    idx2atom: FxHashMap<usize, Rc<Atom>>,
    atom2idx: FxHashMap<Rc<Atom>, usize>,
    edges: FxHashMap<usize, FxHashSet<usize>>,
    compiled_atoms: FxHashMap<Rc<Atom>, proc_macro2::Ident>,
}

impl GenContext {
    fn new() -> Self {
        Self {
            next_idx: 1,
            next_node_idx: 1,
            idx2atom: FxHashMap::default(),
            atom2idx: FxHashMap::default(),
            edges: FxHashMap::default(),
            compiled_atoms: FxHashMap::default(),
        }
    }

    fn register_node(&mut self, atom: Rc<Atom>) -> usize {
        if let Some(idx) = self.atom2idx.get(&atom) {
            *idx
        } else {
            let this_idx = self.next_idx;
            self.next_idx += 1;

            self.idx2atom.insert(this_idx, Rc::clone(&atom));
            self.atom2idx.insert(atom, this_idx);

            this_idx
        }
    }

    fn register_edge(&mut self, from: usize, to: usize) -> Result<(), String> {
        let node_children = self.edges.entry(from).or_default();
        if node_children.contains(&to) {
            Err("Duplicate edge detected".to_string())
        } else {
            node_children.insert(to);
            self.edges.entry(to).or_default();
            Ok(())
        }
    }

    fn register_rule(&mut self, rule: &Rule) -> Result<(), String> {
        let to_idx = self.register_node(Rc::clone(&rule.rhs));

        for atom_type in &rule.lhs {
            if let AtomType::Value { atom, .. } = atom_type {
                let from_idx = self.register_node(Rc::clone(atom));
                self.register_edge(from_idx, to_idx)?;
            }
        }

        Ok(())
    }

    fn cycle_dfs(
        &self,
        node_id: usize,
        explored: &mut FxHashSet<usize>,
        visited: &mut FxHashSet<usize>,
        order: &mut Vec<usize>,
    ) -> Result<Option<Vec<usize>>, String> {
        if explored.contains(&node_id) {
            let position = order
                .iter()
                .position(|v| *v == node_id)
                .ok_or_else(|| "Error deciding cycle order".to_string())?;

            let cycle_order = order[position..].to_vec();
            Ok(Some(cycle_order))
        } else if visited.contains(&node_id) {
            Ok(None)
        } else {
            visited.insert(node_id);
            explored.insert(node_id);
            order.push(node_id);
            let dests = self
                .edges
                .get(&node_id)
                .ok_or_else(|| "Error getting edges of node".to_string())?;

            for dest in dests.iter().copied() {
                if let Some(cycle) = self.cycle_dfs(dest, explored, visited, order)? {
                    return Ok(Some(cycle));
                }
            }

            order.pop();

            Ok(None)
        }
    }

    fn detect_graph_cycles(&self) -> Result<(), String> {
        let start_nodes = self.edges.keys().copied().collect::<Vec<usize>>();

        let mut total_visited = FxHashSet::<usize>::default();

        for node_id in start_nodes.iter().copied() {
            let mut explored = FxHashSet::<usize>::default();
            let mut order = Vec::<usize>::new();

            match self.cycle_dfs(node_id, &mut explored, &mut total_visited, &mut order)? {
                None => {}
                Some(order) => {
                    let mut display_strings = Vec::<String>::with_capacity(order.len() + 1);

                    for cycle_node_id in order {
                        let node = self.idx2atom.get(&cycle_node_id).ok_or_else(|| {
                            "Failed to find node during cycle display creation".to_string()
                        })?;

                        display_strings.push(node.to_string());
                    }

                    let first = display_strings
                        .first()
                        .cloned()
                        .ok_or("Unable to fill cycle display array")?;

                    display_strings.push(first);

                    return Err(format!("Found cycle: {}", display_strings.join(" -> ")));
                }
            }
        }

        Ok(())
    }

    fn next_node_ident(&mut self) -> (proc_macro2::Ident, usize) {
        let this_idx = self.next_node_idx;
        self.next_node_idx += 1;
        (format_ident!("_node_{this_idx}"), this_idx)
    }

    fn compile_atom(
        &mut self,
        atom: &Rc<Atom>,
        tokens: &mut TokenStream,
    ) -> Result<proc_macro2::Ident, String> {
        let maybe_ident = self.compiled_atoms.get(atom);

        if let Some(ident) = maybe_ident {
            Ok(ident.clone())
        } else {
            let (identifier, _) = self.next_node_ident();
            let key = format_ident!("{}", &atom.key);
            let the_value = match &atom.value {
                ValueType::Any => quote! {
                    NodeValue::Key(DirKey::new(DirKeyKind::#key,None))
                },
                ValueType::EnumVariant(variant) => {
                    let variant = format_ident!("{}", variant);
                    quote! {
                        NodeValue::Value(DirValue::#key(#key::#variant))
                    }
                }
                ValueType::Number { number, comparison } => {
                    let comp_type = match comparison {
                        Comparison::Equal => quote! {
                            None
                        },
                        Comparison::LessThan => quote! {
                            Some(NumValueRefinement::LessThan)
                        },
                        Comparison::GreaterThan => quote! {
                            Some(NumValueRefinement::GreaterThan)
                        },
                        Comparison::GreaterThanEqual => quote! {
                            Some(NumValueRefinement::GreaterThanEqual)
                        },
                        Comparison::LessThanEqual => quote! {
                            Some(NumValueRefinement::LessThanEqual)
                        },
                    };

                    quote! {
                        NodeValue::Value(DirValue::#key(NumValue {
                            number: #number,
                            refinement: #comp_type,
                        }))
                    }
                }
            };

            let compiled = quote! {
                let #identifier = graph.make_value_node(#the_value, None, Vec::new(), None::<()>).expect("NodeId derivation failed");
            };

            tokens.extend(compiled);
            self.compiled_atoms
                .insert(Rc::clone(atom), identifier.clone());

            Ok(identifier)
        }
    }

    fn compile_atom_type(
        &mut self,
        atom_type: &AtomType,
        tokens: &mut TokenStream,
    ) -> Result<(proc_macro2::Ident, Relation), String> {
        match atom_type {
            AtomType::Value { relation, atom } => {
                let node_ident = self.compile_atom(atom, tokens)?;

                Ok((node_ident, relation.clone()))
            }

            AtomType::InAggregator {
                key,
                values,
                relation,
            } => {
                let key_ident = format_ident!("{key}");
                let mut values_tokens: Vec<TokenStream> = Vec::new();

                for value in values {
                    let value_ident = format_ident!("{value}");
                    values_tokens.push(quote! { DirValue::#key_ident(#key_ident::#value_ident) });
                }

                let (node_ident, _) = self.next_node_ident();
                let node_code = quote! {
                    let #node_ident = graph.make_in_aggregator(
                        Vec::from_iter([#(#values_tokens),*]),
                        None,
                        None::<()>,
                        Vec::new(),
                    ).expect("Failed to make In aggregator");
                };

                tokens.extend(node_code);

                Ok((node_ident, relation.clone()))
            }
        }
    }

    fn compile_rule(&mut self, rule: &Rule, tokens: &mut TokenStream) -> Result<(), String> {
        let rhs_ident = self.compile_atom(&rule.rhs, tokens)?;
        let mut node_details: Vec<(proc_macro2::Ident, Relation)> =
            Vec::with_capacity(rule.lhs.len());
        for lhs_atom_type in &rule.lhs {
            let details = self.compile_atom_type(lhs_atom_type, tokens)?;
            node_details.push(details);
        }

        if node_details.len() <= 1 {
            let strength = format_ident!("{}", rule.strength.to_string());
            for (from_node, relation) in &node_details {
                let relation = format_ident!("{}", relation.to_string());
                tokens.extend(quote! {
                    graph.make_edge(#from_node, #rhs_ident, Strength::#strength, Relation::#relation)
                        .expect("Failed to make edge");
                });
            }
        } else {
            let mut all_agg_nodes: Vec<TokenStream> = Vec::with_capacity(node_details.len());
            for (from_node, relation) in &node_details {
                let relation = format_ident!("{}", relation.to_string());
                all_agg_nodes.push(quote! { (#from_node, Relation::#relation, Strength::Strong) });
            }

            let strength = format_ident!("{}", rule.strength.to_string());
            let (agg_node_ident, _) = self.next_node_ident();
            tokens.extend(quote! {
                let #agg_node_ident = graph.make_all_aggregator(&[#(#all_agg_nodes),*], None, None::<()>, Vec::new())
                    .expect("Failed to make all aggregator node");

                graph.make_edge(#agg_node_ident, #rhs_ident, Strength::#strength, Relation::Positive)
                    .expect("Failed to create all aggregator edge");

            });
        }

        Ok(())
    }

    fn compile(&mut self, program: Program) -> Result<TokenStream, String> {
        let mut tokens = TokenStream::new();
        for rule in &program.rules {
            self.compile_rule(rule, &mut tokens)?;
        }

        let scope = match &program.scope {
            Scope::Crate => quote! { crate },
            Scope::Extern => quote! { euclid },
        };

        let compiled = quote! {{
            use #scope::{
                dssa::graph::*,
                types::*,
                frontend::dir::{*, enums::*},
            };

            use rustc_hash::{FxHashMap, FxHashSet};

            let mut graph = KnowledgeGraphBuilder::new();

            #tokens

            graph.build()
        }};

        Ok(compiled)
    }
}

pub(crate) fn knowledge_inner(ts: TokenStream) -> syn::Result<TokenStream> {
    let program = syn::parse::<Program>(ts.into())?;
    let mut gen_context = GenContext::new();

    for rule in &program.rules {
        gen_context
            .register_rule(rule)
            .map_err(|msg| syn::Error::new(Span::call_site(), msg))?;
    }

    gen_context
        .detect_graph_cycles()
        .map_err(|msg| syn::Error::new(Span::call_site(), msg))?;

    gen_context
        .compile(program)
        .map_err(|msg| syn::Error::new(Span::call_site(), msg))
}
