//! Lossless body transport and format-aware interpolation for the vault proxy.

use base64::{engine::general_purpose::STANDARD, Engine};
use quick_xml::{events::Event, Reader};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// The connector body inside the JSON proxy envelope. Binary bodies are opaque.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ProxyBody {
    /// Serialized JSON template. Unchanged tokens retain their original bytes.
    Json(String),
    /// A complete, UTF-8 XML/SOAP document.
    Xml(String),
    /// An encoded form. Duplicate keys and ordering are significant.
    FormUrlEncoded(String),
    /// A UTF-8 text template.
    Text(String),
    /// Typed multipart parts; file contents are opaque base64.
    Multipart(Vec<ProxyMultipartPart>),
    /// Opaque base64-encoded bytes, with no token interpolation.
    RawBytes(String),
    /// An absent HTTP body, distinct from JSON null or an empty body.
    None,
}

impl hyperswitch_masking::SerializableSecret for ProxyBody {}

impl std::fmt::Debug for ProxyBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Json(_) => "ProxyBody::Json",
            Self::Xml(_) => "ProxyBody::Xml",
            Self::FormUrlEncoded(_) => "ProxyBody::FormUrlEncoded",
            Self::Text(_) => "ProxyBody::Text",
            Self::Multipart(_) => "ProxyBody::Multipart",
            Self::RawBytes(_) => "ProxyBody::RawBytes",
            Self::None => "ProxyBody::None",
        })
    }
}

/// A multipart field or file. Metadata is never interpolated.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProxyMultipartPart {
    /// A text field whose value may reference vaulted data.
    Text {
        /// Form field name.
        name: String,
        /// UTF-8 value that may contain vault placeholders.
        value: String,
    },
    /// A binary file; bytes must survive the proxy without modification.
    File {
        /// Form field name.
        name: String,
        /// Original file name.
        filename: String,
        /// Content-Type of the opaque file bytes.
        mime_type: String,
        /// Standard base64-encoded file bytes.
        data_base64: String,
    },
}

impl std::fmt::Debug for ProxyMultipartPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Text { .. } => "ProxyMultipartPart::Text",
            Self::File { .. } => "ProxyMultipartPart::File",
        })
    }
}

/// Prepared bytes and the effective connector Content-Type.
pub struct PreparedProxyBody {
    /// `None` means there must be no HTTP body.
    pub bytes: Option<Vec<u8>>,
    /// Original Content-Type, or a default derived from the explicit body type.
    pub content_type: Option<String>,
}

impl std::fmt::Debug for PreparedProxyBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedProxyBody")
            .field("body_length", &self.bytes.as_ref().map(Vec::len))
            .field("content_type", &self.content_type)
            .finish()
    }
}

impl PreparedProxyBody {
    /// Build the connector request from resolved bytes. Recompute framing headers
    /// because interpolation changes the body length and multipart boundary.
    pub fn into_request(
        self,
        url: &str,
        method: crate::request::Method,
        headers: Vec<(String, hyperswitch_masking::Maskable<String>)>,
    ) -> crate::request::Request {
        let mut headers: Vec<_> = headers
            .into_iter()
            .filter(|(name, _)| {
                !["content-type", "content-length", "transfer-encoding"]
                    .iter()
                    .any(|blocked| name.eq_ignore_ascii_case(blocked))
            })
            .collect();
        if let Some(content_type) = self.content_type {
            headers.push(("Content-Type".to_owned(), content_type.into()));
        }
        let mut builder = crate::request::RequestBuilder::new()
            .method(method)
            .url(url)
            .attach_default_headers()
            .headers(headers);
        if let Some(bytes) = self.bytes {
            builder = builder.set_body(crate::request::RequestContent::RawBytes(bytes));
        }
        builder.build()
    }
}

