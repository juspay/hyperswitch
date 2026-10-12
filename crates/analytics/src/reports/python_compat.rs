//! Renders values exactly as the Python report Lambdas did, so moving a report off Lambda does not
//! change a byte of the CSV or webhook body that merchants already parse and verify.
//!
//! The Lambdas wrote psycopg2 values through `csv.writer`, which calls `str()` on each one. So a
//! `json`/`jsonb` column arrives as a Python `dict`/`list` and is written as its `repr()`
//! (`{'card': {'last4': '4242'}}`), a timestamp as `datetime.__str__`, and a float as `repr(float)`.

use std::fmt::Write as _;

/// Python guards recursion around 1000 frames; stopping well short of that keeps the stack bounded.
const MAX_JSON_DEPTH: usize = 512;

/// A JSON document as `json.loads` would hand it to Python.
#[derive(Debug, PartialEq)]
enum PyValue {
    None,
    Bool(bool),
    /// Kept as text, because Python ints have no size limit.
    Int(String),
    Float(f64),
    Str(String),
    List(Vec<Self>),
    /// Insertion-ordered, matching a Python `dict`.
    Dict(Vec<(String, Self)>),
}

/// `str(value)` as `csv.writer` would write a `json`/`jsonb` column fetched by psycopg2, given the
/// column's text form. `None` means SQL `NULL`.
pub fn json_field(json_text: Option<&str>) -> String {
    match json_text {
        None => String::new(),
        Some(text) => match parse_json(text) {
            // `csv.writer` writes `None` as an empty field and a `str` without quotes.
            Some(PyValue::None) => String::new(),
            Some(PyValue::Str(value)) => value,
            Some(value) => {
                let mut out = String::with_capacity(text.len());
                push_repr(&mut out, &value);
                out
            }
            // Postgres validated the document, so this only guards against surprises.
            None => text.to_owned(),
        },
    }
}

/// `repr(float)`, which is also `str(float)` in Python 3.
pub fn float_repr(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_owned();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf"
        } else {
            "inf"
        }
        .to_owned();
    }
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        }
        .to_owned();
    }

    let sign = if value.is_sign_negative() { "-" } else { "" };
    // Rust's `{:e}` emits the shortest digits that round-trip, as Python's `repr` does.
    let scientific = format!("{:e}", value.abs());
    let Some((mantissa, exponent)) = scientific.split_once('e') else {
        return value.to_string();
    };
    let Ok(exponent) = exponent.parse::<i64>() else {
        return value.to_string();
    };
    let digits: String = mantissa.chars().filter(|ch| *ch != '.').collect();
    let Ok(digit_count) = i64::try_from(digits.len()) else {
        return value.to_string();
    };

    // Python places the decimal point `decimal_point` digits into `digits`.
    let decimal_point = exponent + 1;
    let zeros = |count: i64| "0".repeat(usize::try_from(count).unwrap_or(0));

    let body = if decimal_point <= -4 || decimal_point > 16 {
        let (first, rest) = digits.split_at(1);
        let exponent = decimal_point - 1;
        let exponent_sign = if exponent < 0 { '-' } else { '+' };
        match rest.is_empty() {
            true => format!("{first}e{exponent_sign}{:02}", exponent.abs()),
            false => format!("{first}.{rest}e{exponent_sign}{:02}", exponent.abs()),
        }
    } else if decimal_point <= 0 {
        format!("0.{}{digits}", zeros(-decimal_point))
    } else if decimal_point >= digit_count {
        format!("{digits}{}.0", zeros(decimal_point - digit_count))
    } else {
        let split = usize::try_from(decimal_point).unwrap_or(0);
        let (integer, fraction) = digits.split_at(split);
        format!("{integer}.{fraction}")
    };

    format!("{sign}{body}")
}

/// `str(bool)`.
pub fn bool_str(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}

