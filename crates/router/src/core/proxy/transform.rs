//! Instructions the vault proxy runs on a JSON body after vault data is substituted and before
//! it is sent: encrypt fields, serialize once, then sign the bytes.
//!
//! Each step implements [`Step`] and runs one algorithm through its operation's trait
//! ([`encrypt::Encryption`], [`serialize::Serialization`], [`sign::Signing`]). The algorithms are
//! enums, so a new one is a new variant and its match arm.

use std::{collections::HashMap, str::FromStr};

use api_models::proxy::{Encoding, ProxyTransformation, TransformInstruction};
use base64::Engine;
use common_utils::{consts::BASE64_ENGINE, fp_utils::when};
use hyperswitch_masking::Maskable;
use reqwest::header::HeaderName;
use serde_json::Value;

use super::body::{ProxyBody, BODY_HEADERS};

mod encrypt;
mod serialize;
mod sign;

/// Errors omit keys, field values and vault data.
#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    /// Transformation applies to JSON bodies only.
    #[error("Transformation is supported for JSON bodies only")]
    UnsupportedBody,
    /// The steps are not encrypt steps, then one serialize step, then sign steps.
    #[error(
        "Transformation steps must be encrypt steps, then one serialize step, then sign steps"
    )]
    InvalidOrder,
    /// An encrypt field is missing or not a string.
    #[error("Encrypt field is missing or not a string")]
    InvalidField,
    /// A key cannot be decoded or parsed.
    #[error("Invalid transformation key")]
    InvalidKey,
    /// A signed message references a header that is not in the request.
    #[error("Signed message references a header that is not in the request")]
    MissingHeader,
    /// A signed message is empty, or references a header whose value the proxy does not send.
    #[error("Invalid signed message")]
    InvalidMessage,
    /// A signature header name is not a valid HTTP header name.
    #[error("Invalid signature header name")]
    InvalidHeader,
    /// A header is set more than once, or a signature header is already set.
    #[error("Conflicting transformation header")]
    ConflictingHeader,
    /// An algorithm failed.
    #[error("Transformation failed")]
    Failed,
}

/// The body as it moves through the steps: JSON until serialized, then bytes.
#[derive(Debug)]
enum Body {
    Json(Value),
    Bytes(Vec<u8>),
}

/// What each step receives and returns: the body and the headers added so far.
#[derive(Debug)]
struct Payload {
    body: Body,
    headers: Vec<(String, Maskable<String>)>,
}

/// One executable transformation step.
trait Step: std::fmt::Debug + Send + Sync {
    /// Apply this step to the payload.
    fn execute(&self, payload: Payload) -> Result<Payload, TransformError>;
}

/// The transformed body bytes and the headers the steps added.
#[derive(Debug)]
pub struct TransformOutput {
    pub(super) body: Vec<u8>,
    pub(super) headers: Vec<(String, Maskable<String>)>,
}

/// Steps whose instructions passed every check that needs no vault data. It is only built by
/// [`ValidatedPlan::new`], so an unchecked plan never runs.
#[derive(Debug)]
pub struct ValidatedPlan(Vec<Box<dyn Step>>);

impl ValidatedPlan {
    /// Check the instructions against the body template and the request headers. Runs before
    /// the vault is read, as reading it consumes the single-use CVC.
    pub fn new(
        transformation: &ProxyTransformation,
        body: &ProxyBody,
        headers: &HashMap<String, String>,
    ) -> Result<Self, TransformError> {
        let template = match body {
            ProxyBody::Json(template) => Ok(template),
            ProxyBody::Xml(_) => Err(TransformError::UnsupportedBody),
        }?;
        validate_order(&transformation.steps)?;
        validate_signature_headers(&transformation.steps, headers)?;
        transformation
            .steps
            .iter()
            .map(|instruction| step(instruction, template, headers))
            .collect::<Result<_, _>>()
            .map(Self)
    }

