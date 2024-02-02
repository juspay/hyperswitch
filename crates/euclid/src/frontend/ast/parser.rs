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
        /// Parse the input string and return a ParseResult containing a reference to a string and a Self object.
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
/// This function takes a parser `inner` and returns a new parser that skips any leading whitespace before applying the `inner` parser. The input to the returned parser is a reference to a string slice, and the output is a `ParseResult` containing a reference to a string slice and a value of type `O`.
pub fn skip_ws<'a, F: 'a, O>(inner: F) -> impl FnMut(&'a str) -> ParseResult<&str, O>
where
    F: FnMut(&'a str) -> ParseResult<&str, O>,
{
    sequence::preceded(pchar::multispace0, inner)
}

/// This function takes a string input and attempts to parse it into an i64 integer. It uses the `take_while1` combinator to extract a string of ASCII digits, then attempts to parse that string into an i64. If successful, it returns the parsed i64 value, otherwise it returns an `InvalidNumber` error containing the original string.
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

/// This method takes a string input and parses it to extract a string enclosed in double quotes. 
/// It returns the parsed string as a `String` type.
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

/// This function parses an identifier from the input string, which consists of one or more ASCII alphabetic characters or underscores followed by zero or more ASCII alphanumeric characters or underscores. It returns the parsed identifier as a String.
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
/// Parses a string to extract a percentage value and returns it as a result. The input string
/// should be in the format "<number>%" where <number> is a positive integer. If the input
/// string is not in the correct format, an error is returned.
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

/// Parses the input string to extract a number value and returns the result as an `ast::ValueType`.
/// 
/// # Arguments
/// * `input` - A reference to the input string to be parsed
/// 
/// # Returns
/// A `ParseResult` containing a reference to the remaining unparsed input string and the parsed `ast::ValueType`
pub fn number_value(input: &str) -> ParseResult<&str, ast::ValueType> {
    error::context(
        "number_value",
        combinator::map(num_i64, ast::ValueType::Number),
    )(input)
}

/// Parses the input string into an AST (Abstract Syntax Tree) value of type `ast::ValueType::StrValue`.
/// 
/// # Arguments
/// 
/// * `input` - A reference to the input string to be parsed
/// 
/// # Returns
/// 
/// A `ParseResult` containing a reference to the parsed string value and the corresponding AST value type.
/// 
pub fn str_value(input: &str) -> ParseResult<&str, ast::ValueType> {
    error::context(
        "str_value",
        combinator::map(string_str, ast::ValueType::StrValue),
    )(input)
}
/// Parses the input string and returns a string composed of valid enum value characters.
pub fn enum_value_string(input: &str) -> ParseResult<&str, String> {
    combinator::map(
        sequence::pair(
            complete::take_while1(|c: char| c.is_ascii_alphabetic() || c == '_'),
            complete::take_while(|c: char| c.is_ascii_alphanumeric() || c == '_'),
        ),
        |out: (&str, &str)| out.0.to_string() + out.1,
    )(input)
}

/// Parses the input string to extract the value of an enum variant and returns a ParseResult containing the extracted value and the remaining input.
pub fn enum_variant_value(input: &str) -> ParseResult<&str, ast::ValueType> {
    error::context(
        "enum_variant_value",
        combinator::map(enum_value_string, ast::ValueType::EnumVariant),
    )(input)
}

/// Parses a string input to extract a sequence of comma-separated integer values enclosed in parentheses,
/// and returns a Result containing the remaining input and an ast::ValueType::NumberArray if successful.
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

/// Parses an input string to extract an enum variant array value.
///
/// # Arguments
///
/// * `input` - A string slice to be parsed
///
/// # Returns
///
/// * If successful, returns a tuple containing the remaining input and the parsed enum variant array value
/// * If unsuccessful, returns a parse error
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

/// Parses the input string to extract a number comparison, returning the remaining input and the parsed NumberComparison.
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

/// Parses a string input to create a number comparison array value, which consists of a sequence of number comparison values enclosed in parentheses and separated by commas.
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

/// Parses the input string to determine the type of value and returns the parsed result as a `ast::ValueType`.
///
/// # Arguments
///
/// * `input` - A reference to the input string to be parsed
///
/// # Return
///
/// Returns a `ParseResult` containing a reference to the remaining input string and the parsed `ast::ValueType`.
///
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

/// This method takes a string input and parses it to determine the comparison operator type.
/// It returns a ParseResult with the parsed comparison operator type.
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

/// This method takes a string input and attempts to parse it into a comparison
/// expression. It looks for a sequence of an alphabetic string, a comparison type,
/// and a value type, and constructs a Comparison struct from the parsed values.
/// If successful, it returns a ParseResult containing the parsed Comparison and
/// the remaining input. If the input cannot be parsed, it returns an error.
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

/// Parses a comparison expression from the input string and returns the parsed comparison.
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

/// Parses the input string to create a vector of Comparison AST nodes.
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

/// Parses an if statement from the input string and returns a ParseResult with the remaining input and the parsed if statement.
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

/// Parses a string input to extract an array of IfStatement objects representing rule conditions.
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

/// Parses the input string to create an AST representation of a rule, including the rule name, connector selection, and rule conditions array.
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

/// Parses the input string using the EuclidParsable trait and returns the parsed output.
pub fn output<O: EuclidParsable>(input: &str) -> ParseResult<&str, O> {
    O::parse_output(input)
}

/// Parses the input string to extract the default output.
pub fn default_output<O: EuclidParsable + 'static>(input: &str) -> ParseResult<&str, O> {
    error::context(
        "default_output",
        sequence::preceded(
            sequence::pair(skip_ws(complete::tag("default")), skip_ws(pchar::char(':'))),
            skip_ws(output),
        ),
    )(input)
}

/// Parses the input string to construct an abstract syntax tree representing a program with the specified output type.
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