/// `datetime.isoformat(sep)`, which is `str(datetime)` when `sep` is a space. The fraction is
/// printed only when it is non-zero, always as six digits.
pub fn datetime_str(value: time::PrimitiveDateTime, sep: char) -> String {
    let mut out = format!(
        "{:04}-{:02}-{:02}{sep}{:02}:{:02}:{:02}",
        value.year(),
        u8::from(value.month()),
        value.day(),
        value.hour(),
        value.minute(),
        value.second(),
    );
    let microsecond = value.microsecond();
    if microsecond != 0 {
        let _ = write!(out, ".{microsecond:06}");
    }
    out
}

/// Serializes `value` the way `json.dumps(value)` does with default arguments: `", "` and `": "`
/// separators, and every character outside printable ASCII escaped as `\uXXXX`.
pub fn json_dumps<T: serde::Serialize>(value: &T) -> serde_json::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut out, PythonJsonFormatter);
    value.serialize(&mut serializer)?;
    Ok(out)
}

struct PythonJsonFormatter;

impl serde_json::ser::Formatter for PythonJsonFormatter {
    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> std::io::Result<()>
    where
        W: ?Sized + std::io::Write,
    {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }

    fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> std::io::Result<()>
    where
        W: ?Sized + std::io::Write,
    {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }

    fn begin_object_value<W>(&mut self, writer: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + std::io::Write,
    {
        writer.write_all(b": ")
    }

    fn write_string_fragment<W>(&mut self, writer: &mut W, fragment: &str) -> std::io::Result<()>
    where
        W: ?Sized + std::io::Write,
    {
        // serde_json escapes quotes, backslashes and control characters before calling this.
        for ch in fragment.chars() {
            if ch.is_ascii() && ch != '\u{7f}' {
                let mut buffer = [0u8; 1];
                writer.write_all(ch.encode_utf8(&mut buffer).as_bytes())?;
            } else {
                let mut units = [0u16; 2];
                for unit in ch.encode_utf16(&mut units) {
                    write!(writer, "\\u{unit:04x}")?;
                }
            }
        }
        Ok(())
    }
}

fn push_repr(out: &mut String, value: &PyValue) {
    match value {
        PyValue::None => out.push_str("None"),
        PyValue::Bool(value) => out.push_str(bool_str(*value)),
        PyValue::Int(digits) => out.push_str(digits),
        PyValue::Float(value) => out.push_str(&float_repr(*value)),
        PyValue::Str(value) => push_str_repr(out, value),
        PyValue::List(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                push_repr(out, item);
            }
            out.push(']');
        }
        PyValue::Dict(entries) => {
            out.push('{');
            for (index, (key, item)) in entries.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                push_str_repr(out, key);
                out.push_str(": ");
                push_repr(out, item);
            }
            out.push('}');
        }
    }
}

/// `repr(str)`: single quotes unless the text holds a single quote and no double quote.
fn push_str_repr(out: &mut String, value: &str) {
    let quote = if value.contains('\'') && !value.contains('"') {
        '"'
    } else {
        '\''
    };

    out.push(quote);
    for ch in value.chars() {
        let code_point = u32::from(ch);
        match ch {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ if ch == quote => {
                out.push('\\');
                out.push(ch);
            }
            _ if code_point < 0x20 || code_point == 0x7f => {
                let _ = write!(out, "\\x{code_point:02x}");
            }
            _ if ch.is_ascii() || is_printable(code_point) => out.push(ch),
            _ if code_point <= 0xff => {
                let _ = write!(out, "\\x{code_point:02x}");
            }
            _ if code_point <= 0xffff => {
                let _ = write!(out, "\\u{code_point:04x}");
            }
            _ => {
                let _ = write!(out, "\\U{code_point:08x}");
            }
        }
    }
    out.push(quote);
}