/// Errors omit template and resolved field contents.
#[derive(Debug, thiserror::Error)]
pub enum ProxyBodyError {
    /// A placeholder is syntactically invalid.
    #[error("Malformed vault placeholder")]
    InvalidPlaceholder,
    /// The referenced field does not exist or cannot be used as a scalar.
    #[error("Missing or non-scalar vault field: {0}")]
    InvalidField(String),
    /// The body and Content-Type cannot be interpreted safely.
    #[error("Connector body does not match its Content-Type")]
    InvalidContentType,
    /// An XML document or its template placement is invalid.
    #[error("Invalid XML template")]
    InvalidXml,
    /// Invalid binary transport data.
    #[error("Invalid base64 body")]
    InvalidBase64,
    /// Invalid multipart metadata, or a boundary that occurs inside a part.
    #[error("Invalid multipart part or boundary")]
    InvalidMultipart,
    /// Form percent escapes must be valid and decode to UTF-8.
    #[error("Invalid URL-encoded template")]
    InvalidForm,
    /// Serialization failed.
    #[error("Failed to serialize connector body")]
    Serialization,
}

/// Lower-cased media type of a Content-Type header value, without parameters.
pub fn media_type(header: &str) -> String {
    header
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

impl ProxyBody {
    /// Stable, bounded body format for structured logs and metric dimensions.
    pub fn as_metric_str(&self) -> &'static str {
        match self {
            Self::Json(_) => "json",
            Self::Xml(_) => "xml",
            Self::FormUrlEncoded(_) => "form_url_encoded",
            Self::Text(_) => "text",
            Self::Multipart(_) => "multipart",
            Self::RawBytes(_) => "raw_bytes",
            Self::None => "none",
        }
    }

    /// Populate the old request field for callers during a staged rollout. Only
    /// JSON is sent correctly by servers predating the typed body, so other
    /// formats travel once, in `body`, keeping the envelope within size limits.
    pub fn legacy_value(&self) -> Value {
        match self {
            Self::Json(value) => {
                serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.clone()))
            }
            Self::Xml(_)
            | Self::FormUrlEncoded(_)
            | Self::Text(_)
            | Self::Multipart(_)
            | Self::RawBytes(_)
            | Self::None => Value::Null,
        }
    }
    /// Interpret the original API contract without changing JSON callers.
    pub fn from_legacy(body: Value, content_type: Option<&str>) -> Result<Self, ProxyBodyError> {
        let kind = content_type
            .map(media_type)
            .unwrap_or_else(|| "application/json".into());
        if kind == "application/json" || kind.ends_with("+json") {
            return serde_json::to_string(&body)
                .map(Self::Json)
                .map_err(|_| ProxyBodyError::Serialization);
        }
        let text = body
            .as_str()
            .ok_or(ProxyBodyError::InvalidContentType)?
            .to_owned();
        match kind.as_str() {
            "application/x-www-form-urlencoded" => Ok(Self::FormUrlEncoded(text)),
            "text/xml" | "application/xml" => Ok(Self::Xml(text)),
            _ if kind.ends_with("+xml") => Ok(Self::Xml(text)),
            _ if kind.starts_with("text/") => Ok(Self::Text(text)),
            _ => Err(ProxyBodyError::InvalidContentType),
        }
    }

    /// Resolve the template and produce connector bytes, never a JSON wrapper string.
    pub fn prepare(
        &self,
        vault: &Value,
        content_type: Option<&str>,
    ) -> Result<PreparedProxyBody, ProxyBodyError> {
        let declared = content_type.map(media_type);
        let default = match self {
            Self::Json(_) => "application/json",
            Self::Xml(_) => "application/xml",
            Self::FormUrlEncoded(_) => "application/x-www-form-urlencoded",
            Self::Text(_) => "text/plain; charset=utf-8",
            Self::RawBytes(_) => "application/octet-stream",
            Self::Multipart(_) => "multipart/form-data",
            Self::None => "",
        };
        if let Some(kind) = declared.as_deref() {
            let valid = match self {
                Self::Json(_) => kind == "application/json" || kind.ends_with("+json"),
                Self::Xml(_) => {
                    kind == "application/xml" || kind == "text/xml" || kind.ends_with("+xml")
                }
                Self::FormUrlEncoded(_) => kind == "application/x-www-form-urlencoded",
                Self::Text(_) => kind.starts_with("text/") && kind != "text/xml",
                Self::Multipart(_) => kind == "multipart/form-data",
                Self::RawBytes(_) | Self::None => true,
            };
            if !valid {
                return Err(ProxyBodyError::InvalidContentType);
            }
        }
        // Templates are UTF-8; reject a conflicting declaration rather than
        // sending bytes that the connector will decode using another charset.
        let parsed_content_type = content_type
            .filter(|_| !matches!(self, Self::RawBytes(_) | Self::None))
            .map(str::parse::<mime::Mime>)
            .transpose()
            .map_err(|_| ProxyBodyError::InvalidContentType)?;
        if parsed_content_type.as_ref().is_some_and(|content_type| {
            content_type.params().any(|(name, value)| {
                name == mime::CHARSET
                    && !value.as_str().eq_ignore_ascii_case("utf-8")
                    && !value.as_str().eq_ignore_ascii_case("utf8")
            })
        }) {
            return Err(ProxyBodyError::InvalidContentType);
        }
        let effective_type = content_type
            .map(str::to_owned)
            .or_else(|| (!default.is_empty()).then(|| default.to_owned()));
        let bytes = match self {
            Self::Json(template) => resolve_json_template(template, vault)?.into_bytes(),
            Self::Xml(template) => resolve_xml(template, vault)?.into_bytes(),
            Self::FormUrlEncoded(template) => resolve_form(template, vault)?.into_bytes(),
            Self::Text(template) => substitute(template, vault, str::to_owned)?.into_bytes(),
            Self::RawBytes(encoded) => STANDARD
                .decode(encoded)
                .map_err(|_| ProxyBodyError::InvalidBase64)?,
            Self::Multipart(parts) => {
                return prepare_multipart(parts, vault, content_type, parsed_content_type.as_ref())
            }
            Self::None => {
                return Ok(PreparedProxyBody {
                    bytes: None,
                    content_type: effective_type,
                })
            }
        };
        Ok(PreparedProxyBody {
            bytes: Some(bytes),
            content_type: effective_type,
        })
    }
}

