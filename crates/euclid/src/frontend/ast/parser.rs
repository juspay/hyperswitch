use nom::{
    branch, bytes::complete, character::complete as pchar, combinator, error, multi, sequence,
};

use crate::{frontend::ast, types::DummyOutput};
pub type ParseResult<T, U> = nom::IResult<T, U, nom::error::VerboseError<T>>;

pub enum EuclidError {
    InvalidPercentage(String),
    InvalidConnector(String),
    InvalidOperator(String),
    InvalidNumber(String),
}

pub trait EuclidParsable: Sized {
    fn parse_output(input: &str) -> ParseResult<&str, Self>;
}

impl EuclidParsable for DummyOutput {
    fn parse_output(input: &str) -> ParseResult<&str, Self> {
        let string_w = sequence::delimited(
            skip_ws(complete::tag("\"")),
            complete::take_while(|c| c != '"'),
            skip_ws(complete::tag("\"")),
        );
        let full_sequence = multi::many0(sequence::preceded(
            skip_ws(complete::tag(",")),
            sequence::delimited(
                skip_ws(complete::tag("\"")),
                complete::take_while(|c| c != '"'),
                skip_ws(complete::tag("\"")),
            ),
        ));
        let sequence = sequence::pair(string_w, full_sequence);
        error::context(
            "dummy_strings",
            combinator::map(
                sequence::delimited(
                    skip_ws(complete::tag("[")),
                    sequence,
                    skip_ws(complete::tag("]")),
                ),
                |out: (&str, Vec<&str>)| {
                    let mut first = out.1;
                    first.insert(0, out.0);
                    let v = first.iter().map(|s| s.to_string()).collect();
                    Self { outputs: v }
                },
            ),
        )(input)
    }
}
pub fn skip_ws<'a, F: 'a, O>(inner: F) -> impl FnMut(&'a str) -> ParseResult<&str, O>
where
    F: FnMut(&'a str) -> ParseResult<&str, O>,
{
    sequence::preceded(pchar::multispace0, inner)
}

pub fn num_i64(input: &str) -> ParseResult<&str, i64> {
    error::context(
        "num_i32",
        combinator::map_res(
            complete::take_while1(|c: char| c.is_ascii_digit()),
            |o: &str| {
                o.parse::<i64>()
                    .map_err(|_| EuclidError::InvalidNumber(o.to_string()))
            },
        ),
    )(input)
}

pub fn string_str(input: &str) -> ParseResult<&str, String> {
    error::context(
        "String",
        combinator::map(
            sequence::delimited(
                complete::tag("\""),
                complete::take_while1(|c: char| c != '"'),
                complete::tag("\""),
            ),
            |val: &str| val.to_string(),
        ),
    )(input)
}

pub fn identifier(input: &str) -> ParseResult<&str, String> {
    error::context(
        "identifier",
        combinator::map(
            sequence::pair(
                complete::take_while1(|c: char| c.is_ascii_alphabetic() || c == '_'),
                complete::take_while(|c: char| c.is_ascii_alphanumeric() || c == '_'),
            ),
            |out: (&str, &str)| out.0.to_string() + out.1,
        ),
    )(input)
}
pub fn percentage(input: &str) -> ParseResult<&str, u8> {
    error::context(
        "volume_split_percentage",
        combinator::map_res(
            sequence::terminated(
                complete::take_while_m_n(1, 2, |c: char| c.is_ascii_digit()),
                complete::tag("%"),
            ),
            |o: &str| {
                o.parse::<u8>()
                    .map_err(|_| EuclidError::InvalidPercentage(o.to_string()))
            },
        ),
    )(input)
}

pub fn number_value(input: &str) -> ParseResult<&str, ast::ValueType> {
    error::context(
        "number_value",
        combinator::map(num_i64, ast::ValueType::Number),
    )(input)
}

pub fn str_value(input: &str) -> ParseResult<&str, ast::ValueType> {
    error::context(
        "str_value",
        combinator::map(string_str, ast::ValueType::StrValue),
    )(input)
}
pub fn enum_value_string(input: &str) -> ParseResult<&str, String> {
    combinator::map(
        sequence::pair(
            complete::take_while1(|c: char| c.is_ascii_alphabetic() || c == '_'),
            complete::take_while(|c: char| c.is_ascii_alphanumeric() || c == '_'),
        ),
        |out: (&str, &str)| out.0.to_string() + out.1,
    )(input)
}

pub fn enum_variant_value(input: &str) -> ParseResult<&str, ast::ValueType> {
    error::context(
        "enum_variant_value",
        combinator::map(enum_value_string, ast::ValueType::EnumVariant),
    )(input)
}

pub fn number_array_value(input: &str) -> ParseResult<&str, ast::ValueType> {
    let many_with_comma = multi::many0(sequence::preceded(
        skip_ws(complete::tag(",")),
        skip_ws(num_i64),
    ));

    let full_sequence = sequence::pair(skip_ws(num_i64), many_with_comma);

    error::context(
        "number_array_value",
        combinator::map(
            sequence::delimited(
                skip_ws(complete::tag("(")),
                full_sequence,
                skip_ws(complete::tag(")")),
            ),
            |tup: (i64, Vec<i64>)| {
                let mut rest = tup.1;
                rest.insert(0, tup.0);
                ast::ValueType::NumberArray(rest)
            },
        ),
    )(input)
}

pub fn enum_variant_array_value(input: &str) -> ParseResult<&str, ast::ValueType> {
    let many_with_comma = multi::many0(sequence::preceded(
        skip_ws(complete::tag(",")),
        skip_ws(enum_value_string),
    ));

    let full_sequence = sequence::pair(skip_ws(enum_value_string), many_with_comma);

    error::context(
        "enum_variant_array_value",
        combinator::map(
            sequence::delimited(
                skip_ws(complete::tag("(")),
                full_sequence,
                skip_ws(complete::tag(")")),
            ),
            |tup: (String, Vec<String>)| {
                let mut rest = tup.1;
                rest.insert(0, tup.0);
                ast::ValueType::EnumVariantArray(rest)
            },
        ),
    )(input)
}

pub fn number_comparison(input: &str) -> ParseResult<&str, ast::NumberComparison> {
    let operator = combinator::map_res(
        branch::alt((
            complete::tag(">="),
            complete::tag("<="),
            complete::tag(">"),
            complete::tag("<"),
        )),
        |s: &str| match s {
            ">=" => Ok(ast::ComparisonType::GreaterThanEqual),
            "<=" => Ok(ast::ComparisonType::LessThanEqual),
            ">" => Ok(ast::ComparisonType::GreaterThan),
            "<" => Ok(ast::ComparisonType::LessThan),
            _ => Err(EuclidError::InvalidOperator(s.to_string())),
        },
    );

    error::context(
        "number_comparison",
        combinator::map(
            sequence::pair(operator, num_i64),
            |tup: (ast::ComparisonType, i64)| ast::NumberComparison {
                comparison_type: tup.0,
                number: tup.1,
            },
        ),
    )(input)
}

pub fn number_comparison_array_value(input: &str) -> ParseResult<&str, ast::ValueType> {
    let many_with_comma = multi::many0(sequence::preceded(
        skip_ws(complete::tag(",")),
        skip_ws(number_comparison),
    ));

    let full_sequence = sequence::pair(skip_ws(number_comparison), many_with_comma);

    error::context(
        "number_comparison_array_value",
        combinator::map(
            sequence::delimited(
                skip_ws(complete::tag("(")),
                full_sequence,
                skip_ws(complete::tag(")")),
            ),
            |tup: (ast::NumberComparison, Vec<ast::NumberComparison>)| {
                let mut rest = tup.1;
                rest.insert(0, tup.0);
                ast::ValueType::NumberComparisonArray(rest)
            },
        ),
    )(input)
}

pub fn value_type(input: &str) -> ParseResult<&str, ast::ValueType> {
    error::context(
        "value_type",
        branch::alt((
            number_value,
            enum_variant_value,
            enum_variant_array_value,
            number_array_value,
            number_comparison_array_value,
            str_value,
        )),
    )(input)
}

pub fn comparison_type(input: &str) -> ParseResult<&str, ast::ComparisonType> {
    error::context(
        "comparison_operator",
        combinator::map_res(
            branch::alt((
                complete::tag("/="),
                complete::tag(">="),
                complete::tag("<="),
                complete::tag("="),
                complete::tag(">"),
                complete::tag("<"),
            )),
            |s: &str| match s {
                "/=" => Ok(ast::ComparisonType::NotEqual),
                ">=" => Ok(ast::ComparisonType::GreaterThanEqual),
                "<=" => Ok(ast::ComparisonType::LessThanEqual),
                "=" => Ok(ast::ComparisonType::Equal),
                ">" => Ok(ast::ComparisonType::GreaterThan),
                "<" => Ok(ast::ComparisonType::LessThan),
                _ => Err(EuclidError::InvalidOperator(s.to_string())),
            },
        ),
    )(input)
}

pub fn comparison(input: &str) -> ParseResult<&str, ast::Comparison> {
    error::context(
        "condition",
        combinator::map(
            sequence::tuple((
                skip_ws(complete::take_while1(|c: char| {
                    c.is_ascii_alphabetic() || c == '.' || c == '_'
                })),
                skip_ws(comparison_type),
                skip_ws(value_type),
            )),
            |tup: (&str, ast::ComparisonType, ast::ValueType)| ast::Comparison {
                lhs: tup.0.to_string(),
                comparison: tup.1,
                value: tup.2,
                metadata: std::collections::HashMap::new(),
            },
        ),
    )(input)
}

pub fn arbitrary_comparison(input: &str) -> ParseResult<&str, ast::Comparison> {
    error::context(
        "condition",
        combinator::map(
            sequence::tuple((
                skip_ws(string_str),
                skip_ws(comparison_type),
                skip_ws(string_str),
            )),
            |tup: (String, ast::ComparisonType, String)| ast::Comparison {
                lhs: "metadata".to_string(),
                comparison: tup.1,
                value: ast::ValueType::MetadataVariant(ast::MetadataValue {
                    key: tup.0,
                    value: tup.2,
                }),
                metadata: std::collections::HashMap::new(),
            },
        ),
    )(input)
}

pub fn comparison_array(input: &str) -> ParseResult<&str, Vec<ast::Comparison>> {
    let many_with_ampersand = error::context(
        "many_with_amp",
        multi::many0(sequence::preceded(skip_ws(complete::tag("&")), comparison)),
    );

    let full_sequence = sequence::pair(
        skip_ws(branch::alt((comparison, arbitrary_comparison))),
        many_with_ampersand,
    );

    error::context(
        "comparison_array",
        combinator::map(
            full_sequence,
            |tup: (ast::Comparison, Vec<ast::Comparison>)| {
                let mut rest = tup.1;
                rest.insert(0, tup.0);
                rest
            },
        ),
    )(input)
}

pub fn if_statement(input: &str) -> ParseResult<&str, ast::IfStatement> {
    let nested_block = sequence::delimited(
        skip_ws(complete::tag("{")),
        multi::many0(if_statement),
        skip_ws(complete::tag("}")),
    );

    error::context(
        "if_statement",
        combinator::map(
            sequence::pair(comparison_array, combinator::opt(nested_block)),
            |tup: (ast::IfCondition, Option<Vec<ast::IfStatement>>)| ast::IfStatement {
                condition: tup.0,
                nested: tup.1,
            },
        ),
    )(input)
}

pub fn rule_conditions_array(input: &str) -> ParseResult<&str, Vec<ast::IfStatement>> {
    error::context(
        "rules_array",
        sequence::delimited(
            skip_ws(complete::tag("{")),
            multi::many1(if_statement),
            skip_ws(complete::tag("}")),
        ),
    )(input)
}

pub fn rule<O: EuclidParsable>(input: &str) -> ParseResult<&str, ast::Rule<O>> {
    let rule_name = error::context(
        "rule_name",
        combinator::map(
            skip_ws(sequence::pair(
                complete::take_while1(|c: char| c.is_ascii_alphabetic() || c == '_'),
                complete::take_while(|c: char| c.is_ascii_alphanumeric() || c == '_'),
            )),
            |out: (&str, &str)| out.0.to_string() + out.1,
        ),
    );

    let connector_selection = error::context(
        "parse_output",
        sequence::preceded(skip_ws(complete::tag(":")), output),
    );

    error::context(
        "rule",
        combinator::map(
            sequence::tuple((rule_name, connector_selection, rule_conditions_array)),
            |tup: (String, O, Vec<ast::IfStatement>)| ast::Rule {
                name: tup.0,
                connector_selection: tup.1,
                statements: tup.2,
            },
        ),
    )(input)
}

pub fn output<O: EuclidParsable>(input: &str) -> ParseResult<&str, O> {
    O::parse_output(input)
}

pub fn default_output<O: EuclidParsable + 'static>(input: &str) -> ParseResult<&str, O> {
    error::context(
        "default_output",
        sequence::preceded(
            sequence::pair(skip_ws(complete::tag("default")), skip_ws(pchar::char(':'))),
            skip_ws(output),
        ),
    )(input)
}

pub fn program<O: EuclidParsable + 'static>(input: &str) -> ParseResult<&str, ast::Program<O>> {
    error::context(
        "program",
        combinator::map(
            sequence::pair(default_output, multi::many1(skip_ws(rule::<O>))),
            |tup: (O, Vec<ast::Rule<O>>)| ast::Program {
                default_selection: tup.0,
                rules: tup.1,
                metadata: std::collections::HashMap::new(),
            },
        ),
    )(input)
}