/// Python's `str.isprintable` for a non-ASCII code point: false for the categories Cc, Cf, Co, Zs,
/// Zl, Zp and Cn. Cn (unassigned) needs the Unicode database, so unassigned code points are treated
/// as printable; Python would escape them.
fn is_printable(code_point: u32) -> bool {
    !matches!(
        code_point,
        // Cc
        0x80..=0x9f
        // Zs
        | 0xa0 | 0x1680 | 0x2000..=0x200a | 0x202f | 0x205f | 0x3000
        // Zl, Zp
        | 0x2028 | 0x2029
        // Cf
        | 0xad | 0x600..=0x605 | 0x61c | 0x6dd | 0x70f | 0x890..=0x891 | 0x8e2 | 0x180e
        | 0x200b..=0x200f | 0x202a..=0x202e | 0x2060..=0x2064 | 0x2066..=0x206f | 0xfeff
        | 0xfff9..=0xfffb | 0x110bd | 0x110cd | 0x13430..=0x1343f | 0x1bca0..=0x1bca3
        | 0x1d173..=0x1d17a | 0xe0001 | 0xe0020..=0xe007f
        // Co
        | 0xe000..=0xf8ff | 0xf0000..=0xffffd | 0x100000..=0x10fffd
        // Cn: noncharacters, which are always unassigned
        | 0xfdd0..=0xfdef | 0xfffe | 0xffff | 0x1fffe | 0x1ffff | 0x2fffe | 0x2ffff
        | 0x3fffe | 0x3ffff | 0x4fffe | 0x4ffff | 0x5fffe | 0x5ffff | 0x6fffe | 0x6ffff
        | 0x7fffe | 0x7ffff | 0x8fffe | 0x8ffff | 0x9fffe | 0x9ffff | 0xafffe | 0xaffff
        | 0xbfffe | 0xbffff | 0xcfffe | 0xcffff | 0xdfffe | 0xdffff | 0xefffe | 0xeffff
        | 0x10fffe | 0x10ffff
    )
}

/// Parses JSON text into the values `json.loads` would produce, keeping key order.
fn parse_json(text: &str) -> Option<PyValue> {
    let mut parser = JsonParser {
        bytes: text.as_bytes(),
        text,
        position: 0,
    };
    let value = parser.value(0)?;
    parser.skip_whitespace();
    (parser.position == text.len()).then_some(value)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    text: &'a str,
    position: usize,
}

