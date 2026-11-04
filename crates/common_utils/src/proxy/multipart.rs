//! Multipart form bodies: text fields are resolved, file bytes pass through unchanged,
//! and the boundary is checked against every part.

use super::{
    decode_base64,
    placeholder::{substitute, Vault},
    ConnectorContentType, ProxyBodyError, ProxyBodyFormat, ProxyMultipartPart, RenderedBody,
};
use crate::fp_utils::when;

pub(super) struct MultipartBody<'a>(pub(super) &'a [ProxyMultipartPart]);

impl ProxyBodyFormat for MultipartBody<'_> {
    fn name(&self) -> &'static str {
        "multipart"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        Some("multipart/form-data")
    }

    fn accepts(&self, content_type: &ConnectorContentType<'_>) -> bool {
        content_type.is_multipart_form_data()
    }

    fn render(
        &self,
        vault: Vault<'_>,
        content_type: Option<&ConnectorContentType<'_>>,
    ) -> Result<RenderedBody, ProxyBodyError> {
        render_multipart(self.0, vault, content_type)
    }
}

impl ProxyMultipartPart {
    /// The part's header block, ending with the blank line before its data, and the
    /// data itself. Text values may reference vault data; metadata never does, and
    /// file bytes pass through unchanged.
    fn render(&self, vault: Vault<'_>) -> Result<(String, Vec<u8>), ProxyBodyError> {
        match self {
            Self::Text { name, value } => Ok((
                format!(
                    "Content-Disposition: form-data; name=\"{}\"\r\n\r\n",
                    quoted_metadata(name)?
                ),
                substitute(value, vault, str::to_owned)?.into_bytes(),
            )),
            Self::File {
                name,
                filename,
                mime_type,
                data_base64,
            } => {
                when(mime_type.parse::<mime::Mime>().is_err(), || {
                    Err(ProxyBodyError::InvalidMultipart)
                })?;
                Ok((
                    format!(
                        "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
                        quoted_metadata(name)?,
                        quoted_metadata(filename)?,
                        mime_type
                    ),
                    decode_base64(data_base64)?,
                ))
            }
        }
    }
}

fn quoted_metadata(value: &str) -> Result<String, ProxyBodyError> {
    when(value.chars().any(|c| c.is_control()), || {
        Err(ProxyBodyError::InvalidMultipart)
    })?;
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn render_multipart(
    parts: &[ProxyMultipartPart],
    vault: Vault<'_>,
    declared: Option<&ConnectorContentType<'_>>,
) -> Result<RenderedBody, ProxyBodyError> {
    let content_type = declared.map(|declared| declared.header);
    let mut boundaries = declared
        .and_then(|declared| declared.mime.as_ref())
        .into_iter()
        .flat_map(mime::Mime::params)
        .filter(|(name, _)| *name == mime::BOUNDARY)
        .map(|(_, value)| value.as_str().to_owned());
    let invalid = || Err(ProxyBodyError::InvalidMultipart);
    let specified = boundaries.next();
    when(boundaries.next().is_some(), invalid)?;
    let boundary = specified
        .clone()
        .unwrap_or_else(|| format!("hs-proxy-{}", crate::generate_uuid_v4()));
    when(
        boundary.is_empty()
            || boundary.len() > 70
            || boundary.ends_with(' ')
            || !boundary
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"'()+_,-./:=? ".contains(&b)),
        invalid,
    )?;
    let mut bytes = Vec::new();
    for part in parts {
        let (headers, data) = part.render(vault)?;
        when(
            data.windows(boundary.len())
                .any(|window| window == boundary.as_bytes()),
            invalid,
        )?;
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
    Ok(RenderedBody {
        bytes: Some(bytes),
        content_type,
    })
}