fn find_field<'a>(vault: &'a Value, field: &str) -> Option<&'a Value> {
    match vault {
        Value::Object(map) => map
            .get(field)
            .or_else(|| map.values().find_map(|v| find_field(v, field))),
        Value::Array(values) => values.iter().find_map(|v| find_field(v, field)),
        _ => None,
    }
}

fn scalar(vault: &Value, field: &str) -> Result<String, ProxyBodyError> {
    match find_field(vault, field) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        Some(Value::Bool(b)) => Ok(b.to_string()),
        _ => Err(ProxyBodyError::InvalidField(field.to_owned())),
    }
}

fn placeholder(input: &str) -> Result<Option<(&str, usize)>, ProxyBodyError> {
    let Some(rest) = input.strip_prefix("{{") else {
        return Ok(None);
    };
    if !rest.trim_start().starts_with('$') {
        return Ok(None);
    }
    let end = rest.find("}}").ok_or(ProxyBodyError::InvalidPlaceholder)?;
    let field = rest[..end]
        .trim()
        .strip_prefix('$')
        .ok_or(ProxyBodyError::InvalidPlaceholder)?;
    if field.is_empty() || !field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ProxyBodyError::InvalidPlaceholder);
    }
    Ok(Some((field, end + 4)))
}

fn substitute(
    template: &str,
    vault: &Value,
    escape: impl Fn(&str) -> String,
) -> Result<String, ProxyBodyError> {
    let mut output = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some((field, consumed)) = placeholder(rest)? {
            output.push_str(&escape(&scalar(vault, field)?));
            rest = &rest[consumed..];
        } else {
            output.push_str("{{");
            rest = &rest[2..];
        }
    }
    output.push_str(rest);
    Ok(output)
}

fn has_reference(template: &str) -> Result<bool, ProxyBodyError> {
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        rest = &rest[start..];
        if placeholder(rest)?.is_some() {
            return Ok(true);
        }
        rest = &rest[2..];
    }
    Ok(false)
}

fn resolve_json_string(value: &str, vault: &Value) -> Result<Value, ProxyBodyError> {
    if let Some((field, consumed)) = placeholder(value)? {
        if consumed == value.len() {
            return find_field(vault, field)
                .cloned()
                .ok_or_else(|| ProxyBodyError::InvalidField(field.to_owned()));
        }
    }
    substitute(value, vault, str::to_owned).map(Value::String)
}