impl JsonParser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.position += 1;
        }
    }

    fn eat(&mut self, literal: &str) -> Option<()> {
        self.text
            .get(self.position..)?
            .starts_with(literal)
            .then(|| self.position += literal.len())
    }

    fn value(&mut self, depth: usize) -> Option<PyValue> {
        if depth > MAX_JSON_DEPTH {
            return None;
        }
        self.skip_whitespace();
        match self.peek()? {
            b'n' => self.eat("null").map(|()| PyValue::None),
            b't' => self.eat("true").map(|()| PyValue::Bool(true)),
            b'f' => self.eat("false").map(|()| PyValue::Bool(false)),
            b'"' => self.string().map(PyValue::Str),
            b'[' => self.list(depth),
            b'{' => self.dict(depth),
            _ => self.number(),
        }
    }

    fn list(&mut self, depth: usize) -> Option<PyValue> {
        self.position += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek()? == b']' {
            self.position += 1;
            return Some(PyValue::List(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_whitespace();
            match self.peek()? {
                b',' => self.position += 1,
                b']' => {
                    self.position += 1;
                    return Some(PyValue::List(items));
                }
                _ => return None,
            }
        }
    }

    fn dict(&mut self, depth: usize) -> Option<PyValue> {
        self.position += 1;
        let mut entries: Vec<(String, PyValue)> = Vec::new();
        self.skip_whitespace();
        if self.peek()? == b'}' {
            self.position += 1;
            return Some(PyValue::Dict(entries));
        }
        loop {
            self.skip_whitespace();
            if self.peek()? != b'"' {
                return None;
            }
            let key = self.string()?;
            self.skip_whitespace();
            if self.peek()? != b':' {
                return None;
            }
            self.position += 1;
            let item = self.value(depth + 1)?;

            // A repeated key keeps its first position and takes the last value, as a dict does.
            match entries.iter_mut().find(|(existing, _)| *existing == key) {
                Some(entry) => entry.1 = item,
                None => entries.push((key, item)),
            }

            self.skip_whitespace();
            match self.peek()? {
                b',' => self.position += 1,
                b'}' => {
                    self.position += 1;
                    return Some(PyValue::Dict(entries));
                }
                _ => return None,
            }
        }
    }

    fn number(&mut self) -> Option<PyValue> {
        let start = self.position;
        let mut is_float = false;
        while let Some(byte) = self.peek() {
            match byte {
                b'0'..=b'9' | b'-' | b'+' => {}
                b'.' | b'e' | b'E' => is_float = true,
                _ => break,
            }
            self.position += 1;
        }
        let literal = self.text.get(start..self.position)?;
        if literal.is_empty() {
            return None;
        }

        match is_float {
            true => literal.parse::<f64>().ok().map(PyValue::Float),
            false => {
                let digits = literal.strip_prefix('-');
                if !literal
                    .bytes()
                    .skip(usize::from(digits.is_some()))
                    .all(|b| b.is_ascii_digit())
                {
                    return None;
                }
                // `int("-0")` is `0`.
                Some(PyValue::Int(match digits {
                    Some(digits) if digits.bytes().all(|b| b == b'0') => digits.to_owned(),
                    _ => literal.to_owned(),
                }))
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        self.position += 1;
        let mut out = String::new();
        loop {
            let rest = self.text.get(self.position..)?;
            let next_special = rest.find(['"', '\\'])?;
            out.push_str(rest.get(..next_special)?);
            self.position += next_special;

            match self.peek()? {
                b'"' => {
                    self.position += 1;
                    return Some(out);
                }
                _ => {
                    self.position += 1;
                    let escape = self.peek()?;
                    self.position += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => out.push(self.unicode_escape()?),
                        _ => return None,
                    }
                }
            }
        }
    }

    /// Reads the hex digits after `\u`, joining a surrogate pair. Postgres rejects lone surrogates,
    /// so one only appears in malformed input and becomes U+FFFD.
    fn unicode_escape(&mut self) -> Option<char> {
        let high = self.hex4()?;
        if !(0xd800..0xdc00).contains(&high) {
            return Some(char::from_u32(high).unwrap_or(char::REPLACEMENT_CHARACTER));
        }
        if self.eat("\\u").is_none() {
            return Some(char::REPLACEMENT_CHARACTER);
        }
        let low = self.hex4()?;
        if !(0xdc00..0xe000).contains(&low) {
            return Some(char::REPLACEMENT_CHARACTER);
        }
        char::from_u32(0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00))
    }

    fn hex4(&mut self) -> Option<u32> {
        let digits = self.text.get(self.position..self.position + 4)?;
        let value = u32::from_str_radix(digits, 16).ok()?;
        self.position += 4;
        Some(value)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // Expected strings were checked against CPython 3.11.16 (the Lambda runtime) on the same
    // inputs: `repr(float)`, `csv.writer` over `json.loads(...)`, `str(datetime)`, `json.dumps`.

    #[test]
    fn float_repr_matches_python() {
        let cases = [
            (100.0, "100.0"),
            (12.5, "12.5"),
            (0.1, "0.1"),
            (0.001, "0.001"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (1.5e-7, "1.5e-07"),
            (1234567890123456.0, "1234567890123456.0"),
            (1e16, "1e+16"),
            (1.25e17, "1.25e+17"),
            (1.5e300, "1.5e+300"),
            (-42.42, "-42.42"),
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (6500.0 / 100.0, "65.0"),
            (1999.0 / 1000.0, "1.999"),
            (1.0 / 3.0, "0.3333333333333333"),
            (f64::INFINITY, "inf"),
        ];
        for (value, expected) in cases {
            assert_eq!(float_repr(value), expected, "repr({value:?})");
        }
    }

    #[test]
    fn json_field_matches_python_str_of_loaded_value() {
        let cases = [
            (None, ""),
            (Some("null"), ""),
            (Some("\"plain\""), "plain"),
            (Some("true"), "True"),
            (Some("12"), "12"),
            (Some("-0"), "0"),
            (Some("1.50"), "1.5"),
            (Some("1e2"), "100.0"),
            (Some("[]"), "[]"),
            (Some("{}"), "{}"),
            (
                Some(r#"{"card": {"last4": "4242", "exp_year": 2030, "saved": false, "x": null}}"#),
                "{'card': {'last4': '4242', 'exp_year': 2030, 'saved': False, 'x': None}}",
            ),
            (
                Some(r#"[{"product_name": "T-shirt", "quantity": 1, "amount": 6540}, null]"#),
                "[{'product_name': 'T-shirt', 'quantity': 1, 'amount': 6540}, None]",
            ),
            (Some(r#"["card","wallet"]"#), "['card', 'wallet']"),
            (Some(r#"{"a": 1, "b": 2, "a": 3}"#), "{'a': 3, 'b': 2}"),
            (Some(r#"{"q": "it's"}"#), r#"{'q': "it's"}"#),
            (Some(r#"{"q": "it's \"x\""}"#), r#"{'q': 'it\'s "x"'}"#),
            (
                Some(r#"{"s": "a\\b\nc\td\u0001"}"#),
                r"{'s': 'a\\b\nc\td\x01'}",
            ),
            (
                Some(r#"{"name": "Zoë Ångström 東京"}"#),
                "{'name': 'Zoë Ångström 東京'}",
            ),
            (Some(r#"{"name": "\u00e9\ud83d\ude00"}"#), "{'name': 'é😀'}"),
            (
                Some(r#"{"nbsp": "a\u00a0b", "zw": "\u200b"}"#),
                r"{'nbsp': 'a\xa0b', 'zw': '\u200b'}",
            ),
            (
                Some(r#"{"del": "\u007f", "c1": "\u0085"}"#),
                r"{'del': '\x7f', 'c1': '\x85'}",
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(json_field(input), expected, "str(json.loads({input:?}))");
        }
    }

    #[test]
    fn datetime_str_matches_python() {
        let whole = time::macros::datetime!(2026-10-01 09:05:03);
        assert_eq!(datetime_str(whole, ' '), "2026-10-01 09:05:03");
        assert_eq!(datetime_str(whole, 'T'), "2026-10-01T09:05:03");

        let fraction = time::macros::datetime!(2026-10-01 09:05:03.12);
        assert_eq!(datetime_str(fraction, ' '), "2026-10-01 09:05:03.120000");
    }

    #[test]
    fn json_dumps_matches_python() {
        #[derive(serde::Serialize)]
        struct Payload<'a> {
            org_id: &'a str,
            merchant_id: Option<&'a str>,
            data: Data<'a>,
            event: &'a str,
        }
        #[derive(serde::Serialize)]
        struct Data<'a> {
            download_url: &'a str,
            expires_in_hours: u32,
            note: &'a str,
        }

        let payload = Payload {
            org_id: "org_1",
            merchant_id: None,
            data: Data {
                download_url: "https://b.example/k?X-Amz-Signature=ab&x=1/2",
                expires_in_hours: 168,
                note: "é\"\n\u{7f}😀",
            },
            event: "report_generation.completed",
        };

        assert_eq!(
            String::from_utf8(json_dumps(&payload).unwrap()).unwrap(),
            r#"{"org_id": "org_1", "merchant_id": null, "data": {"download_url": "https://b.example/k?X-Amz-Signature=ab&x=1/2", "expires_in_hours": 168, "note": "\u00e9\"\n\u007f\ud83d\ude00"}, "event": "report_generation.completed"}"#
        );
    }
}
