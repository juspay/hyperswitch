//! Multipart form bodies: text fields are resolved, file bytes pass through unchanged,
//! and the boundary is checked against every part.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;

use super::{
    placeholder::substitute, CheckedContentType, PreparedProxyBody, ProxyBodyError,
    ProxyBodyFormat, ProxyMultipartPart,
};

pub(super) struct MultipartBody<'a>(pub(super) &'a [ProxyMultipartPart]);

impl ProxyBodyFormat for MultipartBody<'_> {
    fn name(&self) -> &'static str {
        "multipart"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        Some("multipart/form-data")
    }

    fn accepts(&self, media_type: &str) -> bool {
        media_type == "multipart/form-data"
    }

    fn render(
        &self,
        vault: &Value,
        content_type: &CheckedContentType<'_>,
    ) -> Result<PreparedProxyBody, ProxyBodyError> {
        prepare_multipart(self.0, vault, content_type)
    }
}

/// Rendering of one multipart part. Each [`ProxyMultipartPart`] variant has its own
/// implementation; part metadata is never interpolated.
pub(super) trait MultipartPartFormat {
    /// Stable label used in `Debug` output.
    fn kind(&self) -> &'static str;

    /// The part's header block, ending with the blank line before its data.
    fn headers(&self) -> Result<String, ProxyBodyError>;

    /// The part's data, with vault placeholders resolved where allowed.
    fn data(&self, vault: &Value) -> Result<Vec<u8>, ProxyBodyError>;
}

impl ProxyMultipartPart {
    /// The only place that maps a part variant to its implementation.
    pub(super) fn format(&self) -> Box<dyn MultipartPartFormat + '_> {
        match self {
            Self::Text { name, value } => Box::new(TextPart { name, value }),
            Self::File {
                name,
                filename,
                mime_type,
                data_base64,
            } => Box::new(FilePart {
                name,
                filename,
                mime_type,
                data_base64,
            }),
        }
    }
}

/// A text field whose value may reference vaulted data.
struct TextPart<'a> {
    name: &'a str,
    value: &'a str,
}

impl MultipartPartFormat for TextPart<'_> {
    fn kind(&self) -> &'static str {
        "text"
    }

    fn headers(&self) -> Result<String, ProxyBodyError> {
        Ok(format!(
            "Content-Disposition: form-data; name=\"{}\"\r\n\r\n",
            quoted_metadata(self.name)?
        ))
    }

    fn data(&self, vault: &Value) -> Result<Vec<u8>, ProxyBodyError> {
        Ok(substitute(self.value, vault, str::to_owned)?.into_bytes())
    }
}

/// A binary file whose bytes must survive the proxy unchanged.
struct FilePart<'a> {
    name: &'a str,
    filename: &'a str,
    mime_type: &'a str,
    data_base64: &'a str,
}

impl MultipartPartFormat for FilePart<'_> {
    fn kind(&self) -> &'static str {
        "file"
    }

    fn headers(&self) -> Result<String, ProxyBodyError> {
        if self.mime_type.parse::<mime::Mime>().is_err() {
            return Err(ProxyBodyError::InvalidMultipart);
        }
        Ok(format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
            quoted_metadata(self.name)?,
            quoted_metadata(self.filename)?,
            self.mime_type
        ))
    }

    fn data(&self, _vault: &Value) -> Result<Vec<u8>, ProxyBodyError> {
        STANDARD
            .decode(self.data_base64)
            .map_err(|_| ProxyBodyError::InvalidBase64)
    }
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
    checked: &CheckedContentType<'_>,
) -> Result<PreparedProxyBody, ProxyBodyError> {
    let content_type = checked.header;
    let mut boundaries = checked
        .parsed
        .iter()
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
        let part = part.format();
        let headers = part.headers()?;
        let data = part.data(vault)?;
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