    /// Run the steps on the substituted JSON body.
    pub fn execute(&self, body: Value) -> Result<TransformOutput, TransformError> {
        let payload = Payload {
            body: Body::Json(body),
            headers: Vec::new(),
        };
        let payload = self
            .0
            .iter()
            .try_fold(payload, |payload, step| step.execute(payload))?;
        match payload.body {
            Body::Bytes(body) => Ok(TransformOutput {
                body,
                headers: payload.headers,
            }),
            Body::Json(_) => Err(TransformError::InvalidOrder),
        }
    }
}

/// The executable step for an instruction.
fn step(
    instruction: &TransformInstruction,
    template: &Value,
    headers: &HashMap<String, String>,
) -> Result<Box<dyn Step>, TransformError> {
    match instruction {
        TransformInstruction::Encrypt {
            field,
            scheme,
            key,
            encoding,
        } => encrypt::EncryptStep::new(field, *scheme, key, *encoding, template)
            .map(|step| -> Box<dyn Step> { Box::new(step) }),
        TransformInstruction::Serialize { format } => {
            Ok(Box::new(serialize::SerializeStep::new(*format)))
        }
        TransformInstruction::Sign {
            message,
            scheme,
            key,
            encoding,
            header,
        } => sign::SignStep::new(message, *scheme, key, *encoding, header, headers)
            .map(|step| -> Box<dyn Step> { Box::new(step) }),
    }
}

/// Encrypt steps change the JSON body, so they come before the one serialize step, and nothing
/// may change the bytes once they can be signed.
fn validate_order(steps: &[TransformInstruction]) -> Result<(), TransformError> {
    let phase = |instruction: &TransformInstruction| match instruction {
        TransformInstruction::Encrypt { .. } => 0,
        TransformInstruction::Serialize { .. } => 1,
        TransformInstruction::Sign { .. } => 2,
    };
    let ordered = steps
        .iter()
        .zip(steps.iter().skip(1))
        .all(|(current, next)| phase(current) <= phase(next));
    let serializations = steps
        .iter()
        .filter(|step| matches!(step, TransformInstruction::Serialize { .. }))
        .count();
    when(!ordered || serializations != 1, || {
        Err(TransformError::InvalidOrder)
    })
}

/// A signature header must be a valid name, must not already be in the request or be set by the
/// proxy itself, and must not be written by two steps.
fn validate_signature_headers(
    steps: &[TransformInstruction],
    headers: &HashMap<String, String>,
) -> Result<(), TransformError> {
    let targets: Vec<&str> = steps
        .iter()
        .filter_map(|step| match step {
            TransformInstruction::Sign { header, .. } => Some(header.as_str()),
            TransformInstruction::Encrypt { .. } | TransformInstruction::Serialize { .. } => None,
        })
        .collect();
    targets
        .iter()
        .try_for_each(|target| HeaderName::from_str(target).map(|_| ()))
        .map_err(|_| TransformError::InvalidHeader)?;
    let conflict = targets.iter().enumerate().any(|(index, target)| {
        headers
            .keys()
            .map(String::as_str)
            .chain(BODY_HEADERS)
            .chain(targets.iter().skip(index + 1).copied())
            .any(|name| name.eq_ignore_ascii_case(target))
    });
    when(conflict, || Err(TransformError::ConflictingHeader))
}

/// The value of a request header. Names are case-insensitive, and duplicates with different
/// values are rejected instead of picking one.
fn header_value<'a>(
    headers: &'a HashMap<String, String>,
    name: &str,
) -> Result<&'a str, TransformError> {
    let mut values = headers
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str());
    let first = values.next().ok_or(TransformError::MissingHeader)?;
    when(values.any(|value| value != first), || {
        Err(TransformError::ConflictingHeader)
    })?;
    Ok(first)
}

/// Encode bytes as text.
fn encode(encoding: Encoding, bytes: &[u8]) -> String {
    match encoding {
        Encoding::Base64 => BASE64_ENGINE.encode(bytes),
    }
}