fn resolve_json_template(template: &str, vault: &Value) -> Result<String, ProxyBodyError> {
    // Validate structure once, then replace only string value tokens. This keeps
    // whitespace, field order, numeric spelling and existing escapes unchanged.
    serde_json::from_str::<Value>(template).map_err(|_| ProxyBodyError::Serialization)?;
    let bytes = template.as_bytes();
    let mut output = String::with_capacity(template.len());
    let mut copied = 0;
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte != b'"' {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while let Some(&byte) = bytes.get(index) {
            if byte == b'"' {
                index += 1;
                break;
            }
            index += if byte == b'\\' { 2 } else { 1 };
        }
        let token = template
            .get(start..index)
            .ok_or(ProxyBodyError::Serialization)?;
        let is_key = bytes
            .get(index..)
            .unwrap_or_default()
            .iter()
            .copied()
            .find(|b| !b.is_ascii_whitespace())
            == Some(b':');
        if is_key {
            continue;
        }
        let original: String =
            serde_json::from_str(token).map_err(|_| ProxyBodyError::Serialization)?;
        let resolved = resolve_json_string(&original, vault)?;
        if resolved.as_str() != Some(original.as_str()) {
            output.push_str(&template[copied..start]);
            output.push_str(
                &serde_json::to_string(&resolved).map_err(|_| ProxyBodyError::Serialization)?,
            );
            copied = index;
        }
    }
    output.push_str(&template[copied..]);
    Ok(output)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
        .replace('\t', "&#9;")
}

fn resolve_xml(template: &str, vault: &Value) -> Result<String, ProxyBodyError> {
    let valid_chars = |value: &str| {
        value.chars().all(|c| {
            matches!(c, '\t' | '\r' | '\n')
                || ('\u{20}'..='\u{d7ff}').contains(&c)
                || ('\u{e000}'..='\u{fffd}').contains(&c)
                || ('\u{10000}'..='\u{10ffff}').contains(&c)
        })
    };
    if !valid_chars(template) {
        return Err(ProxyBodyError::InvalidXml);
    }
    let mut reader = Reader::from_str(template);
    let mut output = String::with_capacity(template.len());
    let mut position = 0;
    let mut depth: usize = 0;
    let mut roots = 0;
    loop {
        let event = reader
            .read_event()
            .map_err(|_| ProxyBodyError::InvalidXml)?;
        let next = reader.buffer_position();
        let original = &template[position..next];
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                if element
                    .name()
                    .as_ref()
                    .windows(2)
                    .any(|value| value == b"{{")
                {
                    return Err(ProxyBodyError::InvalidXml);
                }
                for attr in element.attributes() {
                    let attr = attr.map_err(|_| ProxyBodyError::InvalidXml)?;
                    if attr.key.as_ref().windows(2).any(|value| value == b"{{") {
                        return Err(ProxyBodyError::InvalidXml);
                    }
                }
                if depth == 0 {
                    roots += 1;
                }
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
                output.push_str(&substitute(original, vault, xml_escape)?);
            }
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or(ProxyBodyError::InvalidXml)?;
                output.push_str(original);
            }
            Event::Text(_) => {
                if depth == 0 && !original.trim().is_empty() {
                    return Err(ProxyBodyError::InvalidXml);
                }
                output.push_str(&substitute(original, vault, xml_escape)?);
            }
            Event::CData(_) => {
                if depth == 0 {
                    return Err(ProxyBodyError::InvalidXml);
                }
                output.push_str(&substitute(original, vault, |s| {
                    s.replace("]]>", "]]]]><![CDATA[>")
                })?);
            }
            Event::Decl(ref declaration) => {
                let encoding = declaration
                    .encoding()
                    .transpose()
                    .map_err(|_| ProxyBodyError::InvalidXml)?;
                if has_reference(original)?
                    || encoding.is_some_and(|encoding| {
                        !encoding.eq_ignore_ascii_case(b"utf-8")
                            && !encoding.eq_ignore_ascii_case(b"utf8")
                    })
                {
                    return Err(ProxyBodyError::InvalidXml);
                }
                output.push_str(original);
            }
            Event::Eof => break,
            _ => {
                // Token references in comments, declarations, DTDs and processing
                // instructions are not card data positions.
                if has_reference(original)? {
                    return Err(ProxyBodyError::InvalidXml);
                }
                output.push_str(original);
            }
        }
        position = next;
    }
    if roots != 1 || depth != 0 || !valid_chars(&output) {
        return Err(ProxyBodyError::InvalidXml);
    }
    Ok(output)
}

fn decode_form(value: &str) -> Result<String, ProxyBodyError> {
    // urlencoding leaves malformed escapes untouched; reject them explicitly.
    let bytes = value.as_bytes();
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte == b'%' {
            let valid_escape = bytes
                .get(index + 1..index + 3)
                .is_some_and(|hex| hex.iter().all(u8::is_ascii_hexdigit));
            if !valid_escape {
                return Err(ProxyBodyError::InvalidForm);
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    // url::form_urlencoded's public parser replaces invalid UTF-8; urlencoding
    // rejects it instead. `+` is replaced first so that `%2B` still decodes to `+`.
    urlencoding::decode(&value.replace('+', " "))
        .map(std::borrow::Cow::into_owned)
        .map_err(|_| ProxyBodyError::InvalidForm)
}

fn resolve_form(template: &str, vault: &Value) -> Result<String, ProxyBodyError> {
    template
        .split('&')
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            let decoded_key = decode_form(key)?;
            if has_reference(&decoded_key)? {
                return Err(ProxyBodyError::InvalidForm);
            }
            let decoded = decode_form(value)?;
            let resolved = substitute(&decoded, vault, str::to_owned)?;
            if resolved == decoded {
                return Ok(part.to_owned());
            }
            let encoded: String =
                url::form_urlencoded::byte_serialize(resolved.as_bytes()).collect();
            Ok(format!("{key}={encoded}"))
        })
        .collect::<Result<Vec<_>, ProxyBodyError>>()
        .map(|fields| fields.join("&"))
}

fn quoted_metadata(value: &str) -> Result<String, ProxyBodyError> {
    if value.chars().any(|c| c.is_control()) {
        return Err(ProxyBodyError::InvalidMultipart);
    }
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn prepare_multipart(
    parts: &[ProxyMultipartPart],
    vault: &Value,
    content_type: Option<&str>,
    parsed_content_type: Option<&mime::Mime>,
) -> Result<PreparedProxyBody, ProxyBodyError> {
    let mut boundaries = parsed_content_type
        .into_iter()
        .flat_map(mime::Mime::params)
        .filter(|(name, _)| *name == mime::BOUNDARY)
        .map(|(_, value)| value.as_str().to_owned());
    let specified = boundaries.next();
    if boundaries.next().is_some() {
        return Err(ProxyBodyError::InvalidMultipart);
    }
    let boundary = specified
        .clone()
        .unwrap_or_else(|| format!("hs-proxy-{}", crate::generate_uuid_v4()));
    if boundary.is_empty()
        || boundary.len() > 70
        || boundary.ends_with(' ')
        || !boundary
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"'()+_,-./:=? ".contains(&b))
    {
        return Err(ProxyBodyError::InvalidMultipart);
    }
    let mut bytes = Vec::new();
    for part in parts {
        let (headers, data) = match part {
            ProxyMultipartPart::Text { name, value } => (
                format!(
                    "Content-Disposition: form-data; name=\"{}\"\r\n\r\n",
                    quoted_metadata(name)?
                ),
                substitute(value, vault, str::to_owned)?.into_bytes(),
            ),
            ProxyMultipartPart::File {
                name,
                filename,
                mime_type,
                data_base64,
            } => {
                if mime_type.parse::<mime::Mime>().is_err() {
                    return Err(ProxyBodyError::InvalidMultipart);
                }
                (
                    format!("Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n", quoted_metadata(name)?, quoted_metadata(filename)?, mime_type),
                    STANDARD.decode(data_base64).map_err(|_| ProxyBodyError::InvalidBase64)?,
                )
            }
        };
        if data
            .windows(boundary.len())
            .any(|window| window == boundary.as_bytes())
        {
            return Err(ProxyBodyError::InvalidMultipart);
        }
        bytes.extend_from_slice(format!("--{boundary}\r\n{headers}").as_bytes());
        bytes.extend_from_slice(&data);
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    let content_type = if specified.is_some() {
        content_type.map(str::to_owned)
    } else {
        Some(format!(
            "{}; boundary={boundary}",
            content_type.unwrap_or("multipart/form-data")
        ))
    };
    Ok(PreparedProxyBody {
        bytes: Some(bytes),
        content_type,
    })
}
